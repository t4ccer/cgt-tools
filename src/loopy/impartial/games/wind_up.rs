//! Subtraction game played modulo some number `n`.
//!
//! This game has been proposed at Games-at-Dal 2023 conference by Alfie Davies.

use crate::{display, numeric::nimber::Nimber};
use std::{collections::HashSet, fmt::Display};

/// Value of graph vertex - finite or infinite
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Vertex {
    /// Vertex that is equal to some finite nimber.
    Value(Nimber),

    /// Vertex that can move in a finite loop, or escape to one of the nimbers.
    Loop(Vec<Nimber>),

    // TODO: Remove
    /// Vertex that couldn't be determined. Should never happen.
    Unknown,
}

impl Display for Vertex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Vertex::Value(n) => write!(f, "{}", n),
            Vertex::Loop(infs) => {
                write!(f, "∞")?;
                if !infs.is_empty() {
                    display::parens(f, |f| display::commas(f, infs))?;
                }
                Ok(())
            }
            Vertex::Unknown => write!(f, "?"),
        }
    }
}

impl Vertex {
    /// Check if vertex is a finite zero
    fn is_zero(&self) -> bool {
        match self {
            Vertex::Value(val) if val.value() == 0 => true,
            _ => false,
        }
    }
}

/// Modular subtraction game
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindUp {
    graph: Vec<Vertex>,
    subtraction_set: Vec<u32>,
}

impl WindUp {
    /// Get the underlying game graph
    #[inline]
    pub fn graph(&self) -> &Vec<Vertex> {
        &self.graph
    }

    /// Get the subtraction set of the game
    #[inline]
    pub fn subtraction_set(&self) -> &Vec<u32> {
        &self.subtraction_set
    }

    /// Get the `n` component of `Sub(n, a, b)`
    pub fn n(&self) -> u32 {
        self.graph.len() as u32
    }
}

impl Display for WindUp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Sub")?;
        display::parens(f, |f| {
            write!(f, "n={}, ", self.n())?;
            display::braces(f, |f| display::commas(f, &self.subtraction_set()))
        })?;
        write!(f, " = ")?;
        display::brackets(f, |f| display::commas(f, &self.graph()))
    }
}

impl WindUp {
    /// Solve using graph orbiting method.
    ///
    /// # Arguments
    ///
    /// `n` - Size of the game graph. Will be used in `mod n`.
    ///
    /// `subtraction_set` - Subtraction set for the game
    pub fn solve_using_graph(n: u32, subtraction_set: Vec<u32>) -> Self {
        let mut res = WindUp {
            graph: vec![Vertex::Unknown; n as usize],
            subtraction_set,
        };

        // First zero is trivial - the first element is zero by the game definition
        res.graph[0] = Vertex::Value(Nimber::new(0));

        let n = n as i32;

        // First pass - find other zeros
        // element is a zero if for every move to non-zero position there is a response move to zero
        for _ in 0..res.graph.len() {
            'inner: for idx in 1..(res.graph.len() as i32) {
                // Alredy visited and marked as zero
                if !matches!(res.graph[idx as usize], Vertex::Unknown) {
                    continue;
                }

                for first_move in res.subtraction_set() {
                    // Make a move
                    let move_vertex = &res.graph[(idx - *first_move as i32).rem_euclid(n) as usize];

                    // If we can move to zero, we cannot be zero.
                    if !matches!(move_vertex, Vertex::Unknown) {
                        continue 'inner;
                    }

                    // Check if there's a response move to zero
                    let can_respond_to_zero = res.subtraction_set().iter().any(|response_move| {
                        let response_vertex =
                            &res.graph[(idx - *first_move as i32 - *response_move as i32)
                                .rem_euclid(n) as usize];
                        response_vertex.is_zero()
                    });

                    if !can_respond_to_zero {
                        continue 'inner;
                    }
                }

                res.graph[idx as usize] = Vertex::Value(Nimber::new(0));
            }
        }

        // Second pass - compute mex for each finite element
        for _ in 0..res.graph.len() {
            'inner: for idx in 1..(res.graph.len() as i32) {
                if !matches!(res.graph[idx as usize], Vertex::Unknown) {
                    continue;
                }

                let mut for_mex = Vec::with_capacity(res.n() as usize);
                for m in res.subtraction_set() {
                    let v1 = &res.graph[(idx - *m as i32).rem_euclid(n) as usize];
                    match v1 {
                        Vertex::Value(g) => for_mex.push(*g),
                        Vertex::Unknown | Vertex::Loop(_) => continue 'inner,
                    };
                }

                let g = Nimber::mex(for_mex);
                res.graph[idx as usize] = Vertex::Value(g);
            }
        }

        // Third pass - compute infinites
        for _ in 0..res.graph.len() {
            for idx in 0..(res.n() as i32) {
                // If we're a nimber we cannot be an infinity
                if matches!(res.graph[idx as usize], Vertex::Value(_)) {
                    continue;
                }

                let mut infinities = vec![];

                for m in res.subtraction_set() {
                    let v1 = &res.graph[(idx - *m as i32).rem_euclid(n) as usize];
                    if let Vertex::Value(g) = v1 {
                        if !infinities.contains(g) {
                            infinities.push(*g);
                        }
                    }
                }

                res.graph[idx as usize] = Vertex::Loop(infinities);
            }
        }

        res
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
    pub fn solve_using_sequence(period: &[u32], n: u32, subtraction_set: Vec<u32>) -> Self {
        assert!(period.len() > 0, "Period must not be empty");

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
            // Sub(n=12, a=1, b=3)
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
            {
                let r = WindUp {
                    graph: extended_seq
                        .iter()
                        .map(|n| Vertex::Value(Nimber::new(*n)))
                        .collect(),
                    subtraction_set: subtraction_set.clone(),
                };
                eprintln!("{}", r);
            }

            // Cycle/fixpoint! We can break
            if seen.contains(&extended_seq) {
                break;
            }
            seen.insert(extended_seq.clone());
        }

        // TODO: Add statistics: cycle len, sequence len

        WindUp {
            graph: extended_seq
                .iter()
                .map(|n| Vertex::Value(Nimber::new(*n)))
                .collect(),
            subtraction_set: subtraction_set.clone(),
        }
    }
}

#[test]
fn sequence_reduction_graph_equivalence() {
    // Graph and sequence are requivalent on finite games
    let using_sequence =
        WindUp::solve_using_sequence(&[0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 1, 2], 40, vec![6, 7]);
    let using_graph = WindUp::solve_using_graph(40, vec![6, 7]);
    assert_eq!(using_graph, using_sequence);

    // Initial starting sequence doesn't matter for the final result
    // That is actually not always true, see below
    let using_sequence1 =
        WindUp::solve_using_sequence(&[0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 1, 2], 40, vec![6, 7]);
    let using_sequence2 = WindUp::solve_using_sequence(&[1], 40, vec![6, 7]);
    assert_eq!(using_sequence1, using_sequence2);
}

#[test]
fn weird_sequence() {
    let a = 1;
    let b = 2;
    let n = 3;

    let s1 = WindUp::solve_using_sequence(&[0, 0, 0], n, vec![a, b]);
    let s2 = WindUp::solve_using_sequence(&[0, 1, 2], n, vec![a, b]);

    assert_ne!(s1, s2);
}

// TODO: Test conjecture: P(Gr) = Gr iff Sub(n = a+b, {a,b})
