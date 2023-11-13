//! Subtraction game played on a finite subtraction set

use std::fmt::Display;

use crate::{display, numeric::nimber::Nimber};

/// Subtraction game played on an arbitrary finite subtraction set
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Sub {
    // Invariant: sorted
    subtraction_set: Vec<u32>,
}

impl Display for Sub {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Sub")?;
        display::parens(f, |f| display::commas(f, self.subtraction_set()))
    }
}

impl Sub {
    /// Define new subtraction game with a given subtraction set
    #[inline]
    pub fn new(mut subtraction_set: Vec<u32>) -> Self {
        subtraction_set.sort_unstable();
        Self { subtraction_set }
    }

    /// Get the subtraction set of the game
    #[inline]
    pub const fn subtraction_set(&self) -> &Vec<u32> {
        &self.subtraction_set
    }

    /// Get the infinite Grundy sequence of the subtraction game
    #[inline]
    pub fn grundy_sequence(self) -> GrundySequence {
        let largest = self.subtraction_set().last().copied().unwrap_or(0);
        let previous = vec![Nimber::new(0); largest as usize];

        GrundySequence {
            game: self,
            previous,
            current: 0,
        }
    }
}

/// Grundy Sequence of [Sub] iterator using Grundy scale method.
#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct GrundySequence {
    /// The underlying subtraction game ruleset
    game: Sub,

    /// Ring buffer of previous values
    previous: Vec<Nimber>,

    /// Current heap size to compute nim value for
    current: u32,
}

impl Iterator for GrundySequence {
    type Item = Nimber;

    fn next(&mut self) -> Option<Self::Item> {
        let period_len = self.previous.len();

        let mut for_mex = Vec::with_capacity(self.game.subtraction_set().len());

        for m in self.game.subtraction_set() {
            if m > &self.current {
                break;
            }
            let j = (self.current - m) % period_len as u32;

            for_mex.push(self.previous[j as usize]);
        }
        let mex = Nimber::mex(for_mex);

        self.previous[self.current as usize % period_len] = mex;
        self.current += 1;

        Some(mex)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Number of full periods to compute and assert
    const REPETITIONS: usize = 16;

    macro_rules! assert_grundy {
        ($subtraction_set:expr, $period:expr, $period_len:expr) => {
            let seq = Sub::new($subtraction_set.into())
                .grundy_sequence()
                .take(REPETITIONS * $period_len)
                .collect::<Vec<_>>();
            assert_eq!(seq.len(), REPETITIONS * $period_len);
            assert_eq!(
                seq,
                $period
                    .into_iter()
                    .map(Nimber::new)
                    .cycle()
                    .take(REPETITIONS * $period_len)
                    .collect::<Vec<_>>()
            );
        };
    }

    #[test]
    fn correct_grundy_sequence() {
        assert_grundy!([1], [0, 1], 2);
        assert_grundy!([2], [0, 0, 1, 1], 4);
        assert_grundy!([1, 2], [0, 1, 2], 3);
        assert_grundy!([1, 2, 3], [0, 1, 2, 3], 4);
        assert_grundy!([5], [0, 0, 0, 0, 0, 1, 1, 1, 1, 1], 10);
        assert_grundy!([2, 3, 5], [0, 0, 1, 1, 2, 2, 3], 7);
    }
}
