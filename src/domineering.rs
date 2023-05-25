use bit_vec::BitVec;
use lazy_static::{__Deref, lazy_static};
use queues::{IsQueue, Queue};
use std::{collections::HashMap, fmt::Display, sync::RwLock};

use crate::short_canonical_game::{GameBackend, GameId, Options};

// FIXME: some bit array here
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct Grid {
    pub width: usize,
    pub height: usize,
    pub grid: BitVec,
}

impl Grid {
    /// Creates empty grid with given size
    pub fn empty(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            grid: BitVec::from_elem(width * height, false),
        }
    }

    /// Creates a grid from given array of bools
    ///
    /// # Arguments
    ///
    /// * `input` - Lineralized grid of size `width * height`, empty if if value is `false`.
    ///
    /// # Examples
    ///
    /// ```
    /// cgt::domineering::Grid::from_arr(2, 3, &[true, true, false, false, false, true]);
    /// ```
    pub fn from_arr(width: usize, height: usize, field: &[bool]) -> Self {
        Grid {
            width,
            height,
            grid: BitVec::from_fn(width * height, |i| field[i]),
        }
    }

    /// Parses a grid from `.#` notation
    ///
    /// # Arguments
    ///
    /// * `input` - `.#` notation with `|` as rows separator
    ///
    /// # Examples
    ///
    /// ```
    /// cgt::domineering::Grid::parse(3, 3, "..#|.#.|##.").unwrap();
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

    fn moves_for<const DIR_X: usize, const DIR_Y: usize>(&self) -> Vec<Self> {
        let mut moves = Vec::new();

        if self.height == 0 || self.width == 0 {
            return moves;
        }

        for y in 0..(self.height - DIR_Y) {
            for x in 0..(self.width - DIR_X) {
                let next_x = x + DIR_X;
                let next_y = y + DIR_Y;
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
        self.moves_for::<0, 1>()
    }

    pub fn right_moves(&self) -> Vec<Self> {
        self.moves_for::<1, 0>()
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
                write!(f, "|")?;
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

        if filled_top_rows == self.height {
            return Grid::empty(0, 0);
        }

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

        if filled_left_cols == self.width {
            return Grid::empty(0, 0);
        }

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

    fn bfs(&self, visited: &mut BitVec, x: usize, y: usize) -> Option<Grid> {
        let mut grid = BitVec::from_elem(self.width * self.height, true);
        let mut q: Queue<(usize, usize)> = Queue::new();
        let mut size = 0;
        q.add((x, y)).unwrap();
        while let Ok((qx, qy)) = q.remove() {
            visited.set(self.width * qy + qx, true);
            grid.set(self.width * qy + qx, false);
            size += 1;
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
        if size < 2 {
            None
        } else {
            Some(
                Grid {
                    width: self.width,
                    height: self.height,
                    grid,
                }
                .move_top_left(),
            )
        }
    }

    /// Get decompisitons of given position
    pub fn decompositons(&self) -> Vec<Grid> {
        let mut visited = BitVec::from_elem(self.width * self.height, false);
        let mut ds = Vec::new();

        for y in 0..self.height {
            for x in 0..self.width {
                if !self.at(x, y) && !visited[self.width * y + x] {
                    if let Some(g) = self.bfs(&mut visited, x, y) {
                        ds.push(g);
                    }
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
type Cache = HashMap<Grid, GameId>;
lazy_static! {
    static ref CACHE: RwLock<Cache> = RwLock::new(HashMap::new());
}

impl Grid {
    fn lookup(&self) -> Option<GameId> {
        CACHE.read().unwrap().deref().get(self).copied()
    }

    fn cache(self, id: GameId) {
        let mut cache = CACHE.write().unwrap();
        cache.insert(self, id);
    }

    /// Get the canonical form of the position
    ///
    /// # Arguments
    ///
    /// `gb` - Shared cache of short combinatorial games
    ///
    /// # Examples
    ///
    /// ```
    /// let mut gb = cgt::short_canonical_game::GameBackend::new();
    /// let grid = cgt::domineering::Grid::parse(2, 2, ".#|..").unwrap();
    /// let game_id = grid.canonical_form(&mut gb);
    /// assert_eq!(gb.dump_game_to_str(game_id), "*".to_string());
    /// ```
    pub fn canonical_form(&self, gb: &mut GameBackend) -> GameId {
        let grid = self.move_top_left();
        if let Some(id) = grid.lookup() {
            return id;
        }

        let left_moves = grid.left_moves();
        let right_moves = grid.right_moves();

        // NOTE: We don't cache games without moves, not sure if it's worth it
        if left_moves.is_empty() && right_moves.is_empty() {
            return gb.zero_id;
        }

        let options = Options {
            left: left_moves.iter().map(|o| o.canonical_form(gb)).collect(),
            right: right_moves.iter().map(|o| o.canonical_form(gb)).collect(),
        };

        let canonical_form = gb.construct_from_options(options);
        self.clone().cache(canonical_form);
        canonical_form
    }
}

// Values confirmed with gcsuite

#[test]
fn finds_canonical_form_of_one() {
    let mut b = GameBackend::new();
    let grid = Grid::empty(1, 2);
    let game_id = grid.canonical_form(&mut b);
    assert_eq!(b.dump_game_to_str(game_id), "1".to_string());
}

#[test]
fn finds_canonical_form_of_minus_one() {
    let mut b = GameBackend::new();
    let grid = Grid::empty(2, 1);
    let game_id = grid.canonical_form(&mut b);
    assert_eq!(b.dump_game_to_str(game_id), "-1".to_string());
}

#[test]
fn finds_canonical_form_of_two_by_two() {
    let mut b = GameBackend::new();
    let grid = Grid::empty(2, 2);
    let game_id = grid.canonical_form(&mut b);
    assert_eq!(b.dump_game_to_str(game_id), "{1|-1}".to_string());
}

#[test]
fn finds_canonical_form_of_two_by_two_with_noise() {
    let mut b = GameBackend::new();
    let grid = Grid::parse(3, 3, "..#|..#|##.").unwrap();
    let game_id = grid.canonical_form(&mut b);
    assert_eq!(b.dump_game_to_str(game_id), "{1|-1}".to_string());
}

#[test]
fn finds_canonical_form_of_minus_two() {
    let mut b = GameBackend::new();
    let grid = Grid::empty(4, 1);
    let game_id = grid.canonical_form(&mut b);
    assert_eq!(b.dump_game_to_str(game_id), "-2".to_string());
}

#[test]
fn finds_canonical_form_of_l_shape() {
    let mut b = GameBackend::new();
    let grid = Grid::parse(2, 2, ".#|..").unwrap();
    let game_id = grid.canonical_form(&mut b);
    assert_eq!(b.dump_game_to_str(game_id), "*".to_string());
}

#[test]
fn finds_canonical_form_of_long_l_shape() {
    let mut b = GameBackend::new();
    let grid = Grid::parse(3, 3, ".##|.##|...").unwrap();
    let game_id = grid.canonical_form(&mut b);
    assert_eq!(b.dump_game_to_str(game_id), "0".to_string());
}

#[test]
fn finds_canonical_form_of_weird_l_shape() {
    let mut b = GameBackend::new();
    let grid = Grid::parse(3, 3, "..#|..#|...").unwrap();
    let game_id = grid.canonical_form(&mut b);
    assert_eq!(b.dump_game_to_str(game_id), "{1/2|-2}".to_string());
}

#[test]
fn finds_canonical_form_of_three_by_three() {
    let mut b = GameBackend::new();
    let grid = Grid::empty(3, 3);
    let game_id = grid.canonical_form(&mut b);
    assert_eq!(b.dump_game_to_str(game_id), "{1|-1}".to_string());
}

#[test]
fn finds_canonical_form_of_num_nim_sum() {
    // There was a bug in here so here's test case
    let mut b = GameBackend::new();
    let grid = Grid::parse(4, 2, ".#.#|.#..").unwrap();
    let game_id = grid.canonical_form(&mut b);
    assert_eq!(b.dump_game_to_str(game_id), "1*".to_string());
}
