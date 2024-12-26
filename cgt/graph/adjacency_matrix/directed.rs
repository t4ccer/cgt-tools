//! Directed graph

use core::iter::FusedIterator;

use crate::graph::{Graph, VertexIndex};

/// Directed graph, implements [`Graph`] trait
#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DirectedGraph<V> {
    adjacency_matrix: Vec<bool>,
    vertices: Vec<V>,
}

impl<V> DirectedGraph<V> {
    /// Map vertex values
    pub fn map<R>(&self, f: impl FnMut(&V) -> R) -> DirectedGraph<R> {
        DirectedGraph {
            adjacency_matrix: self.adjacency_matrix.clone(),
            vertices: self.vertices.iter().map(f).collect::<Vec<_>>(),
        }
    }
}

impl<V> Graph<V> for DirectedGraph<V>
where
    V: Clone,
{
    type VertexIter = std::iter::Map<std::ops::Range<usize>, fn(usize) -> VertexIndex>;

    type AdjacentIter<'g> = AdjacentIter<'g, V> where V: 'g;

    type DegreeIter<'g> = DegreeIter<'g, V> where V: 'g;

    type EdgesIter<'g> = EdgesIter<'g, V> where V: 'g;

    #[inline]
    fn empty(vertices: &[V]) -> Self
    where
        Self: Sized,
    {
        let size = vertices.len();
        Self {
            adjacency_matrix: vec![false; size * size],
            vertices: vertices.to_vec(),
        }
    }

    #[inline]
    fn size(&self) -> usize {
        self.vertices.len()
    }

    fn vertex_indices(&self) -> Self::VertexIter {
        (0..self.size()).map(|index| VertexIndex { index })
    }

    #[inline]
    fn add_vertex(&mut self, vertex: V) -> VertexIndex {
        let new_vertex_idx = self.size();
        let mut new_adjacency_matrix = vec![false; (self.size() + 1) * (self.size() + 1)];
        for in_v in self.vertex_indices() {
            for out_v in self.vertex_indices() {
                new_adjacency_matrix[(self.size() + 1) * in_v.index + out_v.index] =
                    self.are_adjacent(in_v, out_v);
            }
        }
        self.vertices.push(vertex);
        self.adjacency_matrix = new_adjacency_matrix;
        VertexIndex {
            index: new_vertex_idx,
        }
    }

    fn remove_vertex(&mut self, vertex_to_remove: VertexIndex) {
        debug_assert!(self.size() > 0, "Graph has no vertices");
        let mut new_graph = Self {
            adjacency_matrix: vec![false; (self.size() - 1) * (self.size() - 1)],
            vertices: self.vertices.clone(),
        };
        new_graph.vertices.remove(vertex_to_remove.index);

        for in_v in new_graph.vertex_indices() {
            for out_v in new_graph.vertex_indices() {
                new_graph.connect(
                    out_v,
                    in_v,
                    self.are_adjacent(
                        // Branchless skip over vertex we're removing
                        VertexIndex {
                            index: out_v.index + (out_v >= vertex_to_remove) as usize,
                        },
                        VertexIndex {
                            index: in_v.index + (in_v >= vertex_to_remove) as usize,
                        },
                    ),
                );
            }
        }

        *self = new_graph;
    }

    fn remove_vertices(&mut self, vertices_to_remove: &mut [VertexIndex]) {
        if vertices_to_remove.is_empty() {
            return;
        }

        vertices_to_remove.sort_unstable();
        let mut vertices = Vec::with_capacity(self.size() - vertices_to_remove.len());
        let mut current_removed = 0;
        for v in self.vertex_indices() {
            if Some(v) == vertices_to_remove.get(current_removed).copied() {
                current_removed += 1;
            } else {
                vertices.push(self.vertices[v.index].clone());
            }
        }

        let mut new_graph = Self {
            adjacency_matrix: vec![false; vertices.len() * vertices.len()],
            vertices,
        };

        'loop_v: for v in self.vertex_indices() {
            let mut to_skip_v = 0;
            for to_remove in vertices_to_remove.iter() {
                if *to_remove == v {
                    continue 'loop_v;
                }
                if *to_remove > v {
                    break;
                }
                to_skip_v += 1;
            }

            'loop_u: for u in self.vertex_indices() {
                let mut to_skip_u = 0;
                for to_remove in vertices_to_remove.iter() {
                    if *to_remove == u {
                        continue 'loop_u;
                    }
                    if *to_remove > u {
                        break;
                    }
                    to_skip_u += 1;
                }

                new_graph.connect(
                    VertexIndex {
                        index: v.index - to_skip_v,
                    },
                    VertexIndex {
                        index: u.index - to_skip_u,
                    },
                    self.are_adjacent(v, u),
                );
            }
        }

        *self = new_graph;
    }

    fn connect(&mut self, lhs_vertex: VertexIndex, rhs_vertex: VertexIndex, connect: bool) {
        let size = self.size();
        self.adjacency_matrix[size * lhs_vertex.index + rhs_vertex.index] = connect;
    }

    fn adjacent_to(&self, vertex: VertexIndex) -> Self::AdjacentIter<'_> {
        AdjacentIter {
            vertex,
            idx: VertexIndex { index: 0 },
            graph: self,
        }
    }

    fn are_adjacent(&self, lhs_vertex: VertexIndex, rhs_vertex: VertexIndex) -> bool {
        self.adjacency_matrix[self.size() * lhs_vertex.index + rhs_vertex.index]
    }

    fn edges(&self) -> Self::EdgesIter<'_> {
        EdgesIter {
            u: VertexIndex { index: 0 },
            v: VertexIndex { index: 0 },
            graph: self,
        }
    }

    fn degrees(&self) -> Self::DegreeIter<'_> {
        DegreeIter {
            idx: VertexIndex { index: 0 },
            graph: self,
        }
    }

    fn vertex_degree(&self, vertex: VertexIndex) -> usize {
        self.vertex_indices()
            .filter(|&u| u != vertex && self.are_adjacent(vertex, u))
            .count()
    }

    #[inline]
    fn from_flat_matrix(vec: &[bool], vertices: &[V]) -> Option<Self> {
        let size = vertices.len();
        if vec.len() != size * size {
            return None;
        }

        Some(Self {
            adjacency_matrix: vec.to_vec(),
            vertices: vertices.to_vec(),
        })
    }

    fn from_edges(edges: &[(VertexIndex, VertexIndex)], vertices: &[V]) -> Self {
        let mut graph = Self {
            adjacency_matrix: vec![false; vertices.len() * vertices.len()],
            vertices: vertices.to_vec(),
        };

        for (u, v) in edges.iter().copied() {
            graph.connect(u, v, true);
        }

        graph
    }

    fn get_vertex(&self, vertex: VertexIndex) -> &V {
        &self.vertices[vertex.index]
    }

    fn get_vertex_mut(&mut self, vertex: VertexIndex) -> &mut V {
        &mut self.vertices[vertex.index]
    }
}

