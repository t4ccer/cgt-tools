//! Grid with up to 64 tiles holding a single bit of information.

use crate::grid::{BitTile, CharTile, FiniteGrid, Grid};
use std::{fmt::Display, marker::PhantomData, str::FromStr};

/// Internal representation of a grid
type GridBits = u64;

/// A grid with up to 64 tiles holding a single bit of information.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SmallBitGrid<T> {
    width: u8,
    height: u8,
    grid: GridBits,
    _ty: PhantomData<T>,
}

impl<T> Grid for SmallBitGrid<T>
where
    T: BitTile,
{
    type Item = T;

    fn get(&self, x: u8, y: u8) -> Self::Item {
        let n = self.width as GridBits * y as GridBits + x as GridBits;
        BitTile::bool_to_tile((self.grid >> n) & 1 == 1)
    }

    fn set(&mut self, x: u8, y: u8, value: Self::Item) {
        let val = value.tile_to_bool() as GridBits;
        let n = self.width as GridBits * y as GridBits + x as GridBits;
        self.grid = (self.grid & !(1 << n)) | (val << n);
    }
}

impl<T> FiniteGrid for SmallBitGrid<T>
where
    T: BitTile,
{
    fn width(&self) -> u8 {
        self.width
    }

    fn height(&self) -> u8 {
        self.height
    }

    fn filled(width: u8, height: u8, value: T) -> Option<Self> {
        Self::check_dimensions(width, height)?;

        Some(Self {
            width,
            height,
            grid: if value.tile_to_bool() {
                GridBits::MAX
            } else {
                0
            },
            _ty: PhantomData,
        })
    }

    #[must_use]
    fn zero_size() -> Self {
        Self {
            width: 0,
            height: 0,
            grid: 0,
            _ty: PhantomData,
        }
    }
}

impl<T> Display for SmallBitGrid<T>
where
    T: BitTile + CharTile,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.display(f, '|')
    }
}

impl<T> SmallBitGrid<T>
where
    T: BitTile,
{
    /// Check if dimensions are small enough to fit in the fixed-size bit representation.
    const fn check_dimensions(width: u8, height: u8) -> Option<()> {
        if (width as usize * height as usize) > 8 * std::mem::size_of::<GridBits>() {
            return None;
        }
        Some(())
    }

    /// Creates empty grid with given size.
    ///
    /// # Examples
    ///
    /// ```
    /// use cgt::grid::small_bit_grid::SmallBitGrid;
    ///
    /// assert_eq!(&format!("{}", SmallBitGrid::<bool>::empty(2, 3).unwrap()), "..|..|..");
    /// ```
    ///
    /// # Errors
    /// - Grid has more than 64 tiles
    pub fn empty(width: u8, height: u8) -> Option<Self> {
        Self::check_dimensions(width, height)?;

        Some(Self {
            width,
            height,
            grid: 0,
            _ty: PhantomData,
        })
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
    /// use cgt::grid::small_bit_grid::SmallBitGrid;
    ///
    /// assert_eq!(&format!("{}", SmallBitGrid::<bool>::from_number(3, 2, 0b101110).unwrap()), ".##|#.#");
    /// ```
    ///
    /// # Errors
    /// - Grid has more than 64 tiles
    pub fn from_number(width: u8, height: u8, grid_id: GridBits) -> Option<Self> {
        Self::check_dimensions(width, height)?;
        Some(Self {
            width,
            height,
            grid: grid_id,
            _ty: PhantomData,
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
    /// use cgt::grid::small_bit_grid::SmallBitGrid;
    ///
    /// SmallBitGrid::<bool>::from_arr(2, 3, &[true, true, false, false, false, true]).unwrap();
    /// ```
    ///
    /// # Errors
    /// - Grid has more than 64 tiles
    pub fn from_arr(width: u8, height: u8, grid: &[bool]) -> Option<Self> {
        Self::from_number(width, height, arr_to_bits(grid))
    }

    /// Rotate grid 90° clockwise
    #[must_use]
    #[cfg_attr(feature = "cargo-clippy", allow(clippy::missing_panics_doc))]
    pub fn rotate(&self) -> Self {
        let mut result = Self::empty(self.height(), self.width()).unwrap();
        for y in 0..self.height() {
            for x in 0..self.width() {
                result.set(result.width() - y - 1, x, self.get(x, y));
            }
        }
        result
    }

    /// Flip grid vertically
    #[must_use]
    pub fn vertical_flip(&self) -> Self
    where
        Self: Clone,
    {
        let mut result: Self = self.clone();
        for y in 0..self.height() {
            for x in 0..self.width() {
                result.set(result.width() - x - 1, y, self.get(x, y));
            }
        }
        result
    }

    /// Flip grid horizontally
    #[must_use]
    pub fn horizontal_flip(&self) -> Self
    where
        Self: Clone,
    {
        let mut result: Self = self.clone();
        for y in 0..self.height() {
            for x in 0..self.width() {
                result.set(x, result.height() - y - 1, self.get(x, y));
            }
        }
        result
    }
}

impl<T> FromStr for SmallBitGrid<T>
where
    T: BitTile + CharTile + Default,
{
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s).ok_or(())
    }
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

/// Convert bits in a number to an array but in reverse order.
pub fn bits_to_arr(num: GridBits) -> [bool; 64] {
    let mut grid = [false; 64];

    #[allow(clippy::needless_range_loop)]
    for grid_idx in 0..64 {
        grid[grid_idx] = ((num >> grid_idx) & 1) == 1;
    }
    grid
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_works() {
        let mut grid = SmallBitGrid::parse(".#.|##.").unwrap();
        grid.set(2, 1, true);
        grid.set(0, 0, true);
        grid.set(1, 0, false);
        assert_eq!(&format!("{}", grid), "#..|###",);
    }

    #[test]
    fn bits_to_arr_works() {
        assert_eq!(
            bits_to_arr(0b101_1001),
            [
                true, false, false, true, true, false, true, false, false, false, false, false,
                false, false, false, false, false, false, false, false, false, false, false, false,
                false, false, false, false, false, false, false, false, false, false, false, false,
                false, false, false, false, false, false, false, false, false, false, false, false,
                false, false, false, false, false, false, false, false, false, false, false, false,
                false, false, false, false
            ]
        );
    }

    #[test]
    fn bits_to_arr_to_bits_roundtrip() {
        let inp = 3_874_328;
        assert_eq!(inp, arr_to_bits(&bits_to_arr(inp)),);
    }

    #[test]
    fn parse_grid() {
        let width = 3;
        let height = 3;
        assert_eq!(
            SmallBitGrid::<bool>::parse("..#|.#.|##.").unwrap(),
            SmallBitGrid::from_arr(
                width,
                height,
                &[false, false, true, false, true, false, true, true, false]
            )
            .unwrap()
        );
    }

    #[should_panic]
    #[test]
    fn parse_invalid_char() {
        SmallBitGrid::<bool>::from_str("...#|..X#|.#..").unwrap();
    }

    #[should_panic]
    #[test]
    fn parse_non_rectangular() {
        SmallBitGrid::<bool>::from_str("...#|..#|.#..").unwrap();
    }

    #[should_panic]
    #[test]
    fn parse_non_rectangular_last() {
        SmallBitGrid::<bool>::from_str("...#|..#.|.#.").unwrap();
    }

    #[test]
    fn rotation_works() {
        let position = SmallBitGrid::<bool>::from_str(
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
        let position = SmallBitGrid::<bool>::parse(
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
}
