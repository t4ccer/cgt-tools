//! Directed graph

use core::ops::Range;
use std::{fmt::Display, iter::FusedIterator};

use super::Graph;

/// Directed graph
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DirectedGraph {
    size: usize,
    adjacency_matrix: Vec<bool>,
}

impl Display for DirectedGraph {
    #[allow(clippy::missing_inline_in_public_items)]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (idx, elem) in self.adjacency_matrix.iter().enumerate() {
            write!(f, "{}", u8::from(*elem))?;
            if (idx + 1) % self.size == 0 {
                writeln!(f)?;
            }
        }

        Ok(())
    }
}

impl Graph for DirectedGraph {
    type VertexIter = Range<usize>;

    type AdjacentIter<'g> = AdjacentIter<'g>;

    type DegreeIter<'g> = DegreeIter<'g>;

    type EdgesIter<'g> = EdgesIter<'g>;

    #[inline]
    fn empty(size: usize) -> Self
    where
        Self: Sized,
    {
        Self {
            size,
            adjacency_matrix: vec![false; size * size],
        }
    }

    #[inline]
    fn size(&self) -> usize {
        self.size
    }

    fn vertices<'g>(&'g self) -> Self::VertexIter {
        0..self.size()
    }

    #[inline]
    fn add_vertex(&mut self) -> usize {
        let new_vertex = self.size();
        let mut new_graph = Self::empty(self.size() + 1);
        for in_v in self.vertices() {
            for out_v in self.vertices() {
                new_graph.connect(out_v, in_v, self.are_adjacent(out_v, in_v));
            }
        }
        *self = new_graph;
        new_vertex
    }

    fn remove_vertex(&mut self, vertex_to_remove: usize) {
        debug_assert!(self.size() > 0, "Graph has no vertices");
        let mut new_graph = Self::empty(self.size() - 1);

        for in_v in new_graph.vertices() {
            for out_v in new_graph.vertices() {
                new_graph.connect(
                    out_v,
                    in_v,
                    self.are_adjacent(
                        // Skip over vertex we're removing
                        out_v + (out_v >= vertex_to_remove) as usize,
                        in_v + (in_v >= vertex_to_remove) as usize,
                    ),
                );
            }
        }

        *self = new_graph;
    }

    fn connect(&mut self, lhs_vertex: usize, rhs_vertex: usize, connect: bool) {
        self.adjacency_matrix[self.size * lhs_vertex + rhs_vertex] = connect;
    }

    fn adjacent_to<'g>(&'g self, vertex: usize) -> Self::AdjacentIter<'g> {
        AdjacentIter {
            vertex,
            idx: 0,
            graph: self,
        }
    }

    fn are_adjacent(&self, lhs_vertex: usize, rhs_vertex: usize) -> bool {
        self.adjacency_matrix[self.size * lhs_vertex + rhs_vertex]
    }

    #[inline]
    fn from_flat_matrix(size: usize, vec: &[bool]) -> Option<Self> {
        if vec.len() != size * size {
            return None;
        }

        Some(Self {
            size,
            adjacency_matrix: vec.to_vec(),
        })
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
}

/// Iterator over graph edges, constructed with [`Graph::edges`].
pub struct EdgesIter<'graph> {
    u: usize,
    v: usize,
    graph: &'graph DirectedGraph,
}

impl<'graph> Iterator for EdgesIter<'graph> {
    type Item = (usize, usize);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.u >= self.graph.size() {
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

/// Iterator of adjacent vertices. Obtained by calling [`Graph::adjacent_to`]
#[derive(Debug)]
pub struct AdjacentIter<'graph> {
    vertex: usize,
    idx: usize,
    graph: &'graph DirectedGraph,
}

impl<'graph> Iterator for AdjacentIter<'graph> {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.idx >= self.graph.size {
                return None;
            }
            if self.graph.are_adjacent(self.vertex, self.idx) {
                let res = Some(self.idx);
                self.idx += 1;
                return res;
            }
            self.idx += 1;
        }
    }
}

impl<'graph> FusedIterator for AdjacentIter<'graph> {}

/// Iterator over degrees of vertices in a graph. Obtained with [`Graph::degrees`]
#[derive(Debug)]
pub struct DegreeIter<'graph> {
    idx: usize,
    graph: &'graph DirectedGraph,
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

#[test]
fn adds_new_vertex() {
    let mut g = test_matrix();
    assert_eq!(
        &format!("{g}"),
        "0000\n\
         1001\n\
         0000\n\
         1010\n"
    );

    // adds one empty row and column to previous matrix
    g.add_vertex();
    assert_eq!(
        &format!("{g}"),
        "00000\n\
         10010\n\
         00000\n\
         10100\n\
         00000\n"
    );

    g.remove_vertex(1);
    assert_eq!(
        &format!("{g}"),
        "0000\n\
         0000\n\
         1100\n\
         0000\n"
    );
}

/// ```text
/// 1 -> 3 -> 2
///  \   |
///   \  v
///    > 0
/// ```
#[cfg(test)]
fn test_matrix() -> DirectedGraph {
    let mut m = DirectedGraph::empty(4);
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
        DirectedGraph::from_flat_matrix(
            4,
            &[
                false, false, false, false, //
                true, false, false, true, //
                false, false, false, false, //
                true, false, true, false, //
            ]
        )
        .unwrap()
    );
}

#[test]
fn test_adjacency() {
    let m = test_matrix();
    assert_eq!(m.adjacent_to(0).collect::<Vec<_>>(), vec![]);
    assert_eq!(m.adjacent_to(1).collect::<Vec<_>>(), vec![0, 3]);
    assert_eq!(m.adjacent_to(2).collect::<Vec<_>>(), vec![]);
    assert_eq!(m.adjacent_to(3).collect::<Vec<_>>(), vec![0, 2]);
}

#[test]
fn test_edges() {
    let m = test_matrix();
    assert_eq!(
        m.edges().collect::<Vec<_>>(),
        vec![(1, 0), (3, 0), (3, 2), (1, 3)]
    );
}
