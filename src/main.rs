#![feature(extend_one)]
// mod game;
// mod grid;
mod canonical_short_game;
mod dyadic_rational_number;

// use game::*;
use canonical_short_game::*;
// use grid::*;

fn main() {
    let gs = GameStorage::new();
    println!("{:?}", gs.zero_id);
    println!("{:?}", gs.star_id);

    let foo: Vec<u32> = gs.get_right_options(gs.star_id).collect();
    println!("{:?}", foo);

    // ./target/release/cg  0.54s user 0.04s system 99% cpu 0.575 total
    // println!("{}", Grid::empty(5, 5).to_game());
    // println!("{}", Grid::empty(5, 4).reduce());

    // let grid = Grid::empty(3, 3);
    // println!("{}", grid.to_game());

    // for m in grid.right_moves() {
    // println!("{}\n", m.move_top_left());
    // }

    // println!("{}", Grid::to_game(&Grid::empty(5, 4)));

    // let grid = Grid::empty(4, 5);
    // println!("\"{grid}\"\n  Canonical Form: {}", grid.to_game());

    // let grid = Grid::parse(3, 3, "..#|.#.|##.").unwrap();
    // println!("{}\n", grid);
    // for g in grid.decompositons() {
    //     println!("{g}\n");
    // }

    // Takes 30s with -r
    // println!("{}", Grid::empty(5, 5).to_game());
}
