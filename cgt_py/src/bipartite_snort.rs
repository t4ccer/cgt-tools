use crate::{graph::PyGraph, py_partizan_game};
use cgt::{
    graph::{Graph, VertexIndex, adjacency_matrix::undirected::UndirectedGraph},
    numeric::v2f::V2f,
    short::partizan::{
        games::bipartite_snort::{BipartiteSnort, VertexColor},
        transposition_table::ParallelTranspositionTable,
    },
};
use cgt_py_messages::GraphPreset;
use pyo3::{PyErr, PyResult, pyclass, pymethods};
use std::sync::LazyLock;

static TRANSPOSITION_TABLE: LazyLock<
    ParallelTranspositionTable<BipartiteSnort<VertexColor, UndirectedGraph<VertexColor>>>,
> = LazyLock::new(ParallelTranspositionTable::new);

#[pyclass(name = "BipartiteSnort")]
pub struct PyBipartiteSnort(pub BipartiteSnort<VertexColor, UndirectedGraph<VertexColor>>);

impl PyBipartiteSnort {
    fn edges_iter(&self) -> impl Iterator<Item = (u32, u32)> {
        let blue = self.blue();
        self.0
            .graph
            .edges()
            .map(move |(u, v)| (u.index as u32, v.index as u32 - blue))
    }
}

#[pymethods]
impl PyBipartiteSnort {
    #[new]
    pub fn new(blue: u32, red: u32, edges: Vec<(u32, u32)>) -> PyResult<PyBipartiteSnort> {
        let mut vertices = Vec::with_capacity(blue as usize + red as usize);
        vertices.resize(blue as usize, VertexColor::TintLeft);
        vertices.resize(blue as usize + red as usize, VertexColor::TintRight);
        let mut graph = UndirectedGraph::empty(&vertices);
        for (u, v) in &edges {
            let vert_u = VertexIndex { index: *u as usize };
            let vert_v = VertexIndex {
                index: (blue + v) as usize,
            };
            if vert_u.index >= graph.size() || vert_v.index >= graph.size() {
                return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                    "Invalid edge: ({}, {})",
                    vert_u.index,
                    vert_v.index - blue as usize,
                )));
            }
            graph.connect(vert_u, vert_v, true);
        }
        Ok(PyBipartiteSnort(BipartiteSnort::new(graph)))
    }

    #[getter]
    fn blue(&self) -> u32 {
        self.0
            .graph
            .vertices()
            .filter(|v| matches!(v, VertexColor::TintLeft))
            .count() as u32
    }

    #[getter]
    fn red(&self) -> u32 {
        self.0
            .graph
            .vertices()
            .filter(|v| matches!(v, VertexColor::TintRight))
            .count() as u32
    }

    #[getter]
    fn edges(&self) -> Vec<(u32, u32)> {
        self.edges_iter().collect::<Vec<_>>()
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
        PyGraph::from_preset_unchecked(GraphPreset::BipartiteSnort, graph)
    }

    fn __repr__(&self) -> String {
        let blue = self.blue();
        let red = self.red();
        format!(
            "BipartiteSnort({blue}, {red}, {:?})",
            std::fmt::from_fn(|fmt| fmt
                .debug_list()
                .entries(
                    self.edges_iter()
                        .map(|(u, v)| std::fmt::from_fn(move |fmt| write!(fmt, "({u}, {v})")))
                )
                .finish()),
        )
    }

    fn _repr_svg_(&self) -> String {
        use cgt::drawing::{Draw, svg};
        let bounding_box = self.0.required_canvas::<svg::Canvas>();
        let mut canvas = svg::Canvas::new(bounding_box);
        self.0.draw(&mut canvas);
        canvas.to_svg()
    }
}

py_partizan_game!(PyBipartiteSnort);
