//! Fission game

use crate::{
    grid::{small_bit_grid::SmallBitGrid, FiniteGrid, Grid},
    short::partizan::partizan_game::PartizanGame,
};
use cgt_derive::Tile;
use std::{fmt::Display, hash::Hash, str::FromStr};

/// Tile in the game of Fission
#[derive(Debug, Hash, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Tile)]
pub enum Tile {
    /// Empty tile without stones
    #[tile(default, char('.'), bool(false))]
    Empty,

    /// Stone
    #[tile(char('#'), bool(true))]
    Stone,
}

/// Game of Fission
#[derive(Debug, Hash, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Fission<G = SmallBitGrid<Tile>> {
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
    pub fn new(grid: G) -> Self {
        Fission { grid }
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
                    let mut new_grid: Self = self.clone();
                    new_grid.grid.set(x, y, Tile::Empty);
                    new_grid.grid.set(prev_x, prev_y, Tile::Stone);
                    new_grid.grid.set(next_x, next_y, Tile::Stone);
                    dbg!(format!("{}", &self));
                    dbg!(format!("{}", &new_grid));
                    moves.push(new_grid);
                }
            }
        }

        moves
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
    use crate::short::partizan::transposition_table::TranspositionTable;
    use std::str::FromStr;

    macro_rules! fission {
        ($input:expr) => {
            Fission::from_str($input).expect("Could not parse the game")
        };
    }

    macro_rules! test_canonical_form {
        ($input:expr, $output:expr) => {{
            let tt = TranspositionTable::new();
            let pos: Fission = fission!($input);
            let cf = pos.canonical_form(&tt);
            assert_eq!(cf.to_string(), $output);
        }};
    }

    #[test]
    fn canonical_form() {
        test_canonical_form!("....|..#.|....|....", "0");
        test_canonical_form!(".#.#|....|..#.|....", "-1");
    }
}
