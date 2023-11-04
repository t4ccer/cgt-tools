//! Finite grids

pub mod small_bit_grid;

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
pub trait FiniteGrid: Grid {
    /// Width of the grid.
    fn width(&self) -> u8;

    /// Height of the grid.
    fn height(&self) -> u8;
}
