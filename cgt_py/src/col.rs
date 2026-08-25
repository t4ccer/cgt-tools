use crate::{graph::PyGraph, py_partizan_game};
use cgt::{
    graph::adjacency_matrix::undirected::UndirectedGraph,
    impl_has,
    numeric::v2f::V2f,
    short::partizan::{
        games::col::{Col, VertexColor},
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
    ParallelTranspositionTable<Col<VertexColor, UndirectedGraph<VertexColor>>>,
> = LazyLock::new(ParallelTranspositionTable::new);

#[pyclass(name = "Col")]
pub struct PyCol(pub Col<VertexColor, UndirectedGraph<VertexColor>>);

#[pymethods]
impl PyCol {
    #[new]
    #[pyo3(signature = (graph, edges = None, white = None, blue = None, red = None, green = None))]
    pub fn new(
        graph: &Bound<'_, PyAny>,
        edges: Option<Vec<(u32, u32)>>,
        white: Option<Vec<u32>>,
        blue: Option<Vec<u32>>,
        red: Option<Vec<u32>>,
        green: Option<Vec<u32>>,
    ) -> PyResult<PyCol> {
        if let Ok(graph) = graph.cast::<PyGraph>() {
            if edges.is_some()
                || white.is_some()
                || blue.is_some()
                || red.is_some()
                || green.is_some()
            {
                return Err(PyTypeError::new_err(
                    "Col() takes no other arguments when constructed from a Graph",
                ));
            }
            return graph.borrow().col();
        }

        let vertices = graph.extract::<u32>()?;
        let Some(edges) = edges else {
            return Err(PyTypeError::new_err("Col() missing argument: 'edges'"));
        };
        PyGraph::new(vertices, edges, false, white, blue, red, green)?.col()
    }

    #[getter]
    pub fn graph(&self) -> PyGraph {
        let mut graph = self
            .0
            .graph
            .as_directed()
            .map(|&color| cgt_py_messages::Vertex {
                color: cgt_py_messages::VertexColor::from(color),
                position: V2f::ZERO,
            });
        crate::graph::layout_for_svg(&mut graph);
        PyGraph::from_preset_unchecked(GraphPreset::Col, graph)
    }

    fn __repr__(&self) -> String {
        let graph = self.graph();
        format!(
            "Col({}, {:?}, blue={:?}, red={:?})",
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
        crate::graph::draw_svg(&Col::new(graph))
    }
}

py_partizan_game!(PyCol);
