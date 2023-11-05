//! Ski Jumps game

use crate::{
    grid::{vec_grid::VecGrid, CharTile, FiniteGrid, Grid},
    short::partizan::{canonical_form::CanonicalForm, partizan_game::PartizanGame},
};
use std::{fmt::Display, str::FromStr};

/// Skier type
#[derive(Debug, Hash, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Skier {
    /// Skier that can jump over skiers below
    Jumper,
    /// Skier that was jumped over tunrs into slipper and cannot jump anymore
    Slipper,
}

/// Ski Jumps game grid tile
#[derive(Debug, Hash, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Tile {
    /// Empty tile, without skiers
    Empty,

    /// Left player's skier
    Left(Skier),

    /// Right player's skier
    Right(Skier),
}

impl Default for Tile {
    fn default() -> Self {
        Tile::Empty
    }
}

impl CharTile for Tile {
    fn tile_to_char(self) -> char {
        match self {
            Tile::Empty => '.',
            Tile::Left(Skier::Jumper) => 'L',
            Tile::Left(Skier::Slipper) => 'l',
            Tile::Right(Skier::Jumper) => 'R',
            Tile::Right(Skier::Slipper) => 'r',
        }
    }

    fn char_to_tile(input: char) -> Option<Self> {
        match input {
            '.' => Some(Tile::Empty),
            'L' => Some(Tile::Left(Skier::Jumper)),
            'l' => Some(Tile::Left(Skier::Slipper)),
            'R' => Some(Tile::Right(Skier::Jumper)),
            'r' => Some(Tile::Right(Skier::Slipper)),
            _ => None,
        }
    }
}

// NOTE: Consider caching positions of left and right skiers to avoid quadratic loops
/// Ski Jumps game
#[derive(Debug, Hash, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct SkiJumps {
    grid: VecGrid<Tile>,
}

impl Display for SkiJumps {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.grid.display(f, '|')
    }
}

impl FromStr for SkiJumps {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::new(VecGrid::parse(s).ok_or(())?))
    }
}

impl SkiJumps {
    /// Create new Ski Jumps game from a grid
    #[inline]
    pub fn new(grid: VecGrid<Tile>) -> Self {
        SkiJumps { grid }
    }

    fn jump_available_for(&self, own: Tile, can_jump_over: fn(Tile) -> bool) -> bool {
        for y in 0..self.grid.height() {
            for x in 0..self.grid.width() {
                // Check if in a row below current row, there is a tile that can be jumped over
                for dx in 0..self.grid.width() {
                    if self.grid.get(x, y) == own
                        && y + 1 < self.grid.height()
                        && can_jump_over(self.grid.get(dx, y + 1))
                    {
                        return true;
                    }
                }
            }
        }

        false
    }

    /// Check if left player has any jumping moves
    #[inline]
    pub fn left_jump_available(&self) -> bool {
        // Left jumper can jump over any right piece
        self.jump_available_for(Tile::Left(Skier::Jumper), |other| {
            matches!(other, Tile::Right(_))
        })
    }

    /// Check if right player has any jumping moves
    #[inline]
    pub fn right_jump_available(&self) -> bool {
        // Right jumper can jump over any left piece
        self.jump_available_for(Tile::Right(Skier::Jumper), |other| {
            matches!(other, Tile::Left(_))
        })
    }
}

