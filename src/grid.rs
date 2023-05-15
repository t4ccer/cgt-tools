use bit_vec::BitVec;
use lazy_static::{__Deref, lazy_static};
use std::{collections::HashMap, fmt::Display, sync::RwLock};

use crate::game::Game;

// mod bool_array;
// use bool_array;

// FIXME: some bit array here
#[allow(dead_code)]
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct Grid {
    pub width: usize,
    pub height: usize,
    pub grid: BitVec,
}

impl Grid {
    pub fn empty(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            grid: BitVec::from_elem(width * height, false),
        }
    }

    pub fn from_arr(width: usize, height: usize, field: &[bool]) -> Self {
        Grid {
            width,
            height,
            grid: BitVec::from_fn(width * height, |i| field[i]),
        }
    }

    pub fn parse(width: usize, height: usize, input: &str) -> Option<Self> {
        let mut grid = Grid::empty(width, height);
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
                    return None;
                }
            }
            grid.grid.set(
                width * y + x,
                match chr {
                    '.' => false,
                    '#' => true,
                    _ => return None,
                },
            );
            x += 1;
        }
        Some(grid)
    }
}

#[test]
fn parse_grid() {
    let width = 3;
    let height = 3;
    assert_eq!(
        Grid::parse(width, height, "..#|.#.|##."),
        Some(Grid::from_arr(
            width,
            height,
            &[false, false, true, false, true, false, true, true, false]
        ))
    );
}

impl Grid {
    fn at(&self, x: usize, y: usize) -> bool {
        self.grid[self.width * y + x]
    }

    fn set(&mut self, x: usize, y: usize, val: bool) -> () {
        self.grid.set(self.width * y + x, val);
    }

    // TODO: Use iterator/generator
    fn moves_for(&self, direction: (usize, usize)) -> Vec<Self> {
        let mut moves = Vec::new();
        for y in 0..(self.height - direction.1) {
            for x in 0..(self.width - direction.0) {
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
    let width = 3;
    let height = 3;
    let grid = Grid::parse(width, height, "..#|.#.|##.").unwrap();
    assert_eq!(
        grid.left_moves(),
        vec![
            Grid::parse(width, height, "#.#|##.|##.").unwrap(),
            Grid::parse(width, height, "..#|.##|###").unwrap(),
        ]
    );
}

#[test]
fn finds_right_moves() {
    let width = 3;
    let height = 3;
    let grid = Grid::parse(width, height, "..#|.#.|##.").unwrap();
    assert_eq!(
        grid.right_moves(),
        vec![Grid::parse(width, height, "###|.#.|##.").unwrap(),]
    );
}

impl Display for Grid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for y in 0..self.height {
            for x in 0..self.width {
                let chr = if self.at(x, y) { '#' } else { '.' };
                write!(f, "{}", chr)?;
            }
            write!(f, "\n")?;
        }
        Ok(())
    }
}

// NOTE: This is still not optimal as equivalent game may have different board positions
type Cache = HashMap<Grid, Game>;
lazy_static! {
    static ref CACHE: RwLock<Cache> = RwLock::new(HashMap::new());
}

impl Grid {
    /// Get the canonical form of the game
    pub fn to_game(&self) -> Game {
        let left = self.left_moves();
        let right = self.right_moves();

        if left.is_empty() && right.is_empty() {
            return Game::zero();
        }

        {
            if let Some(game) = CACHE.read().unwrap().deref().get(self) {
                return game.clone();
            }
        }

        let mut left_options: Vec<Game> = Vec::with_capacity(left.len());
        for left_option in left {
            left_options.push(left_option.to_game());
        }

        let mut right_options: Vec<Game> = Vec::with_capacity(right.len());
        for right_option in right {
            right_options.push(right_option.to_game());
        }

        let g = Game {
            left: left_options,
            right: right_options,
        }
        .canonical_form();

        {
            let mut cache = CACHE.write().unwrap();
            cache.insert(self.clone(), g.clone());
        }

        g
    }
}

#[test]
fn finds_simple_game_form() {
    let grid = Grid::parse(3, 3, "..#|.#.|##.").unwrap();
    assert_eq!(grid.to_game(), Game::parse("{1|1}").unwrap(),);
}
