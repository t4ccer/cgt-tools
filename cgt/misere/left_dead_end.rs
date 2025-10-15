#![allow(missing_docs)]

//! Left dead end is a game where every follower is a left end (there is no move for Left)
//!
//! This code is heavily based on <https://github.com/alfiemd/gemau> by Alfie Davies

use crate::{
    parsing::{Parser, impl_from_str_via_parser, lexeme, try_option},
    total::impl_total_wrapper,
};
use auto_ops::impl_op_ex;
use std::{
    cmp::Ordering,
    fmt::{self, Display},
    hash::Hash,
    mem::ManuallyDrop,
};

pub mod interned;

pub trait EmptyContext: 'static {
    const EMPTY: &'static Self;
}

pub trait LeftDeadEndContext {
    type LeftDeadEnd: Clone;

    fn new_integer(&self, integer: u32) -> Self::LeftDeadEnd;

    fn new_moves(&self, g: Vec<Self::LeftDeadEnd>) -> Self::LeftDeadEnd;

    fn moves(&self, g: &Self::LeftDeadEnd) -> impl ExactSizeIterator<Item = Self::LeftDeadEnd>;

    fn to_integer(&self, g: &Self::LeftDeadEnd) -> Option<u32>;

    fn total_cmp(&self, lhs: &Self::LeftDeadEnd, rhs: &Self::LeftDeadEnd) -> Ordering;

    fn is_zero(&self, g: &Self::LeftDeadEnd) -> bool {
        matches!(self.to_integer(g), Some(0))
    }

    fn birthday(&self, g: &Self::LeftDeadEnd) -> u32 {
        self.to_integer(g)
            .unwrap_or_else(|| self.moves(g).map(|g| self.birthday(&g)).max().unwrap_or(0) + 1)
    }

    fn game_ge(&self, lhs: &Self::LeftDeadEnd, rhs: &Self::LeftDeadEnd) -> bool {
        if let Some(lhs) = self.to_integer(lhs) {
            if lhs == 0 {
                return self.is_zero(rhs);
            }

            // Optimization: If integers are not equal then they are incomparable
            if let Some(rhs) = self.to_integer(rhs) {
                return lhs == rhs;
            }
        }

        self.moves(lhs).all(|left_option| {
            self.moves(rhs)
                .any(|right_option| self.game_ge(&left_option, &right_option))
        })
    }

    fn game_cmp(&self, lhs: &Self::LeftDeadEnd, rhs: &Self::LeftDeadEnd) -> Option<Ordering> {
        // Optimization: If integers are not equal then they are incomparable
        if let Some(lhs) = self.to_integer(lhs) {
            if let Some(rhs) = self.to_integer(rhs) {
                if lhs == rhs {
                    return Some(Ordering::Equal);
                }

                return None;
            }
        }

        match (self.game_ge(lhs, rhs), self.game_ge(rhs, lhs)) {
            (true, true) => Some(Ordering::Equal),
            (true, false) => Some(Ordering::Greater),
            (false, true) => Some(Ordering::Less),
            (false, false) => None,
        }
    }

    fn game_eq(&self, lhs: &Self::LeftDeadEnd, rhs: &Self::LeftDeadEnd) -> bool {
        matches!(self.game_cmp(lhs, rhs), Some(Ordering::Equal))
    }

    fn new_sum(&self, lhs: &Self::LeftDeadEnd, rhs: &Self::LeftDeadEnd) -> Self::LeftDeadEnd {
        // Optimization: 0 + g = g
        if self.is_zero(lhs) {
            return rhs.clone();
        }

        if self.is_zero(rhs) {
            return lhs.clone();
        }

        let lhs_options = self.moves(lhs);
        let rhs_options = self.moves(rhs);
        let mut sum_options = Vec::with_capacity(lhs_options.len() + rhs_options.len());

        for g in lhs_options {
            sum_options.push(self.new_sum(&g, rhs));
        }
        for h in rhs_options {
            sum_options.push(self.new_sum(lhs, &h));
        }

        self.new_moves(sum_options)
    }

    fn novel_factors_unordered(
        &self,
        g: &Self::LeftDeadEnd,
    ) -> Vec<(Self::LeftDeadEnd, Self::LeftDeadEnd)> {
        if self.is_zero(g) {
            return Vec::new();
        }

        let mut factors_of_options = Vec::new();

        let own_options = self.moves(g).collect::<Vec<_>>();

        for option in &own_options {
            factors_of_options.push(self.factors(option));
        }

        let mut novel_factors = Vec::new();

        'outer: for (factor_of_first_option, _) in &factors_of_options[0] {
            for factors_of_option in factors_of_options.iter().skip(1) {
                if !factors_of_option
                    .iter()
                    .any(|(g, _)| self.game_eq(g, factor_of_first_option))
                {
                    continue 'outer;
                }
            }
            novel_factors.push(factor_of_first_option);
        }

