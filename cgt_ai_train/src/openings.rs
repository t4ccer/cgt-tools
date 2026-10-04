use crate::{
    checkpoint,
    game::{GameCommand, file_name, game_parser},
};
use anyhow::{Result, bail};
use burn::tensor::backend::AutodiffBackend;
use cgt_ai_core::{
    games::GameId,
    mcts::{Evaluations, Evaluator},
    openings::{OpeningTable, most_balanced, search_first_moves},
    ruleset::Ruleset,
};
use cgt_ai_model::NetEvaluator;
use indicatif::{ProgressBar, ProgressStyle};
use std::path::PathBuf;

#[derive(clap::Args, Debug)]
pub struct OpeningsArgs {
    #[arg(long, value_parser = game_parser())]
    game: GameId,
    #[arg(long)]
    checkpoint: PathBuf,
    /// Defaults to `<game>_openings.json`
    #[arg(long)]
    out: Option<PathBuf>,
    #[arg(long, default_value_t = 2000)]
    simulations: u32,
    #[arg(long, default_value_t = 64)]
    batch: usize,
}

impl GameCommand for OpeningsArgs {
    fn game(&self) -> GameId {
        self.game
    }

    fn run<B: AutodiffBackend, R: Ruleset>(self, rules: R, device: B::Device) -> Result<()> {
        run::<B, R>(&rules, &self, device)
    }
}

struct Counted<E> {
    inner: E,
    bar: ProgressBar,
}

impl<R: Ruleset, E: Evaluator<R>> Evaluator<R> for Counted<E> {
    fn evaluate(&mut self, states: &[R::State]) -> Evaluations {
        let evals = self.inner.evaluate(states);
        let total = self.bar.length().unwrap_or(u64::MAX);
        self.bar
            .set_position((self.bar.position() + states.len() as u64).min(total));
        evals
    }
}

fn run<B: AutodiffBackend, R: Ruleset>(
    rules: &R,
    args: &OpeningsArgs,
    device: B::Device,
) -> Result<()> {
    let Some(initial) = rules.fixed_start() else {
        bail!(
            "{} starts from positions dealt at random, so it has no opening table",
            rules.name()
        );
    };
    let net = checkpoint::load_net::<B, R>(rules, &args.checkpoint, &device)?;
    let first_moves = rules.legal_actions(&initial).len();
    let sims = u64::from(args.simulations);
    let bar = ProgressBar::new(first_moves as u64 * sims).with_style(
        ProgressStyle::with_template(
            "{msg} [{wide_bar}] {human_pos}/{human_len} simulations, {elapsed} elapsed, ETA {eta}",
        )?
        .progress_chars("=> "),
    );
    let mut evaluator = Counted {
        inner: NetEvaluator {
            rules: rules.clone(),
            net,
            device,
        },
        bar: bar.clone(),
    };
    bar.set_message(format!("openings 0/{first_moves}"));
    let values = search_first_moves(
        rules,
        &mut evaluator,
        args.simulations,
        args.batch,
        |searched| {
            // root expansions and terminal leaves make the evaluation count drift
            // from the simulation count, so resynchronise once a batch is done
            bar.set_position(searched as u64 * sims);
            bar.set_message(format!("openings {searched}/{first_moves}"));
            if bar.is_hidden() {
                println!("  searched {searched}/{first_moves} first moves");
            }
        },
    )
    .expect("a game with a fixed start has first moves to search");
    bar.finish();

    let table = OpeningTable {
        game: rules.name().to_string(),
        checkpoint: args.checkpoint.display().to_string(),
        simulations: args.simulations,
        values: values
            .into_iter()
            .map(|(a, v)| (a, (v * 1e4).round() / 1e4))
            .collect(),
    };
    let out = args
        .out
        .clone()
        .unwrap_or_else(|| PathBuf::from(file_name(rules, "openings.json")));
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&out, serde_json::to_string_pretty(&table)?)?;
    println!("wrote {}", out.display());

    println!("most balanced first moves (value for the second player, who is to move):");
    for (a, value) in most_balanced(&table.values, 10) {
        println!("  {:9} {value:+.3}", rules.describe_action(&initial, a));
    }
    let favoured = table.values.values().filter(|&&v| v > 0.0).count();
    println!(
        "the second player (no swap) is favoured after {favoured}/{} first moves",
        table.values.len()
    );
    Ok(())
}
