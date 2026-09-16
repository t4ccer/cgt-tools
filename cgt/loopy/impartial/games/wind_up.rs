//! Subtraction game played modulo some number `n`.
//!
//! This game has been proposed at Games-at-Dal 2023 conference by Alfie Davies.

use crate::{display, loopy::impartial::vertex::Vertex, numeric::nimber::Nimber};
use std::{collections::HashSet, fmt::Display};

/// Modular subtraction game
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct WindUp {
    graph: Vec<Vertex>,
    subtraction_set: Vec<u32>,
}

impl Display for WindUp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "WindUp")?;
        display::parens(f, |f| {
            write!(f, "n={}, ", self.n())?;
            display::braces(f, |f| display::commas(f, self.subtraction_set()))
        })?;
        write!(f, " = ")?;
        display::brackets(f, |f| display::commas(f, self.graph()))
    }
}

impl WindUp {
    /// Solve using the generalized Sprague-Grundy algorithm of Smith.
    ///
    /// See: C. A. B. Smith, Graphs and composite games, J. Combin. Theory 1 (1966), 51-81,
    /// Section 9, conditions (23a)-(23e) and (26) on pp. 69-71 for the finite values, and the
    /// `J` function (32) on p. 73 for the finite options of an infinite vertex.
    ///
    /// # Arguments
    ///
    /// `n` - Size of the game graph. Will be used in `mod n`.
    ///
    /// `subtraction_set` - Subtraction set for the game
    pub fn new_using_graph(n: u32, subtraction_set: Vec<u32>) -> Self {
        let n = n as usize;
        let followers = |idx: usize| {
            subtraction_set
                .iter()
                .map(move |m| (idx as i64 - i64::from(*m)).rem_euclid(n as i64) as usize)
        };

        let mut labels: Vec<Option<Nimber>> = vec![None; n];

        // First element is zero by the game definition
        labels[0] = Some(Nimber::new(0));

        // A vertex gets label k when the mex of its labelled followers is k and every unlabelled
        // follower has a follower labelled k, so that any attempt to enter a loop can be countered
        // by moving back to a k. Label k needs followers labelled 0..k so no label can exceed the
        // size of the subtraction set.
        for k in 0..=(subtraction_set.len() as u32) {
            let k = Nimber::new(k);
            loop {
                let mut changed = false;
                for idx in 1..n {
                    if labels[idx].is_some() {
                        continue;
                    }

                    let finite = followers(idx).filter_map(|f| labels[f]).collect();
                    if Nimber::mex(finite) != k {
                        continue;
                    }

                    let countered = followers(idx)
                        .filter(|f| labels[*f].is_none())
                        .all(|f| followers(f).any(|z| labels[z] == Some(k)));
                    if countered {
                        labels[idx] = Some(k);
                        changed = true;
                    }
                }
                if !changed {
                    break;
                }
            }
        }

        let graph = (0..n)
            .map(|idx| {
                labels[idx].map_or_else(
                    || {
                        let mut escapes: Vec<Nimber> =
                            followers(idx).filter_map(|f| labels[f]).collect();
                        escapes.sort_unstable();
                        escapes.dedup();
                        Vertex::Loop(escapes)
                    },
                    Vertex::Value,
                )
            })
            .collect();

        Self {
            graph,
            subtraction_set,
        }
    }

