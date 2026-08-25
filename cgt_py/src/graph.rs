use cgt::{
    drawing::{Draw, svg},
    graph::{
        Graph, VertexIndex,
        adjacency_matrix::{directed::DirectedGraph, undirected::UndirectedGraph},
    },
    has::Has,
    numeric::v2f::V2f,
    short::partizan::games::{
        bipartite_snort::{self, BipartiteSnort},
        col::{self, Col},
        digraph_placement::{self, DigraphPlacement},
        snort::{self, Snort},
    },
};
use cgt_py_messages::{
    GraphBackendMessage, GraphFrontendMessage, GraphPreset, Sequence, Vertex, VertexColor,
    layout::arrange,
};
use jupyter_rust_widget_backend::{Response, RustWidget};
use pyo3::{
    Bound, IntoPyObjectExt, Py, PyAny, PyResult, Python, exceptions::PyValueError, pyclass,
    pyfunction, pymethods,
};

use crate::{
    bipartite_snort::PyBipartiteSnort, col::PyCol, digraph_placement::PyDigraphPlacement,
    snort::PySnort,
};

#[derive(Debug)]
#[pyclass(name = "VertexColors")]
pub struct PyVertexColors {
    #[pyo3(get)]
    pub(crate) white: Vec<u32>,

    #[pyo3(get)]
    pub(crate) blue: Vec<u32>,

    #[pyo3(get)]
    pub(crate) red: Vec<u32>,

    #[pyo3(get)]
    pub(crate) green: Vec<u32>,
}

impl PyVertexColors {
    fn new() -> PyVertexColors {
        PyVertexColors {
            white: Vec::new(),
            blue: Vec::new(),
            red: Vec::new(),
            green: Vec::new(),
        }
    }
}

#[pymethods]
impl PyVertexColors {
    fn __repr__(&self) -> String {
        format!(
            "{{white={:?}, blue={:?}, red={:?}, green={:?}}}",
            &self.white, &self.blue, &self.red, &self.green,
        )
    }
}

#[pyclass(name = "Graph")]
pub struct PyGraph {
    pub known_preset: Option<GraphPreset>,

    /// Whether the edges of the graph point one way. Undirected edges are stored as a pair
    /// of opposite arcs, so this only decides how edges are added and reported back
    #[pyo3(get)]
    pub directed: bool,

    pub graph: DirectedGraph<Vertex>,
}

pub const SVG_CANVAS_SIZE: V2f = V2f { x: 400.0, y: 250.0 };

pub fn layout_for_svg<G, V>(graph: &mut G)
where
    G: Graph<V>,
    V: Has<V2f>,
{
    arrange::<svg::Canvas, _, _>(graph, SVG_CANVAS_SIZE);
}

/// Draw a game onto a fresh svg canvas cut to fit it
pub fn draw_svg<D>(game: &D) -> String
where
    D: Draw,
{
    let bounding_box = game.required_canvas::<svg::Canvas>();
    let mut canvas = svg::Canvas::new(bounding_box);
    game.draw(&mut canvas);
    canvas.to_svg()
}

impl PyGraph {
    /// Graph of a game whose position the widget of a given preset holds
    pub fn from_preset_unchecked(preset: GraphPreset, graph: DirectedGraph<Vertex>) -> PyGraph {
        PyGraph {
            known_preset: Some(preset),
            directed: preset.directed_edges(),
            graph,
        }
    }

    pub fn from_preset(preset: GraphPreset, graph: DirectedGraph<Vertex>) -> PyResult<PyGraph> {
        let graph = Self::from_preset_unchecked(preset, graph);
        graph.is_valid_for(preset)?;
        Ok(graph)
    }

    fn try_into_graph<T>(&self) -> PyResult<DirectedGraph<T>>
    where
        T: TryFrom<VertexColor>,
        T::Error: std::error::Error,
    {
        self.graph
            .try_map(|&v| T::try_from(v.color))
            .map_err(|err| PyValueError::new_err(err.to_string()))
    }

