//! Opening tables for games played with the pie rule.

use crate::{
    mcts::{Evaluator, Node, SearchConfig, run_mcts},
    ruleset::Ruleset,
};
use rand::{Rng, RngExt};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// For every first move, the deep-search value of the resulting position for the second
/// player, who is to move.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpeningTable {
    pub game: String,
    pub checkpoint: String,
    pub simulations: u32,
    pub values: BTreeMap<usize, f64>,
}

impl OpeningTable {
    pub fn value(&self, action: usize) -> Option<f64> {
        self.values.get(&action).copied()
    }

    pub fn choose(&self, rng: &mut (impl Rng + ?Sized), candidates: usize) -> Option<(usize, f64)> {
        let ranked = most_balanced(&self.values, candidates);
        (!ranked.is_empty()).then(|| ranked[rng.random_range(0..ranked.len())])
    }
}

/// The `count` first moves of `values` closest to even, the closest first.
pub fn most_balanced(values: &BTreeMap<usize, f64>, count: usize) -> Vec<(usize, f64)> {
    // Under the pie rule the second player takes whichever side is winning, so the first player
    // wants the opening whose value is closest to even.
    let mut ranked: Vec<(usize, f64)> = values.iter().map(|(&a, &v)| (a, v)).collect();
    ranked.sort_by(|a, b| a.1.abs().total_cmp(&b.1.abs()));
    ranked.truncate(count);
    ranked
}

/// `second_player_value` is for the player to move after the first move; swapping means
/// taking over the first player's side.
pub fn decide_swap(second_player_value: f64) -> bool {
    second_player_value < 0.0
}

/// The search value of every first move for the second player, who is to move after it, searching
/// `batch` first moves at a time and reporting after each batch how many are done. `None` for games
/// without a fixed starting position.
pub fn search_first_moves<R: Ruleset>(
    rules: &R,
    evaluator: &mut impl Evaluator<R>,
    simulations: u32,
    batch: usize,
    mut searched: impl FnMut(usize),
) -> Option<BTreeMap<usize, f64>> {
    let start = rules.fixed_start()?;
    let mut values = BTreeMap::new();
    // A first move cannot stand in for its mirror images, because the network is not symmetric
    // and gives them different values
    for chunk in rules.legal_actions(&start).chunks(batch.max(1)) {
        let mut roots: Vec<Node<R>> = chunk
            .iter()
            .map(|&a| Node::new(rules, rules.apply(&start, a)))
            .collect();
        run_mcts(
            rules,
            evaluator,
            &mut roots,
            simulations,
            &SearchConfig::default(),
            None,
        );
        values.extend(chunk.iter().zip(&roots).map(|(&a, root)| (a, root.q())));
        searched(values.len());
    }
    Some(values)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        fjords::Fjords,
        mcts::Evaluations,
        quelhas::{Quelhas, State},
    };
    use rand::{SeedableRng, rngs::SmallRng};

    /// Values each position by the first of its empty cells, so that mirror images differ
    struct Lopsided;

    impl Evaluator<Quelhas> for Lopsided {
        fn evaluate(&mut self, states: &[State]) -> Evaluations {
            let num_actions = Quelhas.num_actions();
            Evaluations::new(
                vec![0.0; states.len() * num_actions],
                states
                    .iter()
                    .map(|s| s.empty.bits().trailing_zeros() as f32 / 100.0)
                    .collect(),
                num_actions,
            )
        }
    }

    impl Evaluator<Fjords> for Lopsided {
        fn evaluate(&mut self, _: &[<Fjords as Ruleset>::State]) -> Evaluations {
            unreachable!("Fjords has no first moves to search")
        }
    }

    #[test]
    fn every_first_move_is_searched() {
        let mut done = Vec::new();
        let values = search_first_moves(&Quelhas, &mut Lopsided, 4, 100, |n| done.push(n)).unwrap();
        let first_moves = Quelhas.legal_actions(&State::initial());
        assert_eq!(values.keys().copied().collect::<Vec<_>>(), first_moves);
        assert_eq!(done, [100, 200, 300, 400, 450]);
        // a1-a2 and its mirror image j9-j10 leave different cells empty first
        let mirror = Quelhas.transform_action(first_moves[0], 3);
        assert!((values[&first_moves[0]] - values[&mirror]).abs() > 1e-6);

        assert!(search_first_moves(&Fjords, &mut Lopsided, 4, 100, |_| {}).is_none());
    }

    #[test]
    fn balanced_openings_come_first() {
        let values = BTreeMap::from([(0, 0.5), (1, -0.1), (2, 0.2), (3, -0.05)]);
        assert_eq!(most_balanced(&values, 3), [(3, -0.05), (1, -0.1), (2, 0.2)]);
        assert_eq!(most_balanced(&values, 9).len(), 4);
        let table = OpeningTable {
            game: Quelhas.name().to_owned(),
            checkpoint: String::new(),
            simulations: 1,
            values,
        };
        let mut rng = SmallRng::seed_from_u64(0);
        for _ in 0..20 {
            let (action, _) = table.choose(&mut rng, 2).unwrap();
            assert!([1, 3].contains(&action));
        }
    }
}
