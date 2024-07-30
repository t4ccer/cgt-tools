//! This game is played on a sequence of numbers, and players move by pivoting around
//! a number in the manner of the quicksort algorithm.
//!
//! This is impartial version of the game, where both players can pick both even and odd pivots.
//! For partizan version see TODO.
//!
//! This game has been proposed in [Andreas Chen's "The Quicksort Game"](https://www.diva-portal.org/smash/get/diva2:935354/FULLTEXT01.pdf>).

use std::fmt::Display;

use crate::{display, short::impartial::impartial_game::ImpartialGame};

/// See [quickcheck](self) header
#[derive(Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Quicksort {
    sequence: Vec<u32>,
}

impl Display for Quicksort {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Quicksort")?;
        display::brackets(f, |f| display::commas(f, self.sequence()))
    }
}

impl Quicksort {
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

    /// pivot on `pivot`
    #[must_use]
    pub fn pivot_on(&self, pivot: u32) -> Self {
        let mut res = Self::new(Vec::with_capacity(self.sequence().len()));
        for elem in self.sequence() {
            if *elem < pivot {
                res.sequence.push(*elem);
            }
        }

        if self.sequence().contains(&pivot) {
            res.sequence.push(pivot);
        }

        for elem in self.sequence() {
            if *elem > pivot {
                res.sequence.push(*elem);
            }
        }
        res
    }
}

impl ImpartialGame for Quicksort {
    fn moves(&self) -> Vec<Self> {
        let mut moves = Vec::with_capacity(self.sequence().len());
        for pivot in self.sequence() {
            let new = self.pivot_on(*pivot);
            if &new != self {
                moves.push(new);
            }
        }
        moves
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::numeric::nimber::Nimber;

    #[test]
    fn correct_nim_value() {
        assert_eq!(
            Quicksort::new(vec![1, 2, 3, 6, 5, 4]).nim_value(),
            Nimber::new(2)
        );

        assert_eq!(
            Quicksort::new(vec![4, 1, 6, 5, 7, 3, 8, 2]).nim_value(),
            Nimber::new(5)
        );

        assert_eq!(
            Quicksort::new(vec![4, 1, 6, 5, 7, 8, 2, 3]).nim_value(),
            Nimber::new(0)
        );
    }

    /// Sequence in form of 2,3,4,...,n,1 has nim-value of *(n-1)
    #[test]
    fn one_end_hypothesis() {
        for end in 2..16 {
            let mut sequence = (2..=end).collect::<Vec<u32>>();
            sequence.push(1);
            let quicksort = Quicksort::new(sequence);
            assert_eq!(quicksort.nim_value(), Nimber::new(end - 1));
        }
    }
}
