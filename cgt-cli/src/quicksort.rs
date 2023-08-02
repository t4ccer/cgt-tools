use anyhow::Result;
use cgt::{
    dyadic_rational_number::DyadicRationalNumber,
    nimber::Nimber,
    quicksort::Quicksort,
    short_canonical_game::{Game, GameBackend},
};
use clap::{self, Parser};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Debug, Clone, Copy)]
enum GameValueFilter {
    None,
    NMinusOne,
    Zero,
}

impl FromStr for GameValueFilter {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "none" => Ok(Self::None),
            "n-1" => Ok(Self::NMinusOne),
            "zero" => Ok(Self::Zero),
            unexpected => Err(format!(
                "Unexpected filter '{}'. Expected one of 'none', 'zero', 'n-1'",
                unexpected
            )),
        }
    }
}

#[derive(Serialize, Deserialize)]
struct Report {
    position: String,
    game_value: String,
}

#[derive(Parser, Debug)]
pub struct Args {
    #[arg(long, default_value_t = 1)]
    start_range: u32,

    #[arg(long, default_value_t = 6)]
    end_range: u32,

    #[arg(long, default_value = "none")]
    filter: GameValueFilter,
}

pub fn run(args: Args) -> Result<()> {
    let b = GameBackend::new();

    for max_value in args.start_range..=args.end_range {
        let sorted_range = (1..=max_value).into_iter().collect::<Vec<u32>>();

        let filter: Box<dyn Fn(Game) -> bool> = match args.filter {
            GameValueFilter::None => Box::new(|_| true),
            GameValueFilter::NMinusOne => Box::new(|actual| {
                let expected = b.construct_nimber(
                    DyadicRationalNumber::from(0),
                    Nimber((max_value - 1) as u32),
                );
                expected == actual
            }),
            GameValueFilter::Zero => Box::new(|actual| {
                let expected = b.construct_integer(0);
                expected == actual
            }),
        };

        let range_len = sorted_range.len();
        for game in sorted_range.into_iter().permutations(range_len) {
            let game = Quicksort(game);
            let game_value = game.game(&b);
            if filter(game_value) {
                let report = Report {
                    position: game.to_string(),
                    game_value: b.print_game_to_str(&game_value),
                };
                println!("{}", serde_json::ser::to_string(&report).unwrap());
            }
        }
    }

    Ok(())
}
