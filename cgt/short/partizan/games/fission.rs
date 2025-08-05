//! Fission game
//!
//! Fission is played with black stones on a square grid.
//! On her turn, Left may select any stone, provided that tiles directly above and below are empty,
//! remove that stone and put two stones in the empty tiles directly above and below.
//! Similarly Right playes on sqares to the left and right instead.

use crate::{
    drawing::{self, Canvas, Color, Draw},
    grid::{vec_grid::VecGrid, FiniteGrid, Grid},
    numeric::v2f::V2f,
    short::partizan::partizan_game::PartizanGame,
};
use cgt_derive::Tile;
use std::{fmt::Display, hash::Hash, str::FromStr};

/// Tile in the game of Fission
#[derive(Debug, Hash, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Tile)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Tile {
    /// Empty tile without stones
    #[tile(char('.'), default)]
    Empty,

    /// Stone
    #[tile(char('x'))]
    Stone,

    /// Tile on which stone cannot be placed
    /// Used to model non-rectangular grids
    #[tile(char('#'))]
    Blocked,
}

/// Game of Fission
#[derive(Debug, Hash, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Fission<G = VecGrid<Tile>> {
    grid: G,
}

impl<G> Display for Fission<G>
where
    G: Grid<Item = Tile> + FiniteGrid,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.grid.display(f, '|')
    }
}

impl<G> FromStr for Fission<G>
where
    G: Grid<Item = Tile> + FiniteGrid,
{
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::new(G::parse(s).ok_or(())?))
    }
}

impl<G> Fission<G>
where
    G: Grid<Item = Tile> + FiniteGrid,
{
    /// Create new Fission game from a grid
    #[inline]
    pub const fn new(grid: G) -> Self {
        Self { grid }
    }

    /// Get underlying grid
    #[inline]
    pub const fn grid(&self) -> &G {
        &self.grid
    }

    /// Get underlying grid mutably
    #[inline]
    pub fn grid_mut(&mut self) -> &mut G {
        &mut self.grid
    }

    #[inline]
    fn move_in<const DIR_X: u8, const DIR_Y: u8>(&self, x: u8, y: u8) -> Self
    where
        G: Clone,
    {
        let prev_x = x - DIR_X;
        let prev_y = y - DIR_Y;
        let next_x = x + DIR_X;
        let next_y = y + DIR_Y;

        let mut new_grid = self.grid.clone();
        new_grid.set(x, y, Tile::Empty);
        new_grid.set(prev_x, prev_y, Tile::Stone);
        new_grid.set(next_x, next_y, Tile::Stone);
        Fission::new(new_grid)
    }

    /// Make Left move in given tile without checking if move is legal
    #[inline]
    #[must_use]
    pub fn move_in_left(&self, x: u8, y: u8) -> Self
    where
        G: Clone,
    {
        self.move_in::<0, 1>(x, y)
    }

    /// Make Right move in given tile without checking if move is legal
    #[inline]
    #[must_use]
    pub fn move_in_right(&self, x: u8, y: u8) -> Self
    where
        G: Clone,
    {
        self.move_in::<1, 0>(x, y)
    }

    #[inline]
    fn available_moves_for<const DIR_X: u8, const DIR_Y: u8>(&self) -> Vec<(u8, u8)> {
        let mut moves = Vec::new();

        if self.grid.height() == 0 || self.grid.width() == 0 {
            return moves;
        }

        for y in DIR_Y..(self.grid.height() - DIR_Y) {
            for x in DIR_X..(self.grid.width() - DIR_X) {
                let prev_x = x - DIR_X;
                let prev_y = y - DIR_Y;
                let next_x = x + DIR_X;
                let next_y = y + DIR_Y;

                if self.grid.get(x, y) == Tile::Stone
                    && self.grid.get(prev_x, prev_y) == Tile::Empty
                    && self.grid.get(next_x, next_y) == Tile::Empty
                {
                    moves.push((x, y));
                }
            }
        }

        moves
    }

    /// List available tiles (ones with stone) where Left can move
    #[inline]
    pub fn available_moves_left(&self) -> Vec<(u8, u8)> {
        self.available_moves_for::<0, 1>()
    }

    /// List available tiles (ones with stone) where Right can move
    #[inline]
    pub fn available_moves_right(&self) -> Vec<(u8, u8)> {
        self.available_moves_for::<1, 0>()
    }

    #[inline]
    fn moves_for<const DIR_X: u8, const DIR_Y: u8>(&self) -> Vec<Self>
    where
        G: Clone,
    {
        let mut moves = Vec::new();

        if self.grid.height() == 0 || self.grid.width() == 0 {
            return moves;
        }

        for y in DIR_Y..(self.grid.height() - DIR_Y) {
            for x in DIR_X..(self.grid.width() - DIR_X) {
                let prev_x = x - DIR_X;
                let prev_y = y - DIR_Y;
                let next_x = x + DIR_X;
                let next_y = y + DIR_Y;

                if self.grid.get(x, y) == Tile::Stone
                    && self.grid.get(prev_x, prev_y) == Tile::Empty
                    && self.grid.get(next_x, next_y) == Tile::Empty
                {
                    moves.push(self.move_in::<DIR_X, DIR_Y>(x, y));
                }
            }
        }

        moves
    }
}

impl<G> Draw for Fission<G>
where
    G: Grid<Item = Tile> + FiniteGrid,
{
    fn draw<C>(&self, canvas: &mut C)
    where
        C: Canvas,
    {
        self.grid.draw(canvas, |tile| match tile {
            Tile::Empty => drawing::Tile::Square {
                color: Color::LIGHT_GRAY,
            },
            Tile::Stone => drawing::Tile::Circle {
                tile_color: Color::LIGHT_GRAY,
                circle_color: Color::DARK_GRAY,
            },
            Tile::Blocked => drawing::Tile::Square {
                color: Color::DARK_GRAY,
            },
        });
    }

    fn canvas_size<C>(&self) -> V2f
    where
        C: Canvas,
    {
        self.grid().canvas_size::<C>()
    }
}

impl<G> PartizanGame for Fission<G>
where
    G: Grid<Item = Tile> + FiniteGrid + Clone + Hash + Send + Sync + Eq,
{
    fn left_moves(&self) -> Vec<Self> {
        self.moves_for::<0, 1>()
    }

    fn right_moves(&self) -> Vec<Self> {
        self.moves_for::<1, 0>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::short::partizan::transposition_table::ParallelTranspositionTable;
    use std::str::FromStr;

    macro_rules! fission {
        ($input:expr) => {
            Fission::from_str($input).expect("Could not parse the game")
        };
    }

    macro_rules! test_canonical_form {
        ($input:expr, $output:expr) => {{
            let tt = ParallelTranspositionTable::new();
            let pos: Fission = fission!($input);
            let cf = pos.canonical_form(&tt);
            assert_eq!(cf.to_string(), $output);
        }};
    }

    #[test]
    fn canonical_form() {
        test_canonical_form!("....|..x.|....|....", "0");
        test_canonical_form!("..x.|....|..x.|....", "-2");
        test_canonical_form!(".x.x|....|..x.|....", "-1");
        test_canonical_form!(".x.x|..x.|....|..x.", "{-1|-4}");
        test_canonical_form!("....|..x.|....|#..#", "0");
    }
}
