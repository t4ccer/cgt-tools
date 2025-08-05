//! Konane

use crate::{
    drawing::{self, BoundingBox, Canvas, Color, Draw},
    grid::{vec_grid::VecGrid, FiniteGrid, Grid},
    short::partizan::partizan_game::PartizanGame,
};
use cgt_derive::Tile;
use std::{fmt::Display, hash::Hash, str::FromStr};

/// Tile in the game of Konane
#[derive(Debug, Hash, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Tile)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Tile {
    /// Empty tile without stones
    #[tile(char('.'), default)]
    Empty,

    /// Blue
    #[tile(char('x'))]
    Left,

    /// Red
    #[tile(char('o'))]
    Right,

    /// Tile on which stone cannot be placed
    /// Used to model non-rectangular grids
    #[tile(char('#'))]
    Blocked,
}

/// Game of Konane
#[derive(Debug, Hash, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Konane<G = VecGrid<Tile>> {
    grid: G,
}

impl<G> Konane<G>
where
    G: Grid<Item = Tile> + FiniteGrid,
{
    /// Create new Konane game from a grid
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
}

impl<G> Draw for Konane<G>
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
            Tile::Left => drawing::Tile::Circle {
                tile_color: Color::LIGHT_GRAY,
                circle_color: Color::BLUE,
            },
            Tile::Right => drawing::Tile::Circle {
                tile_color: Color::LIGHT_GRAY,
                circle_color: Color::RED,
            },
            Tile::Blocked => drawing::Tile::Square {
                color: Color::DARK_GRAY,
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

impl<G> PartizanGame for Konane<G>
where
    G: Grid<Item = Tile> + FiniteGrid + Clone + Send + Sync + Eq + Hash,
{
    fn left_moves(&self) -> Vec<Self> {
        let mut res = Vec::new();
        for y in 0..self.grid.height() {
            for x in 0..self.grid.width() {
                if matches!(self.grid.get(x, y), Tile::Left) {
                    let mut g = self.clone();
                    g.grid.set(x, y, Tile::Empty);
                    for dx in 0.. {
                        let r_x = x + 2 * dx + 1;
                        let e_x = x + 2 * dx + 2;

                        if e_x >= self.grid.width() {
                            break;
                        }

                        if matches!(self.grid.get(r_x, y), Tile::Right)
                            && matches!(self.grid.get(e_x, y), Tile::Empty)
                        {
                            g.grid.set(r_x, y, Tile::Empty);
                            let mut g = g.clone();
                            g.grid.set(e_x, y, Tile::Left);
                            res.push(g);
                        } else {
                            break;
                        }
                    }

                    let mut g = self.clone();
                    g.grid.set(x, y, Tile::Empty);
                    for dx in 0.. {
                        let Some(r_x) = x.checked_sub(2 * dx + 1) else {
                            break;
                        };
                        let Some(e_x) = x.checked_sub(2 * dx + 2) else {
                            break;
                        };

                        if matches!(self.grid.get(r_x, y), Tile::Right)
                            && matches!(self.grid.get(e_x, y), Tile::Empty)
                        {
                            g.grid.set(r_x, y, Tile::Empty);
                            let mut g = g.clone();
                            g.grid.set(e_x, y, Tile::Left);
                            res.push(g);
                        } else {
                            break;
                        }
                    }

                    let mut g = self.clone();
                    g.grid.set(x, y, Tile::Empty);
                    for dy in 0.. {
                        let r_y = y + 2 * dy + 1;
                        let e_y = y + 2 * dy + 2;

                        if e_y >= self.grid.height() {
                            break;
                        }

                        if matches!(self.grid.get(x, r_y), Tile::Right)
                            && matches!(self.grid.get(x, e_y), Tile::Empty)
                        {
                            g.grid.set(x, r_y, Tile::Empty);
                            let mut g = g.clone();
                            g.grid.set(x, e_y, Tile::Left);
                            res.push(g);
                        } else {
                            break;
                        }
                    }

                    let mut g = self.clone();
                    g.grid.set(x, y, Tile::Empty);
                    for dy in 0.. {
                        let Some(r_y) = y.checked_sub(2 * dy + 1) else {
                            break;
                        };
                        let Some(e_y) = y.checked_sub(2 * dy + 2) else {
                            break;
                        };

                        if matches!(self.grid.get(x, r_y), Tile::Right)
                            && matches!(self.grid.get(x, e_y), Tile::Empty)
                        {
                            g.grid.set(x, r_y, Tile::Empty);
                            let mut g = g.clone();
                            g.grid.set(x, e_y, Tile::Left);
                            res.push(g);
                        } else {
                            break;
                        }
                    }
                }
            }
        }
        res
    }

    fn right_moves(&self) -> Vec<Self> {
        let mut res = Vec::new();
        for y in 0..self.grid.height() {
            for x in 0..self.grid.width() {
                if matches!(self.grid.get(x, y), Tile::Right) {
                    let mut g = self.clone();
                    g.grid.set(x, y, Tile::Empty);
                    for dx in 0.. {
                        let r_x = x + 2 * dx + 1;
                        let e_x = x + 2 * dx + 2;

                        if e_x >= self.grid.width() {
                            break;
                        }

                        if matches!(self.grid.get(r_x, y), Tile::Left)
                            && matches!(self.grid.get(e_x, y), Tile::Empty)
                        {
                            g.grid.set(r_x, y, Tile::Empty);
                            let mut g = g.clone();
                            g.grid.set(e_x, y, Tile::Right);
                            res.push(g);
                        } else {
                            break;
                        }
                    }

                    let mut g = self.clone();
                    g.grid.set(x, y, Tile::Empty);
                    for dx in 0.. {
                        let Some(r_x) = x.checked_sub(2 * dx + 1) else {
                            break;
                        };
                        let Some(e_x) = x.checked_sub(2 * dx + 2) else {
                            break;
                        };

                        if matches!(self.grid.get(r_x, y), Tile::Left)
                            && matches!(self.grid.get(e_x, y), Tile::Empty)
                        {
                            g.grid.set(r_x, y, Tile::Empty);
                            let mut g = g.clone();
                            g.grid.set(e_x, y, Tile::Right);
                            res.push(g);
                        } else {
                            break;
                        }
                    }

                    let mut g = self.clone();
                    g.grid.set(x, y, Tile::Empty);
                    for dy in 0.. {
                        let r_y = y + 2 * dy + 1;
                        let e_y = y + 2 * dy + 2;

                        if e_y >= self.grid.height() {
                            break;
                        }

                        if matches!(self.grid.get(x, r_y), Tile::Left)
                            && matches!(self.grid.get(x, e_y), Tile::Empty)
                        {
                            g.grid.set(x, r_y, Tile::Empty);
                            let mut g = g.clone();
                            g.grid.set(x, e_y, Tile::Right);
                            res.push(g);
                        } else {
                            break;
                        }
                    }

                    let mut g = self.clone();
                    g.grid.set(x, y, Tile::Empty);
                    for dy in 0.. {
                        let Some(r_y) = y.checked_sub(2 * dy + 1) else {
                            break;
                        };
                        let Some(e_y) = y.checked_sub(2 * dy + 2) else {
                            break;
                        };

                        if matches!(self.grid.get(x, r_y), Tile::Left)
                            && matches!(self.grid.get(x, e_y), Tile::Empty)
                        {
                            g.grid.set(x, r_y, Tile::Empty);
                            let mut g = g.clone();
                            g.grid.set(x, e_y, Tile::Right);
                            res.push(g);
                        } else {
                            break;
                        }
                    }
                }
            }
        }
        res
    }
}

#[test]
fn left_moves() {
    let g: Konane = Konane::from_str(".....|..o..|.oxo.|..o..|.....|..o..|.....").unwrap();
    assert_eq!(
        g.left_moves(),
        vec![
            Konane::from_str(".....|..o..|.o..x|..o..|.....|..o..|.....").unwrap(),
            Konane::from_str(".....|..o..|x..o.|..o..|.....|..o..|.....").unwrap(),
            Konane::from_str(".....|..o..|.o.o.|.....|..x..|..o..|.....").unwrap(),
            Konane::from_str(".....|..o..|.o.o.|.....|.....|.....|..x..").unwrap(),
            Konane::from_str("..x..|.....|.o.o.|..o..|.....|..o..|.....").unwrap(),
        ]
    );
}

#[test]
fn right_moves() {
    let g: Konane = Konane::from_str(".....|..x..|.xox.|..x..|.....|..x..|.....").unwrap();
    assert_eq!(
        g.right_moves(),
        vec![
            Konane::from_str(".....|..x..|.x..o|..x..|.....|..x..|.....").unwrap(),
            Konane::from_str(".....|..x..|o..x.|..x..|.....|..x..|.....").unwrap(),
            Konane::from_str(".....|..x..|.x.x.|.....|..o..|..x..|.....").unwrap(),
            Konane::from_str(".....|..x..|.x.x.|.....|.....|.....|..o..").unwrap(),
            Konane::from_str("..o..|.....|.x.x.|..x..|.....|..x..|.....").unwrap(),
        ]
    );
}

#[test]
fn values() {
    let tt = crate::short::partizan::transposition_table::ParallelTranspositionTable::new();

    let g: Konane = Konane::from_str("..ox").unwrap();
    assert_eq!(g.canonical_form(&tt).to_string(), "1");

    let g: Konane = Konane::from_str(".ox.").unwrap();
    assert_eq!(g.canonical_form(&tt).to_string(), "*");

    let g: Konane = Konane::from_str(".ox.|o..x|....").unwrap();
    assert_eq!(g.canonical_form(&tt).to_string(), "{1|-1}");
}

impl<G> Display for Konane<G>
where
    G: Grid<Item = Tile> + FiniteGrid,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.grid.display(f, '|')
    }
}

impl<G> FromStr for Konane<G>
where
    G: Grid<Item = Tile> + FiniteGrid,
{
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::new(G::parse(s).ok_or(())?))
    }
}
