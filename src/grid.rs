//! Finite grids

use std::fmt::Write;

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

    /// Default, one-line display function for grids using `|` as row separator
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

    /// Parse grid from string following notation from [`display`]
    fn parse(input: &str) -> Option<Self>
    where
        Self: Sized,
        Self::Item: CharTile + Default,
    {
        let width = input.split('|').next()?.len() as u8;
        let height = input.chars().filter(|c| *c == '|').count() as u8 + 1;

        let mut grid = Self::filled(width, height, Default::default())?;
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
