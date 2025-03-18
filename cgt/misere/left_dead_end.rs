#![allow(missing_docs)]

//! Left dead end is a game where every follower is a left end (there is no move for Left)
//!
//! This code is heavily based on <https://github.com/alfiemd/gemau> by Alfie Davies

use crate::parsing::{impl_from_str_via_parser, lexeme, try_option, Parser};
use auto_ops::impl_op_ex;
use std::{cmp::Ordering, fmt::Display, mem::ManuallyDrop};

pub mod interned;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum LeftDeadEndInner {
    Integer(u32),
    Moves(Vec<LeftDeadEndInner>),
}

impl Display for LeftDeadEndInner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use crate::display::{braces, commas};

        match self {
            LeftDeadEndInner::Integer(n) => write!(f, "{}", n),
            LeftDeadEndInner::Moves(moves) => braces(f, |f| commas(f, moves.as_slice())),
        }
    }
}

impl LeftDeadEndInner {
    #[inline(always)]
    fn into_inner_vec(moves: Vec<LeftDeadEnd>) -> Vec<LeftDeadEndInner> {
        // NOTE: rustc can sometimes optimize the naive way to do the same
        // https://godbolt.org/z/dd1vd3Ycq

        let mut md = ManuallyDrop::new(moves);
        let ptr: *mut LeftDeadEnd = md.as_mut_ptr();
        let len = md.len();
        let capacity = md.capacity();
        // SAFETY: LeftDeadEnd is #[repr(transparent)] so the cast is OK
        unsafe { Vec::from_raw_parts(ptr.cast::<LeftDeadEndInner>(), len, capacity) }
    }
}

/// Left dead end is a game where every follower is a left end (there is no move for Left)
#[derive(Debug, Clone)]
#[repr(transparent)]
pub struct LeftDeadEnd {
    inner: LeftDeadEndInner,
}

impl Display for LeftDeadEnd {
    #[inline(always)]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.inner.fmt(f)
    }
}

impl PartialEq for LeftDeadEnd {
    #[inline(always)]
    fn eq(&self, rhs: &Self) -> bool {
        matches!(self.partial_cmp(rhs), Some(Ordering::Equal))
    }
}

impl PartialOrd for LeftDeadEnd {
    fn partial_cmp(&self, rhs: &Self) -> Option<Ordering> {
        // Optimization: If integers are not equal then they are incomparable
        if let Some(lhs) = self.to_integer() {
            if let Some(rhs) = rhs.to_integer() {
                if lhs == rhs {
                    return Some(Ordering::Equal);
                }

                return None;
            }
        }

        match (self >= rhs, self <= rhs) {
            (true, true) => Some(Ordering::Equal),
            (true, false) => Some(Ordering::Greater),
            (false, true) => Some(Ordering::Less),
            (false, false) => None,
        }
    }

    fn ge(&self, rhs: &LeftDeadEnd) -> bool {
        if let Some(lhs) = self.to_integer() {
            if lhs == 0 {
                return rhs.is_zero();
            }

            // Optimization: If integers are not equal then they are incomparable
            if let Some(rhs) = rhs.to_integer() {
                return lhs == rhs;
            }
        }

        let rhs_options = rhs.clone().into_moves();
        self.clone().into_moves().iter().all(|left_option| {
            rhs_options
                .iter()
                .any(|right_option| left_option >= right_option)
        })
    }

    fn le(&self, rhs: &LeftDeadEnd) -> bool {
        if let Some(rhs) = rhs.to_integer() {
            if rhs == 0 {
                return self.is_zero();
            }

            // Optimization: If integers are not equal then they are incomparable
            if let Some(lhs) = self.to_integer() {
                return lhs == rhs;
            }
        }

        let lhs_options = self.clone().into_moves();
        rhs.clone().into_moves().iter().all(|right_option| {
            lhs_options
                .iter()
                .any(|left_option| left_option <= right_option)
        })
    }
}

impl_op_ex!(+|lhs: &LeftDeadEnd, rhs: &LeftDeadEnd| -> LeftDeadEnd {
    LeftDeadEnd::new_sum(lhs, rhs)
});

impl_from_str_via_parser!(LeftDeadEnd);

impl LeftDeadEnd {
    /// Construct new *non-positive* integer of given absolute value
    #[inline(always)]
    pub const fn new_integer(integer: u32) -> LeftDeadEnd {
        LeftDeadEnd {
            inner: LeftDeadEndInner::Integer(integer),
        }
    }

