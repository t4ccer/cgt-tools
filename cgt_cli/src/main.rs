use crate::commands::*;
use anyhow::Result;
use clap::Parser;

pub(crate) mod clap_utils;
mod commands;
mod io;

#[cfg(not(windows))]
#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

fn main() -> Result<()> {
    let args = Args::parse();
    crate::commands::run(args)
}
