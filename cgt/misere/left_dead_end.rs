// Adapted from https://github.com/alfiemd/gemau/blob/main/src/left_dead_end.rs

#![allow(missing_docs)]

use std::{fmt::Display, iter::Sum};

use auto_ops::impl_op_ex;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Number {
    value: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Moves {
    moves: Vec<LeftDeadEnd>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum LeftDeadEnd {
    Number(Number),
    Moves(Moves),
}

impl Display for LeftDeadEnd {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LeftDeadEnd::Number(n) => write!(f, "{}", n.value),
            LeftDeadEnd::Moves(moves) => {
                write!(f, "{{")?;
                for (idx, mov) in moves.moves.iter().enumerate() {
                    if idx != 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{mov}")?;
                }
                write!(f, "}}")
            }
        }
    }
}

impl_op_ex!(+|g: &LeftDeadEnd, h: &LeftDeadEnd| -> LeftDeadEnd { LeftDeadEnd::new_sum(g, h) });
impl_op_ex!(+=|g: &mut LeftDeadEnd, h: &LeftDeadEnd| { *g = LeftDeadEnd::new_sum(g, h) });

impl<'g> Sum<&'g LeftDeadEnd> for LeftDeadEnd {
    fn sum<I: Iterator<Item = &'g LeftDeadEnd>>(iter: I) -> Self {
        let mut res = LeftDeadEnd::new_integer(0);
        for g in iter {
            res += g;
        }
        res
    }
}

impl Sum for LeftDeadEnd {
    fn sum<I: Iterator<Item = LeftDeadEnd>>(iter: I) -> Self {
        let mut res = LeftDeadEnd::new_integer(0);
        for g in iter {
            res += g;
        }
        res
    }
}

impl LeftDeadEnd {
    pub fn new_integer(value: u64) -> LeftDeadEnd {
        LeftDeadEnd::Number(Number { value })
    }

    pub fn new_moves(mut moves: Vec<LeftDeadEnd>) -> LeftDeadEnd {
        if moves.len() == 0 {
            return LeftDeadEnd::new_integer(0);
        }

        moves.sort_unstable();
        moves.dedup();

        let moves = moves.iter().fold(Vec::new(), |mut acc, x| {
            if !moves
                .iter()
                .any(|y| LeftDeadEnd::partial_cmp_games(&y, &x) == Some(std::cmp::Ordering::Less))
                && !acc.contains(x)
            {
                acc.push(x.canonical());
            }
            acc
        });

        if moves.len() == 1 {
            if let LeftDeadEnd::Number(n) = &moves[0] {
                return LeftDeadEnd::new_integer(n.value + 1);
            }
        }

        LeftDeadEnd::Moves(Moves { moves })
    }

    pub fn new_sum(lhs: &LeftDeadEnd, rhs: &LeftDeadEnd) -> LeftDeadEnd {
        if lhs.moves_len() == 0 {
            return rhs.clone();
        }

        if rhs.moves_len() == 0 {
            return lhs.clone();
        }

        let mut moves = Vec::with_capacity(lhs.moves_len() + rhs.moves_len());

        for g in lhs.moves() {
            let s = &g + rhs;
            moves.push(s);
        }

        for h in rhs.moves() {
            let s = lhs + &h;
            moves.push(s);
        }

        LeftDeadEnd::new_moves(moves)
    }

    #[must_use]
    pub fn canonical(&self) -> Self {
        let moves = self.moves().fold(Vec::new(), |mut acc, x| {
            if !self
                .moves()
                .any(|y| LeftDeadEnd::partial_cmp_games(&y, &x) == Some(std::cmp::Ordering::Less))
                && !acc.contains(&x)
            {
                acc.push(x.canonical());
            }
            acc
        });
        LeftDeadEnd::new_moves(moves)
    }

    pub fn is_integer(&self) -> bool {
        matches!(self, LeftDeadEnd::Number(_))
    }

    pub fn birthday(&self) -> u64 {
        match self {
            LeftDeadEnd::Number(n) => n.value,
            LeftDeadEnd::Moves(moves) => {
                let res = moves
                    .moves
                    .iter()
                    .fold(0, |max, sub_move| u64::max(max, sub_move.birthday()))
                    + 1;

                res
            }
        }
    }

    pub fn moves(&self) -> MovesIter {
        MovesIter { idx: 0, game: self }
    }

    pub fn moves_len(&self) -> usize {
        match self {
            LeftDeadEnd::Number(n) if n.value == 0 => 0,
            LeftDeadEnd::Number(_) => 1,
            LeftDeadEnd::Moves(m) => m.moves.len(),
        }
    }

