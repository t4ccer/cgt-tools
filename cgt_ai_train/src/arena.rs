use crate::{
    checkpoint,
    game::{GameCommand, game_parser},
};
use anyhow::Result;
use burn::tensor::backend::AutodiffBackend;
use cgt_ai_core::{
    games::GameId,
    mcts::{Node, SearchConfig, run_mcts},
    ruleset::{Player, Ruleset, random_position},
};
use cgt_ai_model::NetEvaluator;
use indicatif::{ProgressBar, ProgressStyle};
use rand::{Rng, RngExt, SeedableRng, rngs::SmallRng};
use std::path::Path;

#[derive(clap::Args, Debug)]
pub struct ArenaArgs {
    #[arg(long, value_parser = game_parser())]
    pub game: GameId,
    /// Checkpoint path or 'random'
    player_a: String,
    /// Checkpoint path or 'random'
    player_b: String,
    /// Each opening is played twice, sides swapped
    #[arg(short, long, default_value_t = 50)]
    num_openings: usize,
    #[arg(long, default_value_t = 2)]
    opening_plies: usize,
    #[arg(long, default_value_t = 200)]
    simulations: u32,
    /// Defaults to --simulations
    #[arg(long)]
    simulations_b: Option<u32>,
    #[arg(long, default_value_t = 0)]
    seed: u64,
}

impl GameCommand for ArenaArgs {
    fn run<B: AutodiffBackend, R: Ruleset>(self, rules: R, device: B::Device) -> Result<()> {
        run::<B, R>(&rules, &self, &device)
    }
}

enum ArenaPlayer<R: Ruleset, B: AutodiffBackend> {
    Mcts {
        name: String,
        evaluator: Box<NetEvaluator<R, B::InnerBackend>>,
        simulations: u32,
    },
    Random(SmallRng),
}

impl<R: Ruleset, B: AutodiffBackend> ArenaPlayer<R, B> {
    fn new(
        rules: &R,
        spec: &str,
        simulations: u32,
        device: &B::Device,
        seed: u64,
    ) -> Result<ArenaPlayer<R, B>> {
        if spec == "random" {
            return Ok(ArenaPlayer::Random(SmallRng::seed_from_u64(seed)));
        }
        let net = checkpoint::load_net::<B, R>(rules, Path::new(spec), device)?;
        Ok(ArenaPlayer::Mcts {
            name: format!("{spec} ({simulations} sims)"),
            evaluator: Box::new(NetEvaluator {
                rules: rules.clone(),
                net,
                device: device.clone(),
            }),
            simulations,
        })
    }

    fn name(&self) -> &str {
        match self {
            ArenaPlayer::Mcts { name, .. } => name,
            ArenaPlayer::Random(_) => "random",
        }
    }

    fn choose(&mut self, rules: &R, states: &[R::State]) -> Vec<usize> {
        match self {
            ArenaPlayer::Mcts {
                evaluator,
                simulations,
                ..
            } => {
                let mut roots: Vec<Node<R>> = states.iter().map(|&s| Node::new(rules, s)).collect();
                run_mcts(
                    rules,
                    &mut **evaluator,
                    &mut roots,
                    *simulations,
                    &SearchConfig::default(),
                    None,
                );
                roots.iter().map(Node::best_action).collect()
            }
            ArenaPlayer::Random(rng) => states
                .iter()
                .map(|s| {
                    let actions = rules.legal_actions(s);
                    actions[rng.random_range(0..actions.len())]
                })
                .collect(),
        }
    }
}

fn random_openings<R: Ruleset>(
    rules: &R,
    num: usize,
    plies: usize,
    rng: &mut impl Rng,
) -> Vec<R::State> {
    let mut openings = Vec::new();
    while openings.len() < num {
        let state = random_position(rules, plies, rng);
        if rules.winner(&state).is_none() {
            openings.push(state);
        }
    }
    openings
}

struct Progress {
    bar: ProgressBar,
    // expected remaining moves of a game before any game has finished
    prior: f64,
    finished: usize,
}

impl Progress {
    fn new(typical_game_length: usize, opening_plies: usize) -> Result<Progress> {
        let bar = ProgressBar::new(0).with_style(
            ProgressStyle::with_template(
                "{msg} [{wide_bar}] {human_pos}/~{human_len} moves, {elapsed} elapsed, ETA {eta}",
            )?
            .progress_chars("=> "),
        );
        Ok(Progress {
            bar,
            prior: typical_game_length.saturating_sub(opening_plies).max(1) as f64,
            finished: 0,
        })
    }

