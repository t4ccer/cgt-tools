use crate::{
    checkpoint,
    game::{Game, GameCommand},
};
use anyhow::{Result, ensure};
use burn::{
    backend::{Flex, flex::FlexDevice},
    config::Config,
    module::AutodiffModule,
    optim::AdamWConfig,
    tensor::backend::AutodiffBackend,
};
use burn_store::{BurnpackStore, ModuleSnapshot};
use cgt_ai_core::{
    mcts::Evaluator,
    ruleset::{Ruleset, random_position},
};
use cgt_ai_model::NetEvaluator;
use rand::{RngExt, SeedableRng, rngs::SmallRng};
use std::path::PathBuf;

#[derive(clap::Args, Debug)]
pub struct ExportArgs {
    #[arg(long, value_enum)]
    pub game: Game,
    #[arg(long)]
    checkpoint: PathBuf,
    /// Receives `<game>.bpk` (weights) and `<game>.json` (network shape)
    #[arg(long)]
    out_dir: PathBuf,
}

impl GameCommand for ExportArgs {
    fn run<B: AutodiffBackend, R: Ruleset>(self, rules: R, device: B::Device) -> Result<()> {
        run::<B, R>(rules, &self, device)
    }
}

fn run<B: AutodiffBackend, R: Ruleset>(
    rules: R,
    args: &ExportArgs,
    device: B::Device,
) -> Result<()> {
    let trainer = checkpoint::load::<B, R>(&rules, &args.checkpoint, &device, &AdamWConfig::new())?;
    let net = trainer.net.valid();
    std::fs::create_dir_all(&args.out_dir)?;
    let weights = args.out_dir.join(format!("{}.bpk", rules.name()));
    let mut store = BurnpackStore::from_file(&weights).overwrite(true);
    net.save_into(&mut store)?;
    trainer
        .config
        .save(args.out_dir.join(format!("{}.json", rules.name())))?;
    println!("exported {}", weights.display());

    // the export is read back on the CPU backend the browser uses, to catch any
    // weights the format would not round-trip
    let exported = trainer
        .config
        .init::<Flex>(&FlexDevice)
        .load_bytes(std::fs::read(&weights)?)
        .map_err(anyhow::Error::msg)?;
    let mut rng = SmallRng::seed_from_u64(0);
    let states: Vec<R::State> = (0..64)
        .map(|_| {
            let plies = rng.random_range(0..2 * rules.typical_game_length() / 3);
            random_position(&rules, plies, &mut rng)
        })
        .collect();
    let ours = NetEvaluator {
        rules: rules.clone(),
        net,
        device,
    }
    .evaluate(&states);
    let theirs = NetEvaluator {
        rules,
        net: exported,
        device: FlexDevice,
    }
    .evaluate(&states);
    let diff = |a: &[f32], b: &[f32]| {
        a.iter()
            .zip(b)
            .map(|(x, y)| (x - y).abs())
            .fold(0.0f32, f32::max)
    };
    let (policy_diff, value_diff) = (
        diff(ours.all_logits(), theirs.all_logits()),
        diff(ours.values(), theirs.values()),
    );
    println!(
        "max |checkpoint - export| policy logit diff: {policy_diff:.2e}, value diff: {value_diff:.2e}"
    );
    ensure!(
        policy_diff <= 1e-3 && value_diff <= 1e-4,
        "exported weights do not reproduce the checkpoint's outputs"
    );
    Ok(())
}