/// Iterator over graph edges, constructed with [`Graph::edges`].
pub struct EdgesIter<'graph, V> {
    u: VertexIndex,
    v: VertexIndex,
    graph: &'graph DirectedGraph<V>,
}

impl<'graph, V> Iterator for EdgesIter<'graph, V>
where
    V: Clone,
{
    type Item = (VertexIndex, VertexIndex);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.u.index >= self.graph.size() {
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

/// Iterator of adjacent vertices. Obtained by calling [`Graph::adjacent_to`]
#[derive(Debug)]
pub struct AdjacentIter<'graph, V> {
    vertex: VertexIndex,
    idx: VertexIndex,
    graph: &'graph DirectedGraph<V>,
}

impl<'graph, V> Iterator for AdjacentIter<'graph, V>
where
    V: Clone,
{
    type Item = VertexIndex;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.idx.index >= self.graph.size() {
                return None;
            }
            if self.graph.are_adjacent(self.vertex, self.idx) {
                let res = Some(self.idx);
                self.idx.index += 1;
                return res;
            }
            self.idx.index += 1;
        }
    }
}

impl<'graph, V> FusedIterator for AdjacentIter<'graph, V> where V: Clone {}

/// Iterator over degrees of vertices in a graph. Obtained with [`Graph::degrees`]
#[derive(Debug)]
pub struct DegreeIter<'graph, V> {
    idx: VertexIndex,
    graph: &'graph DirectedGraph<V>,
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
/// 1 -> 3 -> 2
///  \   |
///   \  v
///    > 0
/// ```
#[cfg(test)]
fn test_matrix() -> DirectedGraph<()> {
    let mut m = DirectedGraph::empty(&[(), (), (), ()]);
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
        DirectedGraph::from_flat_matrix(
            &[
                false, false, false, false, //
                true, false, false, true, //
                false, false, false, false, //
                true, false, true, false, //
            ],
            &[(), (), (), ()]
        )
        .unwrap()
    );
}

#[test]
fn test_adjacency() {
    let m = test_matrix();
    assert_eq!(
        m.adjacent_to(VertexIndex { index: 0 }).collect::<Vec<_>>(),
        vec![]
    );
    assert_eq!(
        m.adjacent_to(VertexIndex { index: 1 }).collect::<Vec<_>>(),
        vec![VertexIndex { index: 0 }, VertexIndex { index: 3 }]
    );
    assert_eq!(
        m.adjacent_to(VertexIndex { index: 2 }).collect::<Vec<_>>(),
        vec![]
    );
    assert_eq!(
        m.adjacent_to(VertexIndex { index: 3 }).collect::<Vec<_>>(),
        vec![VertexIndex { index: 0 }, VertexIndex { index: 2 }]
    );
}

#[test]
fn test_edges() {
    let m = test_matrix();
    assert_eq!(
        m.edges().collect::<Vec<_>>(),
        vec![
            (VertexIndex { index: 1 }, VertexIndex { index: 0 }),
            (VertexIndex { index: 3 }, VertexIndex { index: 0 }),
            (VertexIndex { index: 3 }, VertexIndex { index: 2 }),
            (VertexIndex { index: 1 }, VertexIndex { index: 3 })
        ]
    );
}
