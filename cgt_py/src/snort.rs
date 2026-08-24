use crate::{graph::PyGraph, py_partizan_game};
use cgt::{
    drawing::Canvas,
    graph::{
        adjacency_matrix::undirected::UndirectedGraph,
        layout::{Bounds, CircleEdge, SpringEmbedder},
    },
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
> = LazyLock::new(|| ParallelTranspositionTable::new());

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
    fn graph(&self) -> PyGraph {
        // TODO: Maybe we should have 2 graph types where position does not matter
        let mut graph = self.0.graph.as_directed().map(|v| cgt_py_messages::Vertex {
            color: cgt_py_messages::VertexColor::from(v.color()),
            position: V2f::ZERO,
        });
        crate::graph::layout_circle(&mut graph);
        PyGraph::from_preset(GraphPreset::Snort, graph)
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
        use cgt::drawing::{Draw, svg};

        let mut graph = self.0.graph.map(|&kind| PositionedVertex {
            kind,
            position: V2f::ZERO,
        });

        // Very arbitrary

        let circle = CircleEdge {
            circle_radius: 128.0,
            vertex_radius: svg::Canvas::vertex_radius(),
            center: V2f { x: 128.0, y: 128.0 },
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
