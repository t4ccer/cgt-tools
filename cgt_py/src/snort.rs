use crate::py_partizan_game;
use cgt::{
    drawing::Canvas,
    graph::{
        Graph, VertexIndex,
        adjacency_matrix::undirected::UndirectedGraph,
        layout::{Bounds, CircleEdge, SpringEmbedder},
    },
    impl_has,
    numeric::v2f::V2f,
    short::partizan::{
        games::snort::{Snort, VertexColor, VertexKind},
        transposition_table::ParallelTranspositionTable,
    },
};
use pyo3::{PyErr, PyResult, pyclass, pymethods};
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
> = LazyLock::new(|| ParallelTranspositionTable::new());

#[pyclass(name = "Snort")]
pub struct PySnort(pub Snort<VertexKind, UndirectedGraph<VertexKind>>);

#[pymethods]
impl PySnort {
    #[new]
    pub fn new(
        vertices: u32,
        edges: Vec<(u32, u32)>,
        blue: Vec<u32>,
        red: Vec<u32>,
    ) -> PyResult<PySnort> {
        for &(u, v) in &edges {
            if u >= vertices || v >= vertices {
                return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                    "Invalid edge: ({u}, {v})",
                )));
            }
        }
        let edges = edges
            .into_iter()
            .map(|(u, v)| {
                (
                    VertexIndex { index: u as usize },
                    VertexIndex { index: v as usize },
                )
            })
            .collect::<Vec<_>>();

        let mut vertices = vec![VertexKind::Single(VertexColor::Empty); vertices as usize];
        for &blue_idx in &blue {
            match vertices.get_mut(blue_idx as usize) {
                Some(kind) => *kind.color_mut() = VertexColor::TintLeft,
                None => {
                    return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                        "Invalid blue vertex: {blue_idx}",
                    )));
                }
            }
        }
        for &red_idx in &red {
            match vertices.get_mut(red_idx as usize) {
                Some(kind) => *kind.color_mut() = VertexColor::TintRight,
                None => {
                    return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                        "Invalid red vertex: {red_idx}",
                    )));
                }
            }
        }

        Ok(PySnort(Snort::new(UndirectedGraph::from_edges(
            &edges, &vertices,
        ))))
    }

    #[getter]
    fn vertices(&self) -> u32 {
        self.0.graph.size() as u32
    }

    #[getter]
    fn edges(&self) -> Vec<(u32, u32)> {
        self.0
            .graph
            .edges()
            .map(move |(u, v)| (u.index as u32, v.index as u32))
            .collect::<Vec<_>>()
    }

    #[getter]
    fn blue(&self) -> Vec<u32> {
        self.0
            .graph
            .vertex_indices()
            .filter_map(|idx| {
                matches!(self.0.graph.get_vertex(idx).color(), VertexColor::TintLeft)
                    .then_some(idx.index as u32)
            })
            .collect()
    }

    #[getter]
    fn red(&self) -> Vec<u32> {
        self.0
            .graph
            .vertex_indices()
            .filter_map(|idx| {
                matches!(self.0.graph.get_vertex(idx).color(), VertexColor::TintRight)
                    .then_some(idx.index as u32)
            })
            .collect()
    }

    fn __repr__(&self) -> String {
        format!(
            "Snort({}, {:?}, {:?}, {:?})",
            self.vertices(),
            self.edges(),
            self.blue(),
            self.red(),
        )
    }

    fn _repr_svg_(&self) -> String {
        use cgt::drawing::{Draw, svg};

        let mut graph = self.0.graph.map(|&kind| PositionedVertex {
            kind,
            position: V2f::ZERO,
        });

        // Very arbitrary

        let circle = CircleEdge {
            circle_radius: 128.0,
            vertex_radius: svg::Canvas::vertex_radius(),
        };
        circle.layout(&mut graph);

        let spring_embedder = SpringEmbedder {
            cooling_rate: 0.99999,
            c_attractive: 1.0,
            c_repulsive: 250.0,
            ideal_spring_length: 40.0,
            iterations: 1 << 14,
            bounds: Some(Bounds {
                lower: V2f::ZERO,
                upper: V2f { x: 400.0, y: 250.0 },
                c_middle_attractive: Some(0.001),
            }),
        };
        spring_embedder.layout(&mut graph);

        let snort = Snort::new(graph);
        let bounding_box = snort.required_canvas::<svg::Canvas>();
        let mut canvas = svg::Canvas::new(bounding_box);
        snort.draw(&mut canvas);
        canvas.to_svg()
    }
}

py_partizan_game!(PySnort);
