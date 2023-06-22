use anyhow::Result;
use clap::{self, Parser, Subcommand};

mod anyhow_utils;
mod domineering;

#[derive(Subcommand, Debug)]
enum Command {
    Domineering(domineering::Args),
}

#[derive(Parser)]
struct Args {
    #[clap(subcommand)]
    command: Command,
}

fn main() -> Result<()> {
    let args = Args::parse();
    match args.command {
        Command::Domineering(args) => domineering::run(args),
    }
}
