use anyhow::Result;
use cgt::{
    numeric::nimber::Nimber,
    short::impartial::{
        games::{pseudo_quicksort::PseudoQuicksort, quicksort::Quicksort},
        impartial_game::ImpartialGame,
    },
};
use clap::{self, Parser, ValueEnum};
use itertools::Itertools;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, ValueEnum)]
enum GameValueFilter {
    None,
    NMinusOne,
    Zero,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Report {
    position: String,
    game_value: String,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum Variant {
    Standard,
    Pseudo,
}

/// Evaluate all positions of Quicksort game in a given range.
#[derive(Debug, Clone, Parser)]
pub struct Args {
    #[arg(long, default_value_t = 1)]
    start_range: u32,

    #[arg(long, default_value_t = 6)]
    end_range: u32,

    #[arg(long, value_enum, default_value_t = GameValueFilter::None)]
    filter: GameValueFilter,

    #[arg(long, value_enum, default_value_t = Variant::Standard)]
    variant: Variant,
}

// There's no reasonable trait so here we go with a macro
macro_rules! handle_variant {
    ($variant:expr, $filter:expr) => {{
        let game_value = $variant.nim_value();

        if $filter(game_value) {
            let report = Report {
                position: $variant.to_string(),
                game_value: game_value.to_string(),
            };
            println!("{}", serde_json::ser::to_string(&report).unwrap());
        }
    }};
}

#[allow(clippy::needless_pass_by_value, clippy::unnecessary_wraps)]
pub fn run(args: Args) -> Result<()> {
    for max_value in args.start_range..=args.end_range {
        let sorted_range = (1..=max_value).collect::<Vec<u32>>();

        let filter: Box<dyn Fn(Nimber) -> bool> = match args.filter {
            GameValueFilter::None => Box::new(|_| true),
            GameValueFilter::NMinusOne => Box::new(|actual| {
                let expected = Nimber::new(max_value - 1);
                expected == actual
            }),
            GameValueFilter::Zero => Box::new(|actual| {
                let expected = Nimber::new(0);
                expected == actual
            }),
        };

        let range_len = sorted_range.len();
        for game in sorted_range.into_iter().permutations(range_len) {
            match args.variant {
                Variant::Standard => handle_variant!(Quicksort::new(game.clone()), filter),
                Variant::Pseudo => handle_variant!(PseudoQuicksort::new(game.clone()), filter),
            }
        }
    }

    Ok(())
}
