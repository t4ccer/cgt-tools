use anyhow::Result;
use cgt::{
    dyadic_rational_number::DyadicRationalNumber, nimber::Nimber, quicksort::Quicksort,
    short_canonical_game::GameBackend,
};
use clap::{self, Parser, Subcommand};
use itertools::Itertools;

#[derive(Parser, Debug)]
pub struct Args {}

pub fn run(args: Args) -> Result<()> {
    let b = GameBackend::new();

    for max_value in 1..=9 {
        let sorted_range = (1..=max_value).into_iter().collect::<Vec<usize>>();
        for game in sorted_range.iter().permutations(sorted_range.len()) {
            let game = Quicksort(game.into_iter().cloned().collect::<Vec<_>>());
            let game_value = game.game(&b);
            let game_str = b.print_game_to_str(&game_value);
            let expected = b.construct_nimber(
                DyadicRationalNumber::from(0),
                Nimber((max_value - 1) as u32),
            );
            if expected == game_value {
                eprintln!("[{}]: {}", &game, &game_str);
            }
        }
    }

    Ok(())
}