    /// Convert to absolute value of integer, if is an integer
    #[inline(always)]
    pub const fn to_integer(&self) -> Option<u32> {
        match self.inner {
            LeftDeadEndInner::Integer(n) => Some(n),
            LeftDeadEndInner::Moves(_) => None,
        }
    }

    /// Construct new position from Right's moves
    #[inline(always)]
    pub fn new_moves(moves: Vec<LeftDeadEnd>) -> LeftDeadEnd {
        let moves = LeftDeadEndInner::into_inner_vec(moves);
        LeftDeadEnd::normalize(LeftDeadEndInner::Moves(moves))
    }

    /// Convert position into Right's moves
    #[inline(always)]
    pub fn into_moves(self) -> Vec<LeftDeadEnd> {
        match self.inner {
            LeftDeadEndInner::Integer(0) => vec![],
            LeftDeadEndInner::Integer(n) => vec![LeftDeadEnd {
                inner: LeftDeadEndInner::Integer(n - 1),
            }],
            LeftDeadEndInner::Moves(moves) => LeftDeadEnd::from_inner_vec(moves),
        }
    }

    #[inline(always)]
    const fn is_zero(&self) -> bool {
        matches!(self.to_integer(), Some(0))
    }

    fn new_sum(lhs: &LeftDeadEnd, rhs: &LeftDeadEnd) -> LeftDeadEnd {
        // Optimization: 0 + g = g
        if lhs.is_zero() {
            return rhs.clone();
        }

        if rhs.is_zero() {
            return lhs.clone();
        }

        if let Some(lhs) = lhs.to_integer() {
            if let Some(rhs) = rhs.to_integer() {
                // Optimization: 1 + n = {n, {n - 1, {n - 2, {n - ..., {1, 1}}}}}
                if lhs == 1 || rhs == 1 {
                    let mut acc = LeftDeadEndInner::Moves(vec![
                        LeftDeadEndInner::Integer(1),
                        LeftDeadEndInner::Integer(1),
                    ]);

                    for i in 2..=(lhs.max(rhs)) {
                        acc = LeftDeadEndInner::Moves(vec![LeftDeadEndInner::Integer(i), acc]);
                    }

                    return LeftDeadEnd { inner: acc };
                }
            }
        }

        let lhs_options = lhs.clone().into_moves();
        let rhs_options = rhs.clone().into_moves();
        let mut sum_options = Vec::with_capacity(lhs_options.len() + rhs_options.len());

        for g in &lhs_options {
            sum_options.push(g + rhs);
        }
        for h in &rhs_options {
            sum_options.push(lhs + h);
        }

        LeftDeadEnd::new_moves(sum_options)
    }

    fn parse(parser: Parser<'_>) -> Option<(Parser<'_>, LeftDeadEnd)> {
        let parser = parser.trim_whitespace();
        if let Some(parser) = parser.parse_ascii_char('{') {
            let parser = parser.trim_whitespace();

            let mut options = Vec::new();
            let mut loop_parser = parser;
            while let Some((parser, option)) = lexeme!(loop_parser, LeftDeadEnd::parse) {
                loop_parser = parser;
                options.push(option);

                match loop_parser.parse_ascii_char(',') {
                    Some(parser) => {
                        loop_parser = parser.trim_whitespace();
                    }
                    None => break,
                }
            }
            let parser = loop_parser.trim_whitespace();
            let parser = try_option!(parser.parse_ascii_char('}'));
            let parser = parser.trim_whitespace();
            Some((parser, LeftDeadEnd::new_moves(options)))
        } else {
            let (parser, integer) = try_option!(lexeme!(parser, Parser::parse_u32));
            Some((parser, LeftDeadEnd::new_integer(integer)))
        }
    }

    #[inline(always)]
    fn from_inner_vec(moves: Vec<LeftDeadEndInner>) -> Vec<LeftDeadEnd> {
        let mut md = ManuallyDrop::new(moves);
        let ptr: *mut LeftDeadEndInner = md.as_mut_ptr();
        let len = md.len();
        let capacity = md.capacity();
        // SAFETY: LeftDeadEnd is #[repr(transparent)] so the cast is OK
        unsafe { Vec::from_raw_parts(ptr.cast::<LeftDeadEnd>(), len, capacity) }
    }

