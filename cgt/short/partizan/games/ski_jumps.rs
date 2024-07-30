//! The game is played on a rectangular grid of squares. Each square is either empty or contains a
//! skier of Left or Right player that can be either a jumper or a slipper.
//!
//! Left may move any of their skiers one or more tiles to the right, but only if the destination
//! is empty and there is no skier in the way. Skiers are allowed to move off the board. Right moves
//! their skiers to the left in the same way
//!
//! A jumper skier may also jump down over a skier of the opposite color, when a skier of the
//! opposite color is below and the destination is empty. The skier that was jumped over turns into
//! a slipper that cannot jump anymore.

use crate::{
    drawing::svg::{self, ImmSvg, Svg},
    grid::{vec_grid::VecGrid, CharTile, FiniteGrid, Grid},
    short::partizan::{canonical_form::CanonicalForm, partizan_game::PartizanGame},
};
use core::fmt;
use std::{fmt::Display, hash::Hash, str::FromStr};

/// Skier type
#[derive(Debug, Hash, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Skier {
    /// Skier that can jump over skiers below
    Jumper,
    /// Skier that was jumped over tunrs into slipper and cannot jump anymore
    Slipper,
}

/// Ski Jumps game grid tile
#[derive(Debug, Hash, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
        Self::Empty
    }
}

impl CharTile for Tile {
    fn tile_to_char(self) -> char {
        match self {
            Self::Empty => '.',
            Self::Left(Skier::Jumper) => 'L',
            Self::Left(Skier::Slipper) => 'l',
            Self::Right(Skier::Jumper) => 'R',
            Self::Right(Skier::Slipper) => 'r',
        }
    }

    fn char_to_tile(input: char) -> Option<Self> {
        match input {
            '.' => Some(Self::Empty),
            'L' => Some(Self::Left(Skier::Jumper)),
            'l' => Some(Self::Left(Skier::Slipper)),
            'R' => Some(Self::Right(Skier::Jumper)),
            'r' => Some(Self::Right(Skier::Slipper)),
            _ => None,
        }
    }
}

// NOTE: Consider caching positions of left and right skiers to avoid quadratic loops
/// Ski Jumps game
#[derive(Debug, Hash, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SkiJumps<G = VecGrid<Tile>> {
    grid: G,
}

impl<G> Display for SkiJumps<G>
where
    G: Grid<Item = Tile> + FiniteGrid,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.grid.display(f, '|')
    }
}

impl<G> FromStr for SkiJumps<G>
where
    G: Grid<Item = Tile> + FiniteGrid,
{
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::new(G::parse(s).ok_or(())?))
    }
}

impl<G> SkiJumps<G>
where
    G: Grid<Item = Tile> + FiniteGrid,
{
    /// Create new Ski Jumps game from a grid
    #[inline]
    pub const fn new(grid: G) -> Self {
        Self { grid }
    }

    /// Check if jumping move is possible
    pub fn jump_available(&self) -> bool {
        for y in 0..self.grid.height() {
            for x in 0..self.grid.width() {
                // Check if in a row below current row, there is a tile that can be jumped over
                let current = self.grid.get(x, y);
                for dx in 0..self.grid.width() {
                    if y + 1 < self.grid.height() {
                        match (current, self.grid.get(dx, y + 1)) {
                            (Tile::Left(Skier::Jumper), Tile::Right(_))
                            | (Tile::Right(Skier::Jumper), Tile::Left(_)) => {
                                return true;
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        false
    }
}

#[cfg(not(tarpaulin_include))]
impl<G> Svg for SkiJumps<G>
where
    G: Grid<Item = Tile> + FiniteGrid,
{
    fn to_svg<W>(&self, buf: &mut W) -> fmt::Result
    where
        W: fmt::Write,
    {
        // Chosen arbitrarily
        let tile_size = 48;
        let grid_width = 4;

        let offset = grid_width / 2;
        let svg_width = self.grid.width() as u32 * tile_size + grid_width;
        let svg_height = self.grid.height() as u32 * tile_size + grid_width;

        ImmSvg::new(buf, svg_width, svg_height, |buf| {
            for y in 0..self.grid.height() {
                for x in 0..self.grid.width() {
                    match self.grid.get(x, y) {
                        Tile::Empty => {}
                        tile => {
                            let text = svg::Text {
                                x: (x as u32 * tile_size + offset + tile_size / 2) as i32,
                                y: (y as u32 * tile_size + offset + (0.6 * tile_size as f32) as u32)
                                    as i32,
                                text: tile.tile_to_char().to_string(),
                                text_anchor: svg::TextAnchor::Middle,
                                ..svg::Text::default()
                            };
                            ImmSvg::text(buf, &text)?;
                        }
                    }
                }
            }

            let grid = svg::Grid {
                x1: 0,
                y1: 0,
                x2: svg_width as i32,
                y2: svg_height as i32,
                grid_width,
                tile_size,
            };
            ImmSvg::grid(buf, &grid)
        })
    }
}

impl<G> PartizanGame for SkiJumps<G>
where
    G: Grid<Item = Tile> + FiniteGrid + Clone + Hash + Send + Sync + Eq,
{
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
                        for dx in (0..=x).rev() {
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
        if !self.jump_available() {
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
    use crate::short::partizan::transposition_table::ParallelTranspositionTable;
    use std::str::FromStr;

    macro_rules! test_canonical_form {
        ($input:expr, $output:expr) => {{
            let tt = ParallelTranspositionTable::new();
            let pos: SkiJumps = SkiJumps::from_str($input).expect("Could not parse the game");
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
