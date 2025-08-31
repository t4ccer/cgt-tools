//! Snort is played on undirected graph. In each turn Left colors a vertex blue and Right colors
//! a vertex red. Players can only choose a vertex that is adjacent to only empty vertices or to
//! vertices in their own color.

use crate::{
    drawing::{BoundingBox, Canvas, Color, Draw},
    graph::{Graph, VertexIndex},
    has::Has,
    numeric::{dyadic_rational_number::DyadicRationalNumber, nimber::Nimber, v2f::V2f},
    short::partizan::{canonical_form::CanonicalForm, partizan_game::PartizanGame},
};
use std::{collections::VecDeque, fmt::Write, hash::Hash, marker::PhantomData, num::NonZeroU32};

/// Color of Snort vertex. Note that we are taking tinting apporach rather than direct tracking
/// of adjacent colors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(
    feature = "serde",
    derive(serde_repr::Serialize_repr, serde_repr::Deserialize_repr)
)]
#[repr(u8)]
pub enum VertexColor {
    /// Vertex without color, not connected to any taken vertices
    Empty = 0,

    /// Vertex that is adjacent to left
    TintLeft = 1,

    /// Vertex that is adjacent to right
    TintRight = 2,
}

impl TryFrom<u8> for VertexColor {
    type Error = ();

    #[inline]
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Empty),
            1 => Ok(Self::TintLeft),
            2 => Ok(Self::TintRight),
            _ => Err(()),
        }
    }
}

/// Type of vertex (or group of them) in the graph. We abstract over vertices to support efficient
/// calculations of positions with star-like structure
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum VertexKind {
    /// Single graph vertex
    Single(VertexColor),

    /// Cluster of vertices that are not connected to each other, but may be connected to other
    /// vertices in the graph.
    Cluster(VertexColor, NonZeroU32),
}

impl VertexKind {
    #[inline]
    /// Get color of the vertex
    pub const fn color(self) -> VertexColor {
        match self {
            Self::Single(color) | Self::Cluster(color, _) => color,
        }
    }

    #[inline]
    /// Get mutable color of the vertex
    pub const fn color_mut(&mut self) -> &mut VertexColor {
        match self {
            Self::Single(color) | Self::Cluster(color, _) => color,
        }
    }

    #[inline]
    const fn degree_factor(self) -> usize {
        match self {
            VertexKind::Single(_) => 1,
            VertexKind::Cluster(_, cluster_size) => cluster_size.get() as usize,
        }
    }
}

/// Position of a [snort](self) game
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Snort<V, G>
where
    G: Graph<V>,
{
    /// Graph of the game
    pub graph: G,
    _v: PhantomData<V>,
}

impl<G> Snort<VertexKind, G>
where
    G: Graph<VertexKind> + Clone,
{
    /// Construct new position on caterpillar `C(n+1, n, n+1)`
    ///
    /// The caterpillar `C(n+1, n, n+1)` consists of a main path of length 3, whose central vertex
    /// has `n` leaves added, and `n+1` leaves are added to the side vertices.
    pub fn new_three_caterpillar(n: NonZeroU32) -> Self {
        let on_edges = n.checked_add(1).unwrap();
        let in_center = n;

        Self::new(G::from_edges(
            &[
                (VertexIndex { index: 0 }, VertexIndex { index: 1 }),
                (VertexIndex { index: 0 }, VertexIndex { index: 2 }),
                (VertexIndex { index: 0 }, VertexIndex { index: 4 }),
                (VertexIndex { index: 1 }, VertexIndex { index: 3 }),
                (VertexIndex { index: 2 }, VertexIndex { index: 5 }),
            ],
            &[
                VertexKind::Single(VertexColor::Empty),
                VertexKind::Single(VertexColor::Empty),
                VertexKind::Single(VertexColor::Empty),
                VertexKind::Cluster(VertexColor::Empty, on_edges),
                VertexKind::Cluster(VertexColor::Empty, in_center),
                VertexKind::Cluster(VertexColor::Empty, on_edges),
            ],
        ))
    }
}

