use anyhow::{Context, Result, ensure};
use burn::{
    module::{AutodiffModule, Module},
    optim::{AdamW, AdamWConfig, Optimizer, adaptor::OptimizerAdaptor},
    record::{FullPrecisionSettings, NamedMpkBytesRecorder, Record, Recorder},
    tensor::backend::AutodiffBackend,
};
use cgt_ai_core::ruleset::Ruleset;
use cgt_ai_model::{GridNet, GridNetConfig, GridNetRecord};
use std::path::Path;

pub struct Trainer<B: AutodiffBackend> {
    pub net: GridNet<B>,
    pub optimizer: OptimizerAdaptor<AdamW, GridNet<B>, B>,
    pub config: GridNetConfig,
    pub iteration: usize,
}

#[derive(Record)]
struct Checkpoint<B: AutodiffBackend> {
    game: String,
    model: GridNetRecord<B>,
    optimizer: <OptimizerAdaptor<AdamW, GridNet<B>, B> as Optimizer<GridNet<B>, B>>::Record,
    iteration: usize,
    channels: usize,
    num_blocks: usize,
    head_channels: usize,
}

fn recorder() -> NamedMpkBytesRecorder<FullPrecisionSettings> {
    NamedMpkBytesRecorder::new()
}

pub fn save<B: AutodiffBackend, R: Ruleset>(
    rules: &R,
    path: &Path,
    trainer: &Trainer<B>,
) -> Result<()> {
    let checkpoint = Checkpoint::<B> {
        game: rules.name().to_string(),
        model: trainer.net.clone().into_record(),
        optimizer: trainer.optimizer.to_record(),
        iteration: trainer.iteration,
        channels: trainer.config.channels,
        num_blocks: trainer.config.num_blocks,
        head_channels: trainer.config.head_channels,
    };
    let bytes = Recorder::<B>::record(&recorder(), checkpoint, ())?;
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, bytes).with_context(|| format!("writing {}", tmp.display()))?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

pub fn load<B: AutodiffBackend, R: Ruleset>(
    rules: &R,
    path: &Path,
    device: &B::Device,
    optimizer: &AdamWConfig,
) -> Result<Trainer<B>> {
    let bytes = std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    let checkpoint: Checkpoint<B> = Recorder::<B>::load(&recorder(), bytes, device)?;
    ensure!(
        checkpoint.game == rules.name(),
        "{} is a {} checkpoint, not {}",
        path.display(),
        checkpoint.game,
        rules.name()
    );
    let config = GridNetConfig {
        head_channels: checkpoint.head_channels,
        ..GridNetConfig::for_ruleset(rules, checkpoint.channels, checkpoint.num_blocks)
    };
    Ok(Trainer {
        net: config.init::<B>(device).load_record(checkpoint.model),
        optimizer: optimizer.init().load_record(checkpoint.optimizer),
        config,
        iteration: checkpoint.iteration,
    })
}

pub fn load_net<B: AutodiffBackend, R: Ruleset>(
    rules: &R,
    path: &Path,
    device: &B::Device,
) -> Result<GridNet<B::InnerBackend>> {
    Ok(load::<B, R>(rules, path, device, &AdamWConfig::new())?
        .net
        .valid())
}