    /// Solve using table/sequence method.
    ///
    /// # Arguments
    ///
    /// `period` - Period of the initial sequence
    ///
    /// `n` - Size of the game graph. Will be used in `mod n`.
    ///
    /// `subtraction_set` - Subtraction set for the game
    ///
    /// # Panics
    /// - `period` is empty
    pub fn new_using_sequence(period: &[u32], n: u32, subtraction_set: Vec<u32>) -> Self {
        assert!(!period.is_empty(), "Period must not be empty");

        let n = n as usize;

        // Repeat classical subtraction period to match the length of the game graph
        let mut extended_seq = Vec::with_capacity(n);
        for idx in 0..n {
            extended_seq.push(period[idx % period.len()]);
        }

        // To keep track when we hit fixpoint/cycle
        let mut seen = HashSet::new();
        seen.insert(extended_seq.clone());

        loop {
            // First element of the new sequence is always zero.
            let mut new_seq = Vec::with_capacity(n);
            new_seq.push(0);

            // Each next element is a mex of elements in the previous sequence to this element points
            // e.g.
            // WindUp(n=12, {1,3})
            // old: 0 0 * * 0 *2
            //      ^     ^
            //      |     |
            //      -------\
            // new: x x x x i ?
            // i = mex(0, *) = *2
            for idx in 1..n {
                let mut for_mex = Vec::new();

                for m in &subtraction_set {
                    let i = (idx as i32 - (*m as i32)).rem_euclid(n as i32) as usize;
                    for_mex.push(Nimber::new(extended_seq[i]));
                }
                let new = Nimber::mex(for_mex).value();
                new_seq.push(new);
            }

            if new_seq == extended_seq {
                break;
            }

            extended_seq = new_seq;

            // Cycle/fixpoint! We can break
            if seen.contains(&extended_seq) {
                break;
            }
            seen.insert(extended_seq.clone());
        }

        // TODO: Add statistics: cycle len, sequence len

        Self {
            graph: extended_seq
                .iter()
                .map(|n| Vertex::Value(Nimber::new(*n)))
                .collect(),
            subtraction_set,
        }
    }

    /// Get the underlying game graph
    #[inline]
    pub const fn graph(&self) -> &Vec<Vertex> {
        &self.graph
    }

    /// Get the subtraction set of the game
    #[inline]
    pub const fn subtraction_set(&self) -> &Vec<u32> {
        &self.subtraction_set
    }

    /// Get the `n` component of `WindUp(n, {...})`
    #[inline]
    pub const fn n(&self) -> u32 {
        self.graph.len() as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sequence_reduction_graph_equivalence() {
        // Graph and sequence are requivalent on finite games
        let using_sequence =
            WindUp::new_using_sequence(&[0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 1, 2], 40, vec![6, 7]);
        let using_graph = WindUp::new_using_graph(40, vec![6, 7]);
        assert_eq!(using_graph, using_sequence);

        // Initial starting sequence doesn't matter for the final result
        // That is actually not always true, see below
        let using_sequence1 =
            WindUp::new_using_sequence(&[0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 1, 2], 40, vec![6, 7]);
        let using_sequence2 = WindUp::new_using_sequence(&[1], 40, vec![6, 7]);
        assert_eq!(using_sequence1, using_sequence2);
    }

    #[test]
    fn weird_sequence() {
        let a = 1;
        let b = 2;
        let n = 3;

        let s1 = WindUp::new_using_sequence(&[0, 0, 0], n, vec![a, b]);
        let s2 = WindUp::new_using_sequence(&[0, 1, 2], n, vec![a, b]);

        assert_ne!(s1, s2);
    }

    fn graph_values(n: u32, subtraction_set: Vec<u32>) -> Vec<String> {
        WindUp::new_using_graph(n, subtraction_set)
            .graph()
            .iter()
            .map(ToString::to_string)
            .collect()
    }

    #[test]
    fn graph_finite_cycle_members() {
        assert_eq!(
            graph_values(7, vec![1, 2, 5]),
            ["0", "*", "*2", "0", "*", "*2", "0"]
        );
    }

    #[test]
    fn graph_loops() {
        assert_eq!(graph_values(4, vec![2]), ["0", "∞", "*", "∞"]);
        assert_eq!(
            graph_values(5, vec![1, 3]),
            ["0", "∞(0)", "∞(0)", "∞(0)", "0"]
        );
        assert_eq!(graph_values(6, vec![2, 3]), ["0", "*", "*", "*2", "0", "0"]);
    }

    // TODO: Test conjecture: P(Gr) = Gr iff WindUp(n = a+b, {a,b})
}
