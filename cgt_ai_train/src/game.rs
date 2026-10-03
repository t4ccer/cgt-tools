use anyhow::Result;
use burn::tensor::backend::AutodiffBackend;
use cgt_ai_core::{quelhas::Quelhas, ruleset::Ruleset};
use clap::ValueEnum;

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum Game {
    Quelhas,
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
        }
    }
}
