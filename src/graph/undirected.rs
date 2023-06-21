use std::fmt::Display;

use super::directed;

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct Graph(directed::Graph);

impl Display for Graph {
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

    #[inline]
    pub fn from_vec(size: usize, vec: Vec<bool>) -> Option<Self> {
        Some(Self(directed::Graph::from_vec(size, vec)?))
    }

    #[inline]
    pub fn from_matrix(size: usize, matrix: Vec<Vec<bool>>) -> Option<Self> {
        Some(Self(directed::Graph::from_matrix(size, matrix)?))
    }

    /// Get number of vertices in the graph.
    #[inline]
    pub fn size(&self) -> usize {
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
    pub fn adjacent_to(&self, vertex: usize) -> Vec<usize> {
        self.0.adjacent_to(vertex)
    }

    /// Add a new disconnected vertex at the end of the graph
    pub fn add_vertex(&mut self) {
        self.0.add_vertex();
    }

    /// Remove a given vertex from the graph, remove all its edges
    pub fn remove_vertex(&mut self, vertex_to_remove: usize) {
        self.0.remove_vertex(vertex_to_remove);
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
