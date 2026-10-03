use burn::{
    tensor::{backend::Backend, f16},
    train::Interrupter,
};
use cgt_ai_core::{
    mcts::{Evaluator, Leaf, Node, SearchConfig, evaluate_leaves, select_leaf},
    ruleset::{Player, Ruleset, random_position},
};
use cgt_ai_model::{Net, NetEvaluator};
use rand::{Rng, RngExt, SeedableRng, rngs::SmallRng};
use std::{
    sync::atomic::{AtomicUsize, Ordering},
    time::Duration,
};

#[derive(Clone, Debug)]
pub struct SelfPlayConfig {
    pub simulations: u32,
    pub fast_simulations: u32,
    pub full_search_prob: f64,
    pub temperature_moves: usize,
    pub random_opening_prob: f64,
    pub parallel_games: usize,
    pub workers: usize,
    pub search: SearchConfig,
}

pub struct Examples<S> {
    pub states: Vec<S>,
    pub policies: Vec<f16>,
    pub outcomes: Vec<f32>,
    num_actions: usize,
}

impl<S> Examples<S> {
    pub const fn new(num_actions: usize) -> Examples<S> {
        Examples {
            states: Vec::new(),
            policies: Vec::new(),
            outcomes: Vec::new(),
            num_actions,
        }
    }

    pub const fn len(&self) -> usize {
        self.outcomes.len()
    }

    pub fn policy(&self, i: usize) -> &[f16] {
        &self.policies[i * self.num_actions..(i + 1) * self.num_actions]
    }

    fn extend(&mut self, other: Examples<S>) {
        self.states.extend(other.states);
        self.policies.extend(other.policies);
        self.outcomes.extend(other.outcomes);
    }
}

#[derive(Default)]
pub struct GameStats {
    pub lengths: Vec<usize>,
    pub left_wins: usize,
}

impl GameStats {
    fn merge(&mut self, other: GameStats) {
        self.lengths.extend(other.lengths);
        self.left_wins += other.left_wins;
    }
}

struct Slot<R: Ruleset> {
    root: Node<R>,
    ply: usize,
    records: Vec<(R::State, Vec<f32>, Player)>,
    target: u32,
    record: bool,
}

impl<R: Ruleset> Slot<R> {
    fn new(rules: &R, cfg: &SelfPlayConfig, rng: &mut impl Rng) -> Slot<R> {
        let ply = usize::from(rng.random::<f64>() < cfg.random_opening_prob);
        let state = random_position(rules, ply, rng);
        let mut slot = Slot {
            root: Node::new(rules, state),
            ply,
            records: Vec::new(),
            target: 0,
            record: false,
        };
        slot.choose_budget(cfg, rng);
        slot
    }

    // KataGo's playout cap randomization: cheap searches play most moves
    // so games are fast, only the expensive ones become policy targets.
    fn choose_budget(&mut self, cfg: &SelfPlayConfig, rng: &mut impl Rng) {
        if cfg.fast_simulations == 0 || rng.random::<f64>() < cfg.full_search_prob {
            (self.target, self.record) = (cfg.simulations, true);
        } else {
            (self.target, self.record) = (cfg.fast_simulations, false);
        }
    }

    fn search_done(&self) -> bool {
        self.root.is_expanded()
            && (self.root.total_visits() >= self.target || self.root.actions().len() == 1)
    }
}

