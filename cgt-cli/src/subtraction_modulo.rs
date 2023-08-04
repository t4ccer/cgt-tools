use anyhow::Result;
use cgt::loopy::impartial::games::subtraction_modulo::Sub;
use clap::{self, Parser};

#[derive(Parser, Debug)]
pub struct Args {
    #[arg(short)]
    a: u32,

    #[arg(short)]
    b: u32,

    #[arg(long)]
    start_n: u32,

    #[arg(long)]
    end_n: u32,
}

pub fn run(args: Args) -> Result<()> {
    for n in args.start_n..=args.end_n {
        let sub = Sub::solve_using_graph(n, args.a, args.b);
        println!("{}", sub);
    }

    Ok(())
}
