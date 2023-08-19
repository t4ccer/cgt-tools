//! This game is played on a sequence of numbers, and players move by pivoting around
//! a number in the manner of the quicksort algorithm.
//!
//! This is impartial version of the game, where both players can pick both even and odd pivots.
//! For partizan version see TODO.
//!
//! This game has been proposed in [Andreas Chen's "The Quicksort Game"](https://www.diva-portal.org/smash/get/diva2:935354/FULLTEXT01.pdf>).

use std::fmt::Display;

use crate::numeric::nimber::Nimber;

/// See [quickcheck](self) header
#[derive(Debug, PartialEq, Eq)]
pub struct Quicksort {
    sequence: Vec<u32>,
}

impl Display for Quicksort {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for elem in self.sequence() {
            write!(f, "{}", elem)?;
        }
        Ok(())
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