/// Returns `None` if `stop` was raised before all games finished.
pub fn play_games<R: Ruleset>(
    rules: &R,
    evaluator: &mut impl Evaluator<R>,
    num_games: usize,
    cfg: &SelfPlayConfig,
    rng: &mut impl Rng,
    stop: &Interrupter,
    moves: &AtomicUsize,
) -> Option<(Examples<R::State>, GameStats)> {
    let num_actions = rules.num_actions();
    let mut stats = GameStats::default();
    let mut examples = Examples::new(num_actions);
    let mut slots: Vec<Slot<R>> = Vec::new();
    let mut started = 0;
    while started < num_games || !slots.is_empty() {
        if stop.should_stop() {
            return None;
        }
        while started < num_games && slots.len() < cfg.parallel_games {
            let slot = Slot::new(rules, cfg, rng);
            moves.fetch_add(slot.ply, Ordering::Relaxed);
            slots.push(slot);
            started += 1;
        }

        let mut requests = Vec::new();
        let mut new_roots = Vec::new();
        for (i, slot) in slots.iter_mut().enumerate() {
            if !slot.root.is_expanded() {
                new_roots.push(i);
                let state = *slot.root.state();
                requests.push((
                    &mut slot.root,
                    Leaf {
                        path: Vec::new(),
                        state,
                    },
                ));
            } else if let Some(leaf) = select_leaf(rules, &mut slot.root, &cfg.search) {
                requests.push((&mut slot.root, leaf));
            }
        }
        evaluate_leaves(evaluator, requests);
        for i in new_roots {
            slots[i].root.add_dirichlet_noise(&cfg.search, rng);
        }

        let mut still_running = Vec::with_capacity(slots.len());
        for mut slot in slots {
            if !slot.search_done() {
                still_running.push(slot);
                continue;
            }

            let state = *slot.root.state();
            if slot.record {
                slot.records.push((
                    state,
                    slot.root.visit_policy(num_actions),
                    rules.to_move(&state),
                ));
            }
            let temperature = if slot.ply < cfg.temperature_moves {
                1.0
            } else {
                0.0
            };
            let action = slot.root.select_action(temperature, rng);
            let idx = slot
                .root
                .child_index(action)
                .expect("the search picks one of its own actions");
            let child = slot.root.into_child(rules, idx);
            slot.ply += 1;
            moves.fetch_add(1, Ordering::Relaxed);

            if let Some(winner) = rules.winner(child.state()) {
                for (state, pi, mover) in slot.records {
                    examples.states.push(state);
                    examples.policies.extend(pi.into_iter().map(f16::from_f32));
                    examples
                        .outcomes
                        .push(if mover == winner { 1.0 } else { -1.0 });
                }
                stats.lengths.push(slot.ply);
                stats.left_wins += usize::from(winner == Player::Left);
                continue;
            }

            slot.root = child;
            slot.choose_budget(cfg, rng);
            if slot.root.is_expanded() {
                slot.root.add_dirichlet_noise(&cfg.search, rng);
            }
            still_running.push(slot);
        }
        slots = still_running;
    }
    Some((examples, stats))
}

/// Plays `num_games` on `cfg.workers` threads, reporting the number of moves played so far.
pub fn generate<R: Ruleset, B: Backend>(
    rules: &R,
    net: &Net<B>,
    device: &B::Device,
    num_games: usize,
    cfg: &SelfPlayConfig,
    rng: &mut impl Rng,
    stop: &Interrupter,
    on_progress: &mut dyn FnMut(usize),
) -> Option<(Examples<R::State>, GameStats)> {
    let moves = AtomicUsize::new(0);
    let workers = cfg.workers.max(1);
    let shares: Vec<(usize, u64)> = (0..workers)
        .map(|i| {
            (
                num_games / workers + usize::from(i < num_games % workers),
                rng.random(),
            )
        })
        .filter(|&(share, _)| share > 0)
        .collect();
    let results: Vec<Option<(Examples<R::State>, GameStats)>> = std::thread::scope(|scope| {
        let handles: Vec<_> = shares
            .into_iter()
            .map(|(share, seed)| {
                let mut evaluator = NetEvaluator {
                    rules: rules.clone(),
                    net: net.clone(),
                    device: device.clone(),
                };
                let moves = &moves;
                scope.spawn(move || {
                    play_games(
                        rules,
                        &mut evaluator,
                        share,
                        cfg,
                        &mut SmallRng::seed_from_u64(seed),
                        stop,
                        moves,
                    )
                })
            })
            .collect();
        while !handles
            .iter()
            .all(std::thread::ScopedJoinHandle::is_finished)
        {
            on_progress(moves.load(Ordering::Relaxed));
            std::thread::sleep(Duration::from_millis(100));
        }
        handles
            .into_iter()
            .map(|h| h.join().expect("self-play worker panicked"))
            .collect()
    });

    let mut examples = Examples::new(rules.num_actions());
    let mut stats = GameStats::default();
    for result in results {
        let (e, s) = result?;
        examples.extend(e);
        stats.merge(s);
    }
    Some((examples, stats))
}
