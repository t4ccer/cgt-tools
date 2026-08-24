//! Col is played on undirected graph. In each turn Left colors a vertex blue and Right colors
//! a vertex red. Players can only choose a vertex that is adjacent to only empty vertices or to
//! vertices in the opponents color.

use crate::{
    drawing::{BoundingBox, Canvas, Color, Draw},
    graph::{Graph, VertexIndex},
    has::Has,
    numeric::v2f::V2f,
    short::partizan::partizan_game::PartizanGame,
};
use std::{collections::VecDeque, fmt::Write, hash::Hash, marker::PhantomData};

/// Color of Col vertex. Note that we are taking tinting approach rather than direct tracking
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

impl VertexColor {
    #[inline]
    const fn try_from_raw(value: u8) -> Option<Self> {
        match value {
            n if n == Self::Empty as u8 => Some(Self::Empty),
            n if n == Self::TintLeft as u8 => Some(Self::TintLeft),
            n if n == Self::TintRight as u8 => Some(Self::TintRight),
            _ => None,
        }
    }
}

/// Position of a [snort](self) game
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Col<V, G> {
    /// Graph of the game
    pub graph: G,
    _v: PhantomData<V>,
}

impl<V, G> Col<V, G>
where
    V: Has<VertexColor> + Clone,
    G: Graph<V> + Clone,
{
    /// Create new Col position from graph
    pub const fn new(graph: G) -> Self {
        Self {
            graph,
            _v: PhantomData,
        }
    }

    /// Iterator over vertices where given player can move
    pub fn available_moves_for<const COLOR: u8>(&self) -> impl Iterator<Item = VertexIndex> + '_ {
        // const ADT generics are unstable, so here we go
        let own_tint_color: VertexColor = const { VertexColor::try_from_raw(COLOR).unwrap() };
        self.graph
            .vertex_indices()
            .map(|v_idx| (v_idx, self.graph.get_vertex(v_idx)))
            .filter(move |(_, vertex)| {
                let vertex_color = *(*(*vertex)).get_inner();
                vertex_color == own_tint_color || vertex_color == VertexColor::Empty
            })
            .map(|(index, _)| index)
    }

    /// Return position after player move in a given vertex. Note that it does not check
    /// if the move is legal
    #[must_use]
    pub fn move_in_vertex<const COLOR: u8>(&self, move_vertex_idx: VertexIndex) -> Self {
        let opponent_tint_color: VertexColor = const {
            match VertexColor::try_from_raw(COLOR).unwrap() {
                VertexColor::Empty => unreachable!(),
                VertexColor::TintLeft => VertexColor::TintRight,
                VertexColor::TintRight => VertexColor::TintLeft,
            }
        };

        let mut position: Self = self.clone();

        let mut to_remove = Vec::with_capacity(self.graph.vertex_degree(move_vertex_idx) + 1);

        // Take vertex
        to_remove.push(move_vertex_idx);

        // Disconnect `move_vertex` from adjacent vertices and tint them
        for adjacent_vertex_idx in self.graph.adjacent_to(move_vertex_idx) {
            // Disconnect move vertex from adjacent
            position
                .graph
                .connect(move_vertex_idx, adjacent_vertex_idx, false);

            // No loops in col graphs
            if adjacent_vertex_idx != move_vertex_idx {
                let adjacent_vertex = position.graph.get_vertex_mut(adjacent_vertex_idx);
                let adjacent_vertex_color = adjacent_vertex.get_inner_mut();

                // Tint adjacent vertex
                if *adjacent_vertex_color == VertexColor::Empty
                    || *adjacent_vertex_color == opponent_tint_color
                {
                    // If adjacent vertex is empty or tinted in opponent's color, tint it as opposite
                    *adjacent_vertex_color = opponent_tint_color;
                } else {
                    // Otherwise the vertex is tinted in own color, and now it would be tinted in both
                    // thus we mark is as taken and disconnect from the graph
                    to_remove.push(adjacent_vertex_idx);
                }
            }
        }

        position.graph.remove_vertices(&mut to_remove);
        position
    }

    /// Get moves for a given player. Works only for `TintLeft` and `TintRight`.
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
            let color = match vertex.get_inner() {
                VertexColor::Empty => "white",
                VertexColor::TintLeft => "blue",
                VertexColor::TintRight => "red",
            };
            let shape = "circle";
            let label = format!("\"{}\"", vertex_relative_idx);

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

impl<V, G> Draw for Col<V, G>
where
    V: Has<VertexColor> + Has<V2f>,
    G: Graph<V> + Clone,
{
    fn draw<C>(&self, canvas: &mut C)
    where
        C: Canvas,
    {
        self.graph.draw(canvas, |canvas, vertex_index| {
            let position: V2f = *self.graph.get_vertex(vertex_index).get_inner();
            let color: VertexColor = *self.graph.get_vertex(vertex_index).get_inner();
            canvas.vertex(
                position,
                match color {
                    VertexColor::Empty => Color::LIGHT_GRAY,
                    VertexColor::TintLeft => Color::BLUE,
                    VertexColor::TintRight => Color::RED,
                },
                vertex_index,
            )
        });
    }

    fn required_canvas<C>(&self) -> BoundingBox
    where
        C: Canvas,
    {
        self.graph.required_canvas::<C>()
    }
}

impl<G> PartizanGame for Col<VertexColor, G>
where
    G: Graph<VertexColor> + Clone + Hash + Eq + Send + Sync,
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
}