    #[inline(always)]
    fn normalize(inner: LeftDeadEndInner) -> LeftDeadEnd {
        match inner {
            LeftDeadEndInner::Integer(_) => LeftDeadEnd { inner },
            LeftDeadEndInner::Moves(moves) => {
                let mut moves = moves
                    .into_iter()
                    .map(|inner| LeftDeadEnd::normalize(inner).inner)
                    .collect::<Vec<_>>();
                moves.sort();
                match moves.as_slice() {
                    [] => LeftDeadEnd {
                        inner: LeftDeadEndInner::Integer(0),
                    },
                    [LeftDeadEndInner::Integer(n)] => LeftDeadEnd {
                        inner: LeftDeadEndInner::Integer(n + 1),
                    },
                    _ => LeftDeadEnd {
                        inner: LeftDeadEndInner::Moves(moves),
                    },
                }
            }
        }
    }

    /// Get novel factors of the position
    pub fn novel_factors(&self) -> Vec<LeftDeadEnd> {
        // Optimization: Factors of an integer are exactly all integers less than or equal to it
        if let Some(integer) = self.to_integer() {
            let mut res = Vec::with_capacity(integer as usize + 2);
            for i in 0..=integer {
                res.push(LeftDeadEnd::new_integer(i));
            }
            return res;
        }

        let mut novel_factors = LeftDeadEndInner::into_inner_vec(self.novel_factors_unordered());
        novel_factors.sort();
        LeftDeadEnd::from_inner_vec(novel_factors)
    }

    pub fn novel_factors_unordered(&self) -> Vec<LeftDeadEnd> {
        // Optimization: Factors of an integer are exactly all integers less than or equal to it
        if let Some(integer) = self.to_integer() {
            let mut factors = Vec::with_capacity(integer as usize + 2);
            for i in 0..=integer {
                factors.push(LeftDeadEnd::new_integer(i));
            }
            return factors;
        }

        if self.is_zero() {
            return Vec::new();
        }

        let mut factors_of_options = Vec::new();

        let own_options = self.clone().into_moves();

        for option in &own_options {
            factors_of_options.push(option.factors());
        }

        let mut novel_factors = Vec::new();

        'outer: for factor_of_first_option in &factors_of_options[0] {
            for factors_of_option in factors_of_options.iter().skip(1) {
                if !factors_of_option.contains(factor_of_first_option) {
                    continue 'outer;
                }
            }
            novel_factors.push(factor_of_first_option);
        }

        let mut new_factors = Vec::new();

        for novel_factor in novel_factors {
            let mut counterparts = Vec::new();

            'outer: for (i, factors_of_option) in factors_of_options.iter().enumerate() {
                for factor_of_option in factors_of_option {
                    if novel_factor + factor_of_option == own_options[i] {
                        counterparts.push(factor_of_option.clone());
                        continue 'outer;
                    }
                }
            }

