//! Fission game

use crate::{
    drawing::svg::{self, ImmSvg, Svg},
    grid::{vec_grid::VecGrid, FiniteGrid, Grid},
    short::partizan::partizan_game::PartizanGame,
};
use cgt_derive::Tile;
use std::{
    fmt::{self, Display},
    hash::Hash,
    str::FromStr,
};

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
                    let mut new_grid = self.clone().grid;
                    new_grid.set(x, y, Tile::Empty);
                    new_grid.set(prev_x, prev_y, Tile::Stone);
                    new_grid.set(next_x, next_y, Tile::Stone);
                    moves.push(Self::new(new_grid));
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

#[cfg(not(tarpaulin_include))]
impl<G> Svg for Fission<G>
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
                        Tile::Empty => {
                            ImmSvg::rect(
                                buf,
                                (x as u32 * tile_size + offset) as i32,
                                (y as u32 * tile_size + offset) as i32,
                                tile_size,
                                tile_size,
                                "white",
                            )?;
                        }
                        Tile::Blocked => {
                            ImmSvg::rect(
                                buf,
                                (x as u32 * tile_size + offset) as i32,
                                (y as u32 * tile_size + offset) as i32,
                                tile_size,
                                tile_size,
                                "gray",
                            )?;
                        }
                        Tile::Stone => {
                            let circle = svg::Circle {
                                cx: (x as u32 * tile_size + offset + tile_size / 2) as i32,
                                cy: (y as u32 * tile_size + offset + tile_size / 2) as i32,
                                r: tile_size / 3,
                                stroke: "black".to_owned(),
                                stroke_width: 2,
                                fill: "gray".to_owned(),
                            };
                            ImmSvg::circle(buf, &circle)?;
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
