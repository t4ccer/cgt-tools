//! Amazons game

use crate::{
    drawing::{self, BoundingBox, Canvas, Color, Draw},
    grid::{decompositions, move_top_left, vec_grid::VecGrid, FiniteGrid, Grid},
    short::partizan::{partizan_game::PartizanGame, Player},
};
use cgt_derive::Tile;
use std::{fmt::Display, hash::Hash, str::FromStr};

/// Tile in the game of Amazons
#[derive(Debug, Hash, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Tile)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Tile {
    /// Empty tile without stones
    #[tile(char('.'), default)]
    Empty,

    /// Tile with Left player's Amazon - black queen
    #[tile(char('x'))]
    Left,

    /// Tile with Right player's Amazon - white queen
    #[tile(char('o'))]
    Right,

    /// Stone
    #[tile(char('#'))]
    Stone,
}

impl Tile {
    #[inline]
    fn is_non_blocking(self) -> bool {
        self != Self::Stone
    }
}

impl From<Player> for Tile {
    fn from(player: Player) -> Tile {
        match player {
            Player::Left => Tile::Left,
            Player::Right => Tile::Right,
        }
    }
}

/// Game of Amazons
#[derive(Debug, Hash, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Amazons<G = VecGrid<Tile>> {
    grid: G,
}

impl<G> Display for Amazons<G>
where
    G: Grid<Item = Tile> + FiniteGrid,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.grid.display(f, '|')
    }
}

impl<G> FromStr for Amazons<G>
where
    G: Grid<Item = Tile> + FiniteGrid,
{
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::new(G::parse(s).ok_or(())?))
    }
}

const DIRECTIONS: [(i32, i32); 8] = [
    (-1, 0),
    (-1, 1),
    (0, 1),
    (1, 1),
    (1, 0),
    (1, -1),
    (0, -1),
    (-1, -1),
];

impl<G> Amazons<G>
where
    G: Grid<Item = Tile> + FiniteGrid,
{
    /// Create new Amazons game from a grid
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

    /// Get available moves for given player
    pub fn moves_for(&self, own_amazon: Tile, should_move_top_left: bool) -> Vec<Self>
    where
        G: Clone + PartialEq,
    {
        let longer_side = self.grid.height().max(self.grid.width());

        let mut moves = Vec::new();
        for y in 0..self.grid.height() as i32 {
            for x in 0..self.grid.width() as i32 {
                if self.grid.get(x as u8, y as u8) == own_amazon {
                    for (amazon_dir_x, amazon_dir_y) in DIRECTIONS {
                        for k in 1..longer_side as i32 {
                            let new_amazon_x = x + amazon_dir_x * k;
                            let new_amazon_y = y + amazon_dir_y * k;

                            if new_amazon_x < 0
                                || new_amazon_x >= self.grid.width() as i32
                                || new_amazon_y < 0
                                || new_amazon_y >= self.grid.height() as i32
                                || self.grid.get(new_amazon_x as u8, new_amazon_y as u8)
                                    != Tile::Empty
                            {
                                break;
                            }
                            let mut new_grid = self.grid.clone();
                            new_grid.set(x as u8, y as u8, Tile::Empty);
                            new_grid.set(new_amazon_x as u8, new_amazon_y as u8, own_amazon);
                            for (arrow_dir_x, arrow_dir_y) in DIRECTIONS {
                                for l in 1..longer_side as i32 {
                                    let new_arrow_x = new_amazon_x + arrow_dir_x * l;
                                    let new_arrow_y = new_amazon_y + arrow_dir_y * l;

                                    if new_arrow_x < 0
                                        || new_arrow_x >= new_grid.width() as i32
                                        || new_arrow_y < 0
                                        || new_arrow_y >= new_grid.height() as i32
                                        || new_grid.get(new_arrow_x as u8, new_arrow_y as u8)
                                            != Tile::Empty
                                    {
                                        break;
                                    }
                                    let mut new_grid = new_grid.clone();
                                    new_grid.set(new_arrow_x as u8, new_arrow_y as u8, Tile::Stone);
                                    if should_move_top_left {
                                        new_grid = move_top_left(&new_grid, Tile::is_non_blocking);
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
}

impl<G> Draw for Amazons<G>
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
            Tile::Left => drawing::Tile::Circle {
                tile_color: Color::LIGHT_GRAY,
                circle_color: Color::BLUE,
            },
            Tile::Right => drawing::Tile::Circle {
                tile_color: Color::LIGHT_GRAY,
                circle_color: Color::RED,
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

impl<G> PartizanGame for Amazons<G>
where
    G: Grid<Item = Tile> + FiniteGrid + Clone + Hash + Send + Sync + Eq,
{
    fn left_moves(&self) -> Vec<Self> {
        self.moves_for(Tile::Left, true)
    }

    fn right_moves(&self) -> Vec<Self> {
        self.moves_for(Tile::Right, true)
    }

    fn decompositions(&self) -> Vec<Self> {
        decompositions(&self.grid, Tile::is_non_blocking, Tile::Stone, &DIRECTIONS)
            .into_iter()
            .map(Self::new)
            .collect::<Vec<_>>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::short::partizan::{
        canonical_form::CanonicalForm, transposition_table::ParallelTranspositionTable,
    };
    use std::str::FromStr;

    macro_rules! amazons {
        ($input:expr) => {
            Amazons::from_str($input).expect("Could not parse the game")
        };
    }

    macro_rules! test_canonical_form {
        ($input:expr, $output:expr) => {{
            let tt = ParallelTranspositionTable::new();
            let pos: Amazons = amazons!($input);
            let cf = pos.canonical_form(&tt);
            let expected = CanonicalForm::from_str($output).unwrap().to_string();
            assert_eq!(cf.to_string(), expected);
        }};
    }

    #[test]
    fn canonical_form() {
        // Confirmed with cgsuite
        test_canonical_form!("x..#|....|.#.o", "{{6|{3|1, {3|0, {1/2|0}}}}, {6|{4*|-3, {3, {3|0, {1/2|0}}|-4}}}|-3, {0, {0|-2}, {1|-3}|-5}, {0, {0, *|0, {0, {1/2, {1|0}|v}|v}}|-5}, {{2, {2|0}|0, {0, {2|0, {2|0}}|0}}, {2, {3|0}|0, {0, {0, ^*|0}|-1}}, {{2|0}, {2|{1|1/4}, {2|0}}|v*, {1/2|{{0|-1}, {*|-1}|-1}}, {{0, ^*|0}|-1}}, {{3|0}, {3|1, {2|0}}, {3, {3|1}|1, {1|0, *}}|-1/16, {0|-1}, {*|-1}}|-5, {v, v*, {0, {0, ^*|0}|-1}|-5}, {{1/2|{-1/4, {0|-1}, {*|-1}|-1}}, {{1|1/4}|{-1/4|-1}}, {{1|{1|0}, {1|*}}|-1/2}|-5}}}");
    }
}
