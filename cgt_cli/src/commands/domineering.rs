use anyhow::Result;
use clap::{self, Parser, Subcommand};

mod common;
mod evaluate;
mod exhaustive_search;
mod genetic_search;
mod latex_table;

#[derive(Subcommand, Debug)]
pub enum Command {
    ExhaustiveSearch(exhaustive_search::Args),
    GeneticSearch(genetic_search::Args),
    Evaluate(evaluate::Args),
    LatexTable(latex_table::Args),
}

#[derive(Parser, Debug)]
pub struct Args {
    #[clap(subcommand)]
    pub command: Command,
}

pub fn run(args: Args) -> Result<()> {
    match args.command {
        Command::ExhaustiveSearch(args) => exhaustive_search::run(args),
        Command::GeneticSearch(args) => genetic_search::run(args),
        Command::Evaluate(args) => evaluate::run(args),
        Command::LatexTable(args) => latex_table::run(args),
    }
}
