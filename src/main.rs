mod game;
mod grid;

use grid::*;

fn main() {
    // let grid = Grid::<3,3>::parse("..#|.#.|##.").unwrap();
    let grid = Grid::<4, 4>::empty();
    println!("{}", grid);

    // for left_move in grid.left_moves() {
    // 	println!("{left_move}");
    // }

    // for right_move in grid.right_moves() {
    // 	println!("{right_move}");
    // }

    // Takes 3s with -r
    let game = grid.to_game();
    println!("{}", game);
}
