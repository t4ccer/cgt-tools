//! Opening tables for games played with the pie rule.

use crate::ruleset::Ruleset;
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

    // Under the pie rule the second player takes whichever side is winning, so
    // the first player wants the opening whose value is closest to even.
    pub fn choose(&self, rng: &mut (impl Rng + ?Sized), candidates: usize) -> Option<(usize, f64)> {
        let mut ranked: Vec<(usize, f64)> = self.values.iter().map(|(&a, &v)| (a, v)).collect();
        ranked.sort_by(|a, b| a.1.abs().total_cmp(&b.1.abs()));
        ranked.truncate(candidates);
        (!ranked.is_empty()).then(|| ranked[rng.random_range(0..ranked.len())])
    }
}

/// `second_player_value` is for the player to move after the first move; swapping means
/// taking over the first player's side.
pub fn decide_swap(second_player_value: f64) -> bool {
    second_player_value < 0.0
}

/// First moves grouped into orbits of the symmetries, keyed by the smallest member.
///
/// Assumes every symmetry maps the initial position to itself.
pub fn first_move_classes<R: Ruleset>(rules: &R) -> BTreeMap<usize, Vec<usize>> {
    let mut classes = BTreeMap::new();
    for action in rules.legal_actions(&rules.initial_state()) {
        let mut orbit: Vec<usize> = (0..rules.num_symmetries())
            .map(|symmetry| rules.transform_action(action, symmetry))
            .collect();
        orbit.sort_unstable();
        orbit.dedup();
        classes.entry(orbit[0]).or_insert(orbit);
    }
    classes
}
