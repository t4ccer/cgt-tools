use crate::commands::Args;
use anyhow::Result;
use cgt::numeric::v2f::V2f;
use clap::Parser;

pub(crate) mod clap_utils;
mod commands;
mod evaluate;
mod io;

/// Room a rendered position is fitted into, so that a hot enough position does not run to
/// an image thousands of pixels across
pub(crate) const MAX_CANVAS_SIZE: V2f = V2f { x: 800.0, y: 600.0 };

#[cfg(not(windows))]
#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

fn main() -> Result<()> {
    let args = Args::parse();
    crate::commands::run(args)
}
