use cgt::{
    graph::{
        Graph, VertexIndex, adjacency_matrix::undirected::UndirectedGraph, layout::CircleEdge,
    },
    has::Has,
    numeric::v2f::V2f,
    short::partizan::games::{
        col::{self, Col},
        snort::{self, Snort},
    },
};
use cgt_py_messages::{
    GraphBackendMessage, GraphFrontendMessage, GraphPreset, Vertex, VertexColor, WidgetGraph,
};
use jupyter_rust_widget_backend::{Response, RustWidget};
use pyo3::{
    Bound, IntoPyObjectExt, Py, PyAny, PyResult, Python, exceptions::PyValueError, pyclass,
    pyfunction, pymethods,
};

use crate::{col::PyCol, snort::PySnort};

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
    pub graph: WidgetGraph,
}

/// Arrange vertices on a circle. Used for graphs that were not laid out by the user, i.e. these
/// constructed from raw vertices and edges
pub fn layout_circle(graph: &mut WidgetGraph) {
    use cgt::drawing::{Canvas, svg};

    CircleEdge {
        circle_radius: 128.0,
        vertex_radius: svg::Canvas::vertex_radius(),
    }
    .layout(graph);
}

impl PyGraph {
    fn try_into_graph<T>(&self) -> PyResult<UndirectedGraph<T>>
    where
        T: TryFrom<VertexColor>,
        T::Error: std::error::Error,
    {
        self.graph
            .try_map(|&v| T::try_from(v.color))
            .map_err(|err| PyValueError::new_err(err.to_string()))
    }
}

#[pymethods]
impl PyGraph {
    #[new]
    #[pyo3(signature = (vertices, edges, white = None, blue = None, red = None, green = None))]
    pub fn new(
        vertices: u32,
        edges: Vec<(u32, u32)>,
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

        let mut graph_edges = Vec::with_capacity(edges.len());
        for (u, v) in edges {
            if u as usize >= vertices.len() || v as usize >= vertices.len() {
                return Err(PyValueError::new_err(format!("Invalid edge: ({u}, {v})")));
            }
            graph_edges.push((
                VertexIndex { index: u as usize },
                VertexIndex { index: v as usize },
            ));
        }

        let mut graph = UndirectedGraph::from_edges(&graph_edges, &vertices);
        layout_circle(&mut graph);
        Ok(PyGraph {
            known_preset: None,
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
            "Graph({}, {:?}, white={:?}, blue={:?}, red={:?}, green={:?})",
            self.vertices(),
            self.edges(),
            &colors.white,
            &colors.blue,
            &colors.red,
            &colors.green,
        )
    }

    fn _repr_svg_(&self) -> String {
        use cgt::drawing::{Canvas, svg};

        // TODO: Detect all V2f::ZERO case and do layout

        let bounding_box = Graph::required_canvas::<svg::Canvas>(&self.graph);
        let mut canvas = svg::Canvas::new(bounding_box);
        self.graph.draw(&mut canvas, |canvas, vertex| {
            let position: V2f = *self.graph.get_vertex(vertex).get_inner();
            let color: VertexColor = *self.graph.get_vertex(vertex).get_inner();
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
            },
            None => Err(PyValueError::new_err(
                "This graph is not associated with any game",
            )),
        }
    }

    #[getter]
    pub fn snort(&self) -> PyResult<PySnort> {
        let graph = self
            .try_into_graph::<snort::VertexColor>()?
            .map(|&color| snort::VertexKind::Single(color));
        Ok(PySnort(Snort::new(graph)))
    }

    #[getter]
    pub fn col(&self) -> PyResult<PyCol> {
        Ok(PyCol(Col::new(self.try_into_graph::<col::VertexColor>()?)))
    }
}

struct GraphWidget {
    preset: GraphPreset,
    graph: WidgetGraph,
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
                message: Some(GraphFrontendMessage::SetGraph(self.graph.clone())),
                run_on_update: false,
            },
            GraphBackendMessage::SetGraph { graph } => {
                self.graph = graph;
                Response {
                    message: Some(GraphFrontendMessage::SetGraph(self.graph.clone())),
                    run_on_update: true,
                }
            }
        }
    }

    fn value<'py>(&mut self) -> impl pyo3::IntoPyObject<'py> {
        PyGraph {
            known_preset: Some(self.preset),
            graph: self.graph.clone(),
        }
    }
}

#[pyfunction(name = "SnortWidget")]
pub fn make_snort_widget(py: Python<'_>) -> PyResult<Bound<'_, PyAny>> {
    GraphWidget {
        preset: GraphPreset::Snort,
        graph: UndirectedGraph::empty(&[]),
    }
    .into_widget(py, "cgt_py")
}

#[pyfunction(name = "ColWidget")]
pub fn make_col_widget(py: Python<'_>) -> PyResult<Bound<'_, PyAny>> {
    GraphWidget {
        preset: GraphPreset::Col,
        graph: UndirectedGraph::empty(&[]),
    }
    .into_widget(py, "cgt_py")
}
