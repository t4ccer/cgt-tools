use crate::{graph::PyGraph, py_partizan_game};
use cgt::{
    graph::adjacency_matrix::directed::DirectedGraph,
    impl_has,
    numeric::v2f::V2f,
    short::partizan::{
        games::digraph_placement::{DigraphPlacement, VertexColor},
        transposition_table::ParallelTranspositionTable,
    },
};
use cgt_py_messages::GraphPreset;
use pyo3::{
    Bound, PyAny, PyResult, exceptions::PyTypeError, pyclass, pymethods, types::PyAnyMethods,
};
use std::sync::LazyLock;

#[derive(Debug, Clone, Copy)]
pub struct PositionedVertex {
    color: VertexColor,
    position: V2f,
}

impl_has!(PositionedVertex -> color -> VertexColor);
impl_has!(PositionedVertex -> position -> V2f);

static TRANSPOSITION_TABLE: LazyLock<
    ParallelTranspositionTable<DigraphPlacement<VertexColor, DirectedGraph<VertexColor>>>,
> = LazyLock::new(ParallelTranspositionTable::new);

#[pyclass(name = "DigraphPlacement")]
pub struct PyDigraphPlacement(pub DigraphPlacement<VertexColor, DirectedGraph<VertexColor>>);

#[pymethods]
impl PyDigraphPlacement {
    #[new]
    #[pyo3(signature = (graph, edges = None, blue = None, red = None))]
    pub fn new(
        graph: &Bound<'_, PyAny>,
        edges: Option<Vec<(u32, u32)>>,
        blue: Option<Vec<u32>>,
        red: Option<Vec<u32>>,
    ) -> PyResult<PyDigraphPlacement> {
        if let Ok(graph) = graph.cast::<PyGraph>() {
            if edges.is_some() || blue.is_some() || red.is_some() {
                return Err(PyTypeError::new_err(
                    "DigraphPlacement() takes no other arguments when constructed from a Graph",
                ));
            }
            return graph.borrow().digraph_placement();
        }

        let vertices = graph.extract::<u32>()?;
        let Some(edges) = edges else {
            return Err(PyTypeError::new_err(
                "DigraphPlacement() missing argument: 'edges'",
            ));
        };
        // Every vertex belongs to one of the players, so vertices left out of both lists stay
        // white and are rejected when the graph is turned into a position
        PyGraph::new(vertices, edges, true, None, blue, red, None)?.digraph_placement()
    }

    #[getter]
    fn graph(&self) -> PyGraph {
        let graph = self.0.graph.map(|&color| cgt_py_messages::Vertex {
            color: cgt_py_messages::VertexColor::from(color),
            position: V2f::ZERO,
        });
        PyGraph::from_preset(GraphPreset::DigraphPlacement, graph)
    }

    fn __repr__(&self) -> String {
        let graph = self.graph();
        format!(
            "DigraphPlacement({}, {:?}, blue={:?}, red={:?})",
            graph.vertices(),
            graph.edges(),
            &graph.colors().blue,
            &graph.colors().red,
        )
    }

    fn _repr_svg_(&self) -> String {
        let mut graph = self.0.graph.map(|&color| PositionedVertex {
            color,
            position: V2f::ZERO,
        });
        crate::graph::layout_for_svg(&mut graph);
        crate::graph::draw_svg(&DigraphPlacement::new(graph))
    }
}

py_partizan_game!(PyDigraphPlacement);
