use anyhow::Result;
use clap::{self, Parser, Subcommand};

mod sum;

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Sum multiple canonical forms
    Sum(sum::Args),
}

#[derive(Parser, Debug)]
pub struct Args {
    #[clap(subcommand)]
    pub command: Command,
}

pub fn run(args: Args) -> Result<()> {
    match args.command {
        Command::Sum(args) => sum::run(args),
    }
}
