use anyhow::Result;
use clap::{self, Parser, Subcommand};

mod range;

#[derive(Subcommand, Debug)]
pub enum Command {
    Range(range::Args),
}

#[derive(Parser, Debug)]
pub struct Args {
    #[clap(subcommand)]
    pub command: Command,
}

pub fn run(args: Args) -> Result<()> {
    match args.command {
        Command::Range(args) => range::run(args),
    }
}
