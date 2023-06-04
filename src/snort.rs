//! Game of Snort

use num_derive::FromPrimitive;
use std::fmt::Display;

use crate::{
    short_canonical_game::{GameId, Moves, PartizanShortGame},
    transposition_table::TranspositionTable,
};

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, FromPrimitive)]
#[repr(u8)]
pub enum VertexColor {
    Empty = 0,
    TintLeft = 1,
    TintRight = 2,
    Left = 3,
    Right = 4,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct AdjacencyMatrix {
    size: usize,
    adjacency_matrix: Vec<bool>,
}

impl Display for AdjacencyMatrix {
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

impl AdjacencyMatrix {
    #[inline]
    pub fn empty(size: usize) -> Self {
        Self {
            size,
            adjacency_matrix: vec![false; size * size],
        }
    }

    #[inline]
    pub fn from_vec(size: usize, vec: Vec<bool>) -> Self {
        assert!(vec.len() == size * size, "Invalid vector size");
        Self {
            size,
            adjacency_matrix: vec,
        }
    }

    #[inline]
    pub fn from_matrix(size: usize, matrix: Vec<Vec<bool>>) -> Self {
        let vec: Vec<bool> = matrix.iter().flatten().copied().collect();
        Self::from_vec(size, vec)
    }

    #[inline]
    pub fn at(&self, x: usize, y: usize) -> bool {
        self.adjacency_matrix[self.size * y + x]
    }

    #[inline]
    pub fn set(&mut self, x: usize, y: usize, value: bool) {
        self.adjacency_matrix[self.size * y + x] = value;
        self.adjacency_matrix[self.size * x + y] = value;
    }

    pub fn adjacent_to(&self, vertex: usize) -> Vec<usize> {
        let mut res = Vec::with_capacity(self.size);
        for idx in 0..self.size {
            if self.at(vertex, idx) {
                res.push(idx);
            }
        }
        res
    }
}

/// ```text
/// 1 - 3 - 2
///  \  |
///   \ |
///     0
/// ```
#[cfg(test)]
fn test_matrix() -> AdjacencyMatrix {
    let mut m = AdjacencyMatrix::empty(4);
    m.set(3, 0, true);
    m.set(3, 2, true);
    m.set(1, 3, true);
    m.set(1, 0, true);
    m
}

#[test]
fn set_adjacency_matrix() {
    let m = test_matrix();
    assert_eq!(
        m,
        AdjacencyMatrix::from_vec(
            4,
            vec![
                false, true, false, true, true, false, false, true, false, false, false, true,
                true, true, true, false
            ]
        )
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

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct Position {
    vertices: Vec<VertexColor>,
    adjacency_matrix: AdjacencyMatrix,
}

impl Position {
    pub fn new(vertices: Vec<VertexColor>, adjacency_matrix: AdjacencyMatrix) -> Self {
        assert_eq!(vertices.len(), adjacency_matrix.size);
        Self {
            vertices,
            adjacency_matrix,
        }
    }

    fn moves_for<const COLOR: u8>(&self) -> Vec<Self> {
        // const ADT generics are unsable, so here we go
        let own_tint_color: VertexColor = num::FromPrimitive::from_u8(COLOR).unwrap();

        let mut moves = Vec::with_capacity(self.adjacency_matrix.size);

        // Vertices where player can move
        let move_vertices = self
            .vertices
            .iter()
            .enumerate()
            .filter(|(_, vertex_color)| {
                **vertex_color == own_tint_color || **vertex_color == VertexColor::Empty
            })
            .map(|(idx, _)| idx);

        // Go through list of vertices with legal move
        for move_vertex in move_vertices {
            let mut position: Position = self.clone();

            // Take vertex
            position.vertices[move_vertex] = if own_tint_color == VertexColor::TintLeft {
                VertexColor::Left
            } else {
                debug_assert_eq!(own_tint_color, VertexColor::TintRight);
                VertexColor::Right
            };

            // Disconnect `move_vertex` from adjecent vertices and tint them
            for adjacent_vertex in self.adjacency_matrix.adjacent_to(move_vertex) {
                // Disconnect move vertex from adjecent
                position
                    .adjacency_matrix
                    .set(move_vertex, adjacent_vertex, false);

                // No loops
                if adjacent_vertex != move_vertex {
                    // Tint adjacent vertex
                    position.vertices[adjacent_vertex] = own_tint_color;
                }
            }
            moves.push(position);
        }
        moves
    }
}

impl PartizanShortGame for Position {
    fn left_moves(&self) -> Vec<Self> {
        self.moves_for::<{ VertexColor::TintLeft as u8 }>()
    }

    fn right_moves(&self) -> Vec<Self> {
        self.moves_for::<{ VertexColor::TintRight as u8 }>()
    }
}

impl Position {
    /// Get the canonical form of the position.
    ///
    /// # Arguments
    ///
    /// `cache` - Shared cache of short combinatorial games.
    ///
    /// # Examples
    ///
    /// ```
    /// use cgt::short_canonical_game::PartizanShortGame;
    /// use cgt::snort::{AdjacencyMatrix, Position, VertexColor};
    /// use cgt::transposition_table::TranspositionTable;
    ///
    /// let mut m = AdjacencyMatrix::empty(3);
    /// m.set(1, 2, true);
    ///
    /// let vs = vec![
    ///     VertexColor::TintLeft,
    ///     VertexColor::TintRight,
    ///     VertexColor::TintLeft,
    /// ];
    ///
    /// let position = Position::new(vs, m);
    /// assert_eq!(position.left_moves().len(), 2);
    /// assert_eq!(position.right_moves().len(), 1);
    ///
    /// let cache = TranspositionTable::new();
    /// let game = position.canonical_form(&cache);
    /// assert_eq!(&cache.game_backend().print_game_to_str(game), "{2|0}");
    /// ```
    pub fn canonical_form(&self, cache: &TranspositionTable<Self>) -> GameId {
        if let Some(id) = cache.grids.get(&self) {
            return id;
        }

        let left_moves = self.left_moves();
        let right_moves = self.right_moves();

        // NOTE: That's redundant, but may increase performance
        // `construct_from_moves` on empty moves results in 0 as well
        if left_moves.is_empty() && right_moves.is_empty() {
            return cache.game_backend.zero_id;
        }

        let moves = Moves {
            left: left_moves.iter().map(|o| o.canonical_form(cache)).collect(),
            right: right_moves
                .iter()
                .map(|o| o.canonical_form(cache))
                .collect(),
        };

        let canonical_form = cache.game_backend.construct_from_moves(moves);
        cache.grids.insert(self.clone(), canonical_form);
        canonical_form
    }
}

#[test]
fn no_moves() {
    let m = AdjacencyMatrix::empty(0);
    let vs = vec![];
    let position = Position::new(vs, m);
    assert_eq!(position.left_moves(), vec![]);
    assert_eq!(position.right_moves(), vec![]);
}

#[test]
fn canonical_form_works() {
    // 0     1 --- 2
    // left  right left
    let mut m = AdjacencyMatrix::empty(3);
    m.set(1, 2, true);

    let vs = vec![
        VertexColor::TintLeft,
        VertexColor::TintRight,
        VertexColor::TintLeft,
    ];

    let position = Position::new(vs, m);
    assert_eq!(position.left_moves().len(), 2);
    assert_eq!(position.right_moves().len(), 1);

    let cache = TranspositionTable::new();
    let game = position.canonical_form(&cache);
    assert_eq!(&cache.game_backend.print_game_to_str(game), "{2|0}");
}
