use anyhow::Result;
use clap::{self, Parser, Subcommand};

mod amazons;
mod anyhow_utils;
mod canonical_form;
mod domineering;
mod quicksort;
mod snort;
mod wind_up;

#[cfg(all(not(windows)))]
#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

#[derive(Subcommand, Debug)]
enum Command {
    Domineering(domineering::Args),
    Snort(snort::Args),
    Quicksort(quicksort::Args),
    WindUp(wind_up::Args),
    CanonicalForm(canonical_form::Args),
    Amazons(amazons::Args),
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
        Command::CanonicalForm(args) => canonical_form::run(args),
        Command::Amazons(args) => amazons::run(args),
    }
}
