use crate::{
    checkpoint,
    game::{GameCommand, game_parser},
};
use anyhow::{Context, Result, ensure};
use burn::{
    backend::{Flex, flex::FlexDevice},
    module::AutodiffModule,
    optim::AdamWConfig,
    tensor::backend::AutodiffBackend,
};
use burn_store::{BurnpackStore, ModuleSnapshot};
use cgt_ai_core::{
    games::GameId,
    mcts::Evaluator,
    openings::OpeningTable,
    ruleset::{Ruleset, random_position},
};
use cgt_ai_model::{ModelFile, ModelHeader, NetEvaluator};
use rand::{RngExt, SeedableRng, rngs::SmallRng};
use std::path::PathBuf;

#[derive(clap::Args, Debug)]
pub struct ExportArgs {
    #[arg(long, value_parser = game_parser())]
    pub game: GameId,
    #[arg(long)]
    checkpoint: PathBuf,
    /// Opening table written by `openings`, which a player needs for its first move in a game
    /// played with the pie rule
    #[arg(long)]
    openings: Option<PathBuf>,
    /// Receives the network, its shape and the opening table as one model file
    #[arg(long)]
    out: PathBuf,
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
    let openings = match &args.openings {
        Some(path) => {
            let table: OpeningTable = serde_json::from_slice(
                &std::fs::read(path).with_context(|| format!("reading {}", path.display()))?,
            )?;
            ensure!(
                table.game == rules.name(),
                "{} is an opening table of {}, not {}",
                path.display(),
                table.game,
                rules.name()
            );
            Some(table)
        }
        None => None,
    };
    let mut store = BurnpackStore::from_bytes(None);
    net.save_into(&mut store)?;
    let file = ModelFile {
        header: ModelHeader {
            game: rules.name().to_owned(),
            network: trainer.config,
            openings,
        },
        weights: store.get_bytes()?.to_vec(),
    };
    if let Some(parent) = args.out.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&args.out, file.to_bytes())?;
    println!("exported {}", args.out.display());

    // the export is read back on the CPU backend the browser uses, to catch any
    // weights the format would not round-trip
    let exported = ModelFile::from_bytes(&std::fs::read(&args.out)?)
        .and_then(|file| file.load_net::<Flex>(&FlexDevice))
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
