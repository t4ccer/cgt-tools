use anyhow::Result;
use cgt::short::partizan::games::snort::Snort;
use clap::Parser;
use std::num::NonZeroU32;

use super::common::analyze_position;

#[derive(Parser, Debug, Clone)]
/// Analyze position on caterpillar (n+1, n, n+1)
pub struct Args {
    #[arg(short)]
    /// `n` in the caterpillar (n+1, n, n+1)
    n: NonZeroU32,

    #[arg(long)]
    /// Do not generate a graphviz graph of the position and immediate children.
    no_graphviz: bool,
}

#[allow(clippy::needless_pass_by_value)]
pub fn run(args: Args) -> Result<()> {
    let position = Snort::new_three_caterpillar(args.n);

    analyze_position(position, !args.no_graphviz)?;
    Ok(())
}
