//! This game is based on [quicksort](crate::short::impartial::games::quicksort) but with one
//! crucial difference: the pivot elements are no longer integers, but are instead numbers of the
//! form 1.5, 2.5, 3.5 and so on.
//!
//! This is impartial version of the game, where both players can pick both even and odd pivots.
//! For partizan version see TODO.
//!
//! This game has been proposed in [Andreas Chen's "The Quicksort Game"](https://www.diva-portal.org/smash/get/diva2:935354/FULLTEXT01.pdf>).

use crate::numeric::nimber::Nimber;
use std::fmt::Display;

/// See [`pseudo_quickcheck`](self) header
#[derive(Debug, PartialEq, Eq)]
pub struct PseudoQuicksort {
    sequence: Vec<u32>,
}

impl Display for PseudoQuicksort {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for elem in self.sequence() {
            write!(f, "{}", elem)?;
        }
        Ok(())
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

    /// Get a unique list of moves from the position
    pub fn moves(&self) -> Vec<Self> {
        let mut res = vec![];
        for pivot in self.sequence() {
            let new = self.pivot_on(*pivot);
            if !res.contains(&new) && &new != self {
                res.push(new);
            }
        }
        res
    }

    /// Calculate the Nim value of the position
    pub fn nim_value(&self) -> Nimber {
        let moves = self.moves();
        let mut game_moves = Vec::with_capacity(moves.len());
        for m in moves {
            game_moves.push(m.nim_value());
        }
        Nimber::mex(game_moves)
    }
}
