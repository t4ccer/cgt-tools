//! The game is played on a rectengular grid. Left places vertical dominoes, Right places
//! horizontal dominoes.

extern crate alloc;
use crate::short::partizan::partizan_game::PartizanGame;
use alloc::collections::vec_deque::VecDeque;
use std::{fmt::Display, str::FromStr};

#[cfg(test)]
use crate::{
    numeric::rational::Rational, short::partizan::transposition_table::TranspositionTable,
};

// TODO: Move generic grid somewhere else

/// Internal representation of a grid
pub type GridBits = u64;

/// A Domineering position on a rectengular grid.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Domineering {
    width: u8,
    height: u8,
    grid: GridBits,
}

/// Convert bits in a number to an array but in reverse order.
pub fn bits_to_arr(num: GridBits) -> [bool; 64] {
    let mut grid = [false; 64];

    #[allow(clippy::needless_range_loop)]
    for grid_idx in 0..64 {
        grid[grid_idx] = ((num >> grid_idx) & 1) == 1;
    }
    grid
}

#[test]
fn bits_to_arr_works() {
    assert_eq!(
        bits_to_arr(0b1011001),
        [
            true, false, false, true, true, false, true, false, false, false, false, false, false,
            false, false, false, false, false, false, false, false, false, false, false, false,
            false, false, false, false, false, false, false, false, false, false, false, false,
            false, false, false, false, false, false, false, false, false, false, false, false,
            false, false, false, false, false, false, false, false, false, false, false, false,
            false, false, false
        ]
    );
}

/// Reverse of [`bits_to_arr`]
///
/// # Panics
/// - `grid` has more than 64 elements
pub fn arr_to_bits(grid: &[bool]) -> GridBits {
    assert!(
        grid.len() <= 8 * std::mem::size_of::<GridBits>(),
        "grid cannot have more than 64 elements"
    );
    let mut res: GridBits = 0;
    for i in (0..grid.len()).rev() {
        res <<= 1;
        res |= grid[i] as GridBits;
    }
    res
}

#[test]
fn bits_to_arr_to_bits_roundtrip() {
    let inp = 3874328;
    assert_eq!(inp, arr_to_bits(&bits_to_arr(inp)),);
}

/// Grid construction error
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum PositionError {
    /// Position larger than 64 tiles.
    TooLarge,

    /// Invalid `.#` input
    CouldNotParse,
}

impl Domineering {
    /// Check if dimensions are small enough to fit in the fixed-size bit representation.
    const fn check_dimensions(width: u8, height: u8) -> Result<(), PositionError> {
        if (width as usize * height as usize) > 8 * std::mem::size_of::<GridBits>() {
            return Err(PositionError::TooLarge);
        }
        Ok(())
    }

    /// Creates empty grid with given size.
    ///
    /// # Examples
    ///
    /// ```
    /// use cgt::short::partizan::games::domineering::Domineering;
    /// assert_eq!(&format!("{}", Domineering::empty(2, 3).unwrap()), "..|..|..");
    /// ```
    ///
    /// # Errors
    /// - Grid has more than 64 tiles
    pub fn empty(width: u8, height: u8) -> Result<Self, PositionError> {
        Self::check_dimensions(width, height)?;

        Ok(Self {
            width,
            height,
            grid: 0,
        })
    }

    /// Creates empty grid of zero size
    #[must_use]
    pub const fn zero_size() -> Self {
        Self {
            width: 0,
            height: 0,
            grid: 0,
        }
    }

    /// Creates filled grid with given size.
    ///
    /// # Examples
    ///
    /// ```
    /// use cgt::short::partizan::games::domineering::Domineering;
    /// assert_eq!(&format!("{}", Domineering::filled(3, 2).unwrap()), "###|###");
    /// ```
    ///
    /// # Errors
    /// - Grid has more than 64 tiles
    pub fn filled(width: u8, height: u8) -> Result<Self, PositionError> {
        Self::check_dimensions(width, height)?;

        Ok(Self {
            width,
            height,
            grid: GridBits::MAX,
        })
    }

