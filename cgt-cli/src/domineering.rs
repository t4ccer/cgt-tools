use anyhow::Result;
use clap::{self, Parser, Subcommand};

pub mod search;

#[derive(Subcommand, Debug)]
pub enum Command {
    Search(search::Args),
}

#[derive(Parser, Debug)]
pub struct Args {
    #[clap(subcommand)]
    pub command: Command,
}

pub fn run(args: Args) -> Result<()> {
    match args.command {
        Command::Search(args) => search::run(args),
    }
}
