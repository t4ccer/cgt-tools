use anyhow::{Context, Result};
use cgt::short::partizan::games::snort::Snort;
use clap::Parser;

use super::common::analyze_position;

#[derive(Parser, Debug, Clone)]
pub struct Args {
    #[arg(short)]
    n: u32,
}

pub fn run(args: Args) -> Result<()> {
    let position = Snort::new_three_star(args.n).context("Could not construct Snort position")?;

    analyze_position(position)?;
    Ok(())
}
