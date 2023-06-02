use anyhow::{bail, Context, Result};
use cgt::domineering::{Position, TranspositionTable};
use cgt::rational::Rational;
use cgt::to_from_file::ToFromFile;
use clap::Parser;
use rayon::prelude::{IntoParallelIterator, ParallelIterator};
use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::sync::atomic::AtomicU64;
use std::sync::{Arc, Mutex};
use std::{thread, time};

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

    /// How often to log progress in seconds
    #[arg(long, default_value_t = 5)]
    progress_interval: u64,

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

    /// Path to write the cache
    #[arg(long)]
    output_path: String,
}

struct ProgressTracker {
    cache: TranspositionTable,
    args: Args,
    iteration: AtomicU64,
    saved: AtomicU64,
    highest_temp: Mutex<Rational>,
    output_buffer: Mutex<BufWriter<File>>,
}

impl ProgressTracker {
    fn new(cache: TranspositionTable, args: Args, output_file: File) -> ProgressTracker {
        ProgressTracker {
            cache,
            args,
            iteration: AtomicU64::new(0),
            saved: AtomicU64::new(0),
            highest_temp: Mutex::new(Rational::NegativeInfinity),
            output_buffer: Mutex::new(BufWriter::new(output_file)),
        }
    }

    fn next_iteration(&self) {
        self.iteration
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    fn write_game(&self, game: &str) {
        self.saved
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        {
            let mut buf = self.output_buffer.lock().unwrap();
            buf.write_all(game.as_bytes()).unwrap();
        }
    }
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

    let cache = match args.cache_read_path {
        Some(ref file_path) => match TranspositionTable::load_from_file(&file_path) {
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

    let output_file =
        File::create(&args.output_path).with_context(|| "Could not open output file")?;
    let progress_tracker = Arc::new(ProgressTracker::new(cache, args, output_file));

    let progress_tracker_cpy = progress_tracker.clone();
    let progress_pid = thread::spawn(move || progress_report(progress_tracker_cpy));

    (progress_tracker.args.start_id..last_id)
        .into_par_iter()
        .for_each(|i| {
            progress_tracker.next_iteration();

            let grid =
                Position::from_number(progress_tracker.args.width, progress_tracker.args.height, i)
                    .unwrap()
                    .move_top_left();
            let decompositions = grid.decompositons();

            // We may want to skip decompositions since we have:
            // (G + H)_t <= max(G_t, H_t)
            // where G_t is the temperature of game G
            if decompositions.len() != 1 && !progress_tracker.args.include_decompositions {
                return;
            }

            let game = Position::canonical_from_from_decompositions(
                decompositions,
                &progress_tracker.cache,
            );
            let temp = progress_tracker.cache.game_backend().temperature(game);

            if let Some(temperature_threshold) = &progress_tracker.args.temperature_threshold {
                if &temp <= temperature_threshold {
                    return;
                }
            }

            let to_write = format!(
                "{}\n{}\n{}\n\n",
                grid,
                progress_tracker
                    .cache
                    .game_backend()
                    .print_game_to_str(game),
                temp
            );
            progress_tracker.write_game(&to_write);

            {
                let mut highest_temp = progress_tracker.highest_temp.lock().unwrap();
                if temp > *highest_temp {
                    *highest_temp = temp;
                }
            }
        });
    progress_pid.join().unwrap();

    if let Some(ref file_path) = progress_tracker.args.cache_write_path {
        eprintln!(
            "Saving {no_games} canonical forms to {file_path}.",
            no_games = progress_tracker.cache.game_backend().known_games_len()
        );
        progress_tracker
            .cache
            .save_to_file(file_path)
            .with_context(|| format!("Could not save cache to {file_path}"))?;
    }

    Ok(())
}

fn progress_report(progress_tracker: Arc<ProgressTracker>) {
    let grid_tiles = progress_tracker.args.width * progress_tracker.args.height;
    let max_last_id: u64 = 1 << grid_tiles;
    let last_id: u64 = match progress_tracker.args.last_id {
        None => max_last_id,
        Some(last_id) => last_id,
    };
    let total_iterations = last_id - progress_tracker.args.start_id;
    let total_len: u32 = last_id.ilog10() + 1;
    let stderr = io::stderr();

    // NOTE: We want do..while behavior so the final 100% progress is shown
    loop {
        let completed_iterations = progress_tracker
            .iteration
            .load(std::sync::atomic::Ordering::SeqCst);
        let saved = progress_tracker
            .saved
            .load(std::sync::atomic::Ordering::SeqCst);
        let completed_iterations_str = format!("{}", completed_iterations);
        let pad_len = total_len - (completed_iterations_str.len() as u32);
        let zeros_padding = "0".repeat(pad_len as usize);
        let percent_progress: f32 = completed_iterations as f32 / total_iterations as f32;
        let now = chrono::offset::Utc::now();
        let is_finished = completed_iterations == total_iterations;
        let known_games = progress_tracker.cache.game_backend().known_games_len();
        let highest_temp = if saved == 0 {
            "N/A".to_string()
        } else {
            format!("{}", progress_tracker.highest_temp.lock().unwrap().clone())
        };

        // NOTE: We may move known_games_len() to atomic counter instead so we won't take read
        // lock on games vec

        let to_write = format!(
            "[{now}]\n\
	     \tProgress: {percent_progress:.6}\n\
	     \tIterations: {zeros_padding}{completed_iterations_str}/{last_id}\n\
	     \tHighest temperature: {highest_temp}\n\
	     \tSaved games: {saved}\n\
	     \tKnown games: {known_games}\n",
        );
        stderr.lock().write_all(to_write.as_bytes()).unwrap();

        {
            let mut buf = progress_tracker.output_buffer.lock().unwrap();
            buf.flush().unwrap();
        }

        if is_finished {
            break;
        }

        thread::sleep(time::Duration::from_secs(
            progress_tracker.args.progress_interval,
        ));
    }
}
