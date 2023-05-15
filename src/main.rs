mod game;

use game::*;

fn main() {
    println!("{}", Game::parse("{|-2}").unwrap().canonical_form());
}
