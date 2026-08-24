use crate::{graph::PyGraph, py_partizan_game};
use cgt::{
    drawing::Canvas,
    graph::{
        Graph,
        adjacency_matrix::undirected::UndirectedGraph,
        layout::{Bounds, CircleEdge, SpringEmbedder},
    },
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
> = LazyLock::new(|| ParallelTranspositionTable::new());

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
        PyGraph::new(vertices, edges, white, blue, red, green)?.col()
    }

    #[getter]
    fn graph(&self) -> PyGraph {
        // TODO: Maybe we should have 2 graph types where position does not matter
        let mut graph = self.0.graph.map(|&color| cgt_py_messages::Vertex {
            color: cgt_py_messages::VertexColor::from(color),
            position: V2f::ZERO,
        });
        crate::graph::layout_circle(&mut graph);
        PyGraph {
            known_preset: Some(GraphPreset::Col),
            graph,
        }
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
        use cgt::drawing::{Draw, svg};

        let mut graph = self.0.graph.map(|&color| PositionedVertex {
            color,
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

        let col = Col::new(graph);
        let bounding_box = col.required_canvas::<svg::Canvas>();
        let mut canvas = svg::Canvas::new(bounding_box);
        col.draw(&mut canvas);
        canvas.to_svg()
    }
}

py_partizan_game!(PyCol);
