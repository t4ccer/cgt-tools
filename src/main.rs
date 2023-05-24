#![feature(extend_one)]

use crate::canonical_game::GameBackend;
use crate::dyadic_rational_number::DyadicRationalNumber;
use crate::grid::Grid;

mod canonical_game;
mod dyadic_rational_number;
mod grid;

fn main() {
    let mut b = GameBackend::new();
    let width: usize = 4;
    let height: usize = 4;
    let total: usize = 1 << (width * height);
    let total_len: u32 = total.ilog10() + 1;

    for i in 0usize..total {
        let mut grid_arr = vec![false; width * height];
        for grid_idx in 0..(width * height) {
            grid_arr[grid_idx] = ((i >> grid_idx) & 1) == 1;
        }
        let grid = Grid::from_arr(width, height, &grid_arr);
        let game = grid.to_game(&mut b);
        if i % 1000 == 0 || i == total - 1 {
            let progress = format!("{}", i);
            let pad_len = total_len - (progress.len() as u32);
            let pad = "0".repeat(pad_len as usize);
            eprintln!("{}{}/{}", pad, progress, total - 1);
        }
        println!("{}\n{}\n", grid, b.dump_game(game));
    }
}
