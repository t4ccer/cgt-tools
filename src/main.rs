mod game;
mod grid;

use game::*;
use grid::*;

fn main() {
    // ./target/release/cg  0.54s user 0.04s system 99% cpu 0.575 total
    // println!("{}", Grid::to_game(&Grid::empty(5, 4)));

    // let grid = Grid::empty(3, 3);
    // println!("{}", grid.to_game());

    println!(
        "{}",
        Game::plus(
            &Game::parse("{{{{{{|}|}|}|{{{{|}|}|}|{|}}},{{{{|}|}|}|{|}}|{{{{|}|}|}|{|}},{{{{{|}|}|}|{|}}|{|}}},{|}|{|},{{{|}|{{|}|{|{|{|}}}}},{{|}|{|{|{|}}}}|{{|}|{|{|{|}}}},{{{|}|{|{|{|}}}}|{|{|{|}}}}}}").unwrap(),
            &Game::parse("{{{{|}|}|{{{|}|}|}}|{{{|}|}|{{{|}|}|}}}").unwrap()
        )
    );

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
