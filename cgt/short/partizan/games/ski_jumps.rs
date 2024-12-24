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
use cgt_derive::Tile;
use core::fmt;
use std::{fmt::Display, hash::Hash, str::FromStr};

/// Ski Jumps game grid tile
#[derive(Debug, Hash, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Tile)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Tile {
    /// Empty tile, without skiers
    #[tile(char('.'), default)]
    Empty,

    /// Left player's jumper
    #[tile(char('L'))]
    LeftJumper,

    /// Left player's slipper
    #[tile(char('l'))]
    LeftSlipper,

    /// Right player's jumper
    #[tile(char('R'))]
    RightJumper,

    /// Right player's slipper
    #[tile(char('r'))]
    RightSlipper,
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

/// Move that player can make
///
/// This is used for both players but the behaviour depends on the skier at the initial tile
#[derive(Debug, Hash, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Move {
    /// Slide off the grid
    SlideOff {
        /// Starting skier position
        from: (u8, u8),
    },

    /// Slide to the side
    Slide {
        /// Starting skier position
        from: (u8, u8),

        /// Target skier horizontal position
        to_x: u8,
    },

    /// Jump over opposing skier
    Jump {
        /// Starting skier position
        from: (u8, u8),
    },
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

    /// Check if jumping move is possible
    pub fn jump_available(&self) -> bool {
        for y in 0..self.grid.height() {
            for x in 0..self.grid.width() {
                // Check if in a row below current row, there is a tile that can be jumped over
                let current = self.grid.get(x, y);
                for dx in 0..self.grid.width() {
                    if y + 1 < self.grid.height() {
                        match (current, self.grid.get(dx, y + 1)) {
                            (Tile::LeftJumper, Tile::RightSlipper | Tile::RightJumper)
                            | (Tile::RightJumper, Tile::LeftSlipper | Tile::LeftJumper) => {
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

    // TODO: Write custom iterators for these, or ideally wait for coroutines to become stable

    /// Get all moves that Left player can make
    pub fn available_left_moves(&self) -> Vec<Move> {
        let mut moves = Vec::new();

        for y in 0..self.grid.height() {
            for x in 0..self.grid.width() {
                match self.grid.get(x, y) {
                    Tile::Empty | Tile::RightJumper | Tile::RightSlipper => {}
                    tile_to_move @ (Tile::LeftJumper | Tile::LeftSlipper) => {
                        // Check sliding moves
                        for dx in (x + 1)..=self.grid.width() {
                            if dx == self.grid.width() {
                                moves.push(Move::SlideOff { from: (x, y) });
                            } else if self.grid.get(dx, y) == Tile::Empty {
                                moves.push(Move::Slide {
                                    from: (x, y),
                                    to_x: dx,
                                });
                            } else {
                                // Blocked, cannot go any further
                                break;
                            }
                        }

                        // Check jump
                        if matches!(tile_to_move, Tile::LeftJumper) && y + 2 < self.grid.height() {
                            match self.grid.get(x, y + 1) {
                                Tile::Empty | Tile::LeftJumper | Tile::LeftSlipper => {}
                                Tile::RightJumper | Tile::RightSlipper => {
                                    moves.push(Move::Jump { from: (x, y) });
                                }
                            }
                        }
                    }
                }
            }
        }

        moves
    }

    /// Get all moves that Right player can make
    pub fn available_right_moves(&self) -> Vec<Move> {
        let mut moves = Vec::new();

        for y in 0..self.grid.height() {
            for x in 0..self.grid.width() {
                match self.grid.get(x, y) {
                    Tile::Empty | Tile::LeftJumper | Tile::LeftSlipper => {}
                    tile_to_move @ (Tile::RightJumper | Tile::RightSlipper) => {
                        // Check sliding moves
                        for dx in (0..=x).rev() {
                            if dx == 0 {
                                moves.push(Move::SlideOff { from: (x, y) });
                            } else if self.grid.get(dx - 1, y) == Tile::Empty {
                                moves.push(Move::Slide {
                                    from: (x, y),
                                    to_x: dx - 1,
                                });
                            } else {
                                // Blocked, cannot go any further
                                break;
                            }
                        }

                        // Check jump
                        if matches!(tile_to_move, Tile::RightJumper) && y + 2 < self.grid.height() {
                            match self.grid.get(x, y + 1) {
                                Tile::Empty | Tile::RightJumper | Tile::RightSlipper => {}
                                Tile::LeftJumper | Tile::LeftSlipper => {
                                    moves.push(Move::Jump { from: (x, y) });
                                }
                            }
                        }
                    }
                }
            }
        }

        moves
    }

    /// Make a move, does not verify if the move is legal
    pub fn move_in(&self, m: Move) -> Self
    where
        G: Clone,
    {
        let mut new_grid = self.grid.clone();
        match m {
            Move::SlideOff { from: (x, y) } => {
                new_grid.set(x, y, Tile::Empty);
            }
            Move::Slide { from: (x, y), to_x } => {
                let prev = new_grid.get(x, y);
                new_grid.set(x, y, Tile::Empty);
                new_grid.set(to_x, y, prev);
            }
            Move::Jump { from: (x, y) } => {
                let jumper = new_grid.get(x, y);
                new_grid.set(x, y, Tile::Empty);
                if matches!(jumper, Tile::RightJumper) {
                    new_grid.set(x, y + 1, Tile::LeftSlipper);
                } else {
                    new_grid.set(x, y + 1, Tile::RightSlipper);
                }
                new_grid.set(x, y + 2, jumper);
            }
        }

        Self::new(new_grid)
    }
}

impl<G> PartizanGame for SkiJumps<G>
where
    G: Grid<Item = Tile> + FiniteGrid + Clone + Hash + Send + Sync + Eq,
{
    fn left_moves(&self) -> Vec<Self> {
        let available_moves = self.available_left_moves();
        let mut moves = Vec::with_capacity(available_moves.len());
        for available in available_moves {
            moves.push(self.move_in(available));
        }
        moves
    }

    fn right_moves(&self) -> Vec<Self> {
        let available_moves = self.available_right_moves();
        let mut moves = Vec::with_capacity(available_moves.len());
        for available in available_moves {
            moves.push(self.move_in(available));
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
                        Tile::LeftJumper | Tile::LeftSlipper => {
                            value += self.grid.width() as i64 - x as i64
                        }
                        Tile::RightJumper | Tile::RightSlipper => value -= (x + 1) as i64,
                    }
                }
            }
            return Some(CanonicalForm::new_integer(value));
        }

        None
    }
}

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
        test_canonical_form!("L....|R....|.....", "9/2");
        test_canonical_form!("L....|....R|l....", "5");
    }
}