        let mut new_factors: Vec<(Self::LeftDeadEnd, Self::LeftDeadEnd)> = Vec::new();

        for novel_factor in novel_factors {
            let mut counterparts = Vec::new();

            'outer: for (i, factors_of_option) in factors_of_options.iter().enumerate() {
                for (factor_of_option, _) in factors_of_option {
                    if self.game_eq(
                        &self.new_sum(novel_factor, factor_of_option),
                        &own_options[i],
                    ) {
                        counterparts.push(factor_of_option.clone());
                        continue 'outer;
                    }
                }
            }

            let counterpart = self.new_moves(counterparts);
            if self.game_eq(&(self.new_sum(novel_factor, &counterpart)), g) {
                if !new_factors
                    .iter()
                    .any(|(g, _)| self.game_eq(g, novel_factor))
                {
                    new_factors.push((novel_factor.clone(), counterpart.clone()));
                }

                if !new_factors
                    .iter()
                    .any(|(g, _)| self.game_eq(g, &counterpart))
                {
                    new_factors.push((counterpart, novel_factor.clone()));
                }
            }
        }

        new_factors
    }

    fn non_novel_factors_unordered(
        &self,
        game: &Self::LeftDeadEnd,
    ) -> Vec<(Self::LeftDeadEnd, Self::LeftDeadEnd)> {
        let mut candidates = vec![];

        for option in self.moves(game) {
            let option_factors = self.factors(&option);
            for (option_factor, _) in option_factors {
                if !candidates.iter().any(|h| self.game_eq(h, &option_factor)) {
                    candidates.push(option_factor);
                }
            }
        }

        let mut factors = vec![];

        for i in 0..candidates.len() {
            for j in i..candidates.len() {
                if self.game_eq(&self.new_sum(&candidates[i], &candidates[j]), game) {
                    factors.push((candidates[i].clone(), candidates[j].clone()));
                    if i != j {
                        factors.push((candidates[j].clone(), candidates[i].clone()));
                    }
                }
            }
        }

        if !factors.iter().any(|(h, _)| self.game_eq(h, game)) {
            factors.push((game.clone(), self.new_integer(0)));
        }
        if !factors
            .iter()
            .any(|(h, _)| self.game_eq(h, &self.new_integer(0)))
        {
            factors.push((self.new_integer(0), game.clone()));
        }

        factors
    }

    /// Get factors of the position
    fn factors(&self, g: &Self::LeftDeadEnd) -> Vec<(Self::LeftDeadEnd, Self::LeftDeadEnd)> {
        let mut factors = self.non_novel_factors_unordered(g);
        for (novel_factor, f) in self.novel_factors_unordered(g) {
            if !factors.iter().any(|(g, _)| self.game_eq(g, &novel_factor)) {
                factors.push((novel_factor, f));
            }
        }

        factors.sort_by(|(lhs_lhs, lhs_rhs), (rhs_lhs, rhs_rhs)| {
            self.total_cmp(lhs_lhs, rhs_lhs)
                .then_with(|| self.total_cmp(lhs_rhs, rhs_rhs))
        });
        factors
    }

    /// Check if the position is atom i.e. has only two factors
    fn is_atom(&self, g: &Self::LeftDeadEnd) -> bool {
        // Optimization: 1 is the only integer with two factors
        if let Some(integer) = self.to_integer(g) {
            return integer == 1;
        }

        self.factors(g).len() == 2
    }

    #[must_use]
    fn canonical(&self, g: &Self::LeftDeadEnd) -> Self::LeftDeadEnd {
        self.new_moves(self.moves(g).fold(Vec::new(), |mut acc, g| {
            if !self
                .moves(&g)
                .any(|h| matches!(self.game_cmp(&h, &g), Some(Ordering::Less)))
                && !acc.iter().any(|h| self.game_eq(h, &g))
            {
                acc.push(self.canonical(&g));
            }
            acc
        }))
    }

    fn next_day(
        &self,
        day: impl IntoIterator<Item = Self::LeftDeadEnd>,
    ) -> impl Iterator<Item = Self::LeftDeadEnd> {
        use itertools::Itertools;

        let mut seen = Vec::new();
        day.into_iter()
            .powerset()
            .map(|moves| self.new_moves(moves))
            .filter(move |g| {
                if !seen.iter().any(|h| self.game_eq(h, &g)) {
                    seen.push(g.clone());
                    true
                } else {
                    false
                }
            })
    }

    fn parse<'p>(&self, parser: Parser<'p>) -> Option<(Parser<'p>, Self::LeftDeadEnd)> {
        let parser = parser.trim_whitespace();
        if let Some(parser) = parser.parse_ascii_char('{') {
            let parser = parser.trim_whitespace();

            let mut options = Vec::new();
            let mut loop_parser = parser;
            while let Some((parser, option)) = lexeme!(loop_parser, |p| self.parse(p)) {
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
            Some((parser, self.new_moves(options)))
        } else {
            let (parser, integer) = try_option!(lexeme!(parser, Parser::parse_u32));
            Some((parser, self.new_integer(integer)))
        }
    }

    fn new_from_string(&self, input: &str) -> Option<Self::LeftDeadEnd> {
        use crate::parsing::Parser;

        let p = Parser::new(input);
        let (_, parsed) = self.parse(p)?;
        Some(parsed)
    }

    /// Write game representation
    #[allow(clippy::missing_errors_doc)]
    fn display(&self, game: &Self::LeftDeadEnd, f: &mut impl fmt::Write) -> fmt::Result {
        if let Some(integer) = self.to_integer(game) {
            write!(f, "{}", integer)?;
        } else {
            write!(f, "{{")?;
            for (idx, option) in self.moves(game).enumerate() {
                if idx != 0 {
                    write!(f, ", ")?;
                }
                self.display(&option, f)?;
            }
            write!(f, "}}")?;
        }

        Ok(())
    }

    /// Write game representation to new string
    fn to_string(&self, game: &Self::LeftDeadEnd) -> String {
        let mut buf = String::new();
        self.display(game, &mut buf).unwrap();
        buf
    }

    fn flexibility(&self, game: &Self::LeftDeadEnd) -> u32 {
        if self.to_integer(game).is_some() {
            0
        } else {
            self.moves(game)
                .map(|g| self.flexibility(&g))
                .max()
                .map_or(0, |f| f + 1)
        }
    }

    fn race(&self, game: &Self::LeftDeadEnd) -> u32 {
        if self.is_zero(game) {
            0
        } else {
            self.moves(game)
                .map(|g| self.race(&g))
                .min()
                .map_or(0, |f| f + 1)
        }
    }
}

