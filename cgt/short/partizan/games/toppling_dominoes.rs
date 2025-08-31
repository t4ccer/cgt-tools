//! Toppling Dominoes is played on a multiple rows of dominoes where each player in their turns
//! chooses one of the dominoes of their color and topples it to the left (or right) removing it
//! and all other dominoes to the left (or right) of it.

use std::fmt::Display;

use crate::{
    grid::{CharTile, FiniteGrid, Grid, small_bit_grid::SmallBitGrid},
    short::partizan::partizan_game::PartizanGame,
};
use cgt_derive::Tile;

/// Color of dominoes
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Tile)]
pub enum Tile {
    /// Blue domino
    #[tile(char('x'), bool(true))]
    Blue,

    /// Red domino
    #[tile(char('o'), bool(false))]
    Red,
}

/// Game of Toppling Dominoes
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TopplingDominoes {
    rows: Vec<SmallBitGrid<Tile>>,
}

impl Display for TopplingDominoes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for y in 0..self.rows.len() {
            let row = &self.rows[y];
            for x in 0..row.width() {
                write!(f, "{}", row.get(x, 0).tile_to_char())?;
            }
            if y != self.rows.len() - 1 {
                write!(f, "|")?;
            }
        }
        Ok(())
    }
}

impl TopplingDominoes {
    /// Create new Toppling Dominoes game from a vector of rows
    #[inline]
    pub const fn new(rows: Vec<SmallBitGrid<Tile>>) -> Self {
        Self { rows }
    }

    fn moves_for(&self, own_tile: Tile) -> Vec<Self> {
        let mut moves = Vec::with_capacity(
            2 * self
                .rows
                .iter()
                .map(|row| {
                    let mut acc = 0;
                    for x in 0..row.width() {
                        acc += (row.get(x, 0) == own_tile) as usize;
                    }
                    acc
                })
                .sum::<usize>(),
        );

        for (row_idx, row) in self.rows.iter().enumerate() {
            for x in 0..row.width() {
                if row.get(x, 0) == own_tile {
                    let mut right = self.clone();
                    right.rows[row_idx].width = x;
                    moves.push(right);

                    let mut left = self.clone();
                    left.rows[row_idx].width -= x + 1;
                    for n in 0..left.rows[row_idx].width {
                        left.rows[row_idx].set(n, 0, row.get(n + x + 1, 0));
                    }
                    moves.push(left);
                }
            }
        }

        moves
    }
}

impl PartizanGame for TopplingDominoes {
    #[inline]
    fn left_moves(&self) -> Vec<Self> {
        self.moves_for(Tile::Blue)
    }

    #[inline]
    fn right_moves(&self) -> Vec<Self> {
        self.moves_for(Tile::Red)
    }

    fn decompositions(&self) -> Vec<Self> {
        let mut decompositions = Vec::with_capacity(self.rows.len());
        for row in &self.rows {
            decompositions.push(Self::new(vec![*row]));
        }
        decompositions
    }
}

#[test]
fn correct_left() {
    let td = TopplingDominoes::new(vec![
        SmallBitGrid::from_arr(
            5,
            1,
            &[Tile::Blue, Tile::Red, Tile::Red, Tile::Blue, Tile::Blue],
        )
        .unwrap(),
    ]);
    assert_eq!(
        td.left_moves()
            .iter()
            .map(std::string::ToString::to_string)
            .collect::<Vec<String>>(),
        ["", "ooxx", "xoo", "x", "xoox", ""]
            .iter()
            .map(|m| (*m).to_string())
            .collect::<Vec<String>>()
    );
}

#[test]
fn correct_right() {
    let td = TopplingDominoes::new(vec![
        SmallBitGrid::from_arr(
            5,
            1,
            &[Tile::Blue, Tile::Red, Tile::Red, Tile::Blue, Tile::Blue],
        )
        .unwrap(),
    ]);
    assert_eq!(
        td.right_moves()
            .iter()
            .map(std::string::ToString::to_string)
            .collect::<Vec<String>>(),
        ["x", "oxx", "xo", "xx"]
            .iter()
            .map(|m| (*m).to_string())
            .collect::<Vec<String>>()
    );
}

#[test]
fn correct_canonical() {
    use crate::short::partizan::transposition_table::ParallelTranspositionTable;

    let td = TopplingDominoes::new(vec![
        SmallBitGrid::from_arr(
            5,
            1,
            &[Tile::Blue, Tile::Red, Tile::Red, Tile::Blue, Tile::Blue],
        )
        .unwrap(),
    ]);
    let tt = ParallelTranspositionTable::new();
    assert_eq!(td.canonical_form(&tt).to_string(), "{1|*}");
}
