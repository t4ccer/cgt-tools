//! Toads and Frogs game

use crate::{
    drawing::svg::{self, ImmSvg, Svg},
    grid::CharTile,
    short::partizan::partizan_game::PartizanGame,
};
use core::fmt;
use std::str::FromStr;

/// Tile on the Toads and Frogs board
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Tile {
    /// Empty tile without any creature
    Empty,

    /// Left player's, moving right
    Toad,

    /// Right player's, moving left
    Frog,
}

impl Default for Tile {
    fn default() -> Self {
        Tile::Empty
    }
}

impl CharTile for Tile {
    fn char_to_tile(input: char) -> Option<Self> {
        match input {
            '.' => Some(Tile::Empty),
            'T' => Some(Tile::Toad),
            'F' => Some(Tile::Frog),
            _ => None,
        }
    }

    fn tile_to_char(self) -> char {
        match self {
            Tile::Empty => '.',
            Tile::Toad => 'T',
            Tile::Frog => 'F',
        }
    }
}

/// Singular row of the Toads and Frogs board
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ToadsAndFrogs {
    tiles: Vec<Tile>,
}

impl FromStr for ToadsAndFrogs {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut tiles = Vec::with_capacity(s.len());
        for c in s.chars() {
            tiles.push(Tile::char_to_tile(c).ok_or(())?);
        }
        Ok(Self { tiles })
    }
}

impl Svg for ToadsAndFrogs {
    fn to_svg<W>(&self, buf: &mut W) -> fmt::Result
    where
        W: fmt::Write,
    {
        let tile_size = 48;
        let grid_width = 4;

        let offset = grid_width / 2;
        let svg_width = self.tiles.len() as u32 * tile_size + grid_width;
        let svg_height = tile_size + grid_width;

        ImmSvg::new(buf, svg_width, svg_height, |buf| {
            ImmSvg::g(buf, "black", |buf| {
                for (x, tile) in self.tiles.iter().enumerate() {
                    let (fill, label) = match tile {
                        Tile::Empty => continue,
                        Tile::Toad => ("blue", 'T'),
                        Tile::Frog => ("red", 'F'),
                    };
                    ImmSvg::rect(
                        buf,
                        (x as u32 * tile_size + offset) as i32,
                        offset as i32,
                        tile_size,
                        tile_size,
                        fill,
                    )?;

                    let label = svg::Text {
                        x: (x as u32 * tile_size + tile_size / 2 + offset) as i32,
                        y: (tile_size * 2 / 3) as i32,
                        text: format!("{}", label),
                        text_anchor: svg::TextAnchor::Middle,
                        ..svg::Text::default()
                    };
                    ImmSvg::text(buf, &label)?;
                }
                Ok(())
            })?;

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
                    moves.push(Self { tiles: new_tiles });
                } else if idx < self.tiles.len() - 2
                    && self.tiles[idx + 1] == opponent
                    && self.tiles[idx + 2] == Tile::Empty
                {
                    let mut new_tiles = self.tiles.clone();
                    new_tiles[idx] = Tile::Empty;
                    new_tiles[idx + 2] = own;
                    moves.push(Self { tiles: new_tiles });
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
                    moves.push(Self { tiles: new_tiles });
                } else if idx > 1
                    && self.tiles[idx - 1] == opponent
                    && self.tiles[idx - 2] == Tile::Empty
                {
                    let mut new_tiles = self.tiles.clone();
                    new_tiles[idx] = Tile::Empty;
                    new_tiles[idx - 2] = own;
                    moves.push(Self { tiles: new_tiles });
                }
            }
        }

        moves
    }
}

#[cfg(test)]
mod tests {
    use crate::short::partizan::{
        canonical_form::CanonicalForm, transposition_table::TranspositionTable,
    };

    use super::*;

    macro_rules! row {
        ($inp:expr) => {
            ToadsAndFrogs::from_str($inp).expect("invalid row")
        };
    }

    macro_rules! assert_canonical_form {
        ($row:expr, $cf:expr) => {
            let tt = TranspositionTable::new();
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
        assert_canonical_form!("TFTF.TF", "0");
        assert_canonical_form!("TFTFTF.", "0");
        assert_canonical_form!("TFTFT.F", "*");
        assert_canonical_form!("TF.TTFF", "0");
        assert_canonical_form!("TFT.TFF", "^");
        assert_canonical_form!(".TTFTFF", "0");
        assert_canonical_form!("T.TFTFF", "{0|^}");
    }
}
