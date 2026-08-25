use crate::{graph::PyGraph, py_partizan_game};
use cgt::{
    graph::adjacency_matrix::undirected::UndirectedGraph,
    impl_has,
    numeric::v2f::V2f,
    short::partizan::{
        games::snort::{Snort, VertexKind},
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
    kind: VertexKind,
    position: V2f,
}

impl_has!(PositionedVertex -> kind -> VertexKind);
impl_has!(PositionedVertex -> position -> V2f);

static TRANSPOSITION_TABLE: LazyLock<
    ParallelTranspositionTable<Snort<VertexKind, UndirectedGraph<VertexKind>>>,
> = LazyLock::new(ParallelTranspositionTable::new);

#[pyclass(name = "Snort")]
pub struct PySnort(pub Snort<VertexKind, UndirectedGraph<VertexKind>>);

#[pymethods]
impl PySnort {
    #[new]
    #[pyo3(signature = (graph, edges = None, white = None, blue = None, red = None, green = None))]
    pub fn new(
        graph: &Bound<'_, PyAny>,
        edges: Option<Vec<(u32, u32)>>,
        white: Option<Vec<u32>>,
        blue: Option<Vec<u32>>,
        red: Option<Vec<u32>>,
        green: Option<Vec<u32>>,
    ) -> PyResult<PySnort> {
        if let Ok(graph) = graph.cast::<PyGraph>() {
            if edges.is_some()
                || white.is_some()
                || blue.is_some()
                || red.is_some()
                || green.is_some()
            {
                return Err(PyTypeError::new_err(
                    "Snort() takes no other arguments when constructed from a Graph",
                ));
            }
            return graph.borrow().snort();
        }

        let vertices = graph.extract::<u32>()?;
        let Some(edges) = edges else {
            return Err(PyTypeError::new_err("Snort() missing argument: 'edges'"));
        };
        PyGraph::new(vertices, edges, false, white, blue, red, green)?.snort()
    }

    #[getter]
    pub fn graph(&self) -> PyGraph {
        let mut graph = self.0.graph.as_directed().map(|v| cgt_py_messages::Vertex {
            color: cgt_py_messages::VertexColor::from(v.color()),
            position: V2f::ZERO,
        });
        crate::graph::layout_for_svg(&mut graph);
        PyGraph::from_preset_unchecked(GraphPreset::Snort, graph)
    }

    fn __repr__(&self) -> String {
        let graph = self.graph();
        format!(
            "Snort({}, {:?}, blue={:?}, red={:?})",
            graph.vertices(),
            graph.edges(),
            &graph.colors().blue,
            &graph.colors().red,
        )
    }

    fn _repr_svg_(&self) -> String {
        let mut graph = self.0.graph.map(|&kind| PositionedVertex {
            kind,
            position: V2f::ZERO,
        });
        crate::graph::layout_for_svg(&mut graph);
        crate::graph::draw_svg(&Snort::new(graph))
    }
}

py_partizan_game!(PySnort);
