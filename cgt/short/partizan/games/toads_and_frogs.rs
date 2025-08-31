//! Game of Toads and Frogs is played on a rectangular grid of squares, altough for simplification
//! we will consider only a single row of squares, rectangular grids can be modeled as sums of row.
//!
//! Left player has trained Toads that can move one square to the right if it is empty, or jump over
//! a Frog to the empty square behind it. Right player moves Frogs and jumps over Toads to the left
//! in the same way.

use crate::{
    drawing::{self, BoundingBox, Canvas, Color, Draw},
    grid::{CharTile, FiniteGrid, Grid, vec_grid::VecGrid},
    short::partizan::partizan_game::PartizanGame,
};
use cgt_derive::Tile;
use std::{
    fmt::{self, Display},
    str::FromStr,
};

/// Tile on the Toads and Frogs board
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Tile)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Tile {
    /// Empty tile without any creature
    #[tile(default, char('.'))]
    Empty,

    /// Left player's, moving right
    #[tile(char('T'))]
    Toad,

    /// Right player's, moving left
    #[tile(char('F'))]
    Frog,
}

/// Singular row of the Toads and Frogs board
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ToadsAndFrogs {
    tiles: Vec<Tile>,
}

impl ToadsAndFrogs {
    /// Creates a new Toads and Frogs game from a row of tiles
    pub const fn new(tiles: Vec<Tile>) -> Self {
        Self { tiles }
    }

    /// Get game row
    pub const fn row(&mut self) -> &Vec<Tile> {
        &self.tiles
    }

    /// Get game row
    pub const fn row_mut(&mut self) -> &mut Vec<Tile> {
        &mut self.tiles
    }

    /// Construct grid equal to underlying game
    pub fn grid(&self) -> VecGrid<Tile> {
        // TODO: Use grid internally
        let mut grid = VecGrid::filled(self.tiles.len() as u8, 1, Tile::Empty).unwrap();
        for (i, t) in self.tiles.iter().copied().enumerate() {
            grid.set(i as u8, 0, t);
        }
        grid
    }
}

impl FromStr for ToadsAndFrogs {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut tiles = Vec::with_capacity(s.len());
        for c in s.chars() {
            tiles.push(Tile::char_to_tile(c).ok_or(())?);
        }
        Ok(Self::new(tiles))
    }
}

impl Display for ToadsAndFrogs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for tile in &self.tiles {
            write!(f, "{}", tile.tile_to_char())?;
        }

        Ok(())
    }
}

impl Draw for ToadsAndFrogs {
    fn draw<C>(&self, canvas: &mut C)
    where
        C: Canvas,
    {
        self.grid().draw(canvas, |tile| match tile {
            Tile::Empty => drawing::Tile::Square {
                color: Color::LIGHT_GRAY,
            },
            Tile::Toad => drawing::Tile::Char {
                tile_color: Color::LIGHT_GRAY,
                text_color: Color::BLUE,
                letter: 'T',
            },
            Tile::Frog => drawing::Tile::Char {
                tile_color: Color::LIGHT_GRAY,
                text_color: Color::RED,
                letter: 'F',
            },
        });
    }

    fn required_canvas<C>(&self) -> BoundingBox
    where
        C: Canvas,
    {
        self.grid().canvas_size::<C>()
    }
}

impl PartizanGame for ToadsAndFrogs {
    fn left_moves(&self) -> Vec<Self> {
        let own = Tile::Toad;
        let opponent = Tile::Frog;

        let mut moves = Vec::new();

        for (idx, tile) in self.tiles.iter().copied().enumerate() {
            if tile == own {
                if idx < self.tiles.len() - 1 && self.tiles[idx + 1] == Tile::Empty {
                    let mut new_tiles = self.tiles.clone();
                    new_tiles[idx] = Tile::Empty;
                    new_tiles[idx + 1] = own;
                    moves.push(Self::new(new_tiles));
                } else if idx + 2 < self.tiles.len()
                    && self.tiles[idx + 1] == opponent
                    && self.tiles[idx + 2] == Tile::Empty
                {
                    let mut new_tiles = self.tiles.clone();
                    new_tiles[idx] = Tile::Empty;
                    new_tiles[idx + 2] = own;
                    moves.push(Self::new(new_tiles));
                }
            }
        }

        moves
    }

    fn right_moves(&self) -> Vec<Self> {
        let own = Tile::Frog;
        let opponent = Tile::Toad;

        let mut moves = Vec::new();

        for (idx, tile) in self.tiles.iter().copied().enumerate() {
            if tile == own {
                if idx > 0 && self.tiles[idx - 1] == Tile::Empty {
                    let mut new_tiles = self.tiles.clone();
                    new_tiles[idx] = Tile::Empty;
                    new_tiles[idx - 1] = own;
                    moves.push(Self::new(new_tiles));
                } else if idx > 1
                    && self.tiles[idx - 1] == opponent
                    && self.tiles[idx - 2] == Tile::Empty
                {
                    let mut new_tiles = self.tiles.clone();
                    new_tiles[idx] = Tile::Empty;
                    new_tiles[idx - 2] = own;
                    moves.push(Self::new(new_tiles));
                }
            }
        }

        moves
    }
}

#[cfg(test)]
mod tests {
    use crate::short::partizan::{
        canonical_form::CanonicalForm, transposition_table::ParallelTranspositionTable,
    };

    use super::*;

    macro_rules! row {
        ($inp:expr) => {
            ToadsAndFrogs::from_str($inp).expect("invalid row")
        };
    }

    macro_rules! assert_canonical_form {
        ($row:expr, $cf:expr) => {
            let tt = ParallelTranspositionTable::new();
            let cf = row!($row).canonical_form(&tt);
            assert_eq!(cf, CanonicalForm::from_str($cf).unwrap());
        };
    }

    #[test]
    fn parses_correctly() {
        row!("T.TFTFF");
    }

    #[test]
    fn left_moves() {
        assert_eq!(row!("T.TFTFF").left_moves(), vec![row!(".TTFTFF")]);
        assert_eq!(row!("TFT.TFF").left_moves(), vec![row!("TF.TTFF")]);
    }

    #[test]
    fn right_moves() {
        assert_eq!(row!("T.TFTFF").right_moves(), vec![row!("TFT.TFF")]);
        assert_eq!(row!(".F.F").right_moves(), vec![row!("F..F"), row!(".FF.")]);
        assert_eq!(row!("TFT.TFF").right_moves(), vec![row!("TFTFT.F")]);
    }

    #[test]
    fn canonical_form() {
        assert_canonical_form!("", "0");
        assert_canonical_form!(".", "0");
        assert_canonical_form!("F", "0");
        assert_canonical_form!("T", "0");
        assert_canonical_form!("TFTF.TF", "0");
        assert_canonical_form!("TFTFTF.", "0");
        assert_canonical_form!("TFTFT.F", "*");
        assert_canonical_form!("TF.TTFF", "0");
        assert_canonical_form!("TFT.TFF", "^");
        assert_canonical_form!(".TTFTFF", "0");
        assert_canonical_form!("T.TFTFF", "{0|^}");
    }
}
