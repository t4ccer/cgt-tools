use anyhow::anyhow;
use anyhow::Result;
use cgt::domineering::Grid;
use cgt::short_canonical_game::GameBackend;
use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    /// Domineering grid width
    #[arg(long)]
    width: usize,

    /// Domineering grid height
    #[arg(long)]
    height: usize,

    /// Starting position id
    #[arg(long, default_value_t = 0)]
    start_id: usize,

    /// Last position id
    #[arg(long, default_value = None)]
    last_id: Option<usize>,

    /// How often to log progress
    #[arg(long, default_value_t = 1000)]
    progress_step: usize,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let grid_tiles = args.width * args.height;
    let max_grid_tiles = 8 * std::mem::size_of::<usize>();

    if grid_tiles > max_grid_tiles {
        Err(anyhow!(
            "Size of grid (width * height) is {}, but it cannot exceed {}",
            grid_tiles,
            max_grid_tiles
        ))?;
    }

    let max_last_id = 1 << grid_tiles;

    let last_id: usize = match args.last_id {
        None => max_last_id,
        Some(last_id) => last_id,
    };

    if last_id > max_last_id {
        Err(anyhow!(
            "last-id is {}, but for this grid it cannot exceed {}",
            last_id,
            max_last_id
        ))?;
    }

    let total_len: u32 = last_id.ilog10() + 1;

    let mut b = GameBackend::new();
    for i in args.start_id..last_id {
        let mut grid_arr = vec![false; grid_tiles];
        for grid_idx in 0..(args.width * args.height) {
            grid_arr[grid_idx] = ((i >> grid_idx) & 1) == 1;
        }
        let grid = Grid::from_arr(args.width, args.height, &grid_arr);
        let game = grid.canonical_form(&mut b);
        if i % args.progress_step == 0 || i == last_id - 1 {
            let progress = format!("{}", i);
            let pad_len = total_len - (progress.len() as u32);
            let pad = "0".repeat(pad_len as usize);
            eprintln!("{}{}/{}", pad, progress, last_id - 1);
        }
        println!("{}\n{}\n", grid, b.dump_game_to_str(game));
    }

    Ok(())
}