impl PartizanGame for SkiJumps {
    fn left_moves(&self) -> Vec<Self> {
        let mut moves = vec![];

        for y in 0..self.grid.height() {
            for x in 0..self.grid.width() {
                match self.grid.get(x, y) {
                    Tile::Empty | Tile::Right(_) => {}
                    tile_to_move @ Tile::Left(skier) => {
                        // Check sliding moves
                        for dx in (x + 1)..=self.grid.width() {
                            if dx == self.grid.width() {
                                let mut new_grid = self.grid.clone();
                                new_grid.set(x, y, Tile::Empty);
                                moves.push(Self::new(new_grid));
                            } else if self.grid.get(dx, y) == Tile::Empty {
                                let mut new_grid = self.grid.clone();
                                new_grid.set(x, y, Tile::Empty);
                                new_grid.set(dx, y, tile_to_move);
                                moves.push(Self::new(new_grid));
                            } else {
                                // Blocked, cannot go any further
                                break;
                            }
                        }

                        // Check jump
                        if skier == Skier::Jumper && y + 1 < self.grid.height() {
                            match self.grid.get(x, y + 1) {
                                Tile::Empty | Tile::Left(_) => {}
                                Tile::Right(_) => {
                                    let mut new_grid = self.grid.clone();
                                    new_grid.set(x, y, Tile::Empty);
                                    new_grid.set(x, y + 1, Tile::Right(Skier::Slipper));
                                    if y + 2 < self.grid.height() {
                                        new_grid.set(x, y + 2, Tile::Left(Skier::Jumper));
                                    }
                                    moves.push(Self::new(new_grid));
                                }
                            }
                        }
                    }
                }
            }
        }

        moves
    }

    fn right_moves(&self) -> Vec<Self> {
        let mut moves = vec![];

        for y in 0..self.grid.height() {
            for x in 0..self.grid.width() {
                match self.grid.get(x, y) {
                    Tile::Empty | Tile::Left(_) => {}
                    tile_to_move @ Tile::Right(skier) => {
                        // Check sliding moves
                        for dx in (0..x + 1).rev() {
                            // We're iterating with 1 off to avoid using negative numbers but still
                            // catch going off grid, so the `dx - 1` hack.

                            if dx == 0 {
                                let mut new_grid = self.grid.clone();
                                new_grid.set(x, y, Tile::Empty);
                                moves.push(Self::new(new_grid));
                            } else if self.grid.get(dx - 1, y) == Tile::Empty {
                                let mut new_grid = self.grid.clone();
                                new_grid.set(x, y, Tile::Empty);
                                new_grid.set(dx - 1, y, tile_to_move);
                                moves.push(Self::new(new_grid));
                            } else {
                                // Blocked, cannot go any further
                                break;
                            }
                        }

                        // Check jump
                        if skier == Skier::Jumper && y + 1 < self.grid.height() {
                            match self.grid.get(x, y + 1) {
                                Tile::Empty | Tile::Right(_) => {}
                                Tile::Left(_) => {
                                    let mut new_grid = self.grid.clone();
                                    new_grid.set(x, y, Tile::Empty);
                                    new_grid.set(x, y + 1, Tile::Left(Skier::Slipper));
                                    if y + 2 < self.grid.height() {
                                        new_grid.set(x, y + 2, Tile::Right(Skier::Jumper));
                                    }
                                    moves.push(Self::new(new_grid));
                                }
                            }
                        }
                    }
                }
            }
        }

        moves
    }

    fn reductions(&self) -> Option<CanonicalForm> {
        // If neither player can jump, the optimal move is to move any of the pieces by one tile
        // so the game value is the difference of sum of distances to the board edge
        if !self.left_jump_available() && !self.right_jump_available() {
            let mut value = 0i64;
            for y in 0..self.grid.height() {
                for x in 0..self.grid.width() {
                    match self.grid.get(x, y) {
                        Tile::Empty => {}
                        Tile::Left(_) => value += self.grid.width() as i64 - x as i64,
                        Tile::Right(_) => value -= (x + 1) as i64,
                    }
                }
            }
            return Some(CanonicalForm::new_integer(value));
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::short::partizan::transposition_table::TranspositionTable;

    macro_rules! test_canonical_form {
        ($input:expr, $output:expr) => {{
            let tt = TranspositionTable::new();
            let pos = SkiJumps::from_str($input).expect("Could not parse the game");
            let cf = pos.canonical_form(&tt);
            assert_eq!(cf.to_string(), $output)
        }};
    }

    #[test]
    fn winning_ways_examples() {
        // I couldn't find other implementations so we're comparing against positions in winning ways
        test_canonical_form!("...L....|..R.....|........", "2");
        test_canonical_form!("........|...l....|.......R|........|......L.", "-1");
        test_canonical_form!(".L...|.R...|.....", "5/2");
        test_canonical_form!("...R.|...L.|.....", "-5/2");
        test_canonical_form!("L....|....R|.....", "1/2");
    }
}
