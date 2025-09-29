//! The game is played on a rectangular grid. Left places vertical dominoes, Right places
//! horizontal dominoes.

extern crate alloc;
use crate::{
    drawing::{self, BoundingBox, Canvas, Color, Draw},
    grid::{self, FiniteGrid, Grid, decompositions, small_bit_grid::SmallBitGrid},
    short::partizan::partizan_game::PartizanGame,
};
use cgt_derive::Tile;
use core::hash::Hash;
use std::{fmt::Display, str::FromStr};

/// Tile on a Domineering grid
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Tile)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Tile {
    /// Tile where domino can be placed
    #[tile(char('.'), bool(false), default)]
    Empty,

    /// Tile occupied by domino
    #[tile(char('#'), bool(true))]
    Taken,
}

impl Tile {
    #[inline]
    fn is_non_blocking(self) -> bool {
        self == Self::Empty
    }
}

/// A Domineering position on a rectangular grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Domineering<G = SmallBitGrid<Tile>> {
    grid: G,
}

impl<G> Display for Domineering<G>
where
    G: Grid<Item = Tile> + FiniteGrid,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.grid.display(f, '|')
    }
}

impl<G> FromStr for Domineering<G>
where
    G: Grid<Item = Tile> + FiniteGrid,
{
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::new(G::parse(s).ok_or(())?))
    }
}

/// Config for [`Domineering::to_latex_with_config`]
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct LatexConfig {
    /// `scale` of `tikzpicture`
    pub scale: f32,

    /// `ysep` of `fit=(current bounding box)` node
    pub vertical_marigin: Option<f32>,

    /// `baseline` of `tikzpicture`
    pub baseline: Option<f32>,
}

impl<G> Domineering<G>
where
    G: Grid<Item = Tile> + FiniteGrid,
{
    /// Create a domineering position from a grid.
    pub const fn new(grid: G) -> Self {
        Self { grid }
    }

    /// Get underlying grid
    pub const fn grid(&self) -> &G {
        &self.grid
    }

    /// Get underlying grid mutably
    pub const fn grid_mut(&mut self) -> &mut G {
        &mut self.grid
    }

    /// Output positions as LaTeX `TikZ` picture where empty tiles are 1x1 tiles
    pub fn to_latex(&self) -> String {
        self.to_latex_with_config(LatexConfig {
            scale: 1.0,
            vertical_marigin: None,
            baseline: None,
        })
    }

    /// Like [`Self::to_latex`] but allows to specify tikz config
    pub fn to_latex_with_config(&self, config: LatexConfig) -> String {
        use std::fmt::Write;

        let scale = config.scale.to_string();

        let mut buf = String::new();
        write!(buf, "\\begin{{tikzpicture}}[scale={}", scale).unwrap();

        // https://tex.stackexchange.com/a/152098/229946
        if let Some(baseline) = config.baseline {
            write!(buf, ", baseline={}", baseline).unwrap();
        }

        write!(buf, "] ",).unwrap();
        for y in 0..self.grid.height() {
            for x in 0..self.grid.width() {
                if self.grid.get(x, y) == Tile::Taken {
                    write!(
                        buf,
                        "\\fill[fill=gray] ({},{}) rectangle ({},{}); ",
                        x,
                        self.grid.height() - y - 1,
                        x + 1,
                        self.grid.height() - y,
                    )
                    .unwrap();
                }
            }
        }
        write!(
            buf,
            "\\draw[step=1cm,black] (0,0) grid ({}, {}); ",
            self.grid.width(),
            self.grid.height()
        )
        .unwrap();

        // https://tex.stackexchange.com/a/152098/229946
        if let Some(vertical_marigin) = config.vertical_marigin {
            write!(
                buf,
                "\\node[fit=(current bounding box),inner ysep={}mm,inner xsep=0]{{}};",
                vertical_marigin
            )
            .unwrap();
        }
        write!(buf, "\\end{{tikzpicture}}").unwrap();
        buf
    }

    /// Get number of empty tiles on a grid
    pub fn free_places(&self) -> usize {
        let mut res = 0;
        for y in 0..self.grid.height() {
            for x in 0..self.grid.width() {
                if self.grid.get(x, y) == Tile::Empty {
                    res += 1;
                }
            }
        }
        res
    }

    fn moves_for<const DIR_X: u8, const DIR_Y: u8>(&self) -> Vec<Self>
    where
        G: Ord + Clone,
    {
        let mut moves = Vec::new();

        if self.grid.width() <= DIR_X || self.grid.height() <= DIR_Y {
            return moves;
        }

        for y in 0..(self.grid.height() - DIR_Y) {
            for x in 0..(self.grid.width() - DIR_X) {
                let next_x = x + DIR_X;
                let next_y = y + DIR_Y;
                if self.grid.get(x, y) == Tile::Empty
                    && self.grid.get(next_x, next_y) == Tile::Empty
                {
                    let mut new_grid = self.clone();
                    new_grid.grid.set(x, y, Tile::Taken);
                    new_grid.grid.set(next_x, next_y, Tile::Taken);
                    moves.push(new_grid.normalize_grid());
                }
            }
        }
        moves.sort_unstable();
        moves.dedup();
        moves
    }

    /// Normalize underlying grid by filling in empty 1x1 tiles
    /// and removing filled rows and columns from the edges
    ///
    /// # Examples
    /// ```
    /// use cgt::short::partizan::games::domineering::Domineering;
    /// use std::str::FromStr;
    ///
    /// let position: Domineering = Domineering::from_str("###|.#.|##.").unwrap();
    /// assert_eq!(&format!("{}", position.normalize_grid()), ".|.");
    /// ```
    #[must_use]
    pub fn normalize_grid(&self) -> Self
    where
        G: Clone,
    {
        let mut grid = self.grid.clone();
        grid::fill_one_by_one_holes_with(&mut grid, Tile::Empty, Tile::Taken);
        let grid = grid::move_top_left(&grid, Tile::is_non_blocking);
        Self::new(grid)
    }
}