            let counterpart = LeftDeadEnd::new_moves(counterparts);
            if &(novel_factor + &counterpart) == self {
                if !new_factors.contains(novel_factor) {
                    new_factors.push(novel_factor.clone());
                }

                if !new_factors.contains(&counterpart) {
                    new_factors.push(counterpart);
                }
            }
        }

        new_factors
    }

    pub fn non_novel_factors_unordered(&self) -> Vec<LeftDeadEnd> {
        // Optimization: Factors of an integer are exactly all integers less than or equal to it
        if let Some(integer) = self.to_integer() {
            let mut acc = Vec::with_capacity(integer as usize + 2);
            for i in 0..=integer {
                acc.push(LeftDeadEnd::new_integer(i));
            }
            return acc;
        }

        let mut candidates = vec![];

        for option in self.clone().into_moves() {
            let option_factors = option.factors();
            for option_factor in option_factors {
                if !candidates.contains(&option_factor) {
                    candidates.push(option_factor);
                }
            }
        }

        let mut factors = vec![];

        for i in 0..candidates.len() {
            for j in i..candidates.len() {
                if &candidates[i] + &candidates[j] == *self {
                    factors.push(candidates[i].clone());
                    if i != j {
                        factors.push(candidates[j].clone());
                    }
                }
            }
        }

        if !factors.contains(self) {
            factors.push(self.clone());
        }
        if !factors.contains(&LeftDeadEnd::new_integer(0)) {
            factors.push(LeftDeadEnd::new_integer(0));
        }

        factors
    }

    /// Get factors of the position
    pub fn factors(&self) -> Vec<LeftDeadEnd> {
        // Optimization: Factors of an integer are exactly all integers less than or equal to it
        if let Some(integer) = self.to_integer() {
            let mut acc = Vec::with_capacity(integer as usize + 2);
            for i in 0..=integer {
                acc.push(LeftDeadEnd::new_integer(i));
            }
            return acc;
        }

        let mut factors = self.non_novel_factors_unordered();
        for novel_factor in self.novel_factors_unordered() {
            if !factors.contains(&novel_factor) {
                factors.push(novel_factor);
            }
        }

        let mut factors = LeftDeadEndInner::into_inner_vec(factors);
        factors.sort();
        LeftDeadEnd::from_inner_vec(factors)
    }

    /// Get game's birthday (height of the game tree)
    pub fn birthday(&self) -> u32 {
        if let Some(n) = self.to_integer() {
            return n;
        }

        self.clone()
            .into_moves()
            .iter()
            .map(LeftDeadEnd::birthday)
            .max()
            .unwrap_or(0)
            + 1
    }

    /// Check if the position is atom i.e. has only two factors
    pub fn is_atom(&self) -> bool {
        // Optimization: 1 is the only integer with two factors
        if let Some(integer) = self.to_integer() {
            return integer == 1;
        }

        self.factors().len() == 2
    }

    #[must_use]
    pub fn canonical(&self) -> LeftDeadEnd {
        LeftDeadEnd::new_moves(self.clone().into_moves().into_iter().fold(
            Vec::new(),
            |mut acc, g| {
                if !self.clone().into_moves().into_iter().any(|h| h < g) && !acc.contains(&g) {
                    acc.push(g.canonical());
                }
                acc
            },
        ))
    }

    pub fn next_day(day: Vec<LeftDeadEnd>) -> Vec<LeftDeadEnd> {
        use itertools::Itertools;

        day.into_iter()
            .powerset()
            .fold(Vec::new(), |mut seen, moves| {
                let g = LeftDeadEnd::new_moves(moves);
                if !seen.contains(&g) {
                    seen.push(g);
                }
                seen
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quickcheck::{Arbitrary, QuickCheck};
    use std::{str::FromStr, u32};

    #[test]
    fn to_from_moves() {
        let five = LeftDeadEnd::new_integer(5);
        assert_eq!(five.to_string(), "5");

        let five_moves = five.into_moves();
        assert_eq!(
            LeftDeadEnd::new_moves(five_moves),
            LeftDeadEnd::new_integer(5)
        );
    }

    #[test]
    fn partial_order() {
        assert_eq!(
            LeftDeadEnd::new_integer(0).partial_cmp(&LeftDeadEnd::new_integer(0)),
            Some(Ordering::Equal)
        );

        assert_eq!(
            LeftDeadEnd::new_integer(5).partial_cmp(&LeftDeadEnd::new_integer(5)),
            Some(Ordering::Equal)
        );

        assert_eq!(
            LeftDeadEnd::new_integer(3).partial_cmp(&LeftDeadEnd::new_integer(2)),
            None
        );

        assert_eq!(
            LeftDeadEnd::new_integer(3).partial_cmp(&LeftDeadEnd::new_moves(vec![
                LeftDeadEnd::new_integer(1),
                LeftDeadEnd::new_integer(2)
            ])),
            Some(Ordering::Greater)
        );

        assert_eq!(
            LeftDeadEnd::new_integer(1).partial_cmp(&LeftDeadEnd::new_moves(vec![
                LeftDeadEnd::new_integer(0),
                LeftDeadEnd::new_integer(1)
            ])),
            Some(Ordering::Greater),
        );

        assert_eq!(
            LeftDeadEnd::new_integer(1).partial_cmp(&LeftDeadEnd::new_moves(vec![
                LeftDeadEnd::new_integer(0),
                LeftDeadEnd::new_integer(0),
            ])),
            Some(Ordering::Equal),
        );
    }

    impl LeftDeadEnd {
        fn arbitrary_sized(gen: &mut quickcheck::Gen, size: &mut usize) -> LeftDeadEnd {
            let is_integer = (u32::arbitrary(gen) % 10) < 4;
            if *size == 0 || is_integer {
                *size = size.saturating_sub(1);
                LeftDeadEnd::new_integer(u32::arbitrary(gen) % 5)
            } else {
                let num_options = (usize::arbitrary(gen) % *size) % 4;
                *size /= num_options + 1;
                let mut options = Vec::with_capacity(num_options);
                for _ in 0..num_options {
                    options.push(LeftDeadEnd::arbitrary_sized(gen, size));
                }
                LeftDeadEnd::new_moves(options)
            }
        }
    }

    impl Arbitrary for LeftDeadEnd {
        fn arbitrary(gen: &mut quickcheck::Gen) -> LeftDeadEnd {
            let mut size = gen.size();
            LeftDeadEnd::arbitrary_sized(gen, &mut size)
        }

        fn shrink(&self) -> Box<dyn Iterator<Item = Self>> {
            match &self.inner {
                LeftDeadEndInner::Integer(0) => quickcheck::empty_shrinker(),
                LeftDeadEndInner::Integer(n) => {
                    Box::new(std::iter::once(LeftDeadEnd::new_integer(n - 1)))
                }
                LeftDeadEndInner::Moves(moves) => Box::new(
                    LeftDeadEnd::from_inner_vec(moves.clone())
                        .shrink()
                        .map(LeftDeadEnd::new_moves),
                ),
            }
        }
    }

    #[test]
    fn parsing_preserves_equality() {
        let mut qc = QuickCheck::new();
        let test = |g: LeftDeadEnd| {
            assert_eq!(LeftDeadEnd::from_str(&g.to_string()).unwrap(), g);
        };
        qc.quickcheck(test as fn(LeftDeadEnd));
    }

    #[test]
    fn is_atom_iff_has_two_factors() {
        let mut qc = QuickCheck::new();
        let test = |g: LeftDeadEnd| {
            let factors = g.factors();
            let is_atomic = g.is_atom();

            assert_eq!(is_atomic, factors.len() == 2);
        };
        qc.quickcheck(test as fn(LeftDeadEnd));
    }

    #[test]
    fn born_by_day() {
        let day0 = vec![LeftDeadEnd::new_integer(0)];

        let day1 = LeftDeadEnd::next_day(day0);
        assert_eq!(
            day1.iter().map(|g| g.to_string()).collect::<Vec<String>>(),
            vec!["0", "1"],
        );

        let day2 = LeftDeadEnd::next_day(day1);
        assert_eq!(
            day2.iter().map(|g| g.to_string()).collect::<Vec<String>>(),
            vec!["0", "1", "2", "{0, 1}"],
        );

        let day3 = LeftDeadEnd::next_day(day2);
        assert_eq!(day3.len(), 10);

        let day4 = LeftDeadEnd::next_day(day3);
        assert_eq!(day4.len(), 52);
    }

    #[test]
    fn addition() {
        assert_eq!(
            (LeftDeadEnd::new_integer(1) + LeftDeadEnd::new_integer(1)).to_string(),
            "{1, 1}"
        );

        assert_eq!(
            (LeftDeadEnd::new_integer(2) + LeftDeadEnd::new_integer(1)).to_string(),
            "{2, {1, 1}}"
        );

        assert_eq!(
            (LeftDeadEnd::new_integer(2) + LeftDeadEnd::new_integer(3)).to_string(),
            "{{3, {2, {1, 1}}}, {{2, {1, 1}}, {2, {1, 1}}}}"
        );
    }

    #[test]
    fn factors() {
        let three = LeftDeadEnd::new_integer(3);
        assert_eq!(
            three
                .factors()
                .iter()
                .map(|g| g.to_string())
                .collect::<Vec<String>>(),
            vec!["0", "1", "2", "3"],
        );
    }

    #[test]
    fn parsing() {
        assert_eq!(
            LeftDeadEnd::from_str("42"),
            Ok(LeftDeadEnd::new_integer(42))
        );

        assert_eq!(
            LeftDeadEnd::from_str("{41}"),
            Ok(LeftDeadEnd::new_integer(42))
        );

        assert_eq!(
            LeftDeadEnd::from_str("{{{}}}"),
            Ok(LeftDeadEnd::new_integer(2))
        );

        assert_eq!(
            LeftDeadEnd::from_str("{1,{2,3}}"),
            Ok(LeftDeadEnd::new_moves(vec![
                LeftDeadEnd::new_integer(1),
                LeftDeadEnd::new_moves(vec![
                    LeftDeadEnd::new_integer(2),
                    LeftDeadEnd::new_integer(3)
                ])
            ]))
        );

        assert_eq!(
            LeftDeadEnd::from_str(" { 1 , { 2 , 3 } } "),
            Ok(LeftDeadEnd::new_moves(vec![
                LeftDeadEnd::new_integer(1),
                LeftDeadEnd::new_moves(vec![
                    LeftDeadEnd::new_integer(2),
                    LeftDeadEnd::new_integer(3)
                ])
            ]))
        );
    }
}
