//! Grid with arbitrary finite size

use crate::grid::{FiniteGrid, Grid};

/// Grid with arbitrary finite size
#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct VecGrid<T> {
    width: u8,
    height: u8,
    grid: Vec<T>,
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

    #[must_use]
    fn zero_size() -> Self {
        Self {
            width: 0,
            height: 0,
            grid: vec![],
        }
    }
}
