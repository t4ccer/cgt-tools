//! Messages between a page and an AI player running in a web worker.
//!
//! Positions are sent as [`Ruleset::write_state`](crate::ruleset::Ruleset::write_state) writes
//! them, so the messages are the same for every game.

use serde::{Deserialize, Serialize};

/// How long the player searches before it answers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Budget {
    /// This many simulations of Monte Carlo tree search.
    Simulations(u32),
    /// As many simulations as fit into this many milliseconds.
    Millis(u32),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Request {
    /// The first move of a game played with the pie rule, from its fixed starting position.
    Opening,
    /// Whether the second player swaps sides after the first move `first`, made from the fixed
    /// starting position of a game played with the pie rule.
    Pie { first: usize, budget: Budget },
    /// A move for the player to move in `position`.
    Move { position: Vec<u8>, budget: Budget },
    /// The chances of the player to move in each of `positions`, as the network sees them at a
    /// glance, without a search.
    Evaluate { positions: Vec<Vec<u8>> },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Response {
    /// The player has loaded and takes requests.
    Ready,
    /// `value` is the search estimate in `[-1, 1]` for the player making the move.
    Move {
        action: usize,
        value: f64,
    },
    /// `value` is the search estimate in `[-1, 1]` for the second player, without swapping.
    Pie {
        swap: bool,
        value: f64,
    },
    /// Chances in `[-1, 1]` of the player to move, one for each position evaluated.
    Values(Vec<f64>),
    Error(String),
}

/// A message together with the request it belongs to, so that replies to abandoned requests can
/// be told apart.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Envelope<T> {
    pub id: u32,
    pub body: T,
}
