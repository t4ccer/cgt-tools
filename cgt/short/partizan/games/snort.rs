//! Snort is played on undirected graph. In each turn Left colors a vertex blue and Right colors
//! a vertex red. Players can only choose a vertex that is adjecent to only empty vertices or to
//! vertices in their own color.

use crate::{
    graph::{adjacency_matrix::undirected::UndirectedGraph, Graph, VertexIndex},
    numeric::{dyadic_rational_number::DyadicRationalNumber, nimber::Nimber},
    short::partizan::{canonical_form::CanonicalForm, partizan_game::PartizanGame},
};
use std::{
    collections::VecDeque,
    fmt::Write,
    hash::Hash,
    num::NonZeroU32,
    ops::{Index, IndexMut},
};

/// Color of Snort vertex. Note that we are taking tinting apporach rather than direct tracking
/// of adjacent colors.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
#[cfg_attr(
    feature = "serde",
    derive(serde_repr::Serialize_repr, serde_repr::Deserialize_repr)
)]
#[repr(u8)]
pub enum VertexColor {
    /// Vertex without color, not connected to any taken vertices
    Empty = 0,

    /// Vertex that is adjecent to left
    TintLeft = 1,

    /// Vertex that is adjecent to right
    TintRight = 2,

    /// Vertex that is either taken or connected to both colors
    Taken = 3,
}

impl TryFrom<u8> for VertexColor {
    type Error = ();

    #[inline]
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Empty),
            1 => Ok(Self::TintLeft),
            2 => Ok(Self::TintRight),
            3 => Ok(Self::Taken),
            _ => Err(()),
        }
    }
}

/// Type of vertex (or group of them) in the graph. We abstract over vertices to support efficient
/// calculations of positions with star-like structure
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
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
    pub fn color_mut(&mut self) -> &mut VertexColor {
        match self {
            Self::Single(color) | Self::Cluster(color, _) => color,
        }
    }

    #[inline]
    fn degree_factor(self) -> usize {
        match self {
            VertexKind::Single(_) => 1,
            VertexKind::Cluster(_, cluster_size) => cluster_size.get() as usize,
        }
    }
}

/// Vertices colors of the game graph
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct VertexColors {
    /// Color of each vertex
    pub inner: Vec<VertexKind>,
}

impl Index<VertexIndex> for VertexColors {
    type Output = VertexKind;

    fn index(&self, index: VertexIndex) -> &Self::Output {
        &self.inner[index.index]
    }
}

impl IndexMut<VertexIndex> for VertexColors {
    fn index_mut(&mut self, index: VertexIndex) -> &mut Self::Output {
        &mut self.inner[index.index]
    }
}

/// Position of a [snort](self) game
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Snort<G = UndirectedGraph> {
    /// Vertices colors of the game graph
    pub vertices: VertexColors,

    /// Get graph of the game. This includes only edges
    pub graph: G,
}

