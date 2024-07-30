//! Finite grids

use std::{collections::VecDeque, fmt::Write};

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

/// Trait for finite grids.
pub trait FiniteGrid: Grid + Sized {
    /// Width of the grid.
    fn width(&self) -> u8;

    /// Height of the grid.
    fn height(&self) -> u8;

    /// Create new gird filled with the same tile
    fn filled(width: u8, height: u8, value: Self::Item) -> Option<Self>;

    /// Create new zero-sized grid
    fn zero_size() -> Self;

    /// Default, one-line display function for grids using `|` as row separator
    #[cfg_attr(feature = "cargo-clippy", allow(clippy::missing_errors_doc))]
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

    /// Parse grid from string following notation from [`Self::display`]
    fn parse(input: &str) -> Option<Self>
    where
        Self::Item: CharTile + Default,
    {
        let row_separator = '|';
        let width = input.split(row_separator).next()?.len() as u8;
        let height = input.chars().filter(|c| *c == row_separator).count() as u8 + 1;

        let mut grid = Self::filled(width, height, Default::default())?;
        let mut x = 0;
        let mut y = 0;

        for chr in input.chars() {
            if chr == row_separator {
                if x == width {
                    x = 0;
                    y += 1;
                    continue;
                }
                // Not a rectangle
                return None;
            }

            if x >= width {
                return None;
            }

            let value = Self::Item::char_to_tile(chr)?;
            grid.set(x, y, value);
            x += 1;
        }

        if x != width {
            // Not a rectangle in the last row
            return None;
        }
        Some(grid)
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
        if self {
            '#'
        } else {
            '.'
        }
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
    is_non_blocking: fn(T) -> bool,
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
    is_non_blocking: fn(T) -> bool,
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
                    is_non_blocking,
                    blocking_tile,
                    directions,
                ));
            }
        }
    }

    ds
}

/// Remove filled rows and columns from the edges
pub fn move_top_left<G, T>(grid: &G, is_non_blocking: fn(T) -> bool) -> G
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
