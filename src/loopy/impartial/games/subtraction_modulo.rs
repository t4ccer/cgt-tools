use crate::numeric::nimber::Nimber;
use std::{collections::HashSet, fmt::Display};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Vertex {
    Value(Nimber),
    Unknown,
}

impl Display for Vertex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Vertex::Value(n) => write!(f, "{}", n),
            Vertex::Unknown => write!(f, "?"),
        }
    }
}

impl Vertex {
    fn is_zero(&self) -> bool {
        match self {
            Vertex::Value(val) if val.0 == 0 => true,
            _ => false,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Sub {
    graph: Vec<Vertex>,
    a: u32,
    b: u32,
}

impl Display for Sub {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Sub(n = {}, a = {}, b = {}) = [",
            self.graph.len(),
            self.a,
            self.b
        )?;
        for (idx, v) in self.graph.iter().enumerate() {
            if idx != 0 {
                write!(f, ", ")?;
            }
            write!(f, "{}", v)?;
        }
        write!(f, "]")?;
        Ok(())
    }
}

impl Sub {
    /// Solve using graph orbiting method.
    ///
    /// # Arguments
    ///
    /// `n` - Size of the game graph. Will be used in `mod n`.
    ///
    /// `a` - First element in subtraction set
    ///
    /// `a` - Second element in subtraction set
    pub fn solve_using_graph(n: usize, a: u32, b: u32) -> Self {
        let mut res = Sub {
            graph: vec![Vertex::Unknown; n],
            a,
            b,
        };

        // First zero is trivial - the first element is zero by the game definition
        res.graph[0] = Vertex::Value(Nimber(0));

        let n = n as i32;

        // First pass - find other zeros
        // element is a zero if for every move to non-zero position there is a move to zero
        for _ in 0..res.graph.len() {
            for idx in 1..(res.graph.len() as i32) {
                // Alredy visited and marked as zero
                if !matches!(res.graph[idx as usize], Vertex::Unknown) {
                    continue;
                }

                // We check for, then idx==0
                // idx -> v1 != 0 -> 0
                // AND
                // idx -> v2 != 0 -> 0

                let v1 = &res.graph[(idx - res.a as i32).rem_euclid(n) as usize];
                let v11 = &res.graph[(idx - res.a as i32 - res.a as i32).rem_euclid(n) as usize];
                let v12 = &res.graph[(idx - res.a as i32 - res.b as i32).rem_euclid(n) as usize];

                let v2 = &res.graph[(idx - res.b as i32).rem_euclid(n) as usize];
                let v21 = &res.graph[(idx - res.b as i32 - res.a as i32).rem_euclid(n) as usize];
                let v22 = &res.graph[(idx - res.b as i32 - res.b as i32).rem_euclid(n) as usize];

                if !matches!((v1, v2), (Vertex::Unknown, Vertex::Unknown)) {
                    continue;
                }

                if !v11.is_zero() && !v12.is_zero() {
                    continue;
                }

                if !v21.is_zero() && !v22.is_zero() {
                    continue;
                }

                res.graph[idx as usize] = Vertex::Value(Nimber(0));
            }
        }

        // Second pass - compute mex for each finite element
        for _ in 0..res.graph.len() {
            for idx in 1..(res.graph.len() as i32) {
                if !matches!(res.graph[idx as usize], Vertex::Unknown) {
                    continue;
                }

                let v1 = &res.graph[(idx - res.a as i32).rem_euclid(n) as usize];
                let v2 = &res.graph[(idx - res.b as i32).rem_euclid(n) as usize];

                let g1 = match v1 {
                    Vertex::Unknown => continue,
                    Vertex::Value(g) => *g,
                };
                let g2 = match v2 {
                    Vertex::Unknown => continue,
                    Vertex::Value(g) => *g,
                };

                let moves = vec![g1, g2];
                let g = Nimber::mex(moves);
                res.graph[idx as usize] = Vertex::Value(g);
            }
        }

        // TODO: Handle infinities and assert that threre's no more unknowns

        res
    }

    /// Solve using table/sequence method.
    ///
    /// # Arguments
    ///
    /// `period` - Period of a regular subtraction game `Sub(a, b)`.
    ///
    /// `n` - Size of the game graph. Will be used in `mod n`.
    ///
    /// `a` - First element in subtraction set
    ///
    /// `a` - Second element in subtraction set
    pub fn solve_using_sequence(period: &[u32], n: usize, a: u32, b: u32) -> Self {
        // List of allowed moves. I'm not sure if it works for more than two elements
        let moves = vec![a, b];

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

                for m in &moves {
                    let i = (idx as i32 - (*m as i32)).rem_euclid(n as i32) as usize;
                    for_mex.push(Nimber(extended_seq[i]));
                }
                let new = Nimber::mex(for_mex).0;
                new_seq.push(new);
            }

            extended_seq = new_seq;

            // Cycle/fixpoint! We can break
            if seen.contains(&extended_seq) {
                break;
            }
            seen.insert(extended_seq.clone());
        }

        Sub {
            graph: extended_seq
                .iter()
                .map(|n| Vertex::Value(Nimber(*n)))
                .collect(),
            a,
            b,
        }
    }
}

#[test]
fn sequence_reduction_graph_equivalence() {
    let using_sequence =
        Sub::solve_using_sequence(&[0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 1, 2], 40, 6, 7);
    let using_graph = Sub::solve_using_graph(40, 6, 7);
    assert_eq!(using_graph, using_sequence);

    // let a = 2;
    // let b = 3;

    // for n in 1..100 {
    // 	let using_sequence = Sub::solve_using_sequence(&[0,0,1,1,2,0], n, a, b);
    // 	eprintln!("[n = {n}] [convergence = {}] {}", using_sequence.0, using_sequence.1);
    // // let mut using_graph = Sub::new_unknown(n, a, b);
    // 	// using_graph.solve_using_graph();
    // }
    // panic!();

    // assert_eq!(using_graph, using_sequence);
}