    /// Parses a grid from `.#` notation.
    ///
    /// # Arguments
    ///
    /// * `input` - `.#` notation with `|` as rows separator
    ///
    /// # Examples
    ///
    /// ```
    /// use cgt::short::partizan::games::domineering::Domineering;
    /// Domineering::parse("..#|.#.|##.").unwrap();
    /// ```
    ///
    /// # Errors
    /// - Grid has more than 64 tiles
    /// - Input is in invalid format
    pub fn parse(input: &str) -> Result<Self, PositionError> {
        // number of chars till first '|' or eof is the width
        // number of '|' + 1 is the height
        let width = input
            .split('|')
            .next()
            .ok_or(PositionError::CouldNotParse)?
            .len() as u8;
        let height = input.chars().filter(|c| *c == '|').count() as u8 + 1;

        let mut grid = Self::empty(width, height)?;
        let mut x = 0;
        let mut y = 0;

        for chr in input.chars() {
            if chr == '|' {
                if x == width {
                    x = 0;
                    y += 1;
                    continue;
                }
                // Not a rectangle
                return Err(PositionError::CouldNotParse);
            }
            grid.set(
                x,
                y,
                match chr {
                    '.' => false,
                    '#' => true,
                    _ => Err(PositionError::CouldNotParse)?,
                },
            );
            x += 1;
        }
        Ok(grid)
    }

    /// Create a grid that correspondes to given size and "internal id".
    ///
    /// # Arguments
    ///
    /// `grid_id` - A number that represents empty and taken grid tiles. Starting from left and the
    /// lowest bit, if bit is 1 then tile is filled, otherwise the tile is empty.
    /// Bits outside grid size are ignored
    ///
    /// # Examples
    ///
    /// ```
    /// use cgt::short::partizan::games::domineering::Domineering;
    /// assert_eq!(&format!("{}", Domineering::from_number(3, 2, 0b101110).unwrap()), ".##|#.#");
    /// ```
    ///
    /// # Errors
    /// - Grid has more than 64 tiles
    pub fn from_number(width: u8, height: u8, grid_id: GridBits) -> Result<Self, PositionError> {
        Self::check_dimensions(width, height)?;
        Ok(Self {
            width,
            height,
            grid: grid_id,
        })
    }

    /// Creates a grid from given array of bools.
    ///
    /// # Arguments
    ///
    /// * `grid` - Lineralized grid of size `width * height`, empty if if value is `false`.
    ///
    /// # Examples
    ///
    /// ```
    /// use cgt::short::partizan::games::domineering::Domineering;
    /// Domineering::from_arr(2, 3, &[true, true, false, false, false, true]).unwrap();
    /// ```
    ///
    /// # Errors
    /// - Grid has more than 64 tiles
    pub fn from_arr(width: u8, height: u8, grid: &[bool]) -> Result<Self, PositionError> {
        Self::from_number(width, height, arr_to_bits(grid))
    }

    /// Get number of columns in the grid
    pub const fn width(&self) -> u8 {
        self.width
    }

    /// Get number of rows in the grid
    pub const fn height(&self) -> u8 {
        self.height
    }

    /// Rotate grid 90° clockwise
    // TODO: Add safe initializers for already verified grids
    // `Self::empty(self.height(), self.width()).unwrap()` can never panic
    #[must_use]
    #[cfg_attr(feature = "cargo-clippy", allow(clippy::missing_panics_doc))]
    pub fn rotate(&self) -> Self {
        let mut result = Self::empty(self.height(), self.width()).unwrap();
        for y in 0..self.height() {
            for x in 0..self.width() {
                result.set(result.width() - y - 1, x, self.at(x, y));
            }
        }
        result
    }

    /// Flip grid vertically
    #[must_use]
    pub fn vertical_flip(&self) -> Self {
        let mut result = *self;
        for y in 0..self.height() {
            for x in 0..self.width() {
                result.set(result.width() - x - 1, y, self.at(x, y));
            }
        }
        result
    }

