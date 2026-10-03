//! Messages between a page and an AI player running in a web worker.
//!
//! Positions are sent as the actions that lead to them from the initial position, so the
//! messages are the same for every game.

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
    /// The first move of a game played with the pie rule.
    Opening,
    /// Whether the second player swaps sides after the first move `first`.
    Pie { first: usize, budget: Budget },
    /// A move for the player to move after the actions in `history`.
    Move { history: Vec<usize>, budget: Budget },
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
    Error(String),
}

/// A message together with the request it belongs to, so that replies to abandoned requests can
/// be told apart.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Envelope<T> {
    pub id: u32,
    pub body: T,
}
