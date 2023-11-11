//! The game is played on a rectengular grid. Left places vertical dominoes, Right places
//! horizontal dominoes.

extern crate alloc;
use crate::{
    drawing::svg::{self, ImmSvg, Svg},
    grid::{small_bit_grid::SmallBitGrid, FiniteGrid, Grid},
    short::partizan::partizan_game::PartizanGame,
};
use alloc::collections::vec_deque::VecDeque;
use core::{fmt, hash::Hash};
use std::{fmt::Display, str::FromStr};

#[cfg(test)]
use crate::{
    numeric::rational::Rational, short::partizan::transposition_table::TranspositionTable,
};

// FIXME
type Tile = bool;

/// A Domineering position on a rectengular grid.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Domineering<G = SmallBitGrid> {
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
    G: Grid<Item = bool> + FiniteGrid,
{
    /// Create a domineering position from a grid.
    pub fn new(grid: G) -> Self {
        Self { grid }
    }

    /// Get underlying grid
    pub fn grid(&self) -> &G {
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
                if self.grid.get(x, y) {
                    write!(
                        buf,
                        "\\fill[fill=gray] ({},{}) rectangle ({},{}); ",
                        x,
                        y,
                        x + 1,
                        y + 1,
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

    /// Remove filled rows and columns from the edges
    ///
    /// # Examples
    /// ```
    /// use cgt::short::partizan::games::domineering::Domineering;
    /// use std::str::FromStr;
    ///
    /// let position = Domineering::from_str("###|.#.|##.").unwrap();
    /// assert_eq!(&format!("{}", position.move_top_left()), ".#.|##.");
    /// ```
    // Panic at `Self::empty(minimized_width, minimized_height).unwrap();` is unreachable
    #[must_use]
    #[cfg_attr(feature = "cargo-clippy", allow(clippy::missing_panics_doc))]
    pub fn move_top_left(&self) -> Self {
        let mut filled_top_rows = 0;
        for y in 0..self.grid.height() {
            let mut should_break = false;
            for x in 0..self.grid.width() {
                // If empty space then break
                if !self.grid.get(x, y) {
                    should_break = true;
                    break;
                }
            }
            if should_break {
                break;
            }
            filled_top_rows += 1;
        }
        let filled_top_rows = filled_top_rows;

        if filled_top_rows == self.grid.height() {
            return Self::new(G::zero_size());
        }

        let mut filled_bottom_rows = 0;
        for y in 0..self.grid.height() {
            let mut should_break = false;
            for x in 0..self.grid.width() {
                // If empty space then break
                if !self.grid.get(x, self.grid.height() - y - 1) {
                    should_break = true;
                    break;
                }
            }
            if should_break {
                break;
            }
            filled_bottom_rows += 1;
        }
        let filled_bottom_rows = filled_bottom_rows;

        let mut filled_left_cols = 0;
        for x in 0..self.grid.width() {
            let mut should_break = false;
            for y in 0..self.grid.height() {
                // If empty space then break
                if !self.grid.get(x, y) {
                    should_break = true;
                    break;
                }
            }
            if should_break {
                break;
            }
            filled_left_cols += 1;
        }
        let filled_left_cols = filled_left_cols;

        if filled_left_cols == self.grid.width() {
            return Self::new(G::zero_size());
        }

        let mut filled_right_cols = 0;
        for x in 0..self.grid.width() {
            let mut should_break = false;
            for y in 0..self.grid.height() {
                // If empty space then break
                if !self.grid.get(self.grid.width() - x - 1, y) {
                    should_break = true;
                    break;
                }
            }
            if should_break {
                break;
            }
            filled_right_cols += 1;
        }
        let filled_right_cols = filled_right_cols;

        let minimized_width = self.grid.width() - filled_left_cols - filled_right_cols;
        let minimized_height = self.grid.height() - filled_top_rows - filled_bottom_rows;

        let mut grid = G::filled(minimized_width, minimized_height, false).unwrap();
        for y in filled_top_rows..(self.grid.height() - filled_bottom_rows) {
            for x in filled_left_cols..(self.grid.width() - filled_right_cols) {
                grid.set(
                    x - filled_left_cols,
                    y - filled_top_rows,
                    self.grid.get(x, y),
                );
            }
        }
        Self::new(grid)
    }

    fn bfs(&self, visited: &mut SmallBitGrid, x: u8, y: u8) -> Self {
        let mut grid = Self::new(G::filled(self.grid.width(), self.grid.height(), true).unwrap());

        let mut q: VecDeque<(u8, u8)> =
            VecDeque::with_capacity(self.grid.width() as usize * self.grid.height() as usize);
        q.push_back((x, y));
        while let Some((qx, qy)) = q.pop_front() {
            visited.set(qx, qy, true);
            grid.grid.set(qx, qy, false);
            let directions: [(i64, i64); 4] = [(1, 0), (-1, 0), (0, 1), (0, -1)];
            for (dx, dy) in directions {
                let lx = (qx as i64) + dx;
                let ly = (qy as i64) + dy;

                if lx >= 0
                    && lx < (self.grid.width() as i64)
                    && ly >= 0
                    && ly < (self.grid.height() as i64)
                    && !self.grid.get(lx as u8, ly as u8)
                    && !visited.get(lx as u8, ly as u8)
                {
                    q.push_back((lx as u8, ly as u8));
                }
            }
        }
        grid.move_top_left()
    }

    /// Get number of empty tiles on a grid
    pub fn free_places(&self) -> usize {
        let mut res = 0;
        for y in 0..self.grid.height() {
            for x in 0..self.grid.width() {
                if !self.grid.get(x, y) {
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
                if !self.grid.get(x, y) && !self.grid.get(next_x, next_y) {
                    let mut new_grid: Self = self.clone();
                    new_grid.grid.set(x, y, true);
                    new_grid.grid.set(next_x, next_y, true);
                    moves.push(new_grid.move_top_left());
                }
            }
        }
        moves.sort_unstable();
        moves.dedup();
        moves
    }
}

impl Svg for Domineering {
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
                    let fill = if self.grid.get(x, y) { "gray" } else { "white" };
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

#[test]
#[should_panic]
fn grid_max_size_is_respected() {
    Domineering::new(SmallBitGrid::empty(10, 10).unwrap());
}

impl<G> PartizanGame for Domineering<G>
where
    G: Grid<Item = bool> + FiniteGrid + Clone + Hash + Send + Sync + Ord,
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
    /// let position = Domineering::from_str("..#|.#.|##.").unwrap();
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
    /// let position = Domineering::from_str("..#|.#.|##.").unwrap();
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
    /// let position = Domineering::from_str("..#|.#.|##.").unwrap();
    /// assert_eq!(
    ///    position.decompositions(),
    ///    vec![
    ///        Domineering::from_str("..|.#").unwrap(),
    ///        Domineering::from_str(".|.").unwrap(),
    ///    ]
    /// );
    /// ```
    fn decompositions(&self) -> Vec<Self> {
        let mut visited = SmallBitGrid::empty(self.grid.width(), self.grid.height()).unwrap();
        let mut ds = Vec::new();

        for y in 0..self.grid.height() {
            for x in 0..self.grid.width() {
                if !self.grid.get(x, y) && !visited.get(x, y) {
                    ds.push(self.bfs(&mut visited, x, y));
                }
            }
        }

        ds
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn parse_display_roundtrip() {
        let inp = "...|#.#|##.|###";
        let pos: Domineering = Domineering::from_str(inp).unwrap();
        assert_eq!(&format!("{}", pos), inp,);
    }

    // Values confirmed with gcsuite

    #[cfg(test)]
    fn test_grid_canonical_form(grid: Domineering, canonical_form: &str) {
        let transposition_table = TranspositionTable::new();
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
        use crate::numeric::rational::Rational;

        let transposition_table = TranspositionTable::new();
        let grid: Domineering = Domineering::from_str("#...|....|....|....").unwrap();
        let game_id = grid.canonical_form(&transposition_table);
        let temp = game_id.temperature();
        dbg!(transposition_table.len());
        assert_eq!(&game_id.to_string(), "{1*|-1*}");
        assert_eq!(temp, Rational::from(1));
    }

    #[test]
    fn latex_works() {
        let position: Domineering = Domineering::from_str("##..|....|#...|..##").unwrap();
        let latex = position.to_latex();
        assert_eq!(
            &latex,
            r#"\begin{tikzpicture}[scale=1] \fill[fill=gray] (0,0) rectangle (1,1); \fill[fill=gray] (1,0) rectangle (2,1); \fill[fill=gray] (0,2) rectangle (1,3); \fill[fill=gray] (2,3) rectangle (3,4); \fill[fill=gray] (3,3) rectangle (4,4); \draw[step=1cm,black] (0,0) grid (4, 4); \end{tikzpicture}"#
        );
    }

    /// Assert temperature value without going through canonical form
    /// Using macro yields better error location on assertion failure
    #[cfg(test)]
    macro_rules! assert_temperature {
        ($grid:expr, $temp:expr) => {
            let grid: Domineering = $grid.unwrap();
            let thermograph = grid.thermograph_direct();
            let expected_temperature = Rational::from($temp);
            assert_eq!(thermograph.get_temperature(), expected_temperature);
        };
    }

    #[test]
    fn temperature_without_game_works() {
        assert_temperature!(Domineering::from_str(""), -1);
        assert_temperature!(Domineering::from_str(".."), -1);
        assert_temperature!(Domineering::from_str("..|.#"), 0);
        // FIXME: takes too long
        // assert_temperature!(Domineering::parse("#...|....|....|...."), 1);
    }
}
