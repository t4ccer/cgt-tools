//! Undirected graph

use core::iter::FusedIterator;

use crate::graph::{
    adjacency_matrix::directed::{self, AdjacentIter},
    Graph, VertexIndex,
};

/// Undirected graph, implements [`Graph`] trait
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct UndirectedGraph<V>(directed::DirectedGraph<V>);

impl<V> UndirectedGraph<V> {
    /// Map vertex values
    pub fn map<R>(&self, f: impl FnMut(&V) -> R) -> UndirectedGraph<R> {
        UndirectedGraph(self.0.map(f))
    }
}

impl<V> Graph<V> for UndirectedGraph<V>
where
    V: Clone,
{
    type VertexIter = std::iter::Map<std::ops::Range<usize>, fn(usize) -> VertexIndex>;

    type AdjacentIter<'g> = AdjacentIter<'g, V> where V: 'g;

    type DegreeIter<'g> = DegreeIter<'g, V> where V: 'g;

    type EdgesIter<'g> = EdgesIter<'g, V> where V: 'g;

    fn empty(vertices: &[V]) -> Self {
        Self(directed::DirectedGraph::empty(vertices))
    }

    fn size(&self) -> usize {
        self.0.size()
    }

    fn vertex_indices(&self) -> Self::VertexIter {
        self.0.vertex_indices()
    }

    fn add_vertex(&mut self, vertex: V) -> VertexIndex {
        self.0.add_vertex(vertex)
    }

    fn remove_vertex(&mut self, vertex_to_remove: VertexIndex) {
        self.0.remove_vertex(vertex_to_remove)
    }

    fn connect(&mut self, lhs_vertex: VertexIndex, rhs_vertex: VertexIndex, connect: bool) {
        self.0.connect(lhs_vertex, rhs_vertex, connect);
        self.0.connect(rhs_vertex, lhs_vertex, connect);
    }

    fn adjacent_to<'g>(&'g self, vertex: VertexIndex) -> Self::AdjacentIter<'g> {
        self.0.adjacent_to(vertex)
    }

    fn are_adjacent(&self, lhs_vertex: VertexIndex, rhs_vertex: VertexIndex) -> bool {
        self.0.are_adjacent(lhs_vertex, rhs_vertex)
    }

    fn edges<'g>(&'g self) -> Self::EdgesIter<'g> {
        EdgesIter {
            u: VertexIndex { index: 0 },
            v: VertexIndex { index: 0 },
            graph: self,
        }
    }

    fn degrees<'g>(&'g self) -> Self::DegreeIter<'g> {
        DegreeIter {
            idx: VertexIndex { index: 0 },
            graph: self,
        }
    }

    fn vertex_degree(&self, vertex: VertexIndex) -> usize {
        self.0.vertex_degree(vertex)
    }

    #[inline]
    fn from_flat_matrix(vec: &[bool], vertices: &[V]) -> Option<Self> {
        Some(Self(directed::DirectedGraph::from_flat_matrix(
            vec, vertices,
        )?))
    }

    #[inline]
    fn from_matrix(matrix: &[&[bool]], vertices: &[V]) -> Option<Self> {
        Some(Self(directed::DirectedGraph::from_matrix(
            matrix, vertices,
        )?))
    }

    fn from_edges(edges: &[(VertexIndex, VertexIndex)], vertices: &[V]) -> Self {
        let mut graph = Self::empty(vertices);

        for (u, v) in edges.iter().copied() {
            graph.connect(u, v, true);
        }

        graph
    }

    fn get_vertex(&self, vertex: VertexIndex) -> &V {
        self.0.get_vertex(vertex)
    }

    fn get_vertex_mut(&mut self, vertex: VertexIndex) -> &mut V {
        self.0.get_vertex_mut(vertex)
    }
}

