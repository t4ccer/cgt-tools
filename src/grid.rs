use std::{collections::HashMap, fmt::Display, mem::MaybeUninit};

use crate::game::Game;

// mod bool_array;
// use bool_array;

// FIXME: some bit array here
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Grid {
    pub grid: Vec<Vec<bool>>,
}

impl Grid {
    pub fn empty(width: usize, height: usize) -> Self {
        Self {
            grid: vec![vec![false; width]; height],
        }
    }

    pub fn parse(input: &str) -> Option<Self> {
        // It's safe here, we don't access the array and initialize every field
        let mut grid = Vec::new();
        let mut row = Vec::new();

        for chr in input.chars() {
            if chr == '|' {
                grid.push(row);
                row = Vec::new();
                continue;
            }
            row.push(match chr {
                '.' => false,
                '#' => true,
                _ => return None,
            });
        }
        grid.push(row);
        Some(Grid { grid })
    }
}

#[test]
fn parse_grid() {
    assert_eq!(
        Grid::parse("..#|.#.|##."),
        Some(Grid {
            grid: vec![
                vec![false, false, true],
                vec![false, true, false],
                vec![true, true, false]
            ]
        })
    );
}

impl Grid {
    fn at(&self, x: usize, y: usize) -> bool {
        self.grid[y][x]
    }

    fn set(&mut self, x: usize, y: usize, val: bool) -> () {
        self.grid[y][x] = val;
    }

    // TODO: Use iterator/generator
    fn moves_for(&self, direction: (usize, usize)) -> Vec<Self> {
        let mut moves = Vec::new();
        let height = match self.grid.len() {
            0 => return moves, // If no height then no moves
            l => l,
        };
        let width = match self.grid[0].len() {
            0 => return moves,
            l => l,
        };

        for y in 0..(height - direction.1) {
            for x in 0..(width - direction.0) {
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
    let grid = Grid::parse("..#|.#.|##.").unwrap();
    assert_eq!(
        grid.left_moves(),
        vec![
            Grid {
                grid: vec![
                    vec![true, false, true],
                    vec![true, true, false],
                    vec![true, true, false]
                ]
            },
            Grid {
                grid: vec![
                    vec![false, false, true],
                    vec![false, true, true],
                    vec![true, true, true]
                ]
            }
        ]
    );
}

#[test]
fn finds_right_moves() {
    let grid = Grid::parse("..#|.#.|##.").unwrap();
    assert_eq!(
        grid.right_moves(),
        vec![Grid {
            grid: vec![
                vec![true, true, true],
                vec![false, true, false],
                vec![true, true, false]
            ]
        }]
    );
}

impl Display for Grid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let height = match self.grid.len() {
            0 => return Ok(()), // If no height then no moves
            l => l,
        };
        let width = match self.grid[0].len() {
            0 => return Ok(()),
            l => l,
        };

        for y in 0..height {
            for x in 0..width {
                let chr = if self.at(x, y) { '#' } else { '.' };
                write!(f, "{}", chr)?;
            }
            write!(f, "\n")?;
        }
        Ok(())
    }
}

// // type Cache = HashMap<Grid, Game>;

impl Grid {
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

#[test]
fn finds_simple_game_form() {
    let grid = Grid::parse("..#|.#.|##.").unwrap();
    assert_eq!(grid.to_game(), Game::parse("{1|1}").unwrap(),);
}
