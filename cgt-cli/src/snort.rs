use anyhow::Result;
use clap::{self, Parser, Subcommand};

pub mod genetic;

#[derive(Subcommand, Debug)]
pub enum Command {
    Genetic(genetic::Args),
}

#[derive(Parser, Debug)]
pub struct Args {
    #[clap(subcommand)]
    pub command: Command,
}

pub fn run(args: Args) -> Result<()> {
    match args.command {
        Command::Genetic(args) => genetic::run(args),
    }
}
