//! Snort is played on undirected graph. In each turn Left colors a vertex blue and Right colors
//! a vertex red. Players can only choose a vertex that is adjecent to only empty vertices or to
//! vertices in their own color.

use crate::{
    graph::undirected::Graph,
    numeric::{dyadic_rational_number::DyadicRationalNumber, nimber::Nimber},
    short::partizan::{canonical_form::CanonicalForm, partizan_game::PartizanGame},
};
use std::{collections::VecDeque, fmt::Write, num::NonZeroU32};

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
    const fn color(self) -> VertexColor {
        match self {
            Self::Single(color) | Self::Cluster(color, _) => color,
        }
    }

    #[inline]
    fn color_mut(&mut self) -> &mut VertexColor {
        match self {
            Self::Single(color) | Self::Cluster(color, _) => color,
        }
    }
}

/// Position of a [snort](self) game
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Snort {
    /// Vertices colors of the game graph
    pub vertices: Vec<VertexKind>,

    /// Get graph of the game. This includes only edges
    pub graph: Graph,
}

impl Snort {
    /// Create new Snort position with all vertices empty.
    pub fn new(graph: Graph) -> Self {
        Self {
            vertices: vec![VertexKind::Single(VertexColor::Empty); graph.size()],
            graph,
        }
    }

    // TODO: Perform that check
    /// Create a Snort position with initial colors. It's up to the user to ensure that no conflicting
    /// colors are connected in the graph.
    /// Returns `None` if `vertices` and `graph` have conflicting sizes.
    pub fn with_colors(vertices: Vec<VertexKind>, graph: Graph) -> Option<Self> {
        if vertices.len() != graph.size() {
            return None;
        }

        Some(Self { vertices, graph })
    }

    /// Construct new "three star" position with `n+1` leafs on edges and `n` in the center
    ///
    /// # Errors
    /// - When number of leafs would be non-positive.
    pub fn new_three_star(n: u32) -> Option<Self> {
        let on_edges = NonZeroU32::new(n + 1)?;
        let in_center = NonZeroU32::new(n)?;

        Self::with_colors(
            vec![
                VertexKind::Single(VertexColor::Empty),
                VertexKind::Single(VertexColor::Empty),
                VertexKind::Single(VertexColor::Empty),
                VertexKind::Cluster(VertexColor::Empty, on_edges),
                VertexKind::Cluster(VertexColor::Empty, in_center),
                VertexKind::Cluster(VertexColor::Empty, on_edges),
            ],
            Graph::from_edges(6, &[(0, 1), (0, 2), (0, 4), (1, 3), (2, 5)]),
        )
    }

    /// Get degree of the underlying game graph, correctly counting clusters of vertices
    ///
    /// Note that using [`Graph::degree`] will yield incorrect results
    #[cfg_attr(feature = "cargo-clippy", allow(clippy::match_on_vec_items))]
    pub fn degree(&self) -> usize {
        let mut degrees = vec![0usize; self.graph.size()];
        for v in self.graph.vertices() {
            for u in self.graph.vertices() {
                if u != v && self.graph.are_adjacent(v, u) {
                    match self.vertices[v] {
                        VertexKind::Single(_) => degrees[u] += 1,
                        VertexKind::Cluster(_, cluster_size) => {
                            degrees[u] += cluster_size.get() as usize;
                        }
                    }
                }
            }
        }

        degrees
            .into_iter()
            .max()
            .expect("graph to have at least 1 vertex")
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
            .iter()
            .enumerate()
            .filter(|(_, vertex)| {
                let vertex_color = vertex.color();
                vertex_color == own_tint_color || vertex_color == VertexColor::Empty
            })
            .map(|(idx, _)| idx);

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
    fn bfs(&self, visited: &mut [bool], v: usize) -> Self {
        let mut vertices_to_take: Vec<usize> = Vec::new();

        let mut q: VecDeque<usize> = VecDeque::new();
        q.push_back(v);
        visited[v] = true;

        while let Some(v) = q.pop_front() {
            vertices_to_take.push(v);

            for u in self.graph.adjacent_to(v) {
                if !visited[u] {
                    visited[u] = true;
                    q.push_back(u);
                }
            }
        }

        let mut new_graph = Graph::empty(vertices_to_take.len());
        for (new_v, old_v) in vertices_to_take.iter().enumerate() {
            for old_u in self.graph.adjacent_to(*old_v) {
                if let Some(new_u) = vertices_to_take.iter().position(|x| *x == old_u) {
                    new_graph.connect(new_v, new_u, true);
                }
            }
        }

        let mut new_vertices = Vec::with_capacity(vertices_to_take.len());
        for v in &vertices_to_take {
            new_vertices.push(self.vertices[*v]);
        }

        Self {
            vertices: new_vertices,
            graph: new_graph,
        }
    }

    /// Render to a [graphviz](https://graphviz.org/) format, that can be later rendered to an
    /// image with external engine.
    pub fn to_graphviz(&self) -> String {
        let mut buf = String::new();

        write!(buf, "graph G {{").unwrap();

        for (vertex_idx, vertex) in self.vertices.iter().enumerate() {
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
                    write!(buf, "{v} -- {u};").unwrap();
                }
            }
        }

        write!(buf, "}}").unwrap();
        buf
    }
}

#[test]
fn degree_works() {
    let snort = Snort::new_three_star(8).unwrap();
    assert_eq!(snort.degree(), 10);

    let snort = Snort::new_three_star(10).unwrap();
    assert_eq!(snort.degree(), 12);
}

impl PartizanGame for Snort {
    fn left_moves(&self) -> Vec<Self> {
        self.moves_for::<{ VertexColor::TintLeft as u8 }>()
    }

    fn right_moves(&self) -> Vec<Self> {
        self.moves_for::<{ VertexColor::TintRight as u8 }>()
    }

    /// Decompose the game graph into disconnected components
    ///
    /// # Examples
    ///
    /// ```
    /// use cgt::graph::undirected::Graph;
    /// use cgt::short::partizan::games::snort::Snort;
    /// use cgt::short::partizan::partizan_game::PartizanGame;
    ///
    /// assert_eq!(
    ///     Snort::new(Graph::from_edges(5, &[(0, 1), (0, 2), (1, 2), (3, 4)])).decompositions(),
    ///     vec![
    ///         Snort::new(Graph::from_edges(3, &[(0, 1), (0, 2), (1, 2)])),
    ///         Snort::new(Graph::from_edges(2, &[(0, 1)]))
    ///     ]
    /// );
    /// ```
    fn decompositions(&self) -> Vec<Self> {
        let mut visited = vec![false; self.vertices.len()];
        let mut res = Vec::new();

        for v in self.graph.vertices() {
            if !matches!(self.vertices[v].color(), VertexColor::Taken) && !visited[v] {
                res.push(self.bfs(&mut visited, v));
            }
        }

        res
    }

    fn reductions(&self) -> Option<CanonicalForm> {
        if let &[vertex] = &self.vertices[..] {
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
    let position = Snort::new(Graph::empty(0));
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
        Graph::empty(1),
    )
    .unwrap();
    let canonical_form = snort.canonical_form(&transposition_table);
    assert_eq!(canonical_form.to_string(), "0");

    let snort = Snort::with_colors(
        vec![VertexKind::Cluster(
            VertexColor::Empty,
            NonZeroU32::new(11).unwrap(),
        )],
        Graph::empty(1),
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
        Graph::empty(2),
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
            Graph::empty(2),
        )
        .unwrap()]
    );
}
