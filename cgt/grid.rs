//! Finite grids

use std::{collections::VecDeque, convert::Infallible, fmt::Write};

use crate::{
    drawing::{self, BoundingBox, Canvas, Hits},
    numeric::v2f::V2f,
    result::UnwrapInfallible,
};

pub mod small_bit_grid;
pub mod vec_grid;

/// A rectangular grid
pub trait Grid {
    /// Type of items stored in the grid.
    type Item;

    /// Get item at given position.
    fn get(&self, x: u8, y: u8) -> Self::Item;

    /// Set item at given position.
    fn set(&mut self, x: u8, y: u8, value: Self::Item);
}

/// Grid parser failure
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseErrorReason {
    /// Row has invalid size
    InvalidRowSize {
        /// Which row was invalid
        row: u8,

        /// Expected row length
        expected: u8,

        /// Actual row length
        actual: u8,
    },

    /// Input character does not represent a tile
    InvalidCharTile(char),
}

impl std::fmt::Display for ParseErrorReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseErrorReason::InvalidRowSize {
                row,
                expected,
                actual,
            } => {
                write!(
                    f,
                    "row {row} has invalid size (expected: {expected}, got: {actual})"
                )
            }
            ParseErrorReason::InvalidCharTile(c) => write!(f, "invalid tile character: `{c}`"),
        }
    }
}

/// Error that happened during parsing grid from string
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GridParseError<E> {
    /// Construction error (e.g. grid too large)
    ConstructionError(E),

    /// Parsing error
    ParseError(ParseErrorReason), // TODO: Input location
}

impl<E> std::fmt::Display for GridParseError<E>
where
    E: std::fmt::Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GridParseError::ConstructionError(err) => write!(f, "construction error: {err}"),
            GridParseError::ParseError(err) => write!(f, "parse error: {err}"),
        }
    }
}

impl<E> std::error::Error for GridParseError<E>
where
    E: std::error::Error + 'static,
{
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            GridParseError::ConstructionError(err) => Some(err),
            GridParseError::ParseError(_) => None,
        }
    }
}

/// Trait for finite grids.
pub trait FiniteGrid: Grid + Sized {
    /// Error when grid cannot be constructed with given dimensions
    type ConstructionError: std::error::Error;

    /// Width of the grid.
    fn width(&self) -> u8;

    /// Height of the grid.
    fn height(&self) -> u8;

    /// Create new gird filled with the same tile
    #[allow(clippy::missing_errors_doc)]
    fn filled(width: u8, height: u8, value: Self::Item) -> Result<Self, Self::ConstructionError>;

    /// Create new zero-sized grid
    fn zero_size() -> Self;

    /// Default, one-line display function for grids using `|` as row separator
    #[allow(clippy::missing_errors_doc)]
    fn display(&self, w: &mut impl Write, sep: char) -> std::fmt::Result
    where
        Self::Item: CharTile,
    {
        for y in 0..self.height() {
            for x in 0..self.width() {
                write!(w, "{}", self.get(x, y).tile_to_char())?;
            }
            if y != self.height() - 1 {
                write!(w, "{sep}")?;
            }
        }
        Ok(())
    }

    /// Map each tile, potentially changing the grid type
    #[allow(clippy::missing_errors_doc)]
    fn try_map<R, E, G>(
        &self,
        mut f: impl FnMut(Self::Item) -> Result<R, E>,
    ) -> Result<Result<G, G::ConstructionError>, E>
    where
        G: FiniteGrid<Item = R>,
    {
        if self.width() == 0 || self.height() == 0 {
            return Ok(Ok(G::zero_size()));
        }

        let initial = f(self.get(0, 0))?;
        let mut g = match G::filled(self.width(), self.height(), initial) {
            Ok(g) => g,
            Err(err) => return Ok(Err(err)),
        };

        for y in 0..self.height() {
            for x in 0..self.width() {
                let tile = f(self.get(x, y))?;
                g.set(x, y, tile);
            }
        }

        Ok(Ok(g))
    }

    /// Map each tile, potentially changing the grid type
    #[allow(clippy::missing_errors_doc)]
    fn map<R, G>(&self, mut f: impl FnMut(Self::Item) -> R) -> Result<G, G::ConstructionError>
    where
        G: FiniteGrid<Item = R>,
    {
        self.try_map(|tile| Ok::<R, Infallible>(f(tile)))
            .unwrap_infallible()
    }

