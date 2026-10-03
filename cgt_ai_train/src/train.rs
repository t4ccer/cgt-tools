use crate::{
    checkpoint::{self, Trainer},
    game::{Game, GameCommand},
    replay::ReplayBuffer,
    report::{IterationReport, Phase, Reporter},
    self_play::{SelfPlayConfig, generate},
};
use anyhow::Result;
use burn::{
    module::{AutodiffModule, Module},
    optim::{AdamWConfig, GradientsParams, Optimizer},
    tensor::{
        Bool, ElementConversion, Tensor, TensorData,
        activation::log_softmax,
        backend::{AutodiffBackend, Backend},
    },
    train::Interrupter,
};
use cgt_ai_core::{mcts::SearchConfig, ruleset::Ruleset};
use cgt_ai_model::{NetConfig, NetInput, encode_batch};
use rand::{Rng, RngExt, SeedableRng, rngs::SmallRng};
use std::{
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

#[derive(clap::Args, Debug)]
pub struct TrainArgs {
    #[arg(long, value_enum)]
    pub game: Game,
    /// How many MORE iterations to run this invocation
    #[arg(long, default_value_t = 20)]
    iterations: usize,
    #[arg(long, default_value_t = 256)]
    games_per_iteration: usize,
    /// MCTS simulations for searches used as policy targets
    #[arg(long, default_value_t = 200)]
    simulations: u32,
    /// 0 disables playout cap randomization
    #[arg(long, default_value_t = 0)]
    fast_simulations: u32,
    #[arg(long, default_value_t = 0.25)]
    full_search_prob: f64,
    #[arg(long, default_value_t = 8)]
    temperature_moves: usize,
    #[arg(long, default_value_t = 0.25)]
    random_opening_prob: f64,
    /// Concurrent games per self-play worker
    #[arg(long, default_value_t = 64)]
    parallel_games: usize,
    /// Self-play threads
    #[arg(long, default_value_t = 4)]
    workers: usize,
    /// Width of the network: channels of a grid network, or features of each vertex in a graph
    /// network. Ignored when --resume-from is set
    #[arg(long, default_value_t = 64)]
    channels: usize,
    /// Depth of the network: residual blocks of a grid network, or layers of a graph network.
    /// Ignored when --resume-from is set
    #[arg(long, default_value_t = 6)]
    num_blocks: usize,
    #[arg(long, default_value_t = 512)]
    batch_size: usize,
    /// Expected number of times each position is trained on
    #[arg(long, default_value_t = 4.0)]
    sample_reuse: f64,
    #[arg(long, default_value_t = 300_000)]
    replay_buffer_size: usize,
    #[arg(long, default_value_t = 10_000)]
    min_buffer_size: usize,
    #[arg(long, default_value_t = 1e-3)]
    lr: f64,
    #[arg(long, default_value_t = 1e-4)]
    weight_decay: f32,
    #[arg(long, default_value = "checkpoints")]
    checkpoint_dir: PathBuf,
    #[arg(long, default_value_t = 5)]
    checkpoint_every: usize,
    #[arg(long, default_value_t = 25)]
    buffer_save_every: usize,
    #[arg(long, default_value_t = 0)]
    seed: u64,
    /// Continue from a checkpoint's weights, optimizer state, and iteration count
    #[arg(long)]
    resume_from: Option<PathBuf>,
    /// Stop early, with a checkpoint, once another iteration would not finish within this
    /// many minutes of the start
    #[arg(long)]
    time_limit: Option<f64>,
    /// Print one line per iteration instead of showing the TUI
    #[arg(long)]
    no_tui: bool,
}

impl GameCommand for TrainArgs {
    fn run<B: AutodiffBackend, R: Ruleset>(self, rules: R, device: B::Device) -> Result<()> {
        run::<B, R>(&rules, &self, &device)
    }
}

struct Batch<B: Backend> {
    x: NetInput<B>,
    illegal: Tensor<B, 2, Bool>,
    pi: Tensor<B, 2>,
    z: Tensor<B, 1>,
}

fn make_batch<R: Ruleset, B: Backend>(
    rules: &R,
    buffer: &ReplayBuffer<R>,
    idx: &[usize],
    rng: &mut impl Rng,
    device: &B::Device,
) -> Batch<B> {
    let (n, num_actions) = (idx.len(), rules.num_actions());
    let mut states = Vec::with_capacity(n);
    let mut illegal = vec![true; n * num_actions];
    let mut pi = vec![0.0f32; n * num_actions];
    let mut z = Vec::with_capacity(n);
    for (k, &i) in idx.iter().enumerate() {
        let symmetry = rng.random_range(0..rules.num_symmetries());
        let state = rules.transform_state(buffer.state(i), symmetry);
        let row = k * num_actions;
        for (a, p) in buffer.policy(i).iter().enumerate() {
            if p.to_f32() != 0.0 {
                pi[row + rules.transform_action(a, symmetry)] = p.to_f32();
            }
        }
        for a in rules.legal_actions(&state) {
            illegal[row + a] = false;
        }
        states.push(state);
        z.push(buffer.outcome(i));
    }
    Batch {
        x: encode_batch(rules, &states, device),
        illegal: Tensor::from_data(TensorData::new(illegal, [n, num_actions]), device),
        pi: Tensor::from_data(TensorData::new(pi, [n, num_actions]), device),
        z: Tensor::from_data(TensorData::new(z, [n]), device),
    }
}

fn train_on_batch<B: AutodiffBackend>(
    trainer: &mut Trainer<B>,
    batch: Batch<B>,
    lr: f64,
) -> (f32, f32) {
    let (logits, value) = trainer.net.forward(batch.x);
    // pi is exactly 0 on illegal actions, and a finite fill keeps 0 * log_prob finite there
    let log_probs = log_softmax(logits.mask_fill(batch.illegal, -1e9), 1);
    let policy_loss = (batch.pi * log_probs).sum_dim(1).mean().neg();
    let value_loss = (value - batch.z).powi_scalar(2).mean();
    let loss = policy_loss.clone() + value_loss.clone();

    let grads = GradientsParams::from_grads(loss.backward(), &trainer.net);
    trainer.net = trainer.optimizer.step(lr, trainer.net.clone(), grads);
    (
        policy_loss.into_scalar().elem(),
        value_loss.into_scalar().elem(),
    )
}

fn mean(xs: &[f32]) -> f32 {
    if xs.is_empty() {
        f32::NAN
    } else {
        xs.iter().sum::<f32>() / xs.len() as f32
    }
}

fn checkpoint_now<B: AutodiffBackend, R: Ruleset>(
    rules: &R,
    dir: &Path,
    trainer: &Trainer<B>,
    buffer: Option<&ReplayBuffer<R>>,
    name: Option<&str>,
) -> Result<PathBuf> {
    let path = dir.join(name.map_or_else(
        || format!("{}_iter{}.mpk", rules.name(), trainer.iteration),
        String::from,
    ));
    checkpoint::save(rules, &path, trainer)?;
    checkpoint::save(rules, &dir.join("latest.mpk"), trainer)?;
    if let Some(buffer) = buffer {
        buffer.save(&dir.join("replay_buffer.bin"))?;
    }
    Ok(path)
}

fn run<B: AutodiffBackend, R: Ruleset>(
    rules: &R,
    args: &TrainArgs,
    device: &B::Device,
) -> Result<()> {
    let stop = Interrupter::new();
    {
        let stop = stop.clone();
        ctrlc::set_handler(move || {
            // the first interrupt waits for a safe point to save; a second one
            // gets out of a hung GPU call
            if stop.should_stop() {
                std::process::exit(130);
            }
            stop.stop(None);
        })?;
    }

    let optimizer = AdamWConfig::new()
        .with_weight_decay(args.weight_decay)
        .with_epsilon(1e-8);
    let mut rng = SmallRng::seed_from_u64(args.seed);
    let mut trainer = if let Some(path) = &args.resume_from {
        let trainer = checkpoint::load::<B, R>(rules, path, device, &optimizer)?;
        rng = SmallRng::seed_from_u64(
            args.seed ^ (trainer.iteration as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15),
        );
        println!(
            "resumed from {} at iteration {} ({:?})",
            path.display(),
            trainer.iteration,
            trainer.config
        );
        trainer
    } else {
        let config = NetConfig::for_ruleset(rules, args.channels, args.num_blocks);
        B::seed(device, args.seed);
        Trainer {
            net: config.init(device),
            optimizer: optimizer.init(),
            config,
            iteration: 0,
        }
    };
    println!(
        "model {:?}: {} parameters",
        trainer.config,
        trainer.net.num_params()
    );

    std::fs::create_dir_all(&args.checkpoint_dir)?;
    let mut buffer = ReplayBuffer::new(rules.clone(), args.replay_buffer_size);
    let buffer_path = args.checkpoint_dir.join("replay_buffer.bin");
    if args.resume_from.is_some() && buffer_path.exists() {
        buffer.load(&buffer_path)?;
        println!(
            "loaded {} positions from {}",
            buffer.len(),
            buffer_path.display()
        );
    }

    let cfg = SelfPlayConfig {
        simulations: args.simulations,
        fast_simulations: args.fast_simulations,
        full_search_prob: args.full_search_prob,
        temperature_moves: args.temperature_moves,
        random_opening_prob: args.random_opening_prob,
        parallel_games: args.parallel_games,
        workers: args.workers,
        search: SearchConfig::default(),
    };

    let start_iteration = trainer.iteration;
    let mut reporter = Reporter::new(
        &stop,
        start_iteration,
        args.iterations,
        buffer.len(),
        rules.typical_game_length(),
        args.no_tui,
    );
    let run_start = Instant::now();
    for i in 1..=args.iterations {
        let iteration_start = Instant::now();
        let net = trainer.net.valid();
        let games = args.games_per_iteration;
        let Some((new_examples, stats)) = generate(
            rules,
            &net,
            device,
            games,
            &cfg,
            &mut rng,
            &stop,
            &mut |moves| reporter.phase(Phase::SelfPlay { moves, games }),
        ) else {
            break;
        };
        buffer.add(&new_examples);
        let self_play_time = iteration_start.elapsed().as_secs_f64();

        let t0 = Instant::now();
        let (mut policy_losses, mut value_losses) = (Vec::new(), Vec::new());
        if buffer.len() >= args.min_buffer_size {
            let steps = ((args.sample_reuse * new_examples.len() as f64 / args.batch_size as f64)
                .round() as usize)
                .max(1);
            for step in 0..steps {
                if stop.should_stop() {
                    break;
                }
                reporter.phase(Phase::Training {
                    steps: step,
                    total: steps,
                });
                let idx: Vec<usize> = (0..args.batch_size.min(buffer.len()))
                    .map(|_| rng.random_range(0..buffer.len()))
                    .collect();
                let batch = make_batch::<R, B>(rules, &buffer, &idx, &mut rng, device);
                let (pl, vl) = train_on_batch(&mut trainer, batch, args.lr);
                policy_losses.push(pl);
                value_losses.push(vl);
            }
        }
        if stop.should_stop() {
            break;
        }
        let train_time = t0.elapsed().as_secs_f64();
        trainer.iteration = start_iteration + i;

        let num_games = stats.lengths.len() as f64;
        reporter.iteration(&IterationReport {
            iteration: trainer.iteration,
            buffer: buffer.len(),
            new_examples: new_examples.len(),
            mean_length: stats.lengths.iter().sum::<usize>() as f64 / num_games,
            left_wins: stats.left_wins as f64 / num_games,
            policy_loss: mean(&policy_losses),
            value_loss: mean(&value_losses),
            steps: policy_losses.len(),
            self_play_time,
            train_time,
        });

        let out_of_time = args.time_limit.is_some_and(|minutes| {
            run_start.elapsed() + iteration_start.elapsed()
                > Duration::from_secs_f64(minutes * 60.0)
        });
        let last = i == args.iterations || out_of_time;
        if trainer.iteration % args.checkpoint_every == 0 || last {
            let save_buffer = trainer.iteration % args.buffer_save_every == 0 || last;
            let path = checkpoint_now(
                rules,
                &args.checkpoint_dir,
                &trainer,
                save_buffer.then_some(&buffer),
                None,
            )?;
            reporter.checkpoint(trainer.iteration, &path);
        }
        if out_of_time {
            break;
        }
    }
    reporter.close();

    if stop.should_stop() {
        println!(
            "\ninterrupted after iteration {}; saving checkpoint before exiting...",
            trainer.iteration
        );
        let path = checkpoint_now(
            rules,
            &args.checkpoint_dir,
            &trainer,
            Some(&buffer),
            Some("interrupted.mpk"),
        )?;
        println!("  saved checkpoint {}", path.display());
        println!(
            "  resume with --resume-from {}",
            args.checkpoint_dir.join("latest.mpk").display()
        );
    }
    Ok(())
}