/// Iterator over graph edges, constructed with [`Graph::edges`].
pub struct EdgesIter<'graph, V> {
    u: VertexIndex,
    v: VertexIndex,
    graph: &'graph UndirectedGraph<V>,
}

impl<'graph, V> Iterator for EdgesIter<'graph, V>
where
    V: Clone,
{
    type Item = (VertexIndex, VertexIndex);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            // graph is undirected so we need to iterate only the triangle
            if self.u > self.v {
                self.u.index = 0;
                self.v.index += 1;
            }

            if self.v.index >= self.graph.size() {
                return None;
            }

            if self.graph.are_adjacent(self.u, self.v) {
                let res = Some((self.u, self.v));
                self.u.index += 1;
                return res;
            }
            self.u.index += 1;
        }
    }
}

impl<'graph, V> FusedIterator for EdgesIter<'graph, V> where V: Clone {}

/// Iterator over degrees of vertices in a graph. Obtained with [`Graph::degrees`]
#[derive(Debug)]
pub struct DegreeIter<'graph, V> {
    idx: VertexIndex,
    graph: &'graph UndirectedGraph<V>,
}

impl<'graph, V> Iterator for DegreeIter<'graph, V>
where
    V: Clone,
{
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        if self.idx.index >= self.graph.size() {
            return None;
        }

        let res = self
            .graph
            .vertex_indices()
            .filter(|&u| u != self.idx && self.graph.are_adjacent(self.idx, u))
            .count();
        self.idx.index += 1;
        Some(res)
    }
}

impl<'graph, V> FusedIterator for DegreeIter<'graph, V> where V: Clone {}

/// ```text
/// 1 - 3 - 2
///  \  |
///   \ |
///     0
/// ```
#[cfg(test)]
fn test_matrix() -> UndirectedGraph<()> {
    let mut m = UndirectedGraph::empty(&[(), (), (), ()]);
    m.connect(VertexIndex { index: 3 }, VertexIndex { index: 0 }, true);
    m.connect(VertexIndex { index: 3 }, VertexIndex { index: 2 }, true);
    m.connect(VertexIndex { index: 1 }, VertexIndex { index: 3 }, true);
    m.connect(VertexIndex { index: 1 }, VertexIndex { index: 0 }, true);
    m
}

#[test]
fn set_adjacency_matrix() {
    let m = test_matrix();
    assert_eq!(
        m,
        UndirectedGraph::from_flat_matrix(
            &[
                false, true, false, true, true, false, false, true, false, false, false, true,
                true, true, true, false
            ],
            &[(), (), (), ()]
        )
        .unwrap()
    );
}

#[test]
fn adjacency() {
    let m = test_matrix();
    assert_eq!(
        m.adjacent_to(VertexIndex { index: 0 }).collect::<Vec<_>>(),
        vec![VertexIndex { index: 1 }, VertexIndex { index: 3 }]
    );
    assert_eq!(
        m.adjacent_to(VertexIndex { index: 1 }).collect::<Vec<_>>(),
        vec![VertexIndex { index: 0 }, VertexIndex { index: 3 }]
    );
    assert_eq!(
        m.adjacent_to(VertexIndex { index: 2 }).collect::<Vec<_>>(),
        vec![VertexIndex { index: 3 }]
    );
    assert_eq!(
        m.adjacent_to(VertexIndex { index: 3 }).collect::<Vec<_>>(),
        vec![
            VertexIndex { index: 0 },
            VertexIndex { index: 1 },
            VertexIndex { index: 2 }
        ]
    );
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
        vec![
            (VertexIndex { index: 0 }, VertexIndex { index: 1 }),
            (VertexIndex { index: 0 }, VertexIndex { index: 3 }),
            (VertexIndex { index: 1 }, VertexIndex { index: 3 }),
            (VertexIndex { index: 2 }, VertexIndex { index: 3 })
        ]
    );
}

#[test]
fn connected() {
    let m = test_matrix();
    assert!(m.is_connected());
}
