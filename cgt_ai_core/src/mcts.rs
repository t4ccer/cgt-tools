//! Batched PUCT search, one leaf per tree per network call.

use crate::ruleset::Ruleset;
use rand::{Rng, RngExt};
use rand_distr::{Distribution, Gamma};

/// Network outputs for a batch of positions.
#[derive(Debug, Clone)]
pub struct Evaluations {
    logits: Vec<f32>,
    values: Vec<f32>,
    num_actions: usize,
}

impl Evaluations {
    /// `logits` holds `num_actions` policy logits per position, `values` one value in `[-1, 1]`
    /// per position, from the point of view of the player to move.
    pub fn new(logits: Vec<f32>, values: Vec<f32>, num_actions: usize) -> Evaluations {
        assert_eq!(logits.len(), values.len() * num_actions);
        Evaluations {
            logits,
            values,
            num_actions,
        }
    }

    pub fn logits(&self, i: usize) -> &[f32] {
        &self.logits[i * self.num_actions..(i + 1) * self.num_actions]
    }

    pub fn value(&self, i: usize) -> f32 {
        self.values[i]
    }

    pub fn values(&self) -> &[f32] {
        &self.values
    }

    pub fn all_logits(&self) -> &[f32] {
        &self.logits
    }
}

pub trait Evaluator<R: Ruleset> {
    fn evaluate(&mut self, states: &[R::State]) -> Evaluations;
}

#[derive(Debug, Clone)]
pub struct SearchConfig {
    pub c_puct: f64,
    pub fpu_reduction: f64,
    pub dirichlet_total_alpha: f64,
    pub dirichlet_weight: f64,
}

impl Default for SearchConfig {
    fn default() -> SearchConfig {
        SearchConfig {
            c_puct: 1.5,
            fpu_reduction: 0.2,
            dirichlet_total_alpha: 10.0,
            dirichlet_weight: 0.25,
        }
    }
}

#[derive(Debug)]
pub struct Node<R: Ruleset> {
    state: R::State,
    actions: Vec<usize>,
    // value of a finished game for the player to move
    terminal: Option<f64>,
    priors: Vec<f64>,
    // edge statistics are from this node's mover's perspective
    edge_visits: Vec<u32>,
    edge_values: Vec<f64>,
    children: Vec<Option<Box<Node<R>>>>,
    total_visits: u32,
    value_sum: f64,
    nn_value: f64,
}

impl<R: Ruleset> Node<R> {
    pub fn new(rules: &R, state: R::State) -> Node<R> {
        let actions = rules.legal_actions(&state);
        // Finding the winner costs about as much as generating the moves, and a game is only
        // over once the player to move is stuck
        let terminal = actions.is_empty().then(|| {
            let winner = rules
                .winner(&state)
                .expect("a game is over exactly when the player to move has no move");
            if winner == rules.to_move(&state) {
                1.0
            } else {
                -1.0
            }
        });
        Node {
            state,
            actions,
            terminal,
            priors: Vec::new(),
            edge_visits: Vec::new(),
            edge_values: Vec::new(),
            children: Vec::new(),
            total_visits: 0,
            value_sum: 0.0,
            nn_value: 0.0,
        }
    }

    pub const fn state(&self) -> &R::State {
        &self.state
    }

    pub fn actions(&self) -> &[usize] {
        &self.actions
    }

    pub fn priors(&self) -> &[f64] {
        &self.priors
    }

    pub fn edge_visits(&self) -> &[u32] {
        &self.edge_visits
    }

    pub const fn total_visits(&self) -> u32 {
        self.total_visits
    }

    pub const fn is_terminal(&self) -> bool {
        self.terminal.is_some()
    }

    pub const fn is_expanded(&self) -> bool {
        !self.priors.is_empty()
    }

    /// Search estimate in `[-1, 1]` for the player to move.
    pub fn q(&self) -> f64 {
        if !self.is_expanded() {
            return self.nn_value;
        }
        (self.nn_value + self.value_sum) / f64::from(1 + self.total_visits)
    }

    pub fn expand(&mut self, logits: &[f32], value: f64) {
        let k = self.actions.len();
        let max = self
            .actions
            .iter()
            .map(|&a| logits[a])
            .fold(f32::NEG_INFINITY, f32::max);
        let mut priors: Vec<f64> = self
            .actions
            .iter()
            .map(|&a| f64::from(logits[a] - max).exp())
            .collect();
        let total: f64 = priors.iter().sum();
        if total > 0.0 && total.is_finite() {
            for p in &mut priors {
                *p /= total;
            }
        } else {
            priors.fill(1.0 / k as f64);
        }
        self.priors = priors;
        self.edge_visits = vec![0; k];
        self.edge_values = vec![0.0; k];
        self.children = (0..k).map(|_| None).collect();
        self.nn_value = value;
    }

    pub fn child(&mut self, rules: &R, idx: usize) -> &mut Node<R> {
        let (state, action) = (&self.state, self.actions[idx]);
        self.children[idx]
            .get_or_insert_with(|| Box::new(Node::new(rules, rules.apply(state, action))))
    }