impl<V, G> Snort<V, G>
where
    V: Has<VertexKind> + Clone,
    G: Graph<V> + Clone,
{
    /// Create new Snort position from graph
    pub const fn new(graph: G) -> Self {
        Self {
            graph,
            _v: PhantomData,
        }
    }

    fn vertex_degree(&self, this_vertex: VertexIndex) -> usize {
        let mut res = 0;
        for one_away in self.graph.vertex_indices() {
            if one_away != this_vertex && self.graph.are_adjacent(this_vertex, one_away) {
                res += self.graph.get_vertex(one_away).get_inner().degree_factor();
            }
        }
        res
    }

    fn vertex_second_degree(&self, this_vertex: VertexIndex) -> usize {
        let mut res = 0;
        let mut seen = vec![false; self.graph.size()];

        for one_away in self.graph.vertex_indices() {
            if one_away != this_vertex && self.graph.are_adjacent(this_vertex, one_away) {
                for two_away in self.graph.vertex_indices() {
                    if two_away != one_away
                        && two_away != this_vertex
                        && self.graph.are_adjacent(one_away, two_away)
                        && !seen[two_away.index]
                    {
                        seen[two_away.index] = true;
                        res += self.graph.get_vertex(two_away).get_inner().degree_factor();
                    }
                }
            }
        }

        res
    }

    /// Get degree of the underlying game graph, correctly counting clusters of vertices
    ///
    /// Note that using [`Graph::degree`] will yield incorrect results
    pub fn degree(&self) -> usize {
        self.graph
            .vertex_indices()
            .map(|v| self.vertex_degree(v))
            .max()
            .unwrap_or(0)
    }

    /// Get second degree of the underlying game graph
    ///
    /// Second degree of a vertex is the number of all vertices two away from a given vertex
    /// and just like in first degree, second degree of a graph is the maximum value among vertices
    pub fn second_degree(&self) -> usize {
        self.graph
            .vertex_indices()
            .map(|v| self.vertex_second_degree(v))
            .max()
            .unwrap_or(0)
    }

    /// Iterator over vertices where given player can move
    pub fn available_moves_for<const COLOR: u8>(&self) -> impl Iterator<Item = VertexIndex> + '_ {
        // const ADT generics are unsable, so here we go
        let own_tint_color: VertexColor = VertexColor::try_from(COLOR).unwrap();
        self.graph
            .vertex_indices()
            .map(|v_idx| self.graph.get_vertex(v_idx))
            .enumerate()
            .filter(move |(_, vertex)| {
                let vertex_color = (*(*vertex)).get_inner().color();
                vertex_color == own_tint_color || vertex_color == VertexColor::Empty
            })
            .map(|(index, _)| VertexIndex { index })
    }

    /// Return position after player move in a given vertex. Note that it does not check
    /// if the move is legal
    #[must_use]
    pub fn move_in_vertex<const COLOR: u8>(&self, move_vertex_idx: VertexIndex) -> Self {
        let own_tint_color: VertexColor = VertexColor::try_from(COLOR).unwrap();
        let mut position: Self = self.clone();

        let mut to_remove = Vec::with_capacity(self.graph.vertex_degree(move_vertex_idx) + 1);

        // Take vertex
        let move_vertex = position.graph.get_vertex_mut(move_vertex_idx);
        match move_vertex.get_inner_mut() {
            VertexKind::Single(_) => {
                to_remove.push(move_vertex_idx);
            }
            VertexKind::Cluster(_, cluster_size) => {
                if cluster_size.get() == 1 {
                    to_remove.push(move_vertex_idx);
                } else {
                    // Vertices in cluster are disconnected so nothing changes color
                    *cluster_size = NonZeroU32::new(cluster_size.get() - 1).unwrap();
                }
            }
        }

        // Disconnect `move_vertex` from adjacent vertices and tint them
        for adjacent_vertex_idx in self.graph.adjacent_to(move_vertex_idx) {
            // Disconnect move vertex from adjacent, we disconnect only single vertices
            // because clusters are still alive. If cluster is dead it's turned into single
            // before (See: 'take vertex' above), so it still works.
            if let VertexKind::Single(_) = position.graph.get_vertex(move_vertex_idx).get_inner() {
                position
                    .graph
                    .connect(move_vertex_idx, adjacent_vertex_idx, false);
            }

            // No loops in snort graphs
            if adjacent_vertex_idx != move_vertex_idx {
                let adjacent_vertex = position.graph.get_vertex_mut(adjacent_vertex_idx);
                let adjacent_vertex_color = adjacent_vertex.get_inner_mut().color_mut();

                // Tint adjacent vertex
                if *adjacent_vertex_color == own_tint_color
                    || *adjacent_vertex_color == VertexColor::Empty
                {
                    // If adjacent vertex is empty or tinted in own color, tint it in own
                    *adjacent_vertex_color = own_tint_color;
                } else {
                    // Otherwise the vertex is tinted in opponents color, so no one can longer
                    // move there, thus we mark is as taken and disconnect from the graph
                    to_remove.push(adjacent_vertex_idx);
                }
            }
        }

        position.graph.remove_vertices(&mut to_remove);
        position
    }

    /// Get moves for a given player. Works only for `TintLeft` and `TintRight`.
    /// Any other input is undefined.
    fn moves_for<const COLOR: u8>(&self) -> Vec<Self> {
        let mut moves = Vec::with_capacity(self.graph.size());
        for move_vertex_idx in self.available_moves_for::<COLOR>() {
            moves.push(self.move_in_vertex::<COLOR>(move_vertex_idx));
        }
        moves
    }

    /// BFS search to get the decompisitons, should be used only as a helper for [`Self::decompositions`]
    fn bfs(&self, visited_vertices: &mut [bool], initial_subgraph_vertex: VertexIndex) -> Self {
        let mut vertices_to_take: Vec<V> = Vec::new();
        let mut vertex_indices_to_take: Vec<VertexIndex> = Vec::new();

        let mut connected_visit_queue: VecDeque<VertexIndex> = VecDeque::new();
        connected_visit_queue.push_back(initial_subgraph_vertex);
        visited_vertices[initial_subgraph_vertex.index] = true;

        while let Some(connected_vertex_idx) = connected_visit_queue.pop_front() {
            vertices_to_take.push(self.graph.get_vertex(connected_vertex_idx).clone());
            vertex_indices_to_take.push(connected_vertex_idx);

            for adjacent_to_connected_idx in self.graph.adjacent_to(connected_vertex_idx) {
                if !visited_vertices[adjacent_to_connected_idx.index] {
                    visited_vertices[adjacent_to_connected_idx.index] = true;
                    connected_visit_queue.push_back(adjacent_to_connected_idx);
                }
            }
        }

        let mut new_graph = G::empty(&vertices_to_take);
        for (new_v, old_v) in vertex_indices_to_take.iter().enumerate() {
            for old_u in self.graph.adjacent_to(*old_v) {
                if let Some(new_u) = vertex_indices_to_take.iter().position(|x| *x == old_u) {
                    new_graph.connect(
                        VertexIndex { index: new_v },
                        VertexIndex { index: new_u },
                        true,
                    );
                }
            }
        }

        Self {
            graph: new_graph,
            _v: PhantomData,
        }
    }

    /// Render to a [graphviz](https://graphviz.org/) format, that can be later rendered to an
    /// image with external engine.
    pub fn to_graphviz(&self) -> String {
        let mut buf = String::new();

        write!(buf, "graph G {{").unwrap();

        for (vertex_relative_idx, vertex_idx) in self.graph.vertex_indices().enumerate() {
            let vertex = self.graph.get_vertex(vertex_idx);
            let color = match vertex.get_inner().color() {
                VertexColor::Empty => "white",
                VertexColor::TintLeft => "blue",
                VertexColor::TintRight => "red",
            };
            let shape = match vertex.get_inner() {
                VertexKind::Single(_) => "circle",
                VertexKind::Cluster(_, _) => "square",
            };
            let label = match vertex.get_inner() {
                VertexKind::Single(_) => format!("\"{}\"", vertex_relative_idx),
                VertexKind::Cluster(_, cluster_size) => {
                    format!("\"{}\\n<{}>\"", vertex_relative_idx, cluster_size.get())
                }
            };

            write!(buf,
                   "{} [label={}, fillcolor={}, style=filled, shape={}, fixedsize=true, width=1, height=1, fontsize=24];",
                   vertex_relative_idx,
                   label,
                   color,
                   shape).unwrap();
        }

        for v in self.graph.vertex_indices() {
            for u in self.graph.vertex_indices() {
                if v < u && self.graph.are_adjacent(v, u) {
                    write!(buf, "{} -- {};", v.index, u.index).unwrap();
                }
            }
        }

        write!(buf, "}}").unwrap();
        buf
    }
}

