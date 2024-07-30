use anyhow::Result;
use clap::{self, Parser, Subcommand};

mod common;
pub mod genetic;
pub mod graph;
pub mod latex;
pub mod three_caterpillar;

#[derive(Subcommand, Debug)]
pub enum Command {
    Genetic(genetic::Args),
    Latex(latex::Args),
    Graph(graph::Args),
    ThreeCaterpillar(three_caterpillar::Args),
}

#[derive(Parser, Debug)]
pub struct Args {
    #[clap(subcommand)]
    pub command: Command,
}

pub fn run(args: Args) -> Result<()> {
    match args.command {
        Command::Genetic(args) => genetic::run(args),
        Command::Latex(args) => latex::run(args),
        Command::Graph(args) => graph::run(args),
        Command::ThreeCaterpillar(args) => three_caterpillar::run(args),
    }
}
