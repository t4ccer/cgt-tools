use anyhow::{bail, Context, Result};
use cgt::domineering::{Position, TranspositionTable};
use cgt::rational::Rational;
use cgt::to_from_file::ToFromFile;
use clap::Parser;
use rayon::prelude::{IntoParallelIterator, ParallelIterator};
use std::io::{self, Write};
use std::sync::atomic::AtomicU64;

mod anyhow_utils;

#[derive(Parser, Debug)]
#[command(author, version, about)]
#[command(
    help_template = "{author-with-newline} {about-section}Version: {version} \n\n\
		     {usage-heading} {usage} \n\
		     {all-args} {tab}"
)]

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

    /// Last position id to check
    #[arg(long, default_value = None)]
    last_id: Option<u64>,

    /// How often to log progress
    #[arg(long, default_value_t = 1000)]
    progress_step: u64,

    /// Path to read the cache
    #[arg(long, default_value = None)]
    cache_read_path: Option<String>,

    /// Path to write the cache
    #[arg(long, default_value = None)]
    cache_write_path: Option<String>,

    /// Do not report positions with this or below this temperature
    #[arg(long, default_value = None)]
    temperature_threshold: Option<Rational>,

    /// Compute positions with decompositions
    #[arg(long, default_value_t = false)]
    include_decompositions: bool,
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
        bail!(
            "last-id is {}, but for this grid it cannot exceed {}.",
            last_id,
            max_last_id - 1
        );
    }

    let total_len: u32 = last_id.ilog10() + 1;

    let cache = match args.cache_read_path {
        Some(file_path) => match TranspositionTable::load_from_file(&file_path) {
            Err(err) => {
                anyhow_utils::warn(
                    err,
                    "Could not load cache from {file_path}. Creating new cache.",
                );
                TranspositionTable::new()
            }
            Ok(cache) => {
                eprintln!(
                    "Loaded {no_games} canonical forms from {file_path}.",
                    no_games = cache.game_backend().known_games_len()
                );
                cache
            }
        },
        None => TranspositionTable::new(),
    };

    let stdout = io::stdout();
    let stderr = io::stderr();

    let progress = AtomicU64::new(0);

    let print_progress = |p: u64| {
        let progress = format!("{}", p);
        let pad_len = total_len - (progress.len() as u32);
        let pad = "0".repeat(pad_len as usize);

        // NOTE: We may move known_games_len() to atomic counter instead so we won't take read
        // lock on games vec

        let to_write = format!(
            "{}{}/{}\tKnown games: {}\n",
            pad,
            progress,
            last_id,
            cache.game_backend().known_games_len()
        );
        stderr.lock().write_all(to_write.as_bytes()).unwrap();
    };

    (args.start_id..last_id).into_par_iter().for_each(|i| {
        let i = last_id - i - 1;

        let progress = progress.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        if progress % args.progress_step == 0 {
            print_progress(progress);
        }

        let grid = Position::from_number(args.width, args.height, i)
            .unwrap()
            .move_top_left();
        let decompositions = grid.decompositons();

        // We may want to skip decompositions since we have:
        // (G + H)_t <= max(G_t, H_t)
        // where G_t is the temperature of game G
        if decompositions.len() != 1 && !args.include_decompositions {
            return;
        }

        let game = Position::canonical_from_from_decompositions(decompositions, &cache);
        let temp = cache.game_backend().temperature(game);

        if let Some(temperature_threshold) = &args.temperature_threshold {
            if &temp <= temperature_threshold {
                return;
            }
        }

        let to_write = format!(
            "{}\n{}\n{}\n\n",
            grid,
            cache.game_backend().print_game_to_str(game),
            temp
        );
        stdout.lock().write_all(to_write.as_bytes()).unwrap();
    });
    print_progress(progress.load(std::sync::atomic::Ordering::SeqCst));

    if let Some(file_path) = args.cache_write_path {
        eprintln!(
            "Saving {no_games} canonical forms to {file_path}.",
            no_games = cache.game_backend().known_games_len()
        );
        cache
            .save_to_file(&file_path)
            .with_context(|| format!("Could not save cache to {file_path}"))?;
    }

    Ok(())
}
