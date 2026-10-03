use crate::{
    checkpoint::{self, Trainer},
    game::{Game, GameCommand},
};
use anyhow::{Context, Result, bail, ensure};
use burn::{optim::AdamWConfig, tensor::backend::AutodiffBackend};
use burn_store::{
    ModuleSnapshot, PytorchStore,
    pytorch::{PytorchReader, PytorchStoreError},
};
use cgt_ai_core::ruleset::{Input, Ruleset};
use cgt_ai_model::{GraphNetConfig, NetConfig};
use std::path::PathBuf;

#[derive(clap::Args, Debug)]
pub struct ImportArgs {
    #[arg(long, value_enum)]
    pub game: Game,
    /// Checkpoint of the `PyTorch` graph network of haaland3000, as its `train.py` saves them
    #[arg(long)]
    pytorch: PathBuf,
    /// Receives the network as a checkpoint, with a fresh optimizer, to export or train on
    #[arg(long)]
    out: PathBuf,
}

impl GameCommand for ImportArgs {
    fn run<B: AutodiffBackend, R: Ruleset>(self, rules: R, device: B::Device) -> Result<()> {
        run::<B, R>(&rules, &self, &device)
    }
}

const STATE_DICT: &str = "model_state_dict";

/// The shape of the network the checkpoint holds, read off the shapes of its weights, which
/// `PyTorch` stores as `[out, in]`
fn read_config(args: &ImportArgs, nodes: usize, features: usize) -> Result<GraphNetConfig> {
    let reader = PytorchReader::with_top_level_key(&args.pytorch, STATE_DICT)
        .with_context(|| format!("reading {}", args.pytorch.display()))?;
    let shape = |key: &str| -> Result<Vec<usize>> {
        Ok(reader
            .get(key)
            .with_context(|| format!("{} has no {key}", args.pytorch.display()))?
            .shape
            .to_vec())
    };
    let num_blocks = reader
        .keys()
        .iter()
        .filter(|key| key.starts_with("conv_layers.") && key.ends_with(".linear.weight"))
        .count();
    let [channels, inputs] = shape("conv_layers.0.linear.weight")?[..] else {
        bail!(
            "the first layer of {} is not linear",
            args.pytorch.display()
        );
    };
    ensure!(
        inputs == features,
        "the network takes {inputs} features of a vertex, but the game has {features}"
    );
    let head_channels = shape("value_head.0.weight")?[0];
    Ok(GraphNetConfig {
        nodes,
        features,
        channels,
        num_blocks,
        head_channels,
    })
}

fn run<B: AutodiffBackend, R: Ruleset>(
    rules: &R,
    args: &ImportArgs,
    device: &B::Device,
) -> Result<()> {
    let Input::Graph { nodes, features } = rules.input() else {
        bail!(
            "only the graph networks of haaland3000 can be imported, and {} needs a grid network",
            rules.name()
        );
    };
    let config = NetConfig::Graph(read_config(args, nodes, features)?);
    let mut net = config.init::<B>(device);
    let mut store = PytorchStore::from_file(&args.pytorch)
        .with_top_level_key(STATE_DICT)
        // `nn.Sequential(Linear, ReLU, Linear)` numbers its layers 0 and 2
        .with_key_remapping(r"^value_head\.0\.(.*)$", "value_fc1.$1")
        .with_key_remapping(r"^value_head\.2\.(.*)$", "value_fc2.$1")
        .skip_enum_variants(true);
    let result = net
        .load_from(&mut store)
        .map_err(|err: PytorchStoreError| anyhow::anyhow!("{err}"))?;
    ensure!(
        result.missing.is_empty() && result.errors.is_empty(),
        "{} does not fit the network: missing {:?}, errors {:?}",
        args.pytorch.display(),
        result.missing,
        result.errors
    );
    let iteration = PytorchReader::load_config(&args.pytorch, Some("iteration")).unwrap_or(0);

    let trainer = Trainer {
        net,
        optimizer: AdamWConfig::new().init(),
        config,
        iteration,
    };
    if let Some(parent) = args.out.parent() {
        std::fs::create_dir_all(parent)?;
    }
    checkpoint::save(rules, &args.out, &trainer)?;
    println!(
        "imported {:?} at iteration {iteration} into {}",
        trainer.config,
        args.out.display()
    );
    Ok(())
}
