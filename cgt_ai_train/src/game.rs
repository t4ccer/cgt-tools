use anyhow::Result;
use burn::tensor::backend::AutodiffBackend;
use cgt_ai_core::{fjords::Fjords, quelhas::Quelhas, ruleset::Ruleset};
use clap::ValueEnum;

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum Game {
    Quelhas,
    Fjords,
}

pub trait GameCommand {
    fn run<B: AutodiffBackend, R: Ruleset>(self, rules: R, device: B::Device) -> Result<()>;
}

impl Game {
    pub fn run<B: AutodiffBackend>(
        self,
        command: impl GameCommand,
        device: B::Device,
    ) -> Result<()> {
        match self {
            Game::Quelhas => command.run::<B, _>(Quelhas, device),
            Game::Fjords => command.run::<B, _>(Fjords, device),
        }
    }
}
