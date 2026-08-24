use cgt::{
    graph::{Graph, adjacency_matrix::undirected::UndirectedGraph},
    has::Has,
    numeric::v2f::V2f,
};
use cgt_py_messages::{GraphBackendMessage, GraphFrontendMessage, VertexColor, WidgetGraph};
use jupyter_rust_widget_backend::{Response, RustWidget};
use pyo3::{Bound, PyAny, PyResult, Python, pyclass, pyfunction, pymethods};

#[pyclass(name = "Graph")]
pub struct PyGraph {
    graph: WidgetGraph,
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
}

struct GraphWidget {
    graph: WidgetGraph,
}

impl RustWidget for GraphWidget {
    type BackendMessage = GraphBackendMessage;
    type FrontendMessage = GraphFrontendMessage;

    fn esm(&self) -> String {
        let bundle = include_str!("../../cgt_py_widgets/dist/bundle.js");
        let epilogue = r#" async function render({model, el}) {
                               await JupyterCGT.render_graph(model, el);
                           }
                           export default { render }"#;
        let mut esm = String::with_capacity(bundle.len() + epilogue.len());
        esm.push_str(bundle);
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
            graph: self.graph.clone(),
        }
    }
}

#[pyfunction(name = "GraphWidget")]
pub fn make_graph_widget(py: Python<'_>) -> PyResult<Bound<'_, PyAny>> {
    GraphWidget {
        graph: UndirectedGraph::empty(&[]),
    }
    .into_widget(py, "cgt_py")
}