    pub fn factors(&self) -> Vec<LeftDeadEnd> {
        let mut potential_factors = Vec::with_capacity(self.moves_len());
        potential_factors.push(LeftDeadEnd::new_integer(0));
        potential_factors.push(self.clone());

        for r in self.moves() {
            let div = r.factors();
            for d in div {
                if potential_factors
                    .iter()
                    .all(|g| !LeftDeadEnd::equal_games(g, &d))
                {
                    potential_factors.push(d);
                }
            }
        }

        let mut factors = Vec::with_capacity(potential_factors.len());

        for i in 0..potential_factors.len() {
            for j in i..potential_factors.len() {
                let sum = &potential_factors[i] + &potential_factors[j];
                if LeftDeadEnd::equal_games(&sum, self) {
                    factors.push(potential_factors[i].clone());
                    if i != j {
                        factors.push(potential_factors[j].clone());
                    }
                }
            }
        }

        factors
    }

    pub fn is_atom(&self) -> bool {
        self.factors().len() == 2
    }

    fn partial_cmp_games(lhs: &LeftDeadEnd, rhs: &LeftDeadEnd) -> Option<std::cmp::Ordering> {
        use std::cmp::Ordering;

        match (
            LeftDeadEnd::ge_games(lhs, rhs),
            LeftDeadEnd::ge_games(rhs, lhs),
        ) {
            (true, true) => Some(Ordering::Equal),
            (true, false) => Some(Ordering::Greater),
            (false, true) => Some(Ordering::Less),
            (false, false) => None,
        }
    }

    pub fn ge_games(lhs: &LeftDeadEnd, rhs: &LeftDeadEnd) -> bool {
        if lhs.moves_len() == 0 {
            return rhs.moves_len() == 0;
        }

        lhs.moves()
            .all(|g| rhs.moves().any(|h| LeftDeadEnd::ge_games(&g, &h)))
    }

    fn equal_games(lhs: &LeftDeadEnd, rhs: &LeftDeadEnd) -> bool {
        matches!(
            LeftDeadEnd::partial_cmp_games(lhs, rhs),
            Some(std::cmp::Ordering::Equal)
        )
    }
}

#[derive(Debug, Clone, Copy)]
pub struct MovesIter<'g> {
    idx: usize,
    game: &'g LeftDeadEnd,
}

impl<'g> Iterator for MovesIter<'g> {
    type Item = LeftDeadEnd;

    fn next(&mut self) -> Option<Self::Item> {
        match self.game {
            LeftDeadEnd::Number(n) if n.value == 0 => None,
            LeftDeadEnd::Number(n) => {
                if self.idx > 0 {
                    None
                } else {
                    self.idx += 1;
                    Some(LeftDeadEnd::new_integer(n.value - 1))
                }
            }
            LeftDeadEnd::Moves(m) => {
                let res = m.moves.get(self.idx).cloned();
                if res.is_some() {
                    self.idx += 1;
                }
                res
            }
        }
    }

    fn count(self) -> usize {
        match self.game {
            LeftDeadEnd::Number(n) if n.value == 0 => 0,
            LeftDeadEnd::Number(_) => (self.idx == 0) as usize,
            LeftDeadEnd::Moves(m) => m.moves.len() - self.idx,
        }
    }
}

pub fn is_factorization_unique(value: &LeftDeadEnd, factors: &[LeftDeadEnd]) -> bool {
    let sum: LeftDeadEnd = factors.iter().sum();
    value == &sum
}

#[test]
fn from_moves_zero() {
    let g = LeftDeadEnd::new_moves(vec![]);
    assert!(g.is_integer());
    assert_eq!(g.to_string(), "0");
}

#[test]
fn from_moves_integer() {
    let g = LeftDeadEnd::new_moves(vec![LeftDeadEnd::new_integer(41)]);
    assert!(g.is_integer());
    assert_eq!(g.to_string(), "42");
}

#[test]
fn birthday_integer() {
    assert_eq!(LeftDeadEnd::new_integer(0).birthday(), 0);
    assert_eq!(LeftDeadEnd::new_integer(42).birthday(), 42);
}

#[test]
fn birthday_moves() {
    let moves = vec![
        LeftDeadEnd::new_integer(3),
        LeftDeadEnd::new_integer(4),
        LeftDeadEnd::Moves(Moves {
            moves: vec![LeftDeadEnd::new_integer(6), LeftDeadEnd::new_integer(8)],
        }),
    ];
    assert_eq!(LeftDeadEnd::Moves(Moves { moves }).birthday(), 10);
}

// #[test]
// fn add() {
//     assert_eq!(
//         (LeftDeadEnd::new_integer(8) + LeftDeadEnd::new_integer(8)),
//         LeftDeadEnd::new_integer(16)
//     );

//     assert_eq!(
//         (LeftDeadEnd::new_integer(2) + LeftDeadEnd::new_integer(2)),
//         LeftDeadEnd::new_integer(4)
//     );

//     assert_eq!(
//         (LeftDeadEnd::new_integer(3) + LeftDeadEnd::new_integer(1)),
//         LeftDeadEnd::new_integer(4)
//     );
// }
