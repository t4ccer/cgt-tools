//! Undirected graph

use core::ops::Range;
use std::{collections::VecDeque, fmt::Display};

use super::directed;

/// Undirected graph
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Graph(directed::Graph);

impl Display for Graph {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl Graph {
    /// Create an empty graph without any edges between vertices
    #[inline]
    pub fn empty(size: usize) -> Self {
        Self(directed::Graph::empty(size))
    }

    /// Create a graph from flattened adjecency matrix. Must be correct length
    #[inline]
    pub fn from_vec(size: usize, vec: Vec<bool>) -> Option<Self> {
        Some(Self(directed::Graph::from_vec(size, vec)?))
    }

    /// Create a graph from adjecency matrix. Must be correct length
    #[inline]
    pub fn from_matrix(size: usize, matrix: &[Vec<bool>]) -> Option<Self> {
        Some(Self(directed::Graph::from_matrix(size, matrix)?))
    }

    /// Create a graph from list of edges
    #[inline]
    pub fn from_edges(size: usize, edges: &[(usize, usize)]) -> Self {
        let mut graph = Self::empty(size);
        for (v, u) in edges {
            graph.connect(*v, *u, true);
        }
        graph
    }

    /// Get number of vertices in the graph.
    #[inline]
    pub const fn size(&self) -> usize {
        self.0.size()
    }

    /// Check if two vertices are adjacent.
    #[inline]
    pub fn are_adjacent(&self, lhs_vertex: usize, rhs_vertex: usize) -> bool {
        self.0.are_adjacent(lhs_vertex, rhs_vertex)
    }

    /// Connect two vertices with an edge.
    #[inline]
    pub fn connect(&mut self, lhs_vertex: usize, rhs_vertex: usize, connect: bool) {
        self.0.connect(lhs_vertex, rhs_vertex, connect);
        self.0.connect(rhs_vertex, lhs_vertex, connect);
    }

    /// Get vertices adjacent to `vertex`.
    #[inline]
    pub fn adjacent_to(&self, vertex: usize) -> Vec<usize> {
        self.0.adjacent_to(vertex)
    }

    /// Get iterator over vertices
    #[inline]
    pub const fn vertices(&self) -> Range<usize> {
        self.0.vertices()
    }

    /// Add a new disconnected vertex at the end of the graph
    #[inline]
    pub fn add_vertex(&mut self) {
        self.0.add_vertex();
    }

    /// Remove a given vertex from the graph, remove all its edges
    #[inline]
    pub fn remove_vertex(&mut self, vertex_to_remove: usize) {
        self.0.remove_vertex(vertex_to_remove);
    }

    /// Get degrees of all vertices in the graph
    #[inline]
    pub fn degrees(&self) -> Vec<usize> {
        let mut degrees = vec![0; self.size()];
        for v in self.vertices() {
            for u in self.vertices() {
                if u != v && self.are_adjacent(v, u) {
                    degrees[v] += 1;
                }
            }
        }
        degrees
    }

    /// Get graph degree (highest vertex degree)
    #[inline]
    pub fn degree(&self) -> usize {
        *self
            .degrees()
            .iter()
            .max()
            .expect("graph to have at least 1 vertex")
    }

    /// Check if graph is connected
    #[inline]
    pub fn is_connected(&self) -> bool {
        if self.size() == 0 {
            return true;
        }

        let mut seen = vec![false; self.size()];
        let mut queue: VecDeque<usize> = VecDeque::with_capacity(self.size());

        seen[0] = true;
        queue.push_back(0);

        while let Some(v) = queue.pop_front() {
            for u in self.vertices() {
                if self.are_adjacent(v, u) && v != u && !seen[u] {
                    seen[u] = true;
                    queue.push_back(u);
                }
            }
        }

        seen.iter().all(|b| *b)
    }
}

/// ```text
/// 1 - 3 - 2
///  \  |
///   \ |
///     0
/// ```
#[cfg(test)]
fn test_matrix() -> Graph {
    let mut m = Graph::empty(4);
    m.connect(3, 0, true);
    m.connect(3, 2, true);
    m.connect(1, 3, true);
    m.connect(1, 0, true);
    m
}

#[test]
fn set_adjacency_matrix() {
    let m = test_matrix();
    assert_eq!(
        m,
        Graph::from_vec(
            4,
            vec![
                false, true, false, true, true, false, false, true, false, false, false, true,
                true, true, true, false
            ]
        )
        .unwrap()
    );
}

#[test]
fn test_adjacency() {
    let m = test_matrix();
    assert_eq!(m.adjacent_to(0), vec![1, 3]);
    assert_eq!(m.adjacent_to(1), vec![0, 3]);
    assert_eq!(m.adjacent_to(2), vec![3]);
    assert_eq!(m.adjacent_to(3), vec![0, 1, 2]);
}