pub trait IsLeftDeadEnd<Context>: Sized
where
    Context: EmptyContext + LeftDeadEndContext<LeftDeadEnd = Self>,
{
    fn new_integer(integer: u32) -> Self {
        Context::EMPTY.new_integer(integer)
    }

    fn new_moves(moves: Vec<Self>) -> Self {
        Context::EMPTY.new_moves(moves)
    }

    fn moves(&self) -> impl ExactSizeIterator<Item = Self> {
        Context::EMPTY.moves(self)
    }

    fn to_integer(&self) -> Option<u32> {
        Context::EMPTY.to_integer(self)
    }

    fn is_zero(&self) -> bool {
        Context::EMPTY.is_zero(self)
    }

    fn birthday(&self) -> u32 {
        Context::EMPTY.birthday(self)
    }

    fn flexibility(&self) -> u32 {
        Context::EMPTY.flexibility(self)
    }

    fn race(&self) -> u32 {
        Context::EMPTY.race(self)
    }

    fn factors(&self) -> Vec<(Self, Self)> {
        Context::EMPTY.factors(self)
    }

    fn is_atom(&self) -> bool {
        Context::EMPTY.is_atom(self)
    }

    #[must_use]
    fn canonical(&self) -> Self {
        Context::EMPTY.canonical(self)
    }

    fn next_day(day: impl IntoIterator<Item = Self>) -> impl Iterator<Item = Self> {
        Context::EMPTY.next_day(day)
    }

    fn parse(parser: Parser<'_>) -> Option<(Parser<'_>, Self)> {
        Context::EMPTY.parse(parser)
    }
}

struct NoContext;

impl LeftDeadEndContext for NoContext {
    type LeftDeadEnd = LeftDeadEnd;

    fn new_integer(&self, integer: u32) -> Self::LeftDeadEnd {
        LeftDeadEnd {
            inner: LeftDeadEndInner::Integer(integer),
        }
    }

    fn new_moves(&self, g: Vec<Self::LeftDeadEnd>) -> Self::LeftDeadEnd {
        let moves = LeftDeadEndInner::into_inner_vec(g);
        LeftDeadEnd::normalize(LeftDeadEndInner::Moves(moves))
    }

