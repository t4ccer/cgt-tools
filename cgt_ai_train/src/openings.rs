use crate::{
    checkpoint,
    game::{Game, GameCommand},
};
use anyhow::Result;
use burn::tensor::backend::AutodiffBackend;
use cgt_ai_core::{
    mcts::{Evaluations, Evaluator, Node, SearchConfig, run_mcts},
    openings::{OpeningTable, first_move_classes},
    ruleset::Ruleset,
};
use cgt_ai_model::NetEvaluator;
use indicatif::{ProgressBar, ProgressStyle};
use std::{collections::BTreeMap, path::PathBuf};

#[derive(clap::Args, Debug)]
pub struct OpeningsArgs {
    #[arg(long, value_enum)]
    pub game: Game,
    #[arg(long)]
    checkpoint: PathBuf,
    #[arg(long, default_value = "openings.json")]
    out: PathBuf,
    #[arg(long, default_value_t = 2000)]
    simulations: u32,
    #[arg(long, default_value_t = 64)]
    batch: usize,
}

impl GameCommand for OpeningsArgs {
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
    let net = checkpoint::load_net::<B, R>(rules, &args.checkpoint, &device)?;
    let classes = first_move_classes(rules);
    let representatives: Vec<usize> = classes.keys().copied().collect();
    let sims = u64::from(args.simulations);
    let bar = ProgressBar::new(representatives.len() as u64 * sims).with_style(
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
    let initial = rules.initial_state();
    let mut values = BTreeMap::new();
    let mut searched = 0;
    bar.set_message(format!("openings 0/{}", representatives.len()));
    for chunk in representatives.chunks(args.batch) {
        let mut roots: Vec<Node<R>> = chunk
            .iter()
            .map(|&a| Node::new(rules, rules.apply(&initial, a)))
            .collect();
        run_mcts(
            rules,
            &mut evaluator,
            &mut roots,
            args.simulations,
            &SearchConfig::default(),
            None,
        );
        for (a, root) in chunk.iter().zip(&roots) {
            for &member in &classes[a] {
                values.insert(member, (root.q() * 1e4).round() / 1e4);
            }
        }
        searched += chunk.len();
        // root expansions and terminal leaves make the evaluation count drift
        // from the simulation count, so resynchronise once a batch is done
        bar.set_position(searched as u64 * sims);
        bar.set_message(format!("openings {searched}/{}", representatives.len()));
        if bar.is_hidden() {
            println!(
                "  searched {searched}/{} opening classes",
                representatives.len()
            );
        }
    }
    bar.finish();

    let table = OpeningTable {
        game: rules.name().to_string(),
        checkpoint: args.checkpoint.display().to_string(),
        simulations: args.simulations,
        values,
    };
    if let Some(parent) = args.out.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&args.out, serde_json::to_string_pretty(&table)?)?;
    println!("wrote {}", args.out.display());

    let value = |a: usize| table.value(a).unwrap_or(f64::NAN);
    let mut ranked = representatives;
    ranked.sort_by(|&a, &b| value(a).abs().total_cmp(&value(b).abs()));
    println!("most balanced first moves (value for the second player, who is to move):");
    for &a in ranked.iter().take(10) {
        println!(
            "  {:9} {:+.3}",
            rules.describe_action(&initial, a),
            value(a)
        );
    }
    let favoured = table.values.values().filter(|&&v| v > 0.0).count();
    println!(
        "the second player (no swap) is favoured after {favoured}/{} first moves",
        table.values.len()
    );
    Ok(())
}
