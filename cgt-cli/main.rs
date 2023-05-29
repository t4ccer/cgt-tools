use anyhow::anyhow;
use anyhow::Result;
use cgt::domineering::Grid;
use cgt::domineering::GridCache;
use cgt::short_canonical_game::GameBackend;
use clap::Parser;
use rayon::prelude::IntoParallelIterator;
use rayon::prelude::ParallelIterator;
use std::io;
use std::io::Write;
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

    let cache_file_path = "game-backend.bin";

    let game_backend = match GameBackend::load_from_file(cache_file_path) {
        Err(e) => {
            eprintln!("Could not load cache, creating fresh one: {e:?}");
            GameBackend::new()
        }
        Ok(b) => b,
    };

    let cache = GridCache::new();
    let stdout = io::stdout();
    let stderr = io::stderr();
    let progress = AtomicU64::new(0);
    (args.start_id..last_id).into_par_iter().for_each(|i| {
        let grid = Grid::from_number(args.width, args.height, i).unwrap();
        let game = grid.canonical_form(&game_backend, &cache);
        let temp = game_backend.temperature(game);
        let to_write = format!(
            "{}\n{}\n{}\n\n",
            grid,
            game_backend.dump_game_to_str(game),
            temp
        );
        {
            stdout.lock().write_all(to_write.as_bytes()).unwrap();
        }
        let progress = progress.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        if progress % args.progress_step == 0 || progress == last_id - 1 {
            let progress = format!("{}", progress);
            let pad_len = total_len - (progress.len() as u32);
            let pad = "0".repeat(pad_len as usize);
            let to_write = format!("{}{}/{}\n", pad, progress, last_id - 1);
            stderr.lock().write_all(to_write.as_bytes()).unwrap();
        }
    });

    match game_backend.save_to_file(cache_file_path) {
        Ok(()) => (),
        Err(e) => eprintln!("Could not save cache. {e:?}"),
    }

    Ok(())
}
