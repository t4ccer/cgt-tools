use anyhow::{Context, Result, ensure};
use burn::{
    module::{AutodiffModule, Module},
    optim::{AdamW, AdamWConfig, Optimizer, adaptor::OptimizerAdaptor},
    record::{FullPrecisionSettings, NamedMpkBytesRecorder, Record, Recorder},
    tensor::backend::AutodiffBackend,
};
use cgt_ai_core::ruleset::Ruleset;
use cgt_ai_model::{GridNetRecord, Net, NetConfig, NetRecord};
use std::path::Path;

pub struct Trainer<B: AutodiffBackend> {
    pub net: Net<B>,
    pub optimizer: OptimizerAdaptor<AdamW, Net<B>, B>,
    pub config: NetConfig,
    pub iteration: usize,
}

#[derive(Record)]
struct Checkpoint<B: AutodiffBackend> {
    game: String,
    model: NetRecord<B>,
    optimizer: <OptimizerAdaptor<AdamW, Net<B>, B> as Optimizer<Net<B>, B>>::Record,
    iteration: usize,
    channels: usize,
    num_blocks: usize,
    head_channels: usize,
}

/// A checkpoint from before networks other than the grid one, which differs only in the record
/// of the model. The optimizer keeps its state by parameter, so its record still fits.
#[derive(Record)]
struct GridCheckpoint<B: AutodiffBackend> {
    game: String,
    model: GridNetRecord<B>,
    optimizer: <OptimizerAdaptor<AdamW, Net<B>, B> as Optimizer<Net<B>, B>>::Record,
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
        channels: trainer.config.channels(),
        num_blocks: trainer.config.num_blocks(),
        head_channels: trainer.config.head_channels(),
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
    let checkpoint: Checkpoint<B> = match Recorder::<B>::load(&recorder(), bytes.clone(), device) {
        Ok(checkpoint) => checkpoint,
        Err(err) => {
            let Ok(old) = Recorder::<B>::load::<GridCheckpoint<B>>(&recorder(), bytes, device)
            else {
                return Err(err.into());
            };
            Checkpoint {
                game: old.game,
                model: NetRecord::Grid(old.model),
                optimizer: old.optimizer,
                iteration: old.iteration,
                channels: old.channels,
                num_blocks: old.num_blocks,
                head_channels: old.head_channels,
            }
        }
    };
    ensure!(
        checkpoint.game == rules.name(),
        "{} is a {} checkpoint, not {}",
        path.display(),
        checkpoint.game,
        rules.name()
    );
    let config = NetConfig::for_ruleset(rules, checkpoint.channels, checkpoint.num_blocks)
        .with_head_channels(checkpoint.head_channels);
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
) -> Result<Net<B::InnerBackend>> {
    Ok(load::<B, R>(rules, path, device, &AdamWConfig::new())?
        .net
        .valid())
}
