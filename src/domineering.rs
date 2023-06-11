//! Domineering game

extern crate alloc;
use alloc::collections::vec_deque::VecDeque;
use std::fmt::Display;

use crate::{
    short_canonical_game::{Game, Moves, PartizanShortGame},
    transposition_table::TranspositionTable,
};

pub type GridBits = u64;

/// A Domineering position on a rectengular grid.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct Position {
    width: u8,
    height: u8,
    grid: GridBits,
}

/// Convert bits in a number to an array but in reverse order.
pub fn bits_to_arr(num: GridBits) -> [bool; 64] {
    let mut grid = [false; 64];
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

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum PositionError {
    TooLarge,
    CouldNotParse,
}

impl Position {
    /// Check if dimensions are small enough to fit in the fixed-size bit representation.
    fn check_dimensions(width: u8, height: u8) -> Result<(), PositionError> {
        if (width as usize * height as usize) > 8 * std::mem::size_of::<GridBits>() {
            Err(PositionError::TooLarge)?
        }
        Ok(())
    }

    /// Creates empty grid with given size.
    ///
    /// # Examples
    ///
    /// ```
    /// use cgt::domineering::Position;
    /// assert_eq!(&format!("{}", Position::empty(2, 3).unwrap()), "..|..|..");
    /// ```
    pub fn empty(width: u8, height: u8) -> Result<Position, PositionError> {
        Position::check_dimensions(width, height)?;

        Ok(Position {
            width,
            height,
            grid: 0,
        })
    }

    /// Creates filled grid with given size.
    ///
    /// # Examples
    ///
    /// ```
    /// use cgt::domineering::Position;
    /// assert_eq!(&format!("{}", Position::filled(3, 2).unwrap()), "###|###");
    /// ```
    pub fn filled(width: u8, height: u8) -> Result<Position, PositionError> {
        Position::check_dimensions(width, height)?;

        Ok(Position {
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
    /// use cgt::domineering::Position;
    /// Position::parse(3, 3, "..#|.#.|##.").unwrap();
    /// ```
    pub fn parse(width: u8, height: u8, input: &str) -> Result<Position, PositionError> {
        let mut grid = Position::empty(width, height)?;
        let mut x = 0;
        let mut y = 0;

        for chr in input.chars() {
            if chr == '|' {
                if x == width {
                    x = 0;
                    y += 1;
                    continue;
                } else {
                    // Not a rectangle
                    Err(PositionError::CouldNotParse)?;
                }
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
    /// use cgt::domineering::Position;
    /// assert_eq!(&format!("{}", Position::from_number(3, 2, 0b101110).unwrap()), ".##|#.#");
    /// ```
    pub fn from_number(
        width: u8,
        height: u8,
        grid_id: GridBits,
    ) -> Result<Position, PositionError> {
        Position::check_dimensions(width, height)?;
        Ok(Position {
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
    /// use cgt::domineering::Position;
    /// Position::from_arr(2, 3, &[true, true, false, false, false, true]).unwrap();
    /// ```
    pub fn from_arr(width: u8, height: u8, grid: &[bool]) -> Result<Position, PositionError> {
        Position::from_number(width, height, arr_to_bits(grid))
    }

    /// Get number of columns in the grid
    pub fn width(&self) -> u8 {
        self.width
    }

    /// Get number of rows in the grid
    pub fn height(&self) -> u8 {
        self.height
    }
}

#[test]
#[should_panic]
fn grid_max_size_is_respected() {
    Position::empty(10, 10).unwrap();
}

#[test]
fn parse_grid() {
    let width = 3;
    let height = 3;
    assert_eq!(
        Position::parse(width, height, "..#|.#.|##.").unwrap(),
        Position::from_arr(
            width,
            height,
            &[false, false, true, false, true, false, true, true, false]
        )
        .unwrap()
    );
}

#[test]
fn set_works() {
    let mut grid = Position::parse(3, 2, ".#.|##.").unwrap();
    grid.set(2, 1, true);
    grid.set(0, 0, true);
    grid.set(1, 0, false);
    assert_eq!(&format!("{}", grid), "#..|###",);
}

impl Position {
    #[inline]
    pub fn at(&self, x: u8, y: u8) -> bool {
        let n = self.width as GridBits * y as GridBits + x as GridBits;
        (self.grid >> n) & 1 == 1
    }

    #[inline]
    pub fn set(&mut self, x: u8, y: u8, val: bool) -> () {
        let val = val as GridBits;
        let n = self.width as GridBits * y as GridBits + x as GridBits;
        self.grid = (self.grid & !(1 << n)) | (val << n);
    }

    fn moves_for<const DIR_X: u8, const DIR_Y: u8>(&self) -> Vec<Position> {
        let mut moves = Vec::new();

        if self.height == 0 || self.width == 0 {
            return moves;
        }

        for y in 0..(self.height - DIR_Y) {
            for x in 0..(self.width - DIR_X) {
                let next_x = x + DIR_X;
                let next_y = y + DIR_Y;
                if !self.at(x, y) && !self.at(next_x, next_y) {
                    let mut new_grid = self.clone();
                    new_grid.set(x, y, true);
                    new_grid.set(next_x, next_y, true);
                    moves.push(new_grid);
                }
            }
        }
        moves
    }
}
impl PartizanShortGame for Position {
    /// Get moves for the Left player as positions she can move to.
    ///
    /// # Examples
    ///
    /// ```
    /// use cgt::domineering::Position;
    /// use crate::cgt::short_canonical_game::PartizanShortGame;
    ///
    /// let width = 3;
    /// let height = 3;
    /// let position = Position::parse(width, height, "..#|.#.|##.").unwrap();
    /// assert_eq!(
    ///     position.left_moves(),
    ///     vec![
    ///         Position::parse(width, height, "#.#|##.|##.").unwrap(),
    ///         Position::parse(width, height, "..#|.##|###").unwrap(),
    ///     ]
    /// );
    /// ```
    fn left_moves(&self) -> Vec<Position> {
        self.moves_for::<0, 1>()
    }

    /// Get moves for the Right player as positions he can move to.
    ///
    /// # Examples
    ///
    /// ```
    /// use cgt::domineering::Position;
    /// use crate::cgt::short_canonical_game::PartizanShortGame;
    ///
    /// let width = 3;
    /// let height = 3;
    /// let position = Position::parse(width, height, "..#|.#.|##.").unwrap();
    /// assert_eq!(
    ///     position.right_moves(),
    ///     vec![Position::parse(width, height, "###|.#.|##.").unwrap(),]
    /// );
    /// ```
    fn right_moves(&self) -> Vec<Position> {
        self.moves_for::<1, 0>()
    }
}

impl Display for Position {
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
    assert_eq!(&format!("{}", Position::parse(3, 4, inp).unwrap()), inp,);
}

impl Position {
    /// Remove filled rows and columns from the edges
    ///
    /// # Examples
    /// ```
    /// use cgt::domineering::Position;
    /// let position = Position::parse(3, 3, "###|.#.|##.").unwrap();
    /// assert_eq!(&format!("{}", position.move_top_left()), ".#.|##.");
    /// ```
    pub fn move_top_left(&self) -> Position {
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
            return Position::empty(0, 0).unwrap();
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
            return Position::empty(0, 0).unwrap();
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

        let mut grid = Position::empty(minimized_width, minimized_height).unwrap();
        for y in filled_top_rows..(self.height - filled_bottom_rows) {
            for x in filled_left_cols..(self.width - filled_right_cols) {
                grid.set(x - filled_left_cols, y - filled_top_rows, self.at(x, y));
            }
        }
        grid
    }

    fn bfs(&self, visited: &mut Position, x: u8, y: u8) -> Position {
        let mut grid = Position::filled(self.width, self.height).unwrap();

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

    /// Get decompisitons of given position
    ///
    /// # Examples
    /// ```
    /// // ..#   ..#   ###
    /// // .#. = .## + ##.
    /// // ##.   ###   ##.
    ///
    /// use cgt::domineering::Position;
    /// let position = Position::parse(3, 3, "..#|.#.|##.").unwrap();
    /// assert_eq!(
    ///    position.decompositons(),
    ///    vec![
    ///        Position::parse(2, 2, "..|.#").unwrap(),
    ///        Position::parse(1, 2, ".|.").unwrap(),
    ///    ]
    /// );
    /// ```
    pub fn decompositons(&self) -> Vec<Position> {
        let mut visited = Position::empty(self.width, self.height).unwrap();
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

impl Position {
    /// Get the canonical form of the position.
    ///
    /// # Arguments
    ///
    /// `cache` - Shared cache of short combinatorial games.
    ///
    /// # Examples
    ///
    /// ```
    /// use cgt::domineering::Position;
    /// use cgt::transposition_table::TranspositionTable;
    ///
    /// let cache = TranspositionTable::new();
    /// let position = Position::parse(2, 2, ".#|..").unwrap();
    /// let game = position.canonical_form(&cache);
    /// assert_eq!(&cache.game_backend().print_game_to_str(game), "*");
    /// ```
    pub fn canonical_form(&self, cache: &TranspositionTable<Self>) -> Game {
        let grid = self.move_top_left();
        if let Some(id) = cache.grids.get(&grid) {
            return id;
        }

        let result = Position::canonical_from_from_decompositions(grid.decompositons(), cache);
        cache.grids.insert(grid, result);
        result
    }

    /// Get the canonical from from the decompositon of the position rather than from the position itself.
    /// Useful if you obtain decompositions before deciding whether to calculate the canonical form.
    pub fn canonical_from_from_decompositions(
        decompositions: Vec<Position>,
        cache: &TranspositionTable<Self>,
    ) -> Game {
        let mut result = cache.game_backend.construct_integer(0);
        for grid in decompositions {
            if let Some(id) = cache.grids.get(&grid) {
                result = cache.game_backend.construct_sum(id, result);
                continue;
            }

            let moves = Moves {
                left: grid
                    .left_moves()
                    .iter()
                    .map(|o| o.canonical_form(cache))
                    .collect(),
                right: grid
                    .right_moves()
                    .iter()
                    .map(|o| o.canonical_form(cache))
                    .collect(),
            };

            let canonical_form = cache.game_backend.construct_from_moves(moves);
            cache.grids.insert(grid, canonical_form);
            result = cache.game_backend.construct_sum(canonical_form, result);
        }

        result
    }
}

// Values confirmed with gcsuite

#[cfg(test)]
fn test_grid_canonical_form(grid: Position, canonical_form: &str) {
    let cache = TranspositionTable::new();
    let game_id = grid.canonical_form(&cache);
    assert_eq!(
        &cache.game_backend.print_game_to_str(game_id),
        canonical_form
    );
}

#[test]
fn finds_canonical_form_of_one() {
    test_grid_canonical_form(Position::empty(1, 2).unwrap(), "1");
}

#[test]
fn finds_canonical_form_of_minus_one() {
    test_grid_canonical_form(Position::empty(2, 1).unwrap(), "-1");
}

#[test]
fn finds_canonical_form_of_two_by_two() {
    test_grid_canonical_form(Position::empty(2, 2).unwrap(), "{1|-1}");
}

#[test]
fn finds_canonical_form_of_two_by_two_with_noise() {
    test_grid_canonical_form(Position::parse(3, 3, "..#|..#|##.").unwrap(), "{1|-1}");
}

#[test]
fn finds_canonical_form_of_minus_two() {
    test_grid_canonical_form(Position::empty(4, 1).unwrap(), "-2");
}

#[test]
fn finds_canonical_form_of_l_shape() {
    test_grid_canonical_form(Position::parse(2, 2, ".#|..").unwrap(), "*");
}

#[test]
fn finds_canonical_form_of_long_l_shape() {
    test_grid_canonical_form(Position::parse(3, 3, ".##|.##|...").unwrap(), "0");
}

#[test]
fn finds_canonical_form_of_weird_l_shape() {
    test_grid_canonical_form(Position::parse(3, 3, "..#|..#|...").unwrap(), "{1/2|-2}");
}

#[test]
fn finds_canonical_form_of_three_by_three() {
    test_grid_canonical_form(Position::empty(3, 3).unwrap(), "{1|-1}");
}

#[test]
fn finds_canonical_form_of_num_nim_sum() {
    test_grid_canonical_form(Position::parse(4, 2, ".#.#|.#..").unwrap(), "1*");
}

#[test]
fn finds_temperature_of_four_by_four_grid() {
    use crate::rational::Rational;

    let cache = TranspositionTable::new();
    let grid = Position::parse(4, 4, "#...|....|....|....").unwrap();
    let game_id = grid.canonical_form(&cache);
    let temp = cache.game_backend.temperature(game_id);
    assert_eq!(&cache.game_backend.print_game_to_str(game_id), "{1*|-1*}");
    assert_eq!(temp, Rational::from(1));
}
