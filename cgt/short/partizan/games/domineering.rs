//! The game is played on a rectengular grid. Left places vertical dominoes, Right places
//! horizontal dominoes.

extern crate alloc;
use crate::{
    drawing::svg::{self, ImmSvg, Svg},
    grid::{decompositions, move_top_left, small_bit_grid::SmallBitGrid, FiniteGrid, Grid},
    short::partizan::partizan_game::PartizanGame,
};
use cgt_derive::Tile;
use core::{fmt, hash::Hash};
use std::{fmt::Display, str::FromStr};

/// Tile on a Domineering grid
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord, Tile)]
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

/// A Domineering position on a rectengular grid.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
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
    pub fn grid_mut(&mut self) -> &mut G {
        &mut self.grid
    }

    /// Output positions as LaTeX `TikZ` picture where empty tiles are 1x1 tiles
    pub fn to_latex(&self) -> String {
        self.to_latex_with_scale(1.)
    }

    /// Like [`Self::to_latex`] but allows to specify image scale. Scale must be positive
    ///
    /// # Panics
    /// - `scale` is negative
    pub fn to_latex_with_scale(&self, scale: f32) -> String {
        use std::fmt::Write;

        assert!(scale >= 0., "Scale must be positive");

        let scale = scale.to_string();

        let mut buf = String::new();
        write!(buf, "\\begin{{tikzpicture}}[scale={}] ", scale).unwrap();
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
            "\\draw[step=1cm,black] (0,0) grid ({}, {}); \\end{{tikzpicture}}",
            self.grid.width(),
            self.grid.height()
        )
        .unwrap();
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

        if self.grid.height() == 0 || self.grid.width() == 0 {
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
                    moves.push(new_grid.move_top_left());
                }
            }
        }
        moves.sort_unstable();
        moves.dedup();
        moves
    }

    /// Remove filled rows and columns from the edges
    ///
    /// # Examples
    /// ```
    /// use cgt::short::partizan::games::domineering::Domineering;
    /// use std::str::FromStr;
    ///
    /// let position: Domineering = Domineering::from_str("###|.#.|##.").unwrap();
    /// assert_eq!(&format!("{}", position.move_top_left()), ".#.|##.");
    /// ```
    // Panic at `Self::empty(minimized_width, minimized_height).unwrap();` is unreachable
    #[must_use]
    #[allow(clippy::missing_panics_doc)]
    pub fn move_top_left(&self) -> Self {
        // TODO: We should use this to also "fill 1x1 holes" i.e. when we have grids that after running
        // bfs has 1x1 regions we can fill them in and reduce grid then.

        Self::new(move_top_left(&self.grid, Tile::is_non_blocking))
    }
}

impl<G> Svg for Domineering<G>
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
                    let fill = match self.grid.get(x, y) {
                        Tile::Empty => "white",
                        Tile::Taken => "gray",
                    };
                    ImmSvg::rect(
                        buf,
                        (x as u32 * tile_size + offset) as i32,
                        (y as u32 * tile_size + offset) as i32,
                        tile_size,
                        tile_size,
                        fill,
                    )?;
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

impl<G> PartizanGame for Domineering<G>
where
    G: Grid<Item = Tile> + FiniteGrid + Clone + Hash + Send + Sync + Ord,
{
    /// Get moves for the Left player as positions she can move to.
    ///
    /// # Examples
    ///
    /// ```
    /// // ..#       ..   .# |
    /// // .#.  = {  .# , #. | <...> }
    /// // ##.            #. |
    ///
    /// use cgt::short::partizan::games::domineering::Domineering;
    /// use crate::cgt::short::partizan::partizan_game::PartizanGame;
    /// use std::str::FromStr;
    ///
    /// let position: Domineering = Domineering::from_str("..#|.#.|##.").unwrap();
    /// assert_eq!(
    ///     position.left_moves(),
    ///     vec![
    ///         Domineering::from_str("..|.#").unwrap(),
    ///         Domineering::from_str(".#|#.|#.").unwrap(),
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
    /// // .#.  = {  <...> | .#. ,
    /// // ##.             | ##.
    ///
    /// use cgt::short::partizan::{partizan_game::PartizanGame, games::domineering::Domineering};
    /// use std::str::FromStr;
    ///
    /// let position: Domineering = Domineering::from_str("..#|.#.|##.").unwrap();
    /// assert_eq!(
    ///     position.right_moves(),
    ///     vec![Domineering::from_str(".#.|##.").unwrap(),]
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
        dbg!(transposition_table.len());
        assert_eq!(&game_id.to_string(), "{1*|-1*}");
        assert_eq!(temp, DyadicRationalNumber::from(1));
    }

    #[test]
    fn latex_works() {
        let position: Domineering = Domineering::from_str("##..|....|#...|..##").unwrap();
        let latex = position.to_latex();
        assert_eq!(
            &latex,
            r#"\begin{tikzpicture}[scale=1] \fill[fill=gray] (0,3) rectangle (1,4); \fill[fill=gray] (1,3) rectangle (2,4); \fill[fill=gray] (0,1) rectangle (1,2); \fill[fill=gray] (2,0) rectangle (3,1); \fill[fill=gray] (3,0) rectangle (4,1); \draw[step=1cm,black] (0,0) grid (4, 4); \end{tikzpicture}"#
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
        // FIXME: takes too long
        // assert_temperature!(Domineering::from_str("#...|....|....|...."), 1);
    }
}