    fn try_into_undirected_graph<T>(&self) -> PyResult<UndirectedGraph<T>>
    where
        T: TryFrom<VertexColor> + Clone,
        T::Error: std::error::Error,
    {
        Ok(UndirectedGraph::from_directed(&self.try_into_graph::<T>()?))
    }

    fn is_valid_for(&self, preset: GraphPreset) -> PyResult<()> {
        match preset {
            GraphPreset::Snort => self.snort().map(drop),
            GraphPreset::Col => self.col().map(drop),
            GraphPreset::DigraphPlacement => self.digraph_placement().map(drop),
            GraphPreset::BipartiteSnort => self.bipartite_snort().map(drop),
        }
    }
}

#[pymethods]
impl PyGraph {
    #[new]
    #[pyo3(signature = (vertices, edges, *, directed = false, white = None, blue = None, red = None, green = None))]
    pub fn new(
        vertices: u32,
        edges: Vec<(u32, u32)>,
        directed: bool,
        white: Option<Vec<u32>>,
        blue: Option<Vec<u32>>,
        red: Option<Vec<u32>>,
        green: Option<Vec<u32>>,
    ) -> PyResult<PyGraph> {
        let mut colors = vec![VertexColor::White; vertices as usize];
        for (color, indices) in [
            (VertexColor::White, white),
            (VertexColor::Blue, blue),
            (VertexColor::Red, red),
            (VertexColor::Green, green),
        ] {
            for v_idx in indices.into_iter().flatten() {
                match colors.get_mut(v_idx as usize) {
                    Some(vertex_color) => *vertex_color = color,
                    None => {
                        return Err(PyValueError::new_err(format!(
                            "Invalid {color} vertex: {v_idx}",
                        )));
                    }
                }
            }
        }
        let vertices = colors
            .into_iter()
            .map(|color| Vertex {
                color,
                position: V2f::ZERO,
            })
            .collect::<Vec<_>>();

        let mut graph = DirectedGraph::empty(&vertices);
        for (u, v) in edges {
            if u as usize >= vertices.len() || v as usize >= vertices.len() {
                return Err(PyValueError::new_err(format!("Invalid edge: ({u}, {v})")));
            }
            let u = VertexIndex { index: u as usize };
            let v = VertexIndex { index: v as usize };
            graph.connect(u, v, true);
            if !directed {
                graph.connect(v, u, true);
            }
        }

        Ok(PyGraph {
            known_preset: None,
            directed,
            graph,
        })
    }

    #[getter]
    pub(crate) fn vertices(&self) -> u32 {
        self.graph.size() as u32
    }

    #[getter]
    pub(crate) fn edges(&self) -> Vec<(u32, u32)> {
        self.graph
            .edges()
            // An undirected edge is a pair of opposite arcs, report it only once
            .filter(|&(u, v)| self.directed || u <= v || !self.graph.are_adjacent(v, u))
            .map(|(u, v)| (u.index as u32, v.index as u32))
            .collect()
    }

    #[getter]
    pub(crate) fn colors(&self) -> PyVertexColors {
        let mut colors = PyVertexColors::new();
        for v_idx in self.graph.vertex_indices() {
            match self.graph.get_vertex(v_idx).color {
                VertexColor::White => colors.white.push(v_idx.index as u32),
                VertexColor::Blue => colors.blue.push(v_idx.index as u32),
                VertexColor::Red => colors.red.push(v_idx.index as u32),
                VertexColor::Green => colors.green.push(v_idx.index as u32),
            }
        }
        colors
    }

    fn __repr__(&self) -> String {
        let colors = self.colors();
        format!(
            "Graph({}, {:?}, directed={}, white={:?}, blue={:?}, red={:?}, green={:?})",
            self.vertices(),
            self.edges(),
            if self.directed { "True" } else { "False" },
            &colors.white,
            &colors.blue,
            &colors.red,
            &colors.green,
        )
    }

