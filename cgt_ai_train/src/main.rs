use anyhow::Result;
use burn::{
    backend::{Autodiff, LibTorch, libtorch::LibTorchDevice},
    tensor::backend::AutodiffBackend,
};
use clap::{Parser, Subcommand};

mod arena;
mod checkpoint;
mod export;
mod game;
mod openings;
mod replay;
mod report;
mod self_play;
mod train;

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// AlphaZero-style self-play training
    Train(train::TrainArgs),
    /// Pit two checkpoints (or 'random') against each other
    Arena(arena::ArenaArgs),
    /// Search every first move deeply and write the opening table used with the pie rule
    Openings(openings::OpeningsArgs),
    /// Write a checkpoint's network and an opening table as one model file, which the website
    /// plays with
    Export(export::ExportArgs),
}

impl Command {
    fn run<B: AutodiffBackend>(self, device: B::Device) -> Result<()> {
        match self {
            Command::Train(args) => game::run::<B>(args, device),
            Command::Arena(args) => game::run::<B>(args, device),
            Command::Openings(args) => game::run::<B>(args, device),
            Command::Export(args) => game::run::<B>(args, device),
        }
    }
}

fn main() -> Result<()> {
    // libtorch otherwise keeps a thread per core busy next to every GPU call
    // without making them any faster
    if std::env::var_os("OMP_NUM_THREADS").is_none() {
        tch::set_num_threads(1);
    }
    let cli = Cli::parse();
    cli.command
        .run::<Autodiff<LibTorch>>(LibTorchDevice::Cuda(0))
}
