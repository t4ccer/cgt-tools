use anyhow::{Context, Result};
use cgt::short::partizan::{
    games::amazons::Amazons, partizan_game::PartizanGame,
    transposition_table::ParallelTranspositionTable,
};
use clap::{self, Parser};
use std::str::FromStr;

/// Evaluate a single Amazons position
#[derive(Debug, Clone, Parser)]
pub struct Args {
    /// Amazons position to evaluate (e.g. '.x.|o#.|..#')
    #[arg(long)]
    position: String,
}

#[allow(clippy::needless_pass_by_value)]
pub fn run(args: Args) -> Result<()> {
    let pos: Amazons = Amazons::from_str(&args.position)
        .ok()
        .context("Could not parse the position")?;
    eprintln!("Game: {}", pos);

    let tt = ParallelTranspositionTable::new();
    let cf = pos.canonical_form(&tt);
    eprintln!("Canonical Form: {}", cf);
    eprintln!("Temperature: {}", cf.temperature());

    Ok(())
}
