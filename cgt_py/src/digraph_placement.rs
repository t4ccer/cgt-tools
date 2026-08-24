use crate::{graph::PyGraph, py_partizan_game};
use cgt::{
    drawing::Canvas,
    graph::{
        adjacency_matrix::directed::DirectedGraph,
        layout::{Bounds, CircleEdge, SpringEmbedder},
    },
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
> = LazyLock::new(|| ParallelTranspositionTable::new());

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
        // TODO: Maybe we should have 2 graph types where position does not matter
        let mut graph = self.0.graph.map(|&color| cgt_py_messages::Vertex {
            color: cgt_py_messages::VertexColor::from(color),
            position: V2f::ZERO,
        });
        crate::graph::layout_circle(&mut graph);
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

        let digraph_placement = DigraphPlacement::new(graph);
        let bounding_box = digraph_placement.required_canvas::<svg::Canvas>();
        let mut canvas = svg::Canvas::new(bounding_box);
        digraph_placement.draw(&mut canvas);
        canvas.to_svg()
    }
}

py_partizan_game!(PyDigraphPlacement);
