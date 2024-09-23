//! Simple graph implementation

use std::collections::VecDeque;

pub mod adjacency_matrix;
pub mod layout;

/// Graph vertex. We assume that all graphs that we implement use 0-based indexing for their vertices
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Vertex {
    /// 0-based index of a vertex in the graph
    pub index: usize,
}

/// Graph
pub trait Graph: Sized {
    /// Iterator over vertices
    type VertexIter: Iterator<Item = Vertex> + Clone;

    /// Iterator over adjacent veritcies adjacent to a given vertex
    type AdjacentIter<'g>: Iterator<Item = Vertex>
    where
        Self: 'g;

    /// Iterator over vertex degrees
    type DegreeIter<'g>: Iterator<Item = usize>
    where
        Self: 'g;

    /// Iterator over edges
    type EdgesIter<'g>: Iterator<Item = (Vertex, Vertex)>
    where
        Self: 'g;

    /// Create an empty graph without any edges between vertices
    fn empty(size: usize) -> Self;

    /// Get number of vertices in the graph.
    fn size(&self) -> usize;

    /// Get iterator over vertices
    fn vertices(&self) -> Self::VertexIter;

    /// Add a new disconnected vertex at the "end" of the graph
    fn add_vertex(&mut self) -> Vertex;

    /// Remove a given vertex from the graph, remove all its edges
    fn remove_vertex(&mut self, vertex_to_remove: Vertex);

    /// Add or remove edge between vertices
    fn connect(&mut self, lhs_vertex: Vertex, rhs_vertex: Vertex, connect: bool);

    /// Get iterator over vertices adjacent to given vertex
    fn adjacent_to<'g>(&'g self, vertex: Vertex) -> Self::AdjacentIter<'g>;

    /// Check if two vertices are adjacent
    fn are_adjacent(&self, lhs_vertex: Vertex, rhs_vertex: Vertex) -> bool {
        self.adjacent_to(lhs_vertex)
            .any(|adjacent| adjacent == rhs_vertex)
    }

    /// Get iterator over edges
    fn edges<'g>(&'g self) -> Self::EdgesIter<'g>;

    /// Get iterator over vertex degrees, in order
    fn degrees<'g>(&'g self) -> Self::DegreeIter<'g>;

    /// Create nw graph from "flat" adjacency matrix.
    ///
    /// # Errors
    /// - if `matrix.len() != size^2`
    #[inline]
    fn from_flat_matrix(size: usize, matrix: &[bool]) -> Option<Self> {
        if matrix.len() != size * size {
            return None;
        }

        let mut g = Self::empty(size);
        for u in g.vertices() {
            for v in g.vertices() {
                g.connect(u, v, matrix[size * u.index + v.index]);
            }
        }

        Some(g)
    }

    /// Create nw graph from adjacency matrix.
    #[inline]
    fn from_matrix(size: usize, matrix: &[&[bool]]) -> Option<Self> {
        let vec: Vec<bool> = matrix.iter().map(|r| r.iter()).flatten().copied().collect();
        Self::from_flat_matrix(size, &vec)
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
        let mut queue: VecDeque<Vertex> = VecDeque::with_capacity(self.size());

        seen[0] = true;
        queue.push_back(Vertex { index: 0 });

        while let Some(v) = queue.pop_front() {
            for u in self.vertices() {
                if self.are_adjacent(v, u) && v != u && !seen[u.index] {
                    seen[u.index] = true;
                    queue.push_back(u);
                }
            }
        }

        seen.into_iter().all(|b| b)
    }

    /// Create a graph from list of edges
    #[inline]
    fn from_edges(size: usize, edges: &[(Vertex, Vertex)]) -> Self {
        let mut graph = Self::empty(size);
        for (v, u) in edges {
            graph.connect(*v, *u, true);
        }
        graph
    }
}