    /// Flip grid horizontally
    #[must_use]
    pub fn horizontal_flip(&self) -> Self {
        let mut result = *self;
        for y in 0..self.height() {
            for x in 0..self.width() {
                result.set(x, result.height() - y - 1, self.at(x, y));
            }
        }
        result
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
        for y in 0..self.height() {
            for x in 0..self.width() {
                if self.at(x, y) {
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
            self.width(),
            self.height()
        )
        .unwrap();
        buf
    }

    /// Remove filled rows and columns from the edges
    ///
    /// # Examples
    /// ```
    /// use cgt::short::partizan::games::domineering::Domineering;
    /// let position = Domineering::parse("###|.#.|##.").unwrap();
    /// assert_eq!(&format!("{}", position.move_top_left()), ".#.|##.");
    /// ```
    // Panic at `Self::empty(minimized_width, minimized_height).unwrap();` is unreachable
    #[must_use]
    #[cfg_attr(feature = "cargo-clippy", allow(clippy::missing_panics_doc))]
    pub fn move_top_left(&self) -> Self {
        let mut filled_top_rows = 0;
        for y in 0..self.height {
            let mut should_break = false;
            for x in 0..self.width {
                // If empty space then break
                if !self.at(x, y) {
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

        if filled_top_rows == self.height {
            return Self::zero_size();
        }

        let mut filled_bottom_rows = 0;
        for y in 0..self.height {
            let mut should_break = false;
            for x in 0..self.width {
                // If empty space then break
                if !self.at(x, self.height - y - 1) {
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
        for x in 0..self.width {
            let mut should_break = false;
            for y in 0..self.height {
                // If empty space then break
                if !self.at(x, y) {
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

        if filled_left_cols == self.width {
            return Self::zero_size();
        }

        let mut filled_right_cols = 0;
        for x in 0..self.width {
            let mut should_break = false;
            for y in 0..self.height {
                // If empty space then break
                if !self.at(self.width - x - 1, y) {
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

        let minimized_width = self.width - filled_left_cols - filled_right_cols;
        let minimized_height = self.height - filled_top_rows - filled_bottom_rows;

        let mut grid = Self::empty(minimized_width, minimized_height).unwrap();
        for y in filled_top_rows..(self.height - filled_bottom_rows) {
            for x in filled_left_cols..(self.width - filled_right_cols) {
                grid.set(x - filled_left_cols, y - filled_top_rows, self.at(x, y));
            }
        }
        grid
    }

    fn bfs(&self, visited: &mut Self, x: u8, y: u8) -> Self {
        let mut grid = Self::filled(self.width, self.height).unwrap();

        let mut q: VecDeque<(u8, u8)> =
            VecDeque::with_capacity(self.width as usize * self.height as usize);
        q.push_back((x, y));
        while let Some((qx, qy)) = q.pop_front() {
            visited.set(qx, qy, true);
            grid.set(qx, qy, false);
            let directions: [(i64, i64); 4] = [(1, 0), (-1, 0), (0, 1), (0, -1)];
            for (dx, dy) in directions {
                let lx = (qx as i64) + dx;
                let ly = (qy as i64) + dy;

                if lx >= 0
                    && lx < (self.width as i64)
                    && ly >= 0
                    && ly < (self.height as i64)
                    && !self.at(lx as u8, ly as u8)
                    && !visited.at(lx as u8, ly as u8)
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
        for y in 0..self.height() {
            for x in 0..self.width() {
                if !self.at(x, y) {
                    res += 1;
                }
            }
        }
        res
    }

    #[inline]
    /// Get value of given tile. Warning: UB if out of range
    pub const fn at(&self, x: u8, y: u8) -> bool {
        let n = self.width as GridBits * y as GridBits + x as GridBits;
        (self.grid >> n) & 1 == 1
    }

    #[inline]
    /// Set value of given tile. Warning: UB if out of range
    pub fn set(&mut self, x: u8, y: u8, val: bool) {
        let val = val as GridBits;
        let n = self.width as GridBits * y as GridBits + x as GridBits;
        self.grid = (self.grid & !(1 << n)) | (val << n);
    }

    fn moves_for<const DIR_X: u8, const DIR_Y: u8>(&self) -> Vec<Self> {
        let mut moves = Vec::new();

        if self.height == 0 || self.width == 0 {
            return moves;
        }

        for y in 0..(self.height - DIR_Y) {
            for x in 0..(self.width - DIR_X) {
                let next_x = x + DIR_X;
                let next_y = y + DIR_Y;
                if !self.at(x, y) && !self.at(next_x, next_y) {
                    let mut new_grid = *self;
                    new_grid.set(x, y, true);
                    new_grid.set(next_x, next_y, true);
                    moves.push(new_grid.move_top_left());
                }
            }
        }
        moves.sort_unstable();
        moves.dedup();
        moves
    }
}

impl FromStr for Domineering {
    type Err = PositionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

#[test]
#[should_panic]
fn grid_max_size_is_respected() {
    Domineering::empty(10, 10).unwrap();
}

#[test]
fn parse_grid() {
    let width = 3;
    let height = 3;
    assert_eq!(
        Domineering::parse("..#|.#.|##.").unwrap(),
        Domineering::from_arr(
            width,
            height,
            &[false, false, true, false, true, false, true, true, false]
        )
        .unwrap()
    );
}

#[test]
fn set_works() {
    let mut grid = Domineering::parse(".#.|##.").unwrap();
    grid.set(2, 1, true);
    grid.set(0, 0, true);
    grid.set(1, 0, false);
    assert_eq!(&format!("{}", grid), "#..|###",);
}

impl PartizanGame for Domineering {
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
    ///
    /// let position = Domineering::parse("..#|.#.|##.").unwrap();
    /// assert_eq!(
    ///     position.left_moves(),
    ///     vec![
    ///         Domineering::parse("..|.#").unwrap(),
    ///         Domineering::parse(".#|#.|#.").unwrap(),
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
    /// use cgt::short::partizan::games::domineering::Domineering;
    /// use crate::cgt::short::partizan::partizan_game::PartizanGame;
    ///
    /// let position = Domineering::parse("..#|.#.|##.").unwrap();
    /// assert_eq!(
    ///     position.right_moves(),
    ///     vec![Domineering::parse(".#.|##.").unwrap(),]
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
    /// use cgt::short::partizan::games::domineering::Domineering;
    /// use crate::cgt::short::partizan::partizan_game::PartizanGame;
    ///
    /// let position = Domineering::parse("..#|.#.|##.").unwrap();
    /// assert_eq!(
    ///    position.decompositions(),
    ///    vec![
    ///        Domineering::parse("..|.#").unwrap(),
    ///        Domineering::parse(".|.").unwrap(),
    ///    ]
    /// );
    /// ```
    fn decompositions(&self) -> Vec<Self> {
        let mut visited = Self::empty(self.width, self.height).unwrap();
        let mut ds = Vec::new();

        for y in 0..self.height {
            for x in 0..self.width {
                if !self.at(x, y) && !visited.at(x, y) {
                    ds.push(self.bfs(&mut visited, x, y));
                }
            }
        }

        ds
    }
}

impl Display for Domineering {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for y in 0..self.height {
            for x in 0..self.width {
                let chr = if self.at(x, y) { '#' } else { '.' };
                write!(f, "{}", chr)?;
            }
            if y != self.height - 1 {
                write!(f, "|")?;
            }
        }
        Ok(())
    }
}

#[test]
fn parse_display_roundtrip() {
    let inp = "...|#.#|##.|###";
    assert_eq!(&format!("{}", Domineering::parse(inp).unwrap()), inp,);
}

// Values confirmed with gcsuite

#[cfg(test)]
fn test_grid_canonical_form(grid: Domineering, canonical_form: &str) {
    let cache = TranspositionTable::new();
    let game_id = grid.canonical_form(&cache);
    assert_eq!(&game_id.to_string(), canonical_form);
}

#[test]
fn finds_canonical_form_of_one() {
    test_grid_canonical_form(Domineering::empty(1, 2).unwrap(), "1");
}

#[test]
fn finds_canonical_form_of_minus_one() {
    test_grid_canonical_form(Domineering::empty(2, 1).unwrap(), "-1");
}

#[test]
fn finds_canonical_form_of_two_by_two() {
    test_grid_canonical_form(Domineering::empty(2, 2).unwrap(), "{1|-1}");
}

#[test]
fn finds_canonical_form_of_two_by_two_with_noise() {
    test_grid_canonical_form(Domineering::parse("..#|..#|##.").unwrap(), "{1|-1}");
}

#[test]
fn finds_canonical_form_of_minus_two() {
    test_grid_canonical_form(Domineering::empty(4, 1).unwrap(), "-2");
}

#[test]
fn finds_canonical_form_of_l_shape() {
    test_grid_canonical_form(Domineering::parse(".#|..").unwrap(), "*");
}

#[test]
fn finds_canonical_form_of_long_l_shape() {
    test_grid_canonical_form(Domineering::parse(".##|.##|...").unwrap(), "0");
}

#[test]
fn finds_canonical_form_of_weird_l_shape() {
    test_grid_canonical_form(Domineering::parse("..#|..#|...").unwrap(), "{1/2|-2}");
}

#[test]
fn finds_canonical_form_of_three_by_three() {
    test_grid_canonical_form(Domineering::empty(3, 3).unwrap(), "{1|-1}");
}

#[test]
fn finds_canonical_form_of_num_nim_sum() {
    test_grid_canonical_form(Domineering::parse(".#.#|.#..").unwrap(), "1*");
}

#[test]
fn finds_temperature_of_four_by_four_grid() {
    use crate::numeric::rational::Rational;

    let cache = TranspositionTable::new();
    let grid = Domineering::parse("#...|....|....|....").unwrap();
    let game_id = grid.canonical_form(&cache);
    let temp = game_id.temperature();
    assert_eq!(&game_id.to_string(), "{1*|-1*}");
    assert_eq!(temp, Rational::from(1));
}

#[test]
fn latex_works() {
    let position = Domineering::parse("##..|....|#...|..##").unwrap();
    let latex = position.to_latex();
    assert_eq!(
        &latex,
        r#"\begin{tikzpicture}[scale=1] \fill[fill=gray] (0,0) rectangle (1,1); \fill[fill=gray] (1,0) rectangle (2,1); \fill[fill=gray] (0,2) rectangle (1,3); \fill[fill=gray] (2,3) rectangle (3,4); \fill[fill=gray] (3,3) rectangle (4,4); \draw[step=1cm,black] (0,0) grid (4, 4); \end{tikzpicture}"#
    );
}

#[test]
fn rotation_works() {
    let position = Domineering::parse(
        "##..|\
	 ....|\
	 #..#",
    )
    .unwrap()
    .rotate();

    assert_eq!(
        &format!("{position}"),
        "#.#|\
	 ..#|\
	 ...|\
	 #.."
    );

    let position = position.rotate();
    assert_eq!(
        &format!("{position}"),
        "#..#|\
	 ....|\
	 ..##"
    );
}

#[test]
fn flip_works() {
    let position = Domineering::parse(
        "##..|\
	 ....|\
	 #..#",
    )
    .unwrap();

    assert_eq!(
        &format!("{}", position.vertical_flip()),
        "..##|\
	 ....|\
	 #..#",
    );

    assert_eq!(
        &format!("{}", position.horizontal_flip()),
        "#..#|\
	 ....|\
	 ##..",
    );
}

/// Assert temperature value without going through canonical form
/// Using macro yields better error location on assertion failure
#[cfg(test)]
macro_rules! assert_temperature {
    ($grid:expr, $temp:expr) => {
        let grid = $grid.unwrap();
        let thermograph = grid.thermograph_direct();
        let expected_temperature = Rational::from($temp);
        assert_eq!(thermograph.get_temperature(), expected_temperature);
    };
}

#[test]
fn temperature_without_game_works() {
    assert_temperature!(Domineering::empty(0, 0), -1);
    assert_temperature!(Domineering::parse(".."), -1);
    assert_temperature!(Domineering::parse("..|.#"), 0);
    // FIXME: takes too long
    // assert_temperature!(Domineering::parse("#...|....|....|...."), 1);
}
