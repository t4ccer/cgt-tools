use bit_vec::BitVec;
use lazy_static::{__Deref, lazy_static};
use queues::{IsQueue, Queue};
use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};
use std::{collections::HashMap, fmt::Display, sync::RwLock};

use crate::game::Game;

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

    /// Parses a grid from '.#' notation
    ///
    /// # Arguments
    ///
    /// * `width` - Grid width
    ///
    /// * `height` - Grid height
    ///
    /// * `input` - '.#' notation with '|' as rows separator
    ///
    /// # Examples
    ///
    /// ```
    /// Grid::parse(3, 3, "..#|.#.|##.").unwrap()
    /// ```
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

    fn has_moves_for(&self, direction: (usize, usize)) -> bool {
        for y in 0..(self.height - direction.1) {
            for x in 0..(self.width - direction.0) {
                let next_x = x + direction.0;
                let next_y = y + direction.1;
                if !self.at(x, y) && !self.at(next_x, next_y) {
                    return true;
                }
            }
        }
        false
    }

    pub fn has_left_moves(&self) -> bool {
        self.has_moves_for((0, 1))
    }

    pub fn has_right_moves(&self) -> bool {
        self.has_moves_for((1, 0))
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
            if y != self.height - 1 {
                write!(f, "\n")?;
            }
        }
        Ok(())
    }
}

impl Grid {
    /// Remove filled rows and columns from the edges
    pub fn move_top_left(&self) -> Grid {
        let mut filled_top_rows = 0;
        for y in 0..self.height {
            let mut should_break = false;
            for x in 0..self.width {
                // If empty space then break
                if !self.at(x, y) {
                    should_break = true;
                    break;
                }
            }
            if should_break {
                break;
            }
            filled_top_rows += 1;
        }
        let filled_top_rows = filled_top_rows;

        let mut filled_bottom_rows = 0;
        for y in 0..self.height {
            let mut should_break = false;
            for x in 0..self.width {
                // If empty space then break
                if !self.at(x, self.height - y - 1) {
                    should_break = true;
                    break;
                }
            }
            if should_break {
                break;
            }
            filled_bottom_rows += 1;
        }
        let filled_bottom_rows = filled_bottom_rows;

        let mut filled_left_cols = 0;
        for x in 0..self.width {
            let mut should_break = false;
            for y in 0..self.height {
                // If empty space then break
                if !self.at(x, y) {
                    should_break = true;
                    break;
                }
            }
            if should_break {
                break;
            }
            filled_left_cols += 1;
        }
        let filled_left_cols = filled_left_cols;

        let mut filled_right_cols = 0;
        for x in 0..self.width {
            let mut should_break = false;
            for y in 0..self.height {
                // If empty space then break
                if !self.at(self.width - x - 1, y) {
                    should_break = true;
                    break;
                }
            }
            if should_break {
                break;
            }
            filled_right_cols += 1;
        }
        let filled_right_cols = filled_right_cols;

        let minimized_width = self.width - filled_left_cols - filled_right_cols;
        let minimized_height = self.height - filled_top_rows - filled_bottom_rows;

        let mut grid = BitVec::from_elem(minimized_width * minimized_height, false);
        for y in filled_top_rows..(self.height - filled_bottom_rows) {
            for x in filled_left_cols..(self.width - filled_right_cols) {
                grid.set(
                    minimized_width * (y - filled_top_rows) + (x - filled_left_cols),
                    self.at(x, y),
                );
            }
        }

        Grid {
            width: minimized_width,
            height: minimized_height,
            grid,
        }
    }

    fn bfs(&self, visited: &mut BitVec, x: usize, y: usize) -> Grid {
        let mut grid = BitVec::from_elem(self.width * self.height, true);
        let mut q: Queue<(usize, usize)> = Queue::new();
        q.add((x, y)).unwrap();
        while let Ok((qx, qy)) = q.remove() {
            visited.set(self.width * qy + qx, true);
            grid.set(self.width * qy + qx, false);

            let directions: [(i64, i64); 4] = [(1, 0), (-1, 0), (0, 1), (0, -1)];
            for (dx, dy) in directions {
                let lx = (qx as i64) + dx;
                let ly = (qy as i64) + dy;

                if lx >= 0
                    && lx < (self.width as i64)
                    && ly >= 0
                    && ly < (self.height as i64)
                    && !self.at(lx as usize, ly as usize)
                    && !visited[self.width * (ly as usize) + (lx as usize)]
                {
                    q.add((lx as usize, ly as usize)).unwrap();
                }
            }
        }
        Grid {
            width: self.width,
            height: self.height,
            grid,
        }
        .move_top_left()
    }

    /// Get decompisitons of given position
    pub fn decompositons(&self) -> Vec<Grid> {
        let mut visited = BitVec::from_elem(self.width * self.height, false);
        let mut ds = Vec::new();

        for y in 0..self.height {
            for x in 0..self.width {
                if !self.at(x, y) && !visited[self.width * y + x] {
                    ds.push(self.bfs(&mut visited, x, y));
                }
            }
        }

        ds
    }
}

#[test]
fn decomposes_simple_grid() {
    let grid = Grid::parse(3, 3, "..#|.#.|##.").unwrap();
    assert_eq!(
        grid.decompositons(),
        vec![
            Grid::parse(2, 2, "..|.#").unwrap(),
            Grid::parse(1, 2, ".|.").unwrap(),
        ]
    );
}

// NOTE: This is still not optimal as equivalent game may have different board positions
type Cache = HashMap<Grid, Game>;
lazy_static! {
    static ref CACHE: RwLock<Cache> = RwLock::new(HashMap::new());
}

impl Grid {
    fn decomp_to_game(grid: &Grid) -> Game {
        if !grid.has_left_moves() && !grid.has_left_moves() {
            return Game::zero();
        }

        {
            if let Some(game) = CACHE.read().unwrap().deref().get(grid) {
                return game.clone();
            }
        }

        let left_moves = grid.left_moves();
        let right_moves = grid.right_moves();

        let mut left_options: Vec<Game> = Vec::with_capacity(left_moves.len());
        for left_move in left_moves {
            left_options.push(left_move.to_game());
        }

        let mut right_options: Vec<Game> = Vec::with_capacity(right_moves.len());
        for right_move in right_moves {
            right_options.push(right_move.to_game());
        }

        Game {
            left: left_options,
            right: right_options,
        }
        .canonical_form()
    }

    /// Get the canonical form of the game
    pub fn to_game(&self) -> Game {
        if !self.has_left_moves() && !self.has_right_moves() {
            return Game::zero();
        }

        {
            if let Some(game) = CACHE.read().unwrap().deref().get(self) {
                return game.clone();
            }
        }

        self.decompositons()
            .par_iter()
            .map(|grid| {
                let g = Grid::decomp_to_game(grid);
                {
                    let mut cache = CACHE.write().unwrap();
                    cache.insert(self.clone(), g.clone());
                }
                g
            })
            .reduce(
                || Game::zero(),
                |mut acc: Game, g: Game| {
                    acc = Game::plus(&g, &acc);
                    acc
                },
            )
            .canonical_form()
    }
}

#[test]
fn finds_simple_game_form() {
    let grid = Grid::parse(3, 3, "..#|.#.|##.").unwrap();
    assert_eq!(grid.to_game(), Game::parse("{1|1}").unwrap(),);
}
