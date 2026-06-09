//! Snort but on tinted, bipartite graph

use crate::{
    drawing::{BoundingBox, Canvas, Color, Draw},
    graph::{Graph, VertexIndex, bipartite::BipartiteGraph},
    numeric::v2f::V2f,
    short::partizan::{canonical_form::CanonicalForm, partizan_game::PartizanGame},
};
use std::{collections::VecDeque, fmt::Write, hash::Hash, marker::PhantomData};

/// Color of Snort vertex. Note that we are taking tinting approach rather than direct tracking
/// of adjacent colors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(
    feature = "serde",
    derive(serde_repr::Serialize_repr, serde_repr::Deserialize_repr)
)]
#[repr(u8)]
pub enum VertexColor {
    /// Vertex that is adjacent to left
    TintLeft = 1,

    /// Vertex that is adjacent to right
    TintRight = 2,
}

impl VertexColor {
    #[inline]
    const fn try_from_raw(value: u8) -> Option<Self> {
        match value {
            n if n == Self::TintLeft as u8 => Some(Self::TintLeft),
            n if n == Self::TintRight as u8 => Some(Self::TintRight),
            _ => None,
        }
    }
}

/// Position of a [snort](self) game
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BipartiteSnort<G> {
    /// Graph of the game
    pub graph: G,
}

impl<G> BipartiteSnort<G>
where
    G: Graph<VertexColor>,
{
    /// Create new Snort position from graph
    pub const fn new(graph: G) -> Self {
        Self { graph }
    }
}

