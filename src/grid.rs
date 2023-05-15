use std::{fmt::Display, mem::MaybeUninit};

use crate::game::Game;

// mod bool_array;
// use bool_array;

// FIXME: some bit array here
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Grid<const W: usize, const H: usize> {
    pub grid: [[bool; W]; H],
}

impl<const W: usize, const H: usize> Grid<W, H> {
    pub fn empty() -> Self {
        let grid: [[bool; W]; H] = [[false; W]; H];

        Self { grid }
    }

    pub fn parse(input: &str) -> Option<Self> {
        // It's safe here, we don't access the array and initialize every field
        let mut grid: [[bool; W]; H] = unsafe { MaybeUninit::uninit().assume_init() };
        let mut x = 0;
        let mut y = 0;

        for chr in input.chars() {
            if chr == '|' {
                if x == W {
                    x = 0;
                    y += 1;
                    continue;
                } else {
                    // Not a rectangle
                    return None;
                }
            }
            grid[y][x] = match chr {
                '.' => false,
                '#' => true,
                _ => return None,
            };
            x += 1;
        }
        Some(Grid { grid })
    }
}

#[test]
fn parse_grid() {
    assert_eq!(
        Grid::<3, 3>::parse("..#|.#.|##."),
        Some(Grid {
            grid: [
                [false, false, true],
                [false, true, false],
                [true, true, false]
            ]
        })
    );
}

impl<const W: usize, const H: usize> Grid<W, H> {
    fn at(&self, x: usize, y: usize) -> bool {
        self.grid[y][x]
    }

    fn set(&mut self, x: usize, y: usize, val: bool) -> () {
        self.grid[y][x] = val;
    }

    // TODO: Use iterator/generator
    fn moves_for(&self, direction: (usize, usize)) -> Vec<Self> {
        let mut moves = Vec::new();
        for y in 0..(H - direction.1) {
            for x in 0..(W - direction.0) {
                let next_x = x + direction.0;
                let next_y = y + direction.1;
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

    pub fn left_moves(&self) -> Vec<Self> {
        self.moves_for((0, 1))
    }

    pub fn right_moves(&self) -> Vec<Self> {
        self.moves_for((1, 0))
    }
}

#[test]
fn finds_left_moves() {
    let grid = Grid::<3, 3>::parse("..#|.#.|##.").unwrap();
    assert_eq!(
        grid.left_moves(),
        vec![
            Grid {
                grid: [
                    [true, false, true],
                    [true, true, false],
                    [true, true, false]
                ]
            },
            Grid {
                grid: [
                    [false, false, true],
                    [false, true, true],
                    [true, true, true]
                ]
            }
        ]
    );
}

#[test]
fn finds_right_moves() {
    let grid = Grid::<3, 3>::parse("..#|.#.|##.").unwrap();
    assert_eq!(
        grid.right_moves(),
        vec![Grid {
            grid: [
                [true, true, true],
                [false, true, false],
                [true, true, false]
            ]
        }]
    );
}

impl<const W: usize, const H: usize> Display for Grid<W, H> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for y in 0..H {
            for x in 0..W {
                let chr = if self.at(x, y) { '#' } else { '.' };
                write!(f, "{}", chr)?;
            }
            write!(f, "\n")?;
        }
        Ok(())
    }
}

impl<const W: usize, const H: usize> Grid<W, H> {
    /// Get the canonical form of the game
    pub fn to_game(&self) -> Game {
        let left = self.left_moves();
        let right = self.right_moves();

        if left.is_empty() && right.is_empty() {
            return Game::zero();
        }

        let mut left_options: Vec<Game> = Vec::with_capacity(left.len());
        for left_option in left {
            left_options.push(left_option.to_game());
        }

        let mut right_options: Vec<Game> = Vec::with_capacity(right.len());
        for right_option in right {
            right_options.push(right_option.to_game());
        }

        Game {
            left: left_options,
            right: right_options,
        }
        .canonical_form()
    }
}
