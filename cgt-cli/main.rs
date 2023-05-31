use anyhow::{anyhow, Result};
use cgt::domineering::{Position, TranspositionTable};
use clap::Parser;
use rayon::prelude::{IntoParallelIterator, ParallelIterator};
use std::io::{self, Write};
use std::sync::atomic::AtomicU64;

#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    /// Domineering grid width
    #[arg(long)]
    width: u8,

    /// Domineering grid height
    #[arg(long)]
    height: u8,

    /// Starting position id
    #[arg(long, default_value_t = 0)]
    start_id: u64,

    /// Last position id
    #[arg(long, default_value = None)]
    last_id: Option<u64>,

    /// How often to log progress
    #[arg(long, default_value_t = 1000)]
    progress_step: u64,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let grid_tiles = args.width * args.height;

    let max_last_id: u64 = 1 << grid_tiles;
    let last_id: u64 = match args.last_id {
        None => max_last_id,
        Some(last_id) => last_id,
    };

    if last_id > max_last_id {
        Err(anyhow!(
            "last-id is {}, but for this grid it cannot exceed {}",
            last_id,
            max_last_id - 1
        ))?;
    }

    let total_len: u32 = last_id.ilog10() + 1;

    let cache = TranspositionTable::new();
    let stdout = io::stdout();
    let stderr = io::stderr();

    let progress = AtomicU64::new(0);

    let print_progress = |p: u64| {
        let progress = format!("{}", p);
        let pad_len = total_len - (progress.len() as u32);
        let pad = "0".repeat(pad_len as usize);
        let to_write = format!("{}{}/{}\n", pad, progress, last_id);
        stderr.lock().write_all(to_write.as_bytes()).unwrap();
    };

    (args.start_id..last_id).into_par_iter().for_each(|i| {
        let i = last_id - i - 1;
        let grid = Position::from_number(args.width, args.height, i)
            .unwrap()
            .move_top_left();
        let decompositions = grid.decompositons();

        // We're interested only in positions without decompositions
        // (G + H)_t <= max(G_t, H_t)
        if decompositions.len() == 1 {
            let game = Position::canonical_from_from_decompositions(decompositions, &cache);
            let temp = cache.game_backend.temperature(game);
            let to_write = format!(
                "{}\n{}\n{}\n\n",
                grid,
                cache.game_backend.print_game_to_str(game),
                temp
            );
            stdout.lock().write_all(to_write.as_bytes()).unwrap();
        }

        let progress = progress.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        if progress % args.progress_step == 0 {
            print_progress(progress);
        }
    });
    print_progress(progress.load(std::sync::atomic::Ordering::SeqCst));

    Ok(())
}
