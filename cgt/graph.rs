//! Simple graph implementation

use std::collections::VecDeque;

use crate::{
    drawing::{BoundingBox, Canvas, Color},
    has::Has,
    numeric::v2f::V2f,
};

pub mod adjacency_matrix;
pub mod layout;

/// Graph vertex. We assume that all graphs that we implement use 0-based indexing for their vertices
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VertexIndex {
    /// 0-based index of a vertex in the graph
    pub index: usize,
}

/// Graph
pub trait Graph<V>: Sized {
    /// Iterator over vertices
    type VertexIter: Iterator<Item = VertexIndex> + Clone;

    /// Iterator over adjacent veritcies adjacent to a given vertex
    type AdjacentIter<'g>: Iterator<Item = VertexIndex>
    where
        Self: 'g;

    /// Iterator over vertex degrees
    type DegreeIter<'g>: Iterator<Item = usize>
    where
        Self: 'g;

    /// Iterator over edges
    type EdgesIter<'g>: Iterator<Item = (VertexIndex, VertexIndex)>
    where
        Self: 'g;

    /// Create an empty graph without any edges between vertices
    fn empty(vertices: &[V]) -> Self;

    /// Get number of vertices in the graph.
    fn size(&self) -> usize;

    /// Get iterator over vertices
    fn vertex_indices(&self) -> Self::VertexIter;

    /// Add a new disconnected vertex at the "end" of the graph
    fn add_vertex(&mut self, vertex: V) -> VertexIndex;

    /// Remove a given vertex from the graph, remove all its edges
    fn remove_vertex(&mut self, vertex_to_remove: VertexIndex);

    /// Remove vertices from the graph and their edges
    fn remove_vertices(&mut self, vertices_to_remove: &mut [VertexIndex]) {
        // Vertex indices shift as we remove them so we need to sort them
        // to remove them in correct order and adjust indices as we go
        vertices_to_remove.sort_unstable();
        for (idx, v) in vertices_to_remove.iter_mut().enumerate() {
            self.remove_vertex(VertexIndex {
                index: v.index - idx,
            });
        }
    }

    /// Add or remove edge between vertices
    fn connect(&mut self, lhs_vertex: VertexIndex, rhs_vertex: VertexIndex, connect: bool);

    /// Get iterator over vertices adjacent to given vertex
    fn adjacent_to(&self, vertex: VertexIndex) -> Self::AdjacentIter<'_>;

    /// Check if two vertices are adjacent
    /// (i.e. there exists an out-edge from `lhs_vertex` to `rhs_vertex`)
    fn are_adjacent(&self, lhs_vertex: VertexIndex, rhs_vertex: VertexIndex) -> bool {
        self.adjacent_to(lhs_vertex)
            .any(|adjacent| adjacent == rhs_vertex)
    }

    /// Get iterator over edges
    fn edges(&self) -> Self::EdgesIter<'_>;

    /// Get iterator over vertex degrees, in order
    fn degrees(&self) -> Self::DegreeIter<'_>;

    /// Get degree of a vertex
    fn vertex_degree(&self, vertex: VertexIndex) -> usize;

    /// Create nw graph from "flat" adjacency matrix.
    ///
    /// # Errors
    /// - if `matrix.len() != size^2`
    fn from_flat_matrix(matrix: &[bool], vertices: &[V]) -> Option<Self>;

    /// Create nw graph from adjacency matrix.
    #[inline]
    fn from_matrix(matrix: &[&[bool]], vertices: &[V]) -> Option<Self> {
        let vec: Vec<bool> = matrix.iter().flat_map(|r| r.iter()).copied().collect();
        Self::from_flat_matrix(&vec, vertices)
    }

    /// Get graph degree (highest vertex degree)
    #[inline]
    fn degree(&self) -> usize {
        self.degrees().max().unwrap_or(0)
    }

    /// Check if graph is connected
    #[inline]
    fn is_connected(&self) -> bool {
        if self.size() == 0 {
            return true;
        }

        let mut seen = vec![false; self.size()];
        let mut queue: VecDeque<VertexIndex> = VecDeque::with_capacity(self.size());

        seen[0] = true;
        queue.push_back(VertexIndex { index: 0 });

        while let Some(v) = queue.pop_front() {
            for u in self.vertex_indices() {
                if self.are_adjacent(v, u) && v != u && !seen[u.index] {
                    seen[u.index] = true;
                    queue.push_back(u);
                }
            }
        }

        seen.into_iter().all(|b| b)
    }

    /// Create a graph from list of edges
    fn from_edges(edges: &[(VertexIndex, VertexIndex)], vertices: &[V]) -> Self;

    /// Get vertex
    fn get_vertex(&self, vertex: VertexIndex) -> &V;

    /// Get vertex mutably
    fn get_vertex_mut(&mut self, vertex: VertexIndex) -> &mut V;

    /// Draw graph on existing canvas
    fn draw<C>(&self, canvas: &mut C, mut get_vertex_color: impl FnMut(&V) -> Color)
    where
        V: Has<V2f>,
        C: Canvas,
    {
        // TODO: Move up graph drawing code to Graph trait
        for this_vertex_idx in self.vertex_indices() {
            let this_position: V2f = *self.get_vertex(this_vertex_idx).get_inner();
            for adjacent_vertex_id in self.adjacent_to(this_vertex_idx) {
                let adjacent_position: V2f = *self.get_vertex(adjacent_vertex_id).get_inner();
                canvas.line(
                    this_position,
                    adjacent_position,
                    C::thin_line_weight(),
                    Color::BLACK,
                );
            }
        }

        for this_vertex_idx in self.vertex_indices() {
            let this_position: V2f = *self.get_vertex(this_vertex_idx).get_inner();
            canvas.vertex(
                this_position,
                get_vertex_color(self.get_vertex(this_vertex_idx)),
                this_vertex_idx,
            );
        }
    }

    /// Get required canvas size to fit whole graph
    fn required_canvas<C>(&self) -> BoundingBox
    where
        V: Has<V2f>,
        C: Canvas,
    {
        if self.size() == 0 {
            return BoundingBox {
                top_left: V2f::ZERO,
                bottom_right: V2f::ZERO,
            };
        }

        let r = C::node_radius();
        self.vertex_indices()
            .map(|idx| *self.get_vertex(idx).get_inner())
            .fold(
                BoundingBox {
                    top_left: V2f {
                        x: f32::INFINITY,
                        y: f32::INFINITY,
                    },
                    bottom_right: V2f {
                        x: f32::NEG_INFINITY,
                        y: f32::NEG_INFINITY,
                    },
                },
                |bounding_box, v: V2f| BoundingBox {
                    top_left: V2f {
                        x: f32::min(bounding_box.top_left.x, v.x - r),
                        y: f32::min(bounding_box.top_left.y, v.y - r),
                    },
                    bottom_right: V2f {
                        x: f32::max(bounding_box.bottom_right.x, v.x + r),
                        y: f32::max(bounding_box.bottom_right.y, v.y + r),
                    },
                },
            )
    }
}