    fn moves(&self, g: &Self::LeftDeadEnd) -> impl ExactSizeIterator<Item = Self::LeftDeadEnd> {
        match g.clone().inner {
            LeftDeadEndInner::Integer(0) => vec![].into_iter(),
            LeftDeadEndInner::Integer(n) => vec![LeftDeadEnd {
                inner: LeftDeadEndInner::Integer(n - 1),
            }]
            .into_iter(),
            LeftDeadEndInner::Moves(moves) => LeftDeadEnd::from_inner_vec(moves).into_iter(),
        }
    }

    fn to_integer(&self, g: &Self::LeftDeadEnd) -> Option<u32> {
        match g.inner {
            LeftDeadEndInner::Integer(n) => Some(n),
            LeftDeadEndInner::Moves(_) => None,
        }
    }

    fn total_cmp(&self, lhs: &Self::LeftDeadEnd, rhs: &Self::LeftDeadEnd) -> Ordering {
        lhs.inner.cmp(&rhs.inner)
    }
}

impl EmptyContext for NoContext {
    const EMPTY: &'static Self = &NoContext;
}

impl IsLeftDeadEnd<NoContext> for LeftDeadEnd {}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum LeftDeadEndInner {
    Integer(u32),
    Moves(Vec<LeftDeadEndInner>),
}

impl Display for LeftDeadEndInner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        NoContext::EMPTY.display(
            &LeftDeadEnd {
                inner: self.clone(),
            },
            f,
        )
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

impl_total_wrapper! {
    /// Left dead end is a game where every follower is a left end (there is no move for Left)
    #[derive(Debug, Clone)]
    struct LeftDeadEnd {
        inner: LeftDeadEndInner
    }
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
        NoContext::EMPTY.game_cmp(self, rhs)
    }

    fn ge(&self, rhs: &LeftDeadEnd) -> bool {
        NoContext::EMPTY.game_ge(self, rhs)
    }

    fn le(&self, rhs: &LeftDeadEnd) -> bool {
        NoContext::EMPTY.game_ge(rhs, self)
    }
}

impl_op_ex!(+|lhs: &LeftDeadEnd, rhs: &LeftDeadEnd| -> LeftDeadEnd {
    NoContext::EMPTY.new_sum(lhs, rhs)
});

impl_from_str_via_parser!(LeftDeadEnd);

impl LeftDeadEnd {
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use quickcheck::{Arbitrary, QuickCheck};
    use std::str::FromStr;

    #[test]
    fn to_from_moves() {
        let five = LeftDeadEnd::new_integer(5);
        assert_eq!(five.to_string(), "5");

        let five_moves = five.moves();
        assert_eq!(
            LeftDeadEnd::new_moves(five_moves.collect()),
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
        fn arbitrary_sized(generator: &mut quickcheck::Gen, size: &mut usize) -> LeftDeadEnd {
            let is_integer = (u32::arbitrary(generator) % 10) < 4;
            if *size == 0 || is_integer {
                *size = size.saturating_sub(1);
                LeftDeadEnd::new_integer(u32::arbitrary(generator) % 5)
            } else {
                let num_options = (usize::arbitrary(generator) % *size) % 4;
                *size /= num_options + 1;
                let mut options = Vec::with_capacity(num_options);
                for _ in 0..num_options {
                    options.push(LeftDeadEnd::arbitrary_sized(generator, size));
                }
                LeftDeadEnd::new_moves(options)
            }
        }
    }

    impl Arbitrary for LeftDeadEnd {
        fn arbitrary(generator: &mut quickcheck::Gen) -> LeftDeadEnd {
            let mut size = generator.size();
            LeftDeadEnd::arbitrary_sized(generator, &mut size)
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

        let day1 = LeftDeadEnd::next_day(day0).collect::<Vec<_>>();
        assert_eq!(
            day1.iter()
                .map(std::string::ToString::to_string)
                .collect::<Vec<String>>(),
            vec!["0", "1"],
        );

        let day2 = LeftDeadEnd::next_day(day1).collect::<Vec<_>>();
        assert_eq!(
            day2.iter()
                .map(std::string::ToString::to_string)
                .collect::<Vec<String>>(),
            vec!["0", "1", "2", "{0, 1}"],
        );

        let day3 = LeftDeadEnd::next_day(day2.clone());
        assert_eq!(day3.count(), 10);

        let day3 = LeftDeadEnd::next_day(day2);
        let day4 = LeftDeadEnd::next_day(day3);
        assert_eq!(day4.count(), 52);
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
                .map(|(g, _)| g.to_string())
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