    // Games run in lockstep and mostly end together, so progress is counted in
    // moves; unfinished games are assumed to last as long as the finished ones.
    fn update(&mut self, finished: &[bool], moves: &[usize]) {
        let lengths: Vec<usize> = finished
            .iter()
            .zip(moves)
            .filter(|&(&done, _)| done)
            .map(|(_, &m)| m)
            .collect();
        let mean = if lengths.is_empty() {
            self.prior
        } else {
            lengths.iter().sum::<usize>() as f64 / lengths.len() as f64
        };
        let expected: f64 = finished
            .iter()
            .zip(moves)
            .map(|(&done, &m)| {
                if done {
                    m as f64
                } else {
                    mean.max(m as f64 + 1.0)
                }
            })
            .sum();
        self.bar.set_length(expected.round() as u64);
        self.bar.set_position(moves.iter().sum::<usize>() as u64);
        self.bar
            .set_message(format!("games {}/{}", lengths.len(), finished.len()));
        if self.bar.is_hidden() && lengths.len() > self.finished {
            println!("  finished {}/{} games", lengths.len(), finished.len());
        }
        self.finished = lengths.len();
    }
}

/// Plays every opening twice with sides swapped; returns whether A won each game.
fn play_games<R: Ruleset, B: AutodiffBackend>(
    rules: &R,
    a: &mut ArenaPlayer<R, B>,
    b: &mut ArenaPlayer<R, B>,
    openings: &[R::State],
    progress: &mut Progress,
) -> Vec<bool> {
    let mut states: Vec<R::State> = openings.iter().flat_map(|&s| [s, s]).collect();
    let mut moves = vec![0; states.len()];
    let a_side: Vec<Player> = openings
        .iter()
        .flat_map(|_| [Player::Left, Player::Right])
        .collect();
    let finished = |states: &[R::State]| -> Vec<bool> {
        states.iter().map(|s| rules.winner(s).is_some()).collect()
    };
    progress.update(&finished(&states), &moves);
    while finished(&states).contains(&false) {
        for (player, plays_a) in [(&mut *a, true), (&mut *b, false)] {
            let idx: Vec<usize> = (0..states.len())
                .filter(|&i| {
                    rules.winner(&states[i]).is_none()
                        && (rules.to_move(&states[i]) == a_side[i]) == plays_a
                })
                .collect();
            if idx.is_empty() {
                continue;
            }
            let chosen: Vec<R::State> = idx.iter().map(|&i| states[i]).collect();
            for (i, action) in idx.into_iter().zip(player.choose(rules, &chosen)) {
                states[i] = rules.apply(&states[i], action);
                moves[i] += 1;
            }
            progress.update(&finished(&states), &moves);
        }
    }
    progress.bar.finish();
    states
        .iter()
        .zip(&a_side)
        .map(|(s, &side)| rules.winner(s) == Some(side))
        .collect()
}

fn run<B: AutodiffBackend, R: Ruleset>(
    rules: &R,
    args: &ArenaArgs,
    device: &B::Device,
) -> Result<()> {
    let mut rng = SmallRng::seed_from_u64(args.seed);
    let mut a = ArenaPlayer::<R, B>::new(
        rules,
        &args.player_a,
        args.simulations,
        device,
        args.seed + 1,
    )?;
    let mut b = ArenaPlayer::<R, B>::new(
        rules,
        &args.player_b,
        args.simulations_b.unwrap_or(args.simulations),
        device,
        args.seed + 2,
    )?;
    let results = play_games(
        rules,
        &mut a,
        &mut b,
        &random_openings(rules, args.num_openings, args.opening_plies, &mut rng),
        &mut Progress::new(rules.typical_game_length(), args.opening_plies)?,
    );

    let n = results.len();
    let wins_a = results.iter().filter(|&&w| w).count();
    let as_left = results.iter().step_by(2).filter(|&&w| w).count();
    let as_right = results.iter().skip(1).step_by(2).filter(|&&w| w).count();
    println!(
        "A {}: {wins_a}/{n} ({:.1}%) [as Left {as_left}/{}, as Right {as_right}/{}]",
        a.name(),
        100.0 * wins_a as f64 / n as f64,
        n / 2,
        n / 2
    );
    println!(
        "B {}: {}/{n} ({:.1}%)",
        b.name(),
        n - wins_a,
        100.0 * (n - wins_a) as f64 / n as f64
    );
    Ok(())
}
