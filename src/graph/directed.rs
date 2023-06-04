use std::fmt::Display;

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct Graph {
    size: usize,
    adjacency_matrix: Vec<bool>,
}

impl Display for Graph {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (idx, elem) in self.adjacency_matrix.iter().enumerate() {
            write!(f, "{}", *elem as u8)?;
            if (idx + 1) % self.size == 0 {
                write!(f, "\n")?;
            }
        }

        Ok(())
    }
}

impl Graph {
    /// Create an empty graph without any edges between vertices
    #[inline]
    pub fn empty(size: usize) -> Self {
        Self {
            size,
            adjacency_matrix: vec![false; size * size],
        }
    }

    #[inline]
    pub fn from_vec(size: usize, vec: Vec<bool>) -> Option<Self> {
        if vec.len() != size * size {
            return None;
        }

        Some(Self {
            size,
            adjacency_matrix: vec,
        })
    }

    #[inline]
    pub fn from_matrix(size: usize, matrix: Vec<Vec<bool>>) -> Option<Self> {
        let vec: Vec<bool> = matrix.iter().flatten().copied().collect();
        Self::from_vec(size, vec)
    }

    /// Get number of vertices in the graph.
    #[inline]
    pub fn size(&self) -> usize {
        self.size
    }

    /// Check if two vertices are adjacent.
    #[inline]
    pub fn are_adjacent(&self, out_vertex: usize, in_vertex: usize) -> bool {
        self.adjacency_matrix[self.size * in_vertex + out_vertex]
    }

    /// Connect two vertices with an edge.
    #[inline]
    pub fn connect(&mut self, out_vertex: usize, in_vertex: usize, connect: bool) {
        self.adjacency_matrix[self.size * in_vertex + out_vertex] = connect;
    }

    /// Get vertices adjacent to `out_vertex`.
    pub fn adjacent_to(&self, out_vertex: usize) -> Vec<usize> {
        let mut res = Vec::with_capacity(self.size);
        for idx in 0..self.size {
            if self.are_adjacent(out_vertex, idx) {
                res.push(idx);
            }
        }
        res
    }
}

/// ```text
/// 1 -> 3 -> 2
///  \   |
///   \  v
///    > 0
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
                false, true, false, true, false, false, false, false, false, false, false, true,
                false, true, false, false
            ]
        )
        .unwrap()
    );
}

#[test]
fn test_adjacency() {
    let m = test_matrix();
    assert_eq!(m.adjacent_to(0), vec![]);
    assert_eq!(m.adjacent_to(1), vec![0, 3]);
    assert_eq!(m.adjacent_to(2), vec![]);
    assert_eq!(m.adjacent_to(3), vec![0, 2]);
}