    /// Parse grid from string following notation from [`Self::display`]
    #[allow(clippy::missing_errors_doc)]
    fn parse(input: &str) -> Result<Self, GridParseError<Self::ConstructionError>>
    where
        Self::Item: CharTile + Default,
    {
        let row_separator = '|';
        let width = input
            .split(row_separator)
            .next()
            .expect("`split` always returns the first element")
            .chars()
            .count() as u8;
        let height = input.chars().filter(|c| *c == row_separator).count() as u8 + 1;

        let mut grid = Self::filled(width, height, Default::default())
            .map_err(GridParseError::ConstructionError)?;

        for (y, row) in input.split(row_separator).enumerate() {
            let y = y as u8;

            // Check the whole row upfront so the error can report its real length
            let row_width = row.chars().count() as u8;
            if row_width != width {
                // Not a rectangle
                return Err(GridParseError::ParseError(
                    ParseErrorReason::InvalidRowSize {
                        row: y,
                        expected: width,
                        actual: row_width,
                    },
                ));
            }

            for (x, chr) in row.chars().enumerate() {
                let value = Self::Item::char_to_tile(chr).ok_or(GridParseError::ParseError(
                    ParseErrorReason::InvalidCharTile(chr),
                ))?;
                grid.set(x as u8, y, value);
            }
        }

        Ok(grid)
    }

    /// Minimum required canvas size to paint the whole grid
    fn canvas_size<C>(&self) -> BoundingBox
    where
        C: Canvas,
    {
        let tile_size = C::tile_size();
        let grid_weight = C::thick_line_weight();
        BoundingBox {
            top_left: V2f::ZERO,
            bottom_right: V2f {
                x: tile_size
                    .x
                    .mul_add(self.width() as f32, grid_weight * (self.width() + 1) as f32),
                y: tile_size.y.mul_add(
                    self.height() as f32,
                    grid_weight * (self.height() + 1) as f32,
                ),
            },
        }
    }

    /// Paint grid on existing canvas, reporting what the pointer is doing to its tiles
    fn draw<C>(
        &self,
        canvas: &mut C,
        mut get_tile: impl FnMut(Self::Item) -> drawing::Tile,
    ) -> Hits<(u8, u8)>
    where
        C: Canvas,
    {
        let mut hits = Hits::new();

        for y in 0..self.height() {
            for x in 0..self.width() {
                let tile = get_tile(self.get(x, y));
                hits.record((x, y), canvas.tile(C::tile_position(x, y), tile));
            }
        }

        canvas.grid(V2f::ZERO, self.width() as u32, self.height() as u32);

        hits
    }

    /// Get tile position from canvas position
    fn tile_at_position<C>(&self, position: V2f) -> Option<(u8, u8)>
    where
        C: Canvas,
    {
        // TODO: Compute that instead of looping
        for y in 0..self.height() {
            for x in 0..self.width() {
                if position.inside_rect(C::tile_position(x, y), C::tile_size()) {
                    return Some((x, y));
                }
            }
        }

        None
    }
}

/// Grid tiles that are representable as a single character, other than `'|'`
pub trait CharTile: Sized {
    /// Convert tile to `char`
    fn tile_to_char(self) -> char;

    /// Convert `char` to tile
    fn char_to_tile(input: char) -> Option<Self>;
}

impl CharTile for bool {
    fn tile_to_char(self) -> char {
        if self { '#' } else { '.' }
    }

    fn char_to_tile(input: char) -> Option<Self> {
        match input {
            '#' => Some(true),
            '.' => Some(false),
            _ => None,
        }
    }
}

/// Grid tiles that can be represented as a single bit
pub trait BitTile: Sized {
    /// Convert tile to `bool`
    fn tile_to_bool(self) -> bool;

    /// Convert `bool` to tile
    fn bool_to_tile(input: bool) -> Self;

    /// Flip the tile
    #[inline]
    #[must_use]
    fn flip(self) -> Self {
        Self::bool_to_tile(!self.tile_to_bool())
    }
}

impl BitTile for bool {
    #[inline]
    fn tile_to_bool(self) -> bool {
        self
    }

    #[inline]
    fn bool_to_tile(input: bool) -> Self {
        input
    }
}

// TODO: SVG tile

// TODO: Use grid of bools
/// Breath first search
#[inline]
fn bfs<G, T>(
    grid: &G,
    visited: &mut G,
    x: u8,
    y: u8,
    mut is_non_blocking: impl FnMut(T) -> bool,
    blocking_tile: T,
    directions: &[(i32, i32)],
) -> G
where
    T: Copy + Default,
    G: Grid<Item = T> + FiniteGrid,
{
    let mut new_grid = G::filled(grid.width(), grid.height(), blocking_tile).unwrap();

    let mut q: VecDeque<(u8, u8)> =
        VecDeque::with_capacity(grid.width() as usize * grid.height() as usize);
    q.push_back((x, y));

    while let Some((qx, qy)) = q.pop_front() {
        visited.set(qx, qy, blocking_tile);
        new_grid.set(qx, qy, grid.get(qx, qy));

        for (dx, dy) in directions {
            let lx = (qx as i32) + dx;
            let ly = (qy as i32) + dy;

            if lx >= 0
                && lx < (grid.width() as i32)
                && ly >= 0
                && ly < (grid.height() as i32)
                && is_non_blocking(grid.get(lx as u8, ly as u8))
                && is_non_blocking(visited.get(lx as u8, ly as u8))
            {
                q.push_back((lx as u8, ly as u8));
            }
        }
    }

    move_top_left(&new_grid, is_non_blocking)
}

