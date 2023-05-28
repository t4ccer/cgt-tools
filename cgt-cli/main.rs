use anyhow::anyhow;
use anyhow::Result;
use cgt::domineering::Grid;
use cgt::domineering::GridCache;
use cgt::short_canonical_game::GameBackend;
use clap::Parser;

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
    let mut game_backend = GameBackend::new();
    let cache = GridCache::new();
    for i in args.start_id..last_id {
        let grid = Grid::from_number(args.width, args.height, i).unwrap();
        println!("{}", grid);

        let game = grid.canonical_form(&mut game_backend, &cache);
        println!("{}", game_backend.dump_game_to_str(game));

        let temp = game_backend.temperature(game);
        println!("{}\n", temp);

        if i % args.progress_step == 0 || i == last_id - 1 {
            let progress = format!("{}", i);
            let pad_len = total_len - (progress.len() as u32);
            let pad = "0".repeat(pad_len as usize);
            eprintln!("{}{}/{}", pad, progress, last_id - 1);
        }
    }

    Ok(())
}