    fn _repr_svg_(&self) -> String {
        use cgt::drawing::{Canvas, svg};

        let mut positioned_graph = self.graph.clone();
        layout_for_svg(&mut positioned_graph);

        let bounding_box = Graph::required_canvas::<svg::Canvas>(&positioned_graph);
        let mut canvas = svg::Canvas::new(bounding_box);
        positioned_graph.draw(&mut canvas, |canvas, vertex| {
            let position: V2f = *positioned_graph.get_vertex(vertex).get_inner();
            let color: VertexColor = *positioned_graph.get_vertex(vertex).get_inner();
            canvas.vertex(position, color.color(), vertex)
        });
        canvas.to_svg()
    }

    #[getter]
    fn game(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        match self.known_preset {
            Some(preset) => match preset {
                GraphPreset::Snort => self.snort()?.into_py_any(py),
                GraphPreset::Col => self.col()?.into_py_any(py),
                GraphPreset::DigraphPlacement => self.digraph_placement()?.into_py_any(py),
                GraphPreset::BipartiteSnort => self.bipartite_snort()?.into_py_any(py),
            },
            None => Err(PyValueError::new_err(
                "This graph is not associated with any game",
            )),
        }
    }

    #[getter]
    pub fn snort(&self) -> PyResult<PySnort> {
        let graph = self
            .try_into_undirected_graph::<snort::VertexColor>()?
            .map(|&color| snort::VertexKind::Single(color));
        Ok(PySnort(Snort::new(graph)))
    }

    #[getter]
    pub fn col(&self) -> PyResult<PyCol> {
        Ok(PyCol(Col::new(
            self.try_into_undirected_graph::<col::VertexColor>()?,
        )))
    }

    #[getter]
    pub fn bipartite_snort(&self) -> PyResult<PyBipartiteSnort> {
        let colored = self.try_into_undirected_graph::<bipartite_snort::VertexColor>()?;

        let mut order = Vec::with_capacity(colored.size());
        for partition in [
            bipartite_snort::VertexColor::TintLeft,
            bipartite_snort::VertexColor::TintRight,
        ] {
            order.extend(
                colored
                    .vertex_indices()
                    .filter(|&v| *colored.get_vertex(v) == partition),
            );
        }

        let vertices = order
            .iter()
            .map(|&v| *colored.get_vertex(v))
            .collect::<Vec<_>>();
        let mut graph = UndirectedGraph::empty(&vertices);
        for (u, &old_u) in order.iter().enumerate() {
            for (v, &old_v) in order.iter().enumerate().skip(u + 1) {
                if !colored.are_adjacent(old_u, old_v) {
                    continue;
                }

                if vertices[u] == vertices[v] {
                    return Err(PyValueError::new_err(format!(
                        "Graph is not bipartite: vertices {} and {} have the same color",
                        old_u.index, old_v.index,
                    )));
                }

                graph.connect(VertexIndex { index: u }, VertexIndex { index: v }, true);
            }
        }

        Ok(PyBipartiteSnort(BipartiteSnort::new(graph)))
    }

    #[getter]
    pub fn digraph_placement(&self) -> PyResult<PyDigraphPlacement> {
        Ok(PyDigraphPlacement(DigraphPlacement::new(
            self.try_into_graph::<digraph_placement::VertexColor>()?,
        )))
    }
}

struct GraphWidget {
    preset: GraphPreset,
    graph: DirectedGraph<Vertex>,
    sequence: Sequence,
}

impl GraphWidget {
    fn set_graph_message(&self) -> GraphFrontendMessage {
        GraphFrontendMessage::SetGraph {
            sequence: self.sequence,
            graph: self.graph.clone(),
        }
    }
}

impl RustWidget for GraphWidget {
    type BackendMessage = GraphBackendMessage;
    type FrontendMessage = GraphFrontendMessage;

