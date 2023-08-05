//! Subtraction game played on a finite subtraction set

use std::fmt::Display;

use crate::{display, numeric::nimber::Nimber};

/// Subtraction game
#[derive(Clone, Debug)]
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
    /// Get the subtraction set of the game
    #[inline]
    pub fn subtraction_set(&self) -> &Vec<u32> {
        &self.subtraction_set
    }
}

impl Sub {
    /// Define new subtraction game with a given subtraction set
    #[inline]
    pub fn new(mut subtraction_set: Vec<u32>) -> Sub {
        subtraction_set.sort();
        Sub { subtraction_set }
    }

    /// Get the infinite Grundy sequence of the subtraction game
    #[inline]
    pub fn grundy_sequence(self) -> GrundySequence {
        GrundySequence {
            game: self,
            previous: vec![],
            current: 0,
        }
    }
}

/// Grundy Sequence of [Sub]
#[derive(Debug)]
pub struct GrundySequence {
    game: Sub,
    previous: Vec<Nimber>,
    current: i32,
}

impl Iterator for GrundySequence {
    type Item = Nimber;

    fn next(&mut self) -> Option<Self::Item> {
        // TODO: purge previous when went over game.subtraction_set().last()

        let mut for_mex = Vec::with_capacity(self.game.subtraction_set().len());
        for m in self.game.subtraction_set() {
            let j = self.current - *m as i32;
            if j < 0 {
                continue;
            }
            for_mex.push(self.previous[j as usize]);
        }
        let mex = Nimber::mex(for_mex);

        self.current += 1;
        self.previous.push(mex);

        Some(mex)
    }
}

impl GrundySequence {
    /// Take first `n` elements of the Grundy sequence
    #[inline]
    pub fn first_n(self, n: usize) -> Vec<Nimber> {
        self.into_iter().take(n).collect::<Vec<_>>()
    }
}

#[test]
fn correct_grundy_sequence() {
    assert_eq!(
        Sub::new(vec![1, 2]).grundy_sequence().first_n(5),
        vec![0, 1, 2, 0, 1]
            .into_iter()
            .map(Nimber::new)
            .collect::<Vec<_>>()
    );
}