    pub fn child_index(&self, action: usize) -> Option<usize> {
        self.actions.binary_search(&action).ok()
    }

    #[must_use]
    pub fn into_child(mut self, rules: &R, idx: usize) -> Node<R> {
        self.child(rules, idx);
        *self.children.swap_remove(idx).unwrap()
    }

    fn select_edge(&self, cfg: &SearchConfig, is_root: bool) -> usize {
        let fpu = self.q() - if is_root { 0.0 } else { cfg.fpu_reduction };
        let sqrt_total = f64::from(self.total_visits.max(1)).sqrt();
        let mut best = 0;
        let mut best_score = f64::NEG_INFINITY;
        for (i, (&n, &w)) in self.edge_visits.iter().zip(&self.edge_values).enumerate() {
            let n = f64::from(n);
            let q = if n > 0.0 { w / n } else { fpu };
            let u = cfg.c_puct * self.priors[i] * (sqrt_total / (1.0 + n));
            if q + u > best_score {
                best_score = q + u;
                best = i;
            }
        }
        best
    }

    pub fn add_dirichlet_noise(&mut self, cfg: &SearchConfig, rng: &mut (impl Rng + ?Sized)) {
        let k = self.actions.len();
        if k <= 1 || cfg.dirichlet_weight <= 0.0 {
            return;
        }
        let Ok(gamma) = Gamma::new(cfg.dirichlet_total_alpha / k as f64, 1.0) else {
            return;
        };
        let noise: Vec<f64> = (0..k).map(|_| gamma.sample(rng)).collect();
        let total: f64 = noise.iter().sum();
        if !(total > 0.0 && total.is_finite()) {
            return;
        }
        for (p, n) in self.priors.iter_mut().zip(noise) {
            *p = (1.0 - cfg.dirichlet_weight).mul_add(*p, cfg.dirichlet_weight * n / total);
        }
    }

    pub fn visit_policy(&self, num_actions: usize) -> Vec<f32> {
        let mut pi = vec![0.0; num_actions];
        if self.total_visits == 0 {
            for &a in &self.actions {
                pi[a] = 1.0 / self.actions.len() as f32;
            }
            return pi;
        }
        let total: u32 = self.edge_visits.iter().sum();
        for (&a, &n) in self.actions.iter().zip(&self.edge_visits) {
            pi[a] = n as f32 / total as f32;
        }
        pi
    }

    pub fn select_action(&self, temperature: f64, rng: &mut (impl Rng + ?Sized)) -> usize {
        if self.total_visits == 0 {
            if !self.is_expanded() {
                return self.actions[rng.random_range(0..self.actions.len())];
            }
            let best = (0..self.actions.len()).fold(0, |b, i| {
                if self.priors[i] > self.priors[b] {
                    i
                } else {
                    b
                }
            });
            return self.actions[best];
        }
        if temperature <= 1e-3 {
            return self.best_action();
        }
        let weights: Vec<f64> = self
            .edge_visits
            .iter()
            .map(|&n| f64::from(n).powf(1.0 / temperature))
            .collect();
        let total: f64 = weights.iter().sum();
        let mut x = rng.random::<f64>() * total;
        for (i, w) in weights.iter().enumerate() {
            if x < *w {
                return self.actions[i];
            }
            x -= w;
        }
        self.actions[weights.iter().rposition(|&w| w > 0.0).unwrap_or(0)]
    }

    /// Most visited action; ties go to the higher prior, then to the later action.
    pub fn best_action(&self) -> usize {
        let key = |i: usize| (self.edge_visits[i], self.priors[i]);
        let best = (0..self.actions.len()).fold(0, |b, i| if key(i) >= key(b) { i } else { b });
        self.actions[best]
    }
}

/// Path from the root to a position that needs a network evaluation.
#[derive(Debug)]
pub struct Leaf<S> {
    pub path: Vec<usize>,
    pub state: S,
}

/// Finished games are backed up here, so only leaves that need evaluating are returned.
pub fn select_leaf<R: Ruleset>(
    rules: &R,
    root: &mut Node<R>,
    cfg: &SearchConfig,
) -> Option<Leaf<R::State>> {
    let mut path = Vec::new();
    let mut node = &mut *root;
    while node.is_expanded() && !node.is_terminal() {
        let idx = node.select_edge(cfg, path.is_empty());
        path.push(idx);
        node = node.child(rules, idx);
    }
    if let Some(value) = node.terminal {
        backup(root, &path, value);
        return None;
    }
    Some(Leaf {
        state: node.state,
        path,
    })
}

/// `leaf_value` is from the perspective of the player to move at the leaf.
pub fn backup<R: Ruleset>(root: &mut Node<R>, path: &[usize], leaf_value: f64) {
    let mut node = root;
    for (depth, &idx) in path.iter().enumerate() {
        let value = if (path.len() - depth) % 2 == 1 {
            -leaf_value
        } else {
            leaf_value
        };
        node.edge_visits[idx] += 1;
        node.edge_values[idx] += value;
        node.value_sum += value;
        node.total_visits += 1;
        node = node.children[idx].as_deref_mut().unwrap();
    }
}

