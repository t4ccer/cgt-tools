use anyhow::Result;
use clap::{self, Parser, Subcommand};

mod evaluate;

#[derive(Subcommand, Debug)]
pub enum Command {
    Evaluate(evaluate::Args),
}

#[derive(Parser, Debug)]
pub struct Args {
    #[clap(subcommand)]
    pub command: Command,
}

pub fn run(args: Args) -> Result<()> {
    match args.command {
        Command::Evaluate(args) => evaluate::run(args),
    }
}
