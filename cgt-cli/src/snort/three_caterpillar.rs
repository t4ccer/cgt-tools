use anyhow::Result;
use cgt::short::partizan::games::snort::Snort;
use clap::Parser;
use std::num::NonZeroU32;

use super::common::analyze_position;

#[derive(Parser, Debug, Clone)]
pub struct Args {
    #[arg(short)]
    n: NonZeroU32,
}

pub fn run(args: Args) -> Result<()> {
    let position = Snort::new_three_caterpillar(args.n);

    analyze_position(position)?;
    Ok(())
}
