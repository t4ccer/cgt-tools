#![allow(missing_docs)]

mod blocking;
mod context;
mod dead_ending;
mod p_free;
mod p_free_blocking;
mod standard;

pub use blocking::{BlockingConstructionError, BlockingContext, BlockingForm, BlockingFormContext};
pub use context::{ConstructionError, GameFormContext, ParseError};
pub use dead_ending::{
    DeadEndingConstructionError, DeadEndingContext, DeadEndingForm, DeadEndingFormContext,
};
pub use p_free::{PFreeConstructionError, PFreeContext, PFreeForm, PFreeFormContext};
pub use p_free_blocking::{
    PFreeBlockingConstructionError, PFreeBlockingContext, PFreeBlockingForm,
    PFreeBlockingFormContext,
};
pub use standard::{StandardForm, StandardFormContext};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    L,
    N,
    P,
    R,
}

impl std::fmt::Display for Outcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Outcome::L => write!(f, "L"),
            Outcome::N => write!(f, "N"),
            Outcome::P => write!(f, "P"),
            Outcome::R => write!(f, "R"),
        }
    }
}

impl PartialOrd for Outcome {
    #[allow(clippy::match_same_arms)]
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        use std::cmp::Ordering;

        match (self, other) {
            (Outcome::L, Outcome::L) => Some(Ordering::Equal),
            (Outcome::L, Outcome::N) => Some(Ordering::Greater),
            (Outcome::L, Outcome::P) => Some(Ordering::Greater),
            (Outcome::L, Outcome::R) => Some(Ordering::Greater),
            (Outcome::N, Outcome::L) => Some(Ordering::Less),
            (Outcome::N, Outcome::N) => Some(Ordering::Equal),
            (Outcome::N, Outcome::P) => None,
            (Outcome::N, Outcome::R) => Some(Ordering::Greater),
            (Outcome::P, Outcome::L) => Some(Ordering::Less),
            (Outcome::P, Outcome::N) => None,
            (Outcome::P, Outcome::P) => Some(Ordering::Equal),
            (Outcome::P, Outcome::R) => Some(Ordering::Greater),
            (Outcome::R, Outcome::L) => Some(Ordering::Less),
            (Outcome::R, Outcome::N) => Some(Ordering::Less),
            (Outcome::R, Outcome::P) => Some(Ordering::Less),
            (Outcome::R, Outcome::R) => Some(Ordering::Equal),
        }
    }
}
