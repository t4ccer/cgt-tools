//! This game is based on [quicksort](crate::short::impartial::games::quicksort) but with one
//! crucial difference: the pivot elements are no longer integers, but are instead numbers of the
//! form 1.5, 2.5, 3.5 and so on.
//!
//! This is impartial version of the game, where both players can pick both even and odd pivots.
//! For partizan version see TODO.
//!
//! This game has been proposed in [Andreas Chen's "The Quicksort Game"](https://www.diva-portal.org/smash/get/diva2:935354/FULLTEXT01.pdf>).

use crate::{display, short::impartial::impartial_game::ImpartialGame};
use std::fmt::Display;

/// See [`pseudo_quickcheck`](self) header
#[derive(Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PseudoQuicksort {
    sequence: Vec<u32>,
}

impl Display for PseudoQuicksort {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "PseudoQuicksort")?;
        display::brackets(f, |f| display::commas(f, self.sequence()))
    }
}

impl PseudoQuicksort {
    /// Create new quicksort position from a given sequence
    #[inline]
    pub fn new(sequence: Vec<u32>) -> Self {
        Self { sequence }
    }

    /// Get the sequence of the quicksort position
    #[inline]
    pub const fn sequence(&self) -> &Vec<u32> {
        &self.sequence
    }

    /// pivot on `pivot+0.5`
    #[must_use]
    pub fn pivot_on(&self, pivot: u32) -> Self {
        let mut res = Self::new(Vec::with_capacity(self.sequence().len()));
        for elem in self.sequence() {
            if *elem <= pivot {
                res.sequence.push(*elem);
            }
        }
        for elem in self.sequence() {
            if *elem > pivot {
                res.sequence.push(*elem);
            }
        }
        res
    }
}

impl ImpartialGame for PseudoQuicksort {
    fn moves(&self) -> Vec<Self> {
        let mut res = vec![];
        for pivot in self.sequence() {
            let new = self.pivot_on(*pivot);
            if !res.contains(&new) && &new != self {
                res.push(new);
            }
        }
        res
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::numeric::nimber::Nimber;

    #[test]
    fn correct_nim_value() {
        assert_eq!(
            PseudoQuicksort::new(vec![1, 2, 3, 6, 5, 4]).nim_value(),
            Nimber::new(0)
        );

        assert_eq!(
            PseudoQuicksort::new(vec![4, 1, 6, 5, 7, 3, 8, 2]).nim_value(),
            Nimber::new(5)
        );

        assert_eq!(
            PseudoQuicksort::new(vec![4, 1, 6, 5, 7, 8, 2, 3]).nim_value(),
            Nimber::new(3)
        );
    }
}