impl<V, G> Draw for Snort<V, G>
where
    V: Has<VertexKind> + Has<V2f>,
    G: Graph<V> + Clone,
{
    fn draw<C>(&self, canvas: &mut C)
    where
        C: Canvas,
    {
        self.graph.draw(canvas, |vertex| {
            match Has::<VertexKind>::get_inner(vertex).color() {
                VertexColor::Empty => Color::LIGHT_GRAY,
                VertexColor::TintLeft => Color::BLUE,
                VertexColor::TintRight => Color::RED,
            }
        });
    }

    fn required_canvas<C>(&self) -> BoundingBox
    where
        C: Canvas,
    {
        self.graph.required_canvas::<C>()
    }
}

#[test]
fn degree_works() {
    use crate::graph::adjacency_matrix::undirected::UndirectedGraph;

    let snort: Snort<VertexKind, UndirectedGraph<VertexKind>> =
        Snort::new_three_caterpillar(NonZeroU32::new(8).unwrap());
    assert_eq!(snort.degree(), 10);
    assert_eq!(snort.second_degree(), 18);

    let snort: Snort<VertexKind, UndirectedGraph<VertexKind>> =
        Snort::new_three_caterpillar(NonZeroU32::new(10).unwrap());
    assert_eq!(snort.degree(), 12);
    assert_eq!(snort.second_degree(), 22);
}

