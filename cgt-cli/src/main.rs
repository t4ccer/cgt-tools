use anyhow::Result;
use clap::{self, Parser, Subcommand};

mod anyhow_utils;
mod domineering;
mod quicksort;
mod snort;
mod wind_up;

#[derive(Subcommand, Debug)]
enum Command {
    Domineering(domineering::Args),
    Snort(snort::Args),
    Quicksort(quicksort::Args),
    WindUp(wind_up::Args),
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
        Command::Snort(args) => snort::run(args),
        Command::Quicksort(args) => quicksort::run(args),
        Command::WindUp(args) => wind_up::run(args),
    }
}
