//! Toads and Frogs game

use std::str::FromStr;

use crate::{grid::CharTile, short::partizan::partizan_game::PartizanGame};

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