impl<G> Snort<G>
where
    G: Graph + Clone,
{
    /// Create new Snort position with all vertices empty.
    pub fn new(graph: G) -> Self {
        Self {
            vertices: VertexColors {
                inner: vec![VertexKind::Single(VertexColor::Empty); graph.size()],
            },
            graph,
        }
    }

    // TODO: Perform that check
    /// Create a Snort position with initial colors. It's up to the user to ensure that no conflicting
    /// colors are connected in the graph.
    /// Returns `None` if `vertices` and `graph` have conflicting sizes.
    pub fn with_colors(vertices: Vec<VertexKind>, graph: G) -> Option<Self> {
        if vertices.len() != graph.size() {
            return None;
        }

        Some(Self {
            vertices: VertexColors { inner: vertices },
            graph,
        })
    }

    /// Construct new position on caterpillar `C(n+1, n, n+1)`
    ///
    /// The caterpillar `C(n+1, n, n+1)` consists of a main path of length 3, whose central vertex
    /// has `n` leaves added, and `n+1` leaves are added to the side vertices.
    pub fn new_three_caterpillar(n: NonZeroU32) -> Self {
        let on_edges = n.checked_add(1).unwrap();
        let in_center = n;

        Self::with_colors(
            vec![
                VertexKind::Single(VertexColor::Empty),
                VertexKind::Single(VertexColor::Empty),
                VertexKind::Single(VertexColor::Empty),
                VertexKind::Cluster(VertexColor::Empty, on_edges),
                VertexKind::Cluster(VertexColor::Empty, in_center),
                VertexKind::Cluster(VertexColor::Empty, on_edges),
            ],
            G::from_edges(
                6,
                &[
                    (VertexIndex { index: 0 }, VertexIndex { index: 1 }),
                    (VertexIndex { index: 0 }, VertexIndex { index: 2 }),
                    (VertexIndex { index: 0 }, VertexIndex { index: 4 }),
                    (VertexIndex { index: 1 }, VertexIndex { index: 3 }),
                    (VertexIndex { index: 2 }, VertexIndex { index: 5 }),
                ],
            ),
        )
        .unwrap()
    }

    fn vertex_degree(&self, this_vertex: VertexIndex) -> usize {
        let mut res = 0;
        for one_away in self.graph.vertices() {
            if one_away != this_vertex && self.graph.are_adjacent(this_vertex, one_away) {
                res += self.vertices[one_away].degree_factor();
            }
        }
        res
    }

    fn vertex_second_degree(&self, this_vertex: VertexIndex) -> usize {
        let mut res = 0;
        let mut seen = vec![false; self.graph.size()];

        for one_away in self.graph.vertices() {
            if one_away != this_vertex && self.graph.are_adjacent(this_vertex, one_away) {
                for two_away in self.graph.vertices() {
                    if two_away != one_away
                        && two_away != this_vertex
                        && self.graph.are_adjacent(one_away, two_away)
                        && !seen[two_away.index]
                    {
                        seen[two_away.index] = true;
                        res += self.vertices[two_away].degree_factor();
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
            .vertices()
            .into_iter()
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
            .vertices()
            .into_iter()
            .map(|v| self.vertex_second_degree(v))
            .max()
            .unwrap_or(0)
    }

    /// Get moves for a given player. Works only for `TintLeft` and `TintRight`.
    /// Any other input is undefined.
    fn moves_for<const COLOR: u8>(&self) -> Vec<Self> {
        // const ADT generics are unsable, so here we go
        let own_tint_color: VertexColor = VertexColor::try_from(COLOR).unwrap();

        let mut moves = Vec::with_capacity(self.graph.size());

        // Vertices where player can move
        let move_vertices = self
            .vertices
            .inner
            .iter()
            .enumerate()
            .filter(|(_, vertex)| {
                let vertex_color = vertex.color();
                vertex_color == own_tint_color || vertex_color == VertexColor::Empty
            })
            .map(|(index, _)| VertexIndex { index });

        // Go through list of vertices with legal move
        for move_vertex_idx in move_vertices {
            let mut position: Self = self.clone();

            // Take vertex
            let move_vertex = &mut position.vertices[move_vertex_idx];
            match move_vertex {
                VertexKind::Single(move_vertex_color) => *move_vertex_color = VertexColor::Taken,
                VertexKind::Cluster(_, cluster_size) => {
                    if *cluster_size == NonZeroU32::new(1).unwrap() {
                        *move_vertex = VertexKind::Single(VertexColor::Taken);
                    } else {
                        // Vertices in cluster are disconnected so nothing changes color
                        *cluster_size = NonZeroU32::new(cluster_size.get() - 1).unwrap();
                    }
                }
            }

            // Disconnect `move_vertex` from adjecent vertices and tint them
            for adjacent_vertex_idx in self.graph.adjacent_to(move_vertex_idx) {
                // Disconnect move vertex from adjecent, we disconnect only single vertices
                // because clusters are still alive. If cluster is dead it's turned into single
                // before (See: 'take vertex' above), so it still works.
                if let VertexKind::Single(_) = position.vertices[move_vertex_idx] {
                    position
                        .graph
                        .connect(move_vertex_idx, adjacent_vertex_idx, false);
                }

                // No loops in snort graphs
                if adjacent_vertex_idx != move_vertex_idx {
                    let adjacent_vertex = &mut position.vertices[adjacent_vertex_idx];
                    let adjacent_vertex_color = adjacent_vertex.color_mut();

                    // Tint adjacent vertex
                    if *adjacent_vertex_color == own_tint_color
                        || *adjacent_vertex_color == VertexColor::Empty
                    {
                        // If adjacent vertex is empty or tinted in own color, tint it in own
                        *adjacent_vertex_color = own_tint_color;
                    } else {
                        // Otherwise the vertex is tinted in opponents color, so no one can longer
                        // move there, thus we mark is as taken and disconnect from the graph
                        *adjacent_vertex_color = VertexColor::Taken;
                        for v in position.graph.vertices() {
                            position.graph.connect(v, adjacent_vertex_idx, false);
                        }
                    }
                }
            }
            moves.push(position);
        }
        moves
    }

    /// BFS search to get the decompisitons, should be used only as a helper for [`Self::decompositions`]
    fn bfs(&self, visited: &mut [bool], v: VertexIndex) -> Self {
        let mut vertices_to_take: Vec<VertexIndex> = Vec::new();

        let mut q: VecDeque<VertexIndex> = VecDeque::new();
        q.push_back(v);
        visited[v.index] = true;

        while let Some(v) = q.pop_front() {
            vertices_to_take.push(v);

            for u in self.graph.adjacent_to(v) {
                if !visited[u.index] {
                    visited[u.index] = true;
                    q.push_back(u);
                }
            }
        }

        let mut new_graph = G::empty(vertices_to_take.len());
        for (new_v, old_v) in vertices_to_take.iter().enumerate() {
            for old_u in self.graph.adjacent_to(*old_v) {
                if let Some(new_u) = vertices_to_take.iter().position(|x| *x == old_u) {
                    new_graph.connect(
                        VertexIndex { index: new_v },
                        VertexIndex { index: new_u },
                        true,
                    );
                }
            }
        }

        let mut new_vertices = Vec::with_capacity(vertices_to_take.len());
        for v in &vertices_to_take {
            new_vertices.push(self.vertices[*v]);
        }

        Self {
            vertices: VertexColors {
                inner: new_vertices,
            },
            graph: new_graph,
        }
    }

    /// Render to a [graphviz](https://graphviz.org/) format, that can be later rendered to an
    /// image with external engine.
    pub fn to_graphviz(&self) -> String {
        let mut buf = String::new();

        write!(buf, "graph G {{").unwrap();

        for (vertex_idx, vertex) in self.vertices.inner.iter().enumerate() {
            let color = match vertex.color() {
                VertexColor::Empty => "white",
                VertexColor::TintLeft => "blue",
                VertexColor::TintRight => "red",
                VertexColor::Taken => continue,
            };
            let shape = match vertex {
                VertexKind::Single(_) => "circle",
                VertexKind::Cluster(_, _) => "square",
            };
            let label = match vertex {
                VertexKind::Single(_) => format!("\"{}\"", vertex_idx),
                VertexKind::Cluster(_, cluster_size) => {
                    format!("\"{}\\n<{}>\"", vertex_idx, cluster_size.get())
                }
            };

            write!(buf,
                   "{} [label={}, fillcolor={}, style=filled, shape={}, fixedsize=true, width=1, height=1, fontsize=24];",
                   vertex_idx,
                   label,
                   color,
                   shape).unwrap();
        }

        for v in self.graph.vertices() {
            for u in self.graph.vertices() {
                if v < u && self.graph.are_adjacent(v, u) {
                    write!(buf, "{} -- {};", v.index, u.index).unwrap();
                }
            }
        }

        write!(buf, "}}").unwrap();
        buf
    }
}

#[test]
fn degree_works() {
    let snort: Snort<UndirectedGraph> = Snort::new_three_caterpillar(NonZeroU32::new(8).unwrap());
    assert_eq!(snort.degree(), 10);

    let snort: Snort<UndirectedGraph> = Snort::new_three_caterpillar(NonZeroU32::new(10).unwrap());
    assert_eq!(snort.degree(), 12);
}

impl<G> PartizanGame for Snort<G>
where
    G: Graph + Clone + Hash + Eq + Send + Sync,
{
    fn left_moves(&self) -> Vec<Self> {
        self.moves_for::<{ VertexColor::TintLeft as u8 }>()
    }

    fn right_moves(&self) -> Vec<Self> {
        self.moves_for::<{ VertexColor::TintRight as u8 }>()
    }

    /// Decompose the game graph into disconnected components
    fn decompositions(&self) -> Vec<Self> {
        let mut visited = vec![false; self.vertices.inner.len()];
        let mut res = Vec::new();

        for v in self.graph.vertices() {
            if !matches!(self.vertices[v].color(), VertexColor::Taken) && !visited[v.index] {
                res.push(self.bfs(&mut visited, v));
            }
        }

        res
    }

    fn reductions(&self) -> Option<CanonicalForm> {
        if let &[vertex] = &self.vertices.inner[..] {
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
                VertexKind::Single(VertexColor::Taken)
                | VertexKind::Cluster(VertexColor::Taken, _) => CanonicalForm::new_integer(0),
            };
            return Some(cf);
        }

        None
    }
}

#[test]
fn no_moves() {
    let position = Snort::new(UndirectedGraph::empty(0));
    assert_eq!(position.left_moves(), vec![]);
    assert_eq!(position.right_moves(), vec![]);
}

#[test]
fn correct_canonical_forms() {
    use crate::short::partizan::transposition_table::ParallelTranspositionTable;
    let transposition_table = ParallelTranspositionTable::new();

    let snort = Snort::with_colors(
        vec![VertexKind::Cluster(
            VertexColor::Empty,
            NonZeroU32::new(10).unwrap(),
        )],
        UndirectedGraph::empty(1),
    )
    .unwrap();
    let canonical_form = snort.canonical_form(&transposition_table);
    assert_eq!(canonical_form.to_string(), "0");

    let snort = Snort::with_colors(
        vec![VertexKind::Cluster(
            VertexColor::Empty,
            NonZeroU32::new(11).unwrap(),
        )],
        UndirectedGraph::empty(1),
    )
    .unwrap();
    let canonical_form = snort.canonical_form(&transposition_table);
    assert_eq!(canonical_form.to_string(), "*");
}

#[test]
fn correct_sensible() {
    use crate::short::partizan::transposition_table::ParallelTranspositionTable;

    let position = Snort::with_colors(
        vec![
            VertexKind::Single(VertexColor::Empty),
            VertexKind::Single(VertexColor::TintLeft),
        ],
        UndirectedGraph::empty(2),
    )
    .unwrap();
    let transposition_table = ParallelTranspositionTable::new();
    assert_eq!(
        position.sensible_left_moves(&transposition_table),
        vec![Snort::with_colors(
            vec![
                VertexKind::Single(VertexColor::Taken),
                VertexKind::Single(VertexColor::TintLeft)
            ],
            UndirectedGraph::empty(2),
        )
        .unwrap()]
    );
}