impl<G> PartizanGame for Snort<VertexKind, G>
where
    G: Graph<VertexKind> + Clone + Hash + Eq + Send + Sync,
{
    fn left_moves(&self) -> Vec<Self> {
        self.moves_for::<{ VertexColor::TintLeft as u8 }>()
    }

    fn right_moves(&self) -> Vec<Self> {
        self.moves_for::<{ VertexColor::TintRight as u8 }>()
    }

    /// Decompose the game graph into disconnected components
    fn decompositions(&self) -> Vec<Self> {
        let mut visited = vec![false; self.graph.size()];
        let mut res = Vec::new();

        for v in self.graph.vertex_indices() {
            if !visited[v.index] {
                res.push(self.bfs(&mut visited, v));
            }
        }

        res
    }

    fn reductions(&self) -> Option<CanonicalForm> {
        let mut vertex_indices = self.graph.vertex_indices();
        if let Some(vertex_idx) = vertex_indices.next() {
            if vertex_indices.next().is_none() {
                let vertex = self.graph.get_vertex(vertex_idx);

                let cf = match vertex {
                    VertexKind::Single(VertexColor::Empty) => {
                        CanonicalForm::new_nimber(DyadicRationalNumber::from(0), Nimber::new(1))
                    }
                    VertexKind::Single(VertexColor::TintLeft) => CanonicalForm::new_integer(1),
                    VertexKind::Single(VertexColor::TintRight) => CanonicalForm::new_integer(-1),
                    VertexKind::Cluster(VertexColor::Empty, cluster_size) => {
                        let nimber = Nimber::new(cluster_size.get() % 2);
                        CanonicalForm::new_nimber(DyadicRationalNumber::from(0), nimber)
                    }
                    VertexKind::Cluster(VertexColor::TintLeft, cluster_size) => {
                        CanonicalForm::new_integer(cluster_size.get() as i64)
                    }
                    VertexKind::Cluster(VertexColor::TintRight, cluster_size) => {
                        CanonicalForm::new_integer(-(cluster_size.get() as i64))
                    }
                };
                return Some(cf);
            }
        }

        None
    }
}

#[test]
fn no_moves() {
    use crate::graph::adjacency_matrix::undirected::UndirectedGraph;

    let position: Snort<VertexKind, UndirectedGraph<VertexKind>> =
        Snort::new(UndirectedGraph::empty(&[]));
    assert_eq!(position.left_moves(), vec![]);
    assert_eq!(position.right_moves(), vec![]);
}

#[test]
fn correct_canonical_forms() {
    use crate::{
        graph::adjacency_matrix::undirected::UndirectedGraph,
        short::partizan::transposition_table::ParallelTranspositionTable,
    };

    let transposition_table = ParallelTranspositionTable::new();

    let snort = Snort::new(UndirectedGraph::empty(&[VertexKind::Cluster(
        VertexColor::Empty,
        NonZeroU32::new(10).unwrap(),
    )]));
    let canonical_form = snort.canonical_form(&transposition_table);
    assert_eq!(canonical_form.to_string(), "0");

    let snort = Snort::new(UndirectedGraph::empty(&[VertexKind::Cluster(
        VertexColor::Empty,
        NonZeroU32::new(11).unwrap(),
    )]));
    let canonical_form = snort.canonical_form(&transposition_table);
    assert_eq!(canonical_form.to_string(), "*");

    let snort: Snort<VertexKind, UndirectedGraph<VertexKind>> =
        Snort::new_three_caterpillar(NonZeroU32::new(2).unwrap());
    let canonical_form = snort.canonical_form(&transposition_table);
    assert_eq!(
        canonical_form.to_string(),
        "{5, {{8|6*}|0}|-5, {0|{-6*|-8}}}"
    );
}

#[test]
fn correct_sensible() {
    use crate::{
        graph::adjacency_matrix::undirected::UndirectedGraph,
        short::partizan::transposition_table::ParallelTranspositionTable,
    };

    let position = Snort::new(UndirectedGraph::empty(&[
        VertexKind::Single(VertexColor::Empty),
        VertexKind::Single(VertexColor::TintLeft),
    ]));
    let transposition_table = ParallelTranspositionTable::new();
    assert_eq!(
        position.sensible_left_moves(&transposition_table),
        vec![Snort::new(UndirectedGraph::empty(&[VertexKind::Single(
            VertexColor::TintLeft
        ),]))]
    );
}