impl<G> Draw for Domineering<G>
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
            Tile::Taken => drawing::Tile::Square {
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

impl<G> PartizanGame for Domineering<G>
where
    G: Grid<Item = Tile> + FiniteGrid + Clone + Hash + Send + Sync + Ord,
{
    /// Get moves for the Left player as positions she can move to.
    ///
    /// # Examples
    ///
    /// ```
    /// // ..#       ..   #. |
    /// // .#.  = {  .# , #. | <...> }
    /// // ##.               
    ///
    /// use cgt::short::partizan::games::domineering::Domineering;
    /// use crate::cgt::short::partizan::partizan_game::PartizanGame;
    /// use std::str::FromStr;
    ///
    /// let position: Domineering = Domineering::from_str("..#|.#.|##.").unwrap();
    ///
    /// assert_eq!(
    ///     position.left_moves(),
    ///     vec![
    ///         Domineering::from_str(".|.").unwrap(),
    ///         Domineering::from_str("..|.#").unwrap(),
    ///     ]
    /// );
    /// ```
    fn left_moves(&self) -> Vec<Self> {
        self.moves_for::<0, 1>()
    }

    /// Get moves for the Right player as positions he can move to.
    ///
    /// # Examples
    ///
    /// ```
    /// // ..#             |
    /// // .#.  = {  <...> | . ,
    /// // ##.             | .
    ///
    /// use cgt::short::partizan::{partizan_game::PartizanGame, games::domineering::Domineering};
    /// use std::str::FromStr;
    ///
    /// let position: Domineering = Domineering::from_str("..#|.#.|##.").unwrap();
    /// assert_eq!(
    ///     position.right_moves(),
    ///     vec![Domineering::from_str(".|.").unwrap(),]
    /// );
    /// ```
    fn right_moves(&self) -> Vec<Self> {
        self.moves_for::<1, 0>()
    }

    /// Get decompisitons of given position
    ///
    /// # Examples
    /// ```
    /// // ..#   ..#   ###
    /// // .#. = .## + ##.
    /// // ##.   ###   ##.
    ///
    /// use cgt::short::partizan::{partizan_game::PartizanGame, games::domineering::Domineering};
    /// use std::str::FromStr;
    ///
    /// let position: Domineering = Domineering::from_str("..#|.#.|##.").unwrap();
    /// assert_eq!(
    ///    position.decompositions(),
    ///    vec![
    ///        Domineering::from_str("..|.#").unwrap(),
    ///        Domineering::from_str(".|.").unwrap(),
    ///    ]
    /// );
    /// ```
    fn decompositions(&self) -> Vec<Self> {
        let directions = [(1, 0), (-1, 0), (0, 1), (0, -1)];
        decompositions(&self.grid, Tile::is_non_blocking, Tile::Taken, &directions)
            .into_iter()
            .map(Self::new)
            .collect::<Vec<_>>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        numeric::dyadic_rational_number::DyadicRationalNumber,
        short::partizan::transposition_table::ParallelTranspositionTable,
    };
    use std::str::FromStr;

    #[test]
    #[should_panic]
    fn grid_max_size_is_respected() {
        Domineering::new(SmallBitGrid::empty(10, 10).unwrap());
    }

    #[test]
    fn parse_display_roundtrip() {
        let inp = "...|#.#|##.|###";
        let pos: Domineering = Domineering::from_str(inp).unwrap();
        assert_eq!(&format!("{}", pos), inp,);
    }

    // Values confirmed with gcsuite

    #[cfg(test)]
    fn test_grid_canonical_form(grid: Domineering, canonical_form: &str) {
        let transposition_table = ParallelTranspositionTable::new();
        let game_id = grid.canonical_form(&transposition_table);
        assert_eq!(&game_id.to_string(), canonical_form);
    }

    #[test]
    fn finds_canonical_form_of_one() {
        test_grid_canonical_form(Domineering::from_str(".|.").unwrap(), "1");
    }

    #[test]
    fn finds_canonical_form_of_minus_one() {
        test_grid_canonical_form(Domineering::from_str("..").unwrap(), "-1");
    }

    #[test]
    fn finds_canonical_form_of_two_by_two() {
        test_grid_canonical_form(Domineering::from_str("..|..").unwrap(), "{1|-1}");
    }

    #[test]
    fn finds_canonical_form_of_two_by_two_with_noise() {
        test_grid_canonical_form(Domineering::from_str("..#|..#|##.").unwrap(), "{1|-1}");
    }

    #[test]
    fn finds_canonical_form_of_minus_two() {
        test_grid_canonical_form(Domineering::from_str("....").unwrap(), "-2");
    }

    #[test]
    fn finds_canonical_form_of_l_shape() {
        test_grid_canonical_form(Domineering::from_str(".#|..").unwrap(), "*");
    }

    #[test]
    fn finds_canonical_form_of_long_l_shape() {
        test_grid_canonical_form(Domineering::from_str(".##|.##|...").unwrap(), "0");
    }

    #[test]
    fn finds_canonical_form_of_weird_l_shape() {
        test_grid_canonical_form(Domineering::from_str("..#|..#|...").unwrap(), "{1/2|-2}");
    }

    #[test]
    fn finds_canonical_form_of_three_by_three() {
        test_grid_canonical_form(Domineering::from_str("...|...|...").unwrap(), "{1|-1}");
    }

    #[test]
    fn finds_canonical_form_of_num_nim_sum() {
        test_grid_canonical_form(Domineering::from_str(".#.#|.#..").unwrap(), "1*");
    }

    #[test]
    #[cfg(not(miri))]
    fn finds_temperature_of_four_by_four_grid() {
        let transposition_table = ParallelTranspositionTable::new();
        let grid: Domineering = Domineering::from_str("#...|....|....|....").unwrap();
        let game_id = grid.canonical_form(&transposition_table);
        let temp = game_id.temperature();
        assert_eq!(&game_id.to_string(), "{1*|-1*}");
        assert_eq!(temp, DyadicRationalNumber::from(1));
    }

    #[test]
    fn latex_works() {
        let position: Domineering = Domineering::from_str("##..|....|#...|..##").unwrap();
        let latex = position.to_latex();
        assert_eq!(
            &latex,
            r"\begin{tikzpicture}[scale=1] \fill[fill=gray] (0,3) rectangle (1,4); \fill[fill=gray] (1,3) rectangle (2,4); \fill[fill=gray] (0,1) rectangle (1,2); \fill[fill=gray] (2,0) rectangle (3,1); \fill[fill=gray] (3,0) rectangle (4,1); \draw[step=1cm,black] (0,0) grid (4, 4); \end{tikzpicture}"
        );
    }

    /// Assert temperature value without going through canonical form
    /// Using macro yields better error location on assertion failure
    #[cfg(test)]
    macro_rules! assert_temperature {
        ($grid:expr, $temp:expr) => {
            let grid: Domineering = $grid.unwrap();
            let thermograph = grid.thermograph_direct();
            let expected_temperature = DyadicRationalNumber::from($temp);
            assert_eq!(thermograph.temperature(), expected_temperature);
        };
    }

    #[test]
    fn temperature_without_game_works() {
        assert_temperature!(Domineering::from_str(""), -1);
        assert_temperature!(Domineering::from_str(".."), -1);
        assert_temperature!(Domineering::from_str("..|.#"), 0);
        assert_temperature!(Domineering::from_str("#...|....|....|...."), 1);
    }
}