    fn esm(&self) -> String {
        let bundle = include_str!("../../cgt_py_widgets/dist/bundle.js");
        let preset = format!("const preset = {};", self.preset.into_flag_bits());
        let epilogue = r#" async function render({model, el}) {
                               await JupyterCGT.render_graph(model, el, preset);
                           }
                           export default { render }"#;
        let mut esm = String::with_capacity(bundle.len() + preset.len() + epilogue.len());
        esm.push_str(bundle);
        esm.push_str(&preset);
        esm.push_str(epilogue);
        esm
    }

    fn handle_message(&mut self, event: Self::BackendMessage) -> Response<Self::FrontendMessage> {
        match event {
            GraphBackendMessage::Initialized => Response {
                message: Some(self.set_graph_message()),
                run_on_update: false,
            },
            GraphBackendMessage::SetGraph { sequence, graph } => {
                let taken = sequence > self.sequence;
                if taken {
                    self.sequence = sequence;
                    self.graph = graph;
                }

                Response {
                    message: Some(self.set_graph_message()),
                    run_on_update: taken,
                }
            }
        }
    }

    fn value<'py>(&mut self) -> impl pyo3::IntoPyObject<'py> {
        PyGraph::from_preset_unchecked(self.preset, self.graph.clone())
    }
}

fn graph_from_position(preset: GraphPreset, position: &Bound<'_, PyAny>) -> PyResult<PyGraph> {
    if let Ok(graph) = position.cast::<PyGraph>() {
        return PyGraph::from_preset(preset, graph.borrow().graph.clone());
    }

    let graph = match preset {
        GraphPreset::Snort => position.cast::<PySnort>()?.borrow().graph(),
        GraphPreset::Col => position.cast::<PyCol>()?.borrow().graph(),
        GraphPreset::DigraphPlacement => position.cast::<PyDigraphPlacement>()?.borrow().graph(),
        GraphPreset::BipartiteSnort => position.cast::<PyBipartiteSnort>()?.borrow().graph(),
    };

    Ok(graph)
}

fn make_graph_widget<'py>(
    py: Python<'py>,
    preset: GraphPreset,
    position: Option<&Bound<'_, PyAny>>,
) -> PyResult<Bound<'py, PyAny>> {
    let graph = match position {
        None => DirectedGraph::empty(&[]),
        Some(position) => graph_from_position(preset, position)?.graph,
    };
    GraphWidget {
        preset,
        graph,
        sequence: Sequence::INITIAL,
    }
    .into_widget(py, "cgt_py")
}

#[pyfunction(name = "SnortWidget")]
#[pyo3(signature = (position = None))]
pub fn make_snort_widget<'py>(
    py: Python<'py>,
    position: Option<&Bound<'_, PyAny>>,
) -> PyResult<Bound<'py, PyAny>> {
    make_graph_widget(py, GraphPreset::Snort, position)
}

#[pyfunction(name = "ColWidget")]
#[pyo3(signature = (position = None))]
pub fn make_col_widget<'py>(
    py: Python<'py>,
    position: Option<&Bound<'_, PyAny>>,
) -> PyResult<Bound<'py, PyAny>> {
    make_graph_widget(py, GraphPreset::Col, position)
}

#[pyfunction(name = "DigraphPlacementWidget")]
#[pyo3(signature = (position = None))]
pub fn make_digraph_placement_widget<'py>(
    py: Python<'py>,
    position: Option<&Bound<'_, PyAny>>,
) -> PyResult<Bound<'py, PyAny>> {
    make_graph_widget(py, GraphPreset::DigraphPlacement, position)
}

#[pyfunction(name = "BipartiteSnortWidget")]
#[pyo3(signature = (position = None))]
pub fn make_bipartite_snort_widget<'py>(
    py: Python<'py>,
    position: Option<&Bound<'_, PyAny>>,
) -> PyResult<Bound<'py, PyAny>> {
    make_graph_widget(py, GraphPreset::BipartiteSnort, position)
}
