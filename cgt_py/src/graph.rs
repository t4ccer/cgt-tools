use cgt::{
    graph::{Graph, adjacency_matrix::undirected::UndirectedGraph},
    has::Has,
    numeric::v2f::V2f,
    short::partizan::games::snort::{self, Snort},
};
use cgt_py_messages::{
    GraphBackendMessage, GraphFrontendMessage, GraphPreset, VertexColor, WidgetGraph,
};
use jupyter_rust_widget_backend::{Response, RustWidget};
use pyo3::{
    Bound, IntoPyObjectExt, Py, PyAny, PyResult, Python, exceptions::PyValueError, pyclass,
    pyfunction, pymethods,
};

use crate::snort::PySnort;

#[pyclass(name = "Graph")]
pub struct PyGraph {
    known_preset: Option<GraphPreset>,
    graph: WidgetGraph,
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
    #[getter]
    fn vertices(&self) -> u32 {
        self.graph.size() as u32
    }

    #[getter]
    fn edges(&self) -> Vec<(u32, u32)> {
        self.graph
            .edges()
            .map(|(u, v)| (u.index as u32, v.index as u32))
            .collect()
    }

    // TODO: This should be a dict or something
    /// Color of each vertex, in vertex order
    #[getter]
    fn colors(&self) -> Vec<String> {
        self.graph
            .vertices()
            .map(|vertex| {
                let color: VertexColor = *vertex.get_inner();
                color.to_string()
            })
            .collect()
    }

    fn __repr__(&self) -> String {
        format!(
            "Graph({}, {:?}, {:?})",
            self.vertices(),
            self.edges(),
            self.colors()
        )
    }

    fn _repr_svg_(&self) -> String {
        use cgt::drawing::{Canvas, svg};

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
    fn snort(&self) -> PyResult<PySnort> {
        let graph = self
            .try_into_graph::<snort::VertexColor>()?
            .map(|&color| snort::VertexKind::Single(color));
        Ok(PySnort(Snort::new(graph)))
    }

    #[getter]
    fn col(&self) -> PyResult<()> {
        // TODO: fn col once we have PyCol
        todo!()
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