impl<G> BipartiteSnort<G>
where
    G: Graph<VertexColor> + Clone,
{
    /// Iterator over vertices where given player can move
    pub fn available_moves_for<const COLOR: u8>(&self) -> impl Iterator<Item = VertexIndex> + '_ {
        // const ADT generics are unstable, so here we go
        let own_tint_color: VertexColor = const { VertexColor::try_from_raw(COLOR).unwrap() };
        self.graph
            .vertex_indices()
            .map(|v_idx| self.graph.get_vertex(v_idx))
            .enumerate()
            .filter(move |(_, vertex_color)| **vertex_color == own_tint_color)
            .map(|(index, _)| VertexIndex { index })
    }

    /// Return position after player move in a given vertex. Note that it does not check
    /// if the move is legal
    #[must_use]
    pub fn move_in_vertex<const COLOR: u8>(&self, move_vertex_idx: VertexIndex) -> Self {
        let own_tint_color: VertexColor = const { VertexColor::try_from_raw(COLOR).unwrap() };
        let mut position: Self = self.clone();

        let mut to_remove = Vec::with_capacity(self.graph.vertex_degree(move_vertex_idx) + 1);
        to_remove.push(move_vertex_idx);

        // Disconnect `move_vertex` from adjacent vertices and tint them
        for adjacent_vertex_idx in self.graph.adjacent_to(move_vertex_idx) {
            position
                .graph
                .connect(move_vertex_idx, adjacent_vertex_idx, false);

            // No loops in snort graphs
            if adjacent_vertex_idx != move_vertex_idx {
                let adjacent_vertex_color = position.graph.get_vertex_mut(adjacent_vertex_idx);

                // Tint adjacent vertex
                if *adjacent_vertex_color == own_tint_color {
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
        let mut vertices_to_take: Vec<VertexColor> = Vec::new();
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

        Self { graph: new_graph }
    }

    /// Render to a [graphviz](https://graphviz.org/) format, that can be later rendered to an
    /// image with external engine.
    pub fn to_graphviz(&self) -> String {
        let mut buf = String::new();

        write!(buf, "graph G {{").unwrap();

        for (vertex_relative_idx, vertex_idx) in self.graph.vertex_indices().enumerate() {
            let vertex_color = self.graph.get_vertex(vertex_idx);
            let color = match vertex_color {
                VertexColor::TintLeft => "blue",
                VertexColor::TintRight => "red",
            };
            write!(buf,
                   "{} [label=\"{}\", fillcolor={}, style=filled, shape=circle, fixedsize=true, width=1, height=1, fontsize=24];",
                   vertex_relative_idx,
                   vertex_relative_idx,
                   color,
            ).unwrap();
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

// In terms of vertex radius
const NODE_DISTANCE: f32 = 3.0;
const PARTITION_DISTANCE: f32 = 7.0;

impl<G> Draw for BipartiteSnort<G>
where
    G: Graph<VertexColor> + Clone,
{
    fn draw<C>(&self, canvas: &mut C)
    where
        C: Canvas,
    {
        // NOTE: This probably could be computed on the fly
        let mut positions = vec![V2f::ZERO; self.graph.size()];

        for (off, vertex_index) in self
            .graph
            .vertex_indices()
            .filter(|idx| *self.graph.get_vertex(*idx) == VertexColor::TintLeft)
            .enumerate()
        {
            positions[vertex_index.index] = V2f {
                x: C::vertex_radius(),
                y: C::vertex_radius() + (C::vertex_radius() * (2.0 + NODE_DISTANCE)) * off as f32,
            };
        }

        for (off, vertex_index) in self
            .graph
            .vertex_indices()
            .filter(|idx| *self.graph.get_vertex(*idx) == VertexColor::TintRight)
            .enumerate()
        {
            positions[vertex_index.index] = V2f {
                x: C::vertex_radius() * (3.0 + PARTITION_DISTANCE),
                y: C::vertex_radius() + (C::vertex_radius() * (2.0 + NODE_DISTANCE)) * off as f32,
            };
        }

        for this_vertex_idx in self.graph.vertex_indices() {
            for adjacent_vertex_idx in self.graph.adjacent_to(this_vertex_idx) {
                canvas.line(
                    positions[this_vertex_idx.index],
                    positions[adjacent_vertex_idx.index],
                    C::thin_line_weight(),
                    Color::BLACK,
                );
            }
        }

        for vertex_idx in self.graph.vertex_indices() {
            let position = positions[vertex_idx.index];
            let color = match self.graph.get_vertex(vertex_idx) {
                VertexColor::TintLeft => Color::BLUE,
                VertexColor::TintRight => Color::RED,
            };
            canvas.vertex(position, color, vertex_idx);
        }
    }

    fn required_canvas<C>(&self) -> BoundingBox
    where
        C: Canvas,
    {
        let blue = self
            .graph
            .vertex_indices()
            .filter(|idx| *self.graph.get_vertex(*idx) == VertexColor::TintLeft)
            .count();
        let red = self
            .graph
            .vertex_indices()
            .filter(|idx| *self.graph.get_vertex(*idx) == VertexColor::TintRight)
            .count();
        let higher = blue.max(red);

        BoundingBox {
            top_left: V2f::ZERO,
            bottom_right: V2f {
                x: (PARTITION_DISTANCE + 4.0) * C::vertex_radius(),
                y: (C::vertex_radius() * (2.0 + NODE_DISTANCE)) * higher as f32
                    - (C::vertex_radius() * NODE_DISTANCE),
            },
        }
    }
}

impl<G> PartizanGame for BipartiteSnort<G>
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

    fn reductions(&self) -> Option<CanonicalForm> {
        if self.graph.vertices().all(|v| *v == VertexColor::TintLeft) {
            return Some(CanonicalForm::new_integer(self.graph.size() as i64));
        }

        if self.graph.vertices().all(|v| *v == VertexColor::TintRight) {
            return Some(CanonicalForm::new_integer(-(self.graph.size() as i64)));
        }

        None
    }
}

/// Iterator of connected bipartite graphs
///
/// Can generate positions with up to 64 edges
pub struct BipartiteSnortIterator<G, F> {
    blue: u32,
    red: u32,
    current_mask: u64,
    callback: F,
    _graph: PhantomData<G>,
}

impl<G, F> core::fmt::Debug for BipartiteSnortIterator<G, F> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BipartiteSnortIterator")
            .field("blue", &self.blue)
            .field("red", &self.red)
            .field("current_mask", &self.current_mask)
            .finish_non_exhaustive()
    }
}

impl<G> BipartiteSnortIterator<G, fn()> {
    /// Create new position iterator
    pub const fn new(blue: u32, red: u32) -> Self {
        BipartiteSnortIterator::with_callback(blue, red, || {})
    }
}

impl<G, F> BipartiteSnortIterator<G, F> {
    /// Create new position iterator with callback on each attempt (possibly not connected)
    pub const fn with_callback(blue: u32, red: u32, callback: F) -> Self {
        let total_possible_edges = blue * red;

        assert!(
            total_possible_edges <= 64,
            "Total possible edges (blue * red) cannot exceed 64 due to bitmask limitations."
        );

        Self {
            blue,
            red,
            current_mask: 0,
            callback,
            _graph: PhantomData,
        }
    }

    const fn max_combinations(&self) -> u64 {
        Self::upper_bound(self.blue, self.red)
    }

    /// Get upper bound on the number of iterations
    pub const fn upper_bound(blue: u32, red: u32) -> u64 {
        let total_possible_edges = blue * red;
        if total_possible_edges == 64 {
            u64::MAX
        } else {
            1u64 << total_possible_edges
        }
    }
}

impl<G, F> Iterator for BipartiteSnortIterator<G, F>
where
    F: FnMut(),
    G: Graph<VertexColor>,
{
    type Item = (BipartiteGraph, BipartiteSnort<G>);

    fn next(&mut self) -> Option<Self::Item> {
        while self.current_mask < self.max_combinations() {
            (self.callback)();
            let mask = self.current_mask;
            self.current_mask += 1;

            let mut vertices = Vec::with_capacity((self.blue + self.red) as usize);
            vertices.resize(self.blue as usize, VertexColor::TintLeft);
            vertices.resize((self.blue + self.red) as usize, VertexColor::TintRight);
            let mut graph = G::empty(&vertices);

            let mut edge_index = 0;

            for u in 0..self.blue {
                for v in self.blue..(self.blue + self.red) {
                    if (mask & (1u64 << edge_index)) != 0 {
                        graph.connect(
                            VertexIndex { index: u as usize },
                            VertexIndex { index: v as usize },
                            true,
                        );
                    }
                    edge_index += 1;
                }
            }

            if graph.is_connected() {
                return Some((
                    BipartiteGraph {
                        blue: self.blue,
                        red: self.red,
                        mask,
                    },
                    BipartiteSnort::new(graph),
                ));
            }
        }

        None
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (
            0,
            Some((self.max_combinations() - self.current_mask) as usize),
        )
    }
}
