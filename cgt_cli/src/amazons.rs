use anyhow::{Context, Result};
use cgt::short::partizan::{
    games::amazons::Amazons, partizan_game::PartizanGame,
    transposition_table::ParallelTranspositionTable,
};
use clap::{self, Parser};
use std::str::FromStr;

#[derive(Debug, Clone, Parser)]
pub struct Args {
    #[arg(long)]
    position: String,
}

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