pub fn expand_leaf<R: Ruleset>(root: &mut Node<R>, path: &[usize], logits: &[f32], value: f64) {
    let mut node = &mut *root;
    for &idx in path {
        node = node.children[idx].as_deref_mut().unwrap();
    }
    node.expand(logits, value);
    backup(root, path, value);
}

pub fn evaluate_leaves<'a, R: Ruleset>(
    evaluator: &mut impl Evaluator<R>,
    requests: impl IntoIterator<Item = (&'a mut Node<R>, Leaf<R::State>)>,
) {
    let requests: Vec<(&mut Node<R>, Leaf<R::State>)> = requests.into_iter().collect();
    if requests.is_empty() {
        return;
    }
    let states: Vec<R::State> = requests.iter().map(|(_, leaf)| leaf.state).collect();
    let evals = evaluator.evaluate(&states);
    for (i, (root, leaf)) in requests.into_iter().enumerate() {
        expand_leaf(root, &leaf.path, evals.logits(i), f64::from(evals.value(i)));
    }
}

/// Searches all `roots` in lockstep, one leaf per root per network call, until each has
/// `num_simulations` visits.
pub fn run_mcts<R: Ruleset>(
    rules: &R,
    evaluator: &mut impl Evaluator<R>,
    roots: &mut [Node<R>],
    num_simulations: u32,
    cfg: &SearchConfig,
    noise_rng: Option<&mut dyn Rng>,
) {
    let mut live: Vec<&mut Node<R>> = roots.iter_mut().filter(|r| !r.is_terminal()).collect();
    let unexpanded = live.iter_mut().filter(|r| !r.is_expanded()).map(|r| {
        let state = r.state;
        (
            &mut **r,
            Leaf {
                path: Vec::new(),
                state,
            },
        )
    });
    evaluate_leaves(evaluator, unexpanded);
    if let Some(rng) = noise_rng {
        for r in &mut live {
            r.add_dirichlet_noise(cfg, rng);
        }
    }

    loop {
        let mut requests = Vec::new();
        let mut running = false;
        for r in &mut live {
            if r.total_visits >= num_simulations {
                continue;
            }
            running = true;
            if let Some(leaf) = select_leaf(rules, r, cfg) {
                requests.push((&mut **r, leaf));
            }
        }
        if !running {
            return;
        }
        evaluate_leaves(evaluator, requests);
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SearchResult {
    pub action: usize,
    /// Search estimate in `[-1, 1]` for the player to move.
    pub value: f64,
}

pub fn choose_move<R: Ruleset>(
    rules: &R,
    evaluator: &mut impl Evaluator<R>,
    state: R::State,
    num_simulations: u32,
) -> Option<SearchResult> {
    let mut roots = [Node::new(rules, state)];
    run_mcts(
        rules,
        evaluator,
        &mut roots,
        num_simulations,
        &SearchConfig::default(),
        None,
    );
    let [root] = roots;
    (!root.is_terminal()).then(|| SearchResult {
        action: root.best_action(),
        value: root.q(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        quelhas::{Board, Quelhas, State},
        ruleset::Player,
    };

    struct Uniform;

    impl Evaluator<Quelhas> for Uniform {
        fn evaluate(&mut self, states: &[State]) -> Evaluations {
            let num_actions = Quelhas.num_actions();
            Evaluations::new(
                vec![0.0; states.len() * num_actions],
                vec![0.0; states.len()],
                num_actions,
            )
        }
    }

    #[test]
    fn visits_add_up() {
        let mut roots = [
            Node::new(&Quelhas, State::initial()),
            Node::new(
                &Quelhas,
                State {
                    turn: Player::Right,
                    ..State::initial()
                },
            ),
        ];
        run_mcts(
            &Quelhas,
            &mut Uniform,
            &mut roots,
            300,
            &SearchConfig::default(),
            None,
        );
        for root in &roots {
            assert_eq!(root.total_visits(), 300);
            assert_eq!(root.edge_visits().iter().sum::<u32>(), 300);
        }
    }

    fn mover_wins(state: State) -> bool {
        let actions = Quelhas.legal_actions(&state);
        actions.is_empty()
            || actions
                .into_iter()
                .any(|a| !mover_wins(Quelhas.apply(&state, a)))
    }

    #[test]
    fn finds_winning_moves_in_small_positions() {
        for bits in 0..1u32 << 9 {
            let mut empty = Board::from_bits(0);
            for i in 0..9 {
                if bits >> i & 1 == 1 {
                    empty = Board::from_bits(empty.bits() | 1 << (i / 3 * 10 + i % 3));
                }
            }
            for turn in [Player::Left, Player::Right] {
                let state = State { empty, turn };
                if Quelhas.winner(&state).is_some() || !mover_wins(state) {
                    continue;
                }
                let result = choose_move(&Quelhas, &mut Uniform, state, 800).unwrap();
                assert!(
                    !mover_wins(Quelhas.apply(&state, result.action)),
                    "{turn:?} to move on\n{empty}"
                );
            }
        }
    }
}
