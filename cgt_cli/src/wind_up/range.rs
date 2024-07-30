use anyhow::{bail, Result};
use cgt::loopy::impartial::games::wind_up::WindUp;
use clap::{self, Parser};

/// Evaluate all positions of WindUp game in a given graph size range.
#[derive(Parser, Debug)]
pub struct Args {
    /// Comma separated list of values in the subtraction set
    #[arg(long, num_args=1.., value_delimiter=',')]
    moves: Vec<u32>,

    /// Starting graph size
    #[arg(long, default_value_t = 1)]
    start_n: u32,

    /// Final graph size
    #[arg(long, default_value_t = 20)]
    end_n: u32,
}

pub fn run(args: Args) -> Result<()> {
    if args.moves.is_empty() {
        bail!("Subtraction set cannot be empty. Use --moves a,b,... to specify it.");
    }

    for n in args.start_n..=args.end_n {
        // FIXME: Display sequence table
        // let _sub = Sub::solve_using_sequence(&[1], n, args.moves.clone());
        // println!("{}", sub);

        let sub = WindUp::new_using_graph(n, args.moves.clone());
        println!("{}", sub);
    }

    Ok(())
}