/// Decompose a grid
pub fn decompositions<G, T>(
    grid: &G,
    mut is_non_blocking: impl FnMut(T) -> bool,
    blocking_tile: T,
    directions: &[(i32, i32)],
) -> Vec<G>
where
    T: Copy + Default,
    G: Grid<Item = T> + FiniteGrid,
{
    let mut visited: G = G::filled(grid.width(), grid.height(), T::default())
        .expect("unreachable: grid with this size already exists");
    let mut ds = Vec::new();

    for y in 0..grid.height() {
        for x in 0..grid.width() {
            if is_non_blocking(grid.get(x, y)) && is_non_blocking(visited.get(x, y)) {
                ds.push(bfs(
                    grid,
                    &mut visited,
                    x,
                    y,
                    &mut is_non_blocking,
                    blocking_tile,
                    directions,
                ));
            }
        }
    }

    ds
}

/// If `empty_tile` is surrounded (up/down/left/right, diagonals do not count) by other tiles
/// (i.e. `!=` to `empty_tile`) then it will get replaced by `filler_tile`
pub fn fill_one_by_one_holes_with<G, T>(grid: &mut G, empty_tile: T, filler_tile: T)
where
    T: Copy + PartialEq,
    G: Grid<Item = T> + FiniteGrid,
{
    for y in 0..grid.height() {
        for x in 0..grid.width() {
            if grid.get(x, y) == empty_tile {
                let mut is_surrounded = true;
                for (dx, dy) in [(0, -1), (0, 1), (-1, 0), (1, 0)] {
                    let lx = (x as i32) + dx;
                    let ly = (y as i32) + dy;

                    if lx >= 0
                        && lx < (grid.width() as i32)
                        && ly >= 0
                        && ly < (grid.height() as i32)
                        && grid.get(lx as u8, ly as u8) == empty_tile
                    {
                        is_surrounded = false;
                        break;
                    }
                }

                if is_surrounded {
                    grid.set(x, y, filler_tile);
                }
            }
        }
    }
}

#[test]
fn fill_one_by_one_holes_with_test() {
    use small_bit_grid::SmallBitGrid;

    let mut grid: SmallBitGrid<bool> = SmallBitGrid::from_arr(
        3,
        3,
        &[true, true, true, true, false, true, true, true, true],
    )
    .unwrap();
    fill_one_by_one_holes_with(&mut grid, false, true);
    assert_eq!(
        grid,
        SmallBitGrid::from_arr(
            3,
            3,
            &[true, true, true, true, true, true, true, true, true],
        )
        .unwrap()
    );
}

/// Remove filled rows and columns from the edges
pub fn move_top_left<G, T>(grid: &G, mut is_non_blocking: impl FnMut(T) -> bool) -> G
where
    T: Copy + Default,
    G: Grid<Item = T> + FiniteGrid,
{
    let mut filled_top_rows = 0;
    'outer: for y in 0..grid.height() {
        for x in 0..grid.width() {
            // If empty space then break
            if is_non_blocking(grid.get(x, y)) {
                break 'outer;
            }
        }
        filled_top_rows += 1;
    }
    let filled_top_rows = filled_top_rows;

    if filled_top_rows == grid.height() {
        return G::zero_size();
    }

    let mut filled_bottom_rows = 0;
    'outer: for y in 0..grid.height() {
        for x in 0..grid.width() {
            // If empty space then break
            if is_non_blocking(grid.get(x, grid.height() - y - 1)) {
                break 'outer;
            }
        }
        filled_bottom_rows += 1;
    }
    let filled_bottom_rows = filled_bottom_rows;

    let mut filled_left_cols = 0;
    'outer: for x in 0..grid.width() {
        for y in 0..grid.height() {
            // If empty space then break
            if is_non_blocking(grid.get(x, y)) {
                break 'outer;
            }
        }
        filled_left_cols += 1;
    }
    let filled_left_cols = filled_left_cols;

    if filled_left_cols == grid.width() {
        return G::zero_size();
    }

    let mut filled_right_cols = 0;
    'outer: for x in 0..grid.width() {
        for y in 0..grid.height() {
            // If empty space then break
            if is_non_blocking(grid.get(grid.width() - x - 1, y)) {
                break 'outer;
            }
        }
        filled_right_cols += 1;
    }
    let filled_right_cols = filled_right_cols;

    let minimized_width = grid.width() - filled_left_cols - filled_right_cols;
    let minimized_height = grid.height() - filled_top_rows - filled_bottom_rows;

    let mut new_grid = G::filled(minimized_width, minimized_height, T::default())
        .expect("unreachable: size is smaller than original grid");
    for y in filled_top_rows..(grid.height() - filled_bottom_rows) {
        for x in filled_left_cols..(grid.width() - filled_right_cols) {
            new_grid.set(x - filled_left_cols, y - filled_top_rows, grid.get(x, y));
        }
    }
    new_grid
}
