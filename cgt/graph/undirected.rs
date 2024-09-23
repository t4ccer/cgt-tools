//! Undirected graph

use core::ops::Range;
use std::{fmt::Display, iter::FusedIterator};

use super::{
    directed::{self, AdjacentIter},
    Graph,
};

/// Undirected graph
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct UndirectedGraph(directed::DirectedGraph);

impl Display for UndirectedGraph {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl Graph for UndirectedGraph {
    type VertexIter = Range<usize>;

    type AdjacentIter<'g> = AdjacentIter<'g>;

    type DegreeIter<'g> = DegreeIter<'g>;

    type EdgesIter<'g> = EdgesIter<'g>;

    fn empty(size: usize) -> Self {
        Self(directed::DirectedGraph::empty(size))
    }

    fn size(&self) -> usize {
        self.0.size()
    }

    fn vertices(&self) -> Self::VertexIter {
        self.0.vertices()
    }

    fn add_vertex(&mut self) -> usize {
        self.0.add_vertex()
    }

    fn remove_vertex(&mut self, vertex_to_remove: usize) {
        self.0.remove_vertex(vertex_to_remove)
    }

    fn connect(&mut self, lhs_vertex: usize, rhs_vertex: usize, connect: bool) {
        self.0.connect(lhs_vertex, rhs_vertex, connect);
        self.0.connect(rhs_vertex, lhs_vertex, connect);
    }

    fn adjacent_to<'g>(&'g self, vertex: usize) -> Self::AdjacentIter<'g> {
        self.0.adjacent_to(vertex)
    }

    fn are_adjacent(&self, lhs_vertex: usize, rhs_vertex: usize) -> bool {
        self.0.are_adjacent(lhs_vertex, rhs_vertex)
    }

    fn edges<'g>(&'g self) -> Self::EdgesIter<'g> {
        EdgesIter {
            u: 0,
            v: 0,
            graph: self,
        }
    }

    fn degrees<'g>(&'g self) -> Self::DegreeIter<'g> {
        DegreeIter {
            idx: 0,
            graph: self,
        }
    }

    /// Create a graph from flattened adjecency matrix. Must be correct length
    #[inline]
    fn from_flat_matrix(size: usize, vec: &[bool]) -> Option<Self> {
        Some(Self(directed::DirectedGraph::from_flat_matrix(size, vec)?))
    }

    /// Create a graph from adjecency matrix. Must be correct length
    #[inline]
    fn from_matrix(size: usize, matrix: &[&[bool]]) -> Option<Self> {
        Some(Self(directed::DirectedGraph::from_matrix(size, matrix)?))
    }
}

/// Iterator over graph edges, constructed with [`Graph::edges`].
pub struct EdgesIter<'graph> {
    u: usize,
    v: usize,
    graph: &'graph UndirectedGraph,
}

impl<'graph> Iterator for EdgesIter<'graph> {
    type Item = (usize, usize);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            // graph is undirected so we need to iterate only the triangle
            if self.u > self.v {
                self.u = 0;
                self.v += 1;
            }

            if self.v >= self.graph.size() {
                return None;
            }

            if self.graph.are_adjacent(self.u, self.v) {
                let res = Some((self.u, self.v));
                self.u += 1;
                return res;
            }
            self.u += 1;
        }
    }
}

impl<'graph> FusedIterator for EdgesIter<'graph> {}

/// Iterator over degrees of vertices in a graph. Obtained with [`Graph::degrees`]
#[derive(Debug)]
pub struct DegreeIter<'graph> {
    idx: usize,
    graph: &'graph UndirectedGraph,
}

impl<'graph> Iterator for DegreeIter<'graph> {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        if self.idx >= self.graph.size() {
            return None;
        }

        let res = self
            .graph
            .vertices()
            .filter(|&u| u != self.idx && self.graph.are_adjacent(self.idx, u))
            .count();
        self.idx += 1;
        Some(res)
    }
}

impl<'graph> FusedIterator for DegreeIter<'graph> {}

/// ```text
/// 1 - 3 - 2
///  \  |
///   \ |
///     0
/// ```
#[cfg(test)]
fn test_matrix() -> UndirectedGraph {
    let mut m = UndirectedGraph::empty(4);
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
        UndirectedGraph::from_flat_matrix(
            4,
            &[
                false, true, false, true, true, false, false, true, false, false, false, true,
                true, true, true, false
            ]
        )
        .unwrap()
    );
}

#[test]
fn adjacency() {
    let m = test_matrix();
    assert_eq!(m.adjacent_to(0).collect::<Vec<_>>(), vec![1, 3]);
    assert_eq!(m.adjacent_to(1).collect::<Vec<_>>(), vec![0, 3]);
    assert_eq!(m.adjacent_to(2).collect::<Vec<_>>(), vec![3]);
    assert_eq!(m.adjacent_to(3).collect::<Vec<_>>(), vec![0, 1, 2]);
}

#[test]
fn degrees() {
    let m = test_matrix();
    assert_eq!(m.degrees().collect::<Vec<_>>(), vec![2, 2, 1, 3]);
    assert_eq!(m.degree(), 3);
}

#[test]
fn edges() {
    let m = test_matrix();
    assert_eq!(
        m.edges().collect::<Vec<_>>(),
        vec![(0, 1), (0, 3), (1, 3), (2, 3)]
    );
}

#[test]
fn connected() {
    let m = test_matrix();
    assert!(m.is_connected());
}
