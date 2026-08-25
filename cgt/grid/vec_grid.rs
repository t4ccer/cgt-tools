//! Grid with arbitrary finite size

use crate::grid::{FiniteGrid, Grid};
use std::convert::Infallible;

/// Grid with arbitrary finite size
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct VecGrid<T> {
    width: u8,
    height: u8,
    grid: Vec<T>,
}

impl<T> VecGrid<T> {
    // TODO: Unify these with the FiniteGrid methods

    /// Transform grid tiles
    pub fn map<U>(&self, mut f: impl FnMut(&T) -> U) -> VecGrid<U> {
        match self.try_map::<U, Infallible>(|t| Ok(f(t))) {
            Ok(grid) => grid,
            Err(err) => match err {},
        }
    }

    /// Transform grid tiles
    ///
    /// # Errors
    /// Propagates the first error returned by `f`
    pub fn try_map<U, E>(&self, mut f: impl FnMut(&T) -> Result<U, E>) -> Result<VecGrid<U>, E> {
        let mut new_grid = VecGrid {
            width: self.width,
            height: self.height,
            grid: Vec::with_capacity(self.grid.len()),
        };

        for y in 0..self.height {
            for x in 0..self.width {
                let elem = &self.grid[(self.width as usize) * (y as usize) + (x as usize)];
                new_grid.grid.push(f(elem)?);
            }
        }

        Ok(new_grid)
    }
}

impl<T> Grid for VecGrid<T>
where
    T: Clone,
{
    type Item = T;

    fn get(&self, x: u8, y: u8) -> Self::Item {
        self.grid[(self.width as usize) * (y as usize) + (x as usize)].clone()
    }

    fn set(&mut self, x: u8, y: u8, value: Self::Item) {
        self.grid[(self.width as usize) * (y as usize) + (x as usize)] = value;
    }
}

impl<T> FiniteGrid for VecGrid<T>
where
    T: Copy,
{
    fn width(&self) -> u8 {
        self.width
    }

    fn height(&self) -> u8 {
        self.height
    }

    fn filled(width: u8, height: u8, value: T) -> Option<Self> {
        Some(Self {
            width,
            height,
            grid: vec![value; width as usize * height as usize],
        })
    }

    fn zero_size() -> Self {
        Self {
            width: 0,
            height: 0,
            grid: vec![],
        }
    }
}
