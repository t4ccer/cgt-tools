//! Left Dead Ends with interned storage

use append_only_vec::AppendOnlyVec;
use dashmap::DashMap;
use itertools::Itertools;
use std::{cmp::Ordering, io, iter::FusedIterator};

/// Interned Left Dead End
///
/// Note that `Eq` and `Ord` traits operate on the internal state and does not behave like
/// game comparison. They are implemented for the sake of data structures. Use
/// [`Interner::partial_cmp`] instead to get game comparison.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LeftDeadEnd {
    /// Index into storage vector when non-negative, an integer offset by one if negative
    idx: i64,
}

impl LeftDeadEnd {
    /// Construct new Left Dead End equal to an integer.
    ///
    /// Integers are not interned so it can be constructed without [`Interner`] and use everywhere
    pub const fn new_integer(integer: u32) -> LeftDeadEnd {
        LeftDeadEnd {
            idx: -(integer as i64) - 1,
        }
    }

    /// Get integer value if the game is an integer
    pub const fn to_integer(self) -> Option<u32> {
        if self.idx < 0 {
            Some((-self.idx - 1) as u32)
        } else {
            None
        }
    }

    const fn is_zero(self) -> bool {
        self.idx == -1
    }
}

/// Left Dead End Interner
///
/// Interner acts as a storage of Left Dead Ends. It stores only games that are not integers
#[derive(Debug)]
pub struct Interner {
    /// Storage of game moves
    games: AppendOnlyVec<Box<[LeftDeadEnd]>>,

    /// Mapping from game moves to its index in `games` vector
    table: DashMap<Box<[LeftDeadEnd]>, usize, ahash::RandomState>,

    /// Cache for [`Interner::factors`]
    factors: DashMap<LeftDeadEnd, Box<[(LeftDeadEnd, LeftDeadEnd)]>, ahash::RandomState>,
}

impl Interner {
    /// Construct new interner with empty storage
    pub fn new() -> Interner {
        Interner {
            games: AppendOnlyVec::new(),
            factors: DashMap::default(),
            table: DashMap::default(),
        }
    }

    /// Intern new game from avaliable moves
    pub fn new_moves(&self, mut moves: Box<[LeftDeadEnd]>) -> LeftDeadEnd {
        if moves.is_empty() {
            LeftDeadEnd::new_integer(0)
        } else if moves.len() == 1 && moves[0].to_integer().is_some() {
            LeftDeadEnd::new_integer(moves[0].to_integer().unwrap() + 1)
        } else {
            moves.sort();

            let idx = self
                .table
                .entry(moves.clone())
                .or_insert_with(|| self.games.push(moves));
            LeftDeadEnd { idx: *idx as i64 }
        }
    }

    pub fn new_from_string(&self, input: &str) -> Option<LeftDeadEnd> {
        use crate::parsing::Parser;

        let p = Parser::new(input);
        let (_, parsed) = self.parse(p)?;
        Some(parsed)
    }

    /// Get iterator over game's moves
    pub fn into_moves(&self, game: LeftDeadEnd) -> impl Iterator<Item = LeftDeadEnd> + use<'_> {
        if game.is_zero() {
            MovesIter {
                game: None,
                i: 0,
                moves: &[],
            }
        } else if game.to_integer().is_some() {
            MovesIter {
                game: Some(game),
                i: 0,
                moves: &[],
            }
        } else {
            MovesIter {
                game: Some(game),
                i: 0,
                moves: &self.games[game.idx as usize],
            }
        }
    }

    /// Intern new game from the sum of two other games
    pub fn new_sum(&self, lhs: LeftDeadEnd, rhs: LeftDeadEnd) -> LeftDeadEnd {
        if lhs.is_zero() {
            return rhs;
        }
        if rhs.is_zero() {
            return lhs;
        }

        if let Some(lhs) = lhs.to_integer() {
            if let Some(rhs) = rhs.to_integer() {
                // Optimization: 1 + n = {n, {n - 1, {n - 2, {n - ..., {1, 1}}}}}
                if lhs == 1 || rhs == 1 {
                    let mut acc = self.new_moves(
                        vec![LeftDeadEnd::new_integer(1), LeftDeadEnd::new_integer(1)]
                            .into_boxed_slice(),
                    );

                    for i in 2..=(lhs.max(rhs)) {
                        acc = self
                            .new_moves(vec![LeftDeadEnd::new_integer(i), acc].into_boxed_slice());
                    }

                    return acc;
                }
            }
        }

        let mut sum_options =
            Vec::with_capacity(self.into_moves(lhs).count() + self.into_moves(rhs).count());

        for g in self.into_moves(lhs) {
            sum_options.push(self.new_sum(g, rhs));
        }
        for h in self.into_moves(rhs) {
            sum_options.push(self.new_sum(lhs, h));
        }

        self.new_moves(sum_options.into_boxed_slice())
    }

    /// Write game representation
    #[allow(clippy::missing_errors_doc)]
    pub fn display(&self, game: LeftDeadEnd, f: &mut impl io::Write) -> io::Result<()> {
        if let Some(integer) = game.to_integer() {
            write!(f, "{}", integer)?;
        } else {
            write!(f, "{{")?;
            for (idx, option) in self.into_moves(game).enumerate() {
                if idx != 0 {
                    write!(f, ", ")?;
                }
                self.display(option, f)?;
            }
            write!(f, "}}")?;
        }

        Ok(())
    }

    /// Write game representation to new string
    pub fn to_string(&self, game: LeftDeadEnd) -> String {
        let mut buf = Vec::new();
        self.display(game, &mut buf).unwrap();
        String::from_utf8(buf).unwrap()
    }

    /// Check if iterator contains an entry equal (game equaltiy) to `game`
    pub fn contains(
        &self,
        mut games: impl Iterator<Item = LeftDeadEnd>,
        game: LeftDeadEnd,
    ) -> bool {
        games.any(|g| self.eq(g, game))
    }

    /// Check if game is an atom (has only two factors)
    pub fn is_atom(&self, game: LeftDeadEnd) -> bool {
        // Optimization: 1 is the only integer with two factors
        if let Some(integer) = game.to_integer() {
            return integer == 1;
        }

        self.factors(game).len() == 2
    }

    /// Get game's birthday (height of the game tree)
    pub fn birthday(&self, game: LeftDeadEnd) -> u32 {
        if let Some(n) = game.to_integer() {
            return n;
        }

        self.into_moves(game)
            .map(|option| self.birthday(option))
            .max()
            .unwrap_or(0)
            + 1
    }

    pub fn flexibility(&self, game: LeftDeadEnd) -> u32 {
        if game.to_integer().is_some() {
            0
        } else {
            self.into_moves(game)
                .map(|g| self.flexibility(g))
                .max()
                .map_or(0, |f| f + 1)
        }
    }

    pub fn race(&self, game: LeftDeadEnd) -> u32 {
        if game.is_zero() {
            0
        } else {
            self.into_moves(game)
                .map(|g| self.race(g))
                .min()
                .map_or(0, |f| f + 1)
        }
    }

    pub fn racing(&self, game: LeftDeadEnd) -> impl Iterator<Item = LeftDeadEnd> + use<'_> {
        let b = self.race(game);
        self.into_moves(game)
            .filter(move |g| self.race(*g) + 1 == b)
    }

    pub fn stalling(&self, game: LeftDeadEnd) -> impl Iterator<Item = LeftDeadEnd> + use<'_> {
        let b = self.birthday(game);
        self.into_moves(game)
            .filter(move |g| self.birthday(*g) + 1 == b)
    }

    /// Compute game factors
    pub fn factors(&self, game: LeftDeadEnd) -> Vec<(LeftDeadEnd, LeftDeadEnd)> {
        if let Some(cached) = self.factors.get(&game) {
            return Vec::from(cached.as_ref());
        }

        let mut factors = self.non_novel_factors_unordered(game);
        for (novel_factor, f) in self.novel_factors_unordered(game) {
            if !self.contains(factors.iter().map(|(g, _)| *g), novel_factor) {
                factors.push((novel_factor, f));
            }
        }

        factors.sort();

        self.factors
            .insert(game, factors.clone().into_boxed_slice());

        factors
    }

    fn novel_factors_unordered(&self, game: LeftDeadEnd) -> Vec<(LeftDeadEnd, LeftDeadEnd)> {
        if game.is_zero() {
            return Vec::new();
        }

        let own_options = self.into_moves(game).collect::<Vec<_>>();
        let mut factors_of_options = Vec::with_capacity(own_options.len());
        for option in own_options.iter().copied() {
            factors_of_options.push(self.factors(option));
        }

        let mut novel_factors = Vec::new();

        'outer: for (factor_of_first_option, _) in &factors_of_options[0] {
            for factors_of_option in &factors_of_options[1..] {
                if !self.contains(
                    factors_of_option.iter().map(|(g, _)| *g),
                    *factor_of_first_option,
                ) {
                    continue 'outer;
                }
            }
            novel_factors.push(*factor_of_first_option);
        }

        let mut new_factors = Vec::new();

        for novel_factor in novel_factors {
            let mut counterparts = Vec::new();

            'outer: for (i, factors_of_option) in factors_of_options.iter().enumerate() {
                for (factor_of_option, _) in factors_of_option {
                    let sum = self.new_sum(novel_factor, *factor_of_option);
                    if self.eq(sum, own_options[i]) {
                        counterparts.push(*factor_of_option);
                        continue 'outer;
                    }
                }
            }

            let counterpart = self.new_moves(counterparts.into_boxed_slice());
            let sum = self.new_sum(novel_factor, counterpart);
            if self.eq(sum, game) {
                if !self.contains(new_factors.iter().map(|(g, _)| *g), novel_factor) {
                    new_factors.push((novel_factor, counterpart));
                }

                if !self.contains(new_factors.iter().map(|(g, _)| *g), counterpart) {
                    new_factors.push((counterpart, novel_factor));
                }
            }
        }

        new_factors
    }

    fn non_novel_factors_unordered(&self, game: LeftDeadEnd) -> Vec<(LeftDeadEnd, LeftDeadEnd)> {
        let mut candidates = vec![];

        for option in self.into_moves(game) {
            let option_factors = self.factors(option);
            for (option_factor, _) in option_factors {
                if !self.contains(candidates.iter().copied(), option_factor) {
                    candidates.push(option_factor);
                }
            }
        }

        let mut factors = vec![];

        for i in 0..candidates.len() {
            for j in i..candidates.len() {
                let sum = self.new_sum(candidates[i], candidates[j]);
                if self.eq(sum, game) {
                    factors.push((candidates[i], candidates[j]));
                    if i != j {
                        factors.push((candidates[j], candidates[i]));
                    }
                }
            }
        }

        if !self.contains(factors.iter().map(|(g, _)| *g), game) {
            factors.push((game, LeftDeadEnd::new_integer(0)));
        }
        if !self.contains(factors.iter().map(|(g, _)| *g), LeftDeadEnd::new_integer(0)) {
            factors.push((LeftDeadEnd::new_integer(0), game));
        }

        factors
    }

    /// Construct iterator over the games born up to next birthday
    ///
    /// Note that this computes powerset of the input iterator and is quite slow past day 4
    pub fn next_day<I>(&self, day: I) -> impl Iterator<Item = LeftDeadEnd> + use<'_, I>
    where
        I: Iterator<Item = LeftDeadEnd>,
    {
        let mut seen = Vec::new();
        day.powerset().filter_map(move |moves| {
            let g = self.new_moves(moves.into_boxed_slice());
            if self.contains(seen.iter().copied(), g) {
                None
            } else {
                seen.push(g);
                Some(g)
            }
        })
    }

    /// Perform game comparision
    pub fn partial_cmp(&self, lhs: LeftDeadEnd, rhs: LeftDeadEnd) -> Option<Ordering> {
        if lhs == rhs {
            return Some(Ordering::Equal);
        }

        // Optimization: If integers are not equal then they are incomparable
        if let Some(lhs) = lhs.to_integer() {
            if let Some(rhs) = rhs.to_integer() {
                if lhs == rhs {
                    return Some(Ordering::Equal);
                }

                return None;
            }
        }

        match (self.le(rhs, lhs), self.le(lhs, rhs)) {
            (true, true) => Some(Ordering::Equal),
            (true, false) => Some(Ordering::Greater),
            (false, true) => Some(Ordering::Less),
            (false, false) => None,
        }
    }

    /// Check if game is greater than or equal to other game
    ///
    /// This is more efficient than [`Interner::partial_cmp`]
    pub fn ge(&self, lhs: LeftDeadEnd, rhs: LeftDeadEnd) -> bool {
        if lhs == rhs {
            return true;
        }

        if let Some(lhs) = lhs.to_integer() {
            if lhs == 0 {
                return rhs.is_zero();
            }

            // Optimization: If integers are not equal then they are incomparable
            if let Some(rhs) = rhs.to_integer() {
                return lhs == rhs;
            }
        }

        self.into_moves(lhs).all(|left_option| {
            self.into_moves(rhs)
                .any(|right_option| self.ge(left_option, right_option))
        })
    }

    /// Check if game is less than or equal to other game
    ///
    /// This is more efficient than [`Interner::partial_cmp`]
    pub fn le(&self, lhs: LeftDeadEnd, rhs: LeftDeadEnd) -> bool {
        self.ge(rhs, lhs)
    }

    /// Check if game is equal to other game
    ///
    /// This is as efficient as [`Interner::partial_cmp`]
    pub fn eq(&self, lhs: LeftDeadEnd, rhs: LeftDeadEnd) -> bool {
        self.partial_cmp(lhs, rhs) == Some(Ordering::Equal)
    }

    /// Get count of interned games
    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> usize {
        self.games.len()
    }

    /// Game without dominated options
    pub fn canonical(&self, game: LeftDeadEnd) -> LeftDeadEnd {
        let moves = self.into_moves(game).fold(Vec::new(), |mut acc, g| {
            if !self
                .into_moves(game)
                .any(|h| matches!(self.partial_cmp(h, g), Some(Ordering::Less)))
                && !self.contains(acc.iter().copied(), g)
            {
                acc.push(self.canonical(g));
            }
            acc
        });
        self.new_moves(moves.into_boxed_slice())
    }

    pub fn parse<'p>(
        &self,
        parser: crate::parsing::Parser<'p>,
    ) -> Option<(crate::parsing::Parser<'p>, LeftDeadEnd)> {
        use crate::parsing::{Parser, lexeme, try_option};

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
            Some((parser, self.new_moves(options.into_boxed_slice())))
        } else {
            let (parser, integer) = try_option!(lexeme!(parser, Parser::parse_u32));
            Some((parser, LeftDeadEnd::new_integer(integer)))
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct MovesIter<'a> {
    game: Option<LeftDeadEnd>,
    i: usize,
    moves: &'a [LeftDeadEnd],
}

impl Iterator for MovesIter<'_> {
    type Item = LeftDeadEnd;

    fn next(&mut self) -> Option<Self::Item> {
        let game = self.game?;
        if let Some(integer) = game.to_integer() {
            self.game = None;
            Some(LeftDeadEnd::new_integer(integer - 1))
        } else {
            let g = self.moves.get(self.i).copied()?;
            self.i += 1;
            Some(g)
        }
    }

    fn count(self) -> usize {
        match self.game {
            None => 0,
            Some(g) if g.to_integer().is_some() => 1,
            Some(_) => self.moves[self.i..].len(),
        }
    }
}

impl FusedIterator for MovesIter<'_> {}

#[test]
fn to_integer() {
    let interner = Interner::new();

    let zero = interner.new_moves(vec![].into_boxed_slice());
    assert_eq!(zero.to_integer(), Some(0));

    let three = interner.new_moves(vec![LeftDeadEnd::new_integer(2)].into_boxed_slice());
    assert_eq!(three.to_integer(), Some(3));
}

#[test]
fn identical() {
    let interner = Interner::new();
    let g = interner.new_moves(
        vec![LeftDeadEnd::new_integer(2), LeftDeadEnd::new_integer(3)].into_boxed_slice(),
    );
    let h = interner.new_moves(
        vec![LeftDeadEnd::new_integer(3), LeftDeadEnd::new_integer(2)].into_boxed_slice(),
    );

    // == for identical
    assert_eq!(g, h);
}

#[test]
fn partial_order() {
    let interner = Interner::new();

    assert_eq!(
        interner.partial_cmp(LeftDeadEnd::new_integer(0), LeftDeadEnd::new_integer(0)),
        Some(Ordering::Equal)
    );

    assert_eq!(
        interner.partial_cmp(LeftDeadEnd::new_integer(5), LeftDeadEnd::new_integer(5)),
        Some(Ordering::Equal)
    );

    assert_eq!(
        interner.partial_cmp(LeftDeadEnd::new_integer(3), LeftDeadEnd::new_integer(2)),
        None
    );

    let g = interner.new_moves(
        vec![LeftDeadEnd::new_integer(1), LeftDeadEnd::new_integer(2)].into_boxed_slice(),
    );
    assert_eq!(
        interner.partial_cmp(LeftDeadEnd::new_integer(3), g),
        Some(Ordering::Greater)
    );

    let g = interner.new_moves(
        vec![LeftDeadEnd::new_integer(0), LeftDeadEnd::new_integer(1)].into_boxed_slice(),
    );
    assert_eq!(
        interner.partial_cmp(LeftDeadEnd::new_integer(1), g),
        Some(Ordering::Greater),
    );

    let g = interner.new_moves(
        vec![LeftDeadEnd::new_integer(0), LeftDeadEnd::new_integer(0)].into_boxed_slice(),
    );
    assert_eq!(
        interner.partial_cmp(LeftDeadEnd::new_integer(1), g),
        Some(Ordering::Equal),
    );

    let g = interner.new_sum(LeftDeadEnd::new_integer(2), LeftDeadEnd::new_integer(2));
    assert_eq!(interner.partial_cmp(LeftDeadEnd::new_integer(3), g), None);
}

#[test]
fn parsing() {
    let input = "{{{0, {1, 0}, {2, 0}, {2, 1}}, {{0, {1, 0}}, {{1, 0}, {{{1, 0}}}, {2, 0}, {{2, 0}}}, {{{0, {1, 0}}}}}, {{0, {1, 0}, {2, 0}, {2, 1}}, {{1, 0}, {{{1, 0}}}, {2, 0}, {{2, 0}}}, {{{1, 0}}, {{{1, 0}}}}, {{0, {1, 0}, {2, 0}, {2, 1}}}}, {{{0, {1, 0}}}, {{{0, {1, 0}}}}}}, {{0, {1, 0}, {2, 0}, {2, 1}}, {{0, {2, 0}, {2, 1}}, {{1, 0}, {{{1, 0}}}, {2, 0}, {{2, 0}}}, {{{1, 0}}, {{{1, 0}}}}, {{0, {2, 0}, {2, 1}}}}, {{0, {1, 0}, {2, 0}, {2, 1}}, {{1, 0}, {{{1, 0}}}, {2, 0}, {{2, 0}}}, {{2, 0}, {{{2, 0}}}}, {{{0, {1, 0}, {2, 0}, {2, 1}}}}}, {{{0, {1, 0}, {2, 0}, {2, 1}}}, {{{0, {1, 0}, {2, 0}, {2, 1}}}}}}, {{{0, {1, 0}, {2, 0}, {2, 1}}, {{0, {2, 0}, {2, 1}}, {{1, 0}, {{{1, 0}}}, {2, 0}, {{2, 0}}}, {{{1, 0}}, {{{1, 0}}}}, {{0, {2, 0}, {2, 1}}}}, {{0, {1, 0}, {2, 0}, {2, 1}}, {{1, 0}, {{{1, 0}}}, {2, 0}, {{2, 0}}}, {{2, 0}, {{{2, 0}}}}, {{{0, {1, 0}, {2, 0}, {2, 1}}}}}, {{{0, {1, 0}, {2, 0}, {2, 1}}}, {{{0, {1, 0}, {2, 0}, {2, 1}}}}}}, {{{0, {1, 0}, {2, 0}, {2, 1}}, {{0, {2, 0}, {2, 1}}, {{1, 0}, {{{1, 0}}}, {2, 0}, {{2, 0}}}, {{{1, 0}}, {{{1, 0}}}}, {{0, {2, 0}, {2, 1}}}}, {{0, {1, 0}, {2, 0}, {2, 1}}, {{1, 0}, {{{1, 0}}}, {2, 0}, {{2, 0}}}, {{2, 0}, {{{2, 0}}}}, {{{0, {1, 0}, {2, 0}, {2, 1}}}}}, {{{0, {1, 0}, {2, 0}, {2, 1}}}, {{{0, {1, 0}, {2, 0}, {2, 1}}}}}}}, {{{0, {1, 0}, {2, 0}, {2, 1}}, {{1, 0}, {{{1, 0}}}, {2, 0}, {{2, 0}}}, {{{1, 0}}, {{{1, 0}}}}, {{0, {1, 0}, {2, 0}, {2, 1}}}}, {{0, {1, 0}, {2, 0}, {2, 1}}, {{1, 0}, {{{1, 0}}}, {2, 0}, {{2, 0}}}, {{2, 0}, {{{2, 0}}}}, {{{0, {1, 0}, {2, 0}, {2, 1}}}}}, {{{0, {1, 0}, {2, 0}, {2, 1}}, {{1, 0}, {{{1, 0}}}, {2, 0}, {{2, 0}}}, {{2, 0}, {{{2, 0}}}}, {{{0, {1, 0}, {2, 0}, {2, 1}}}}}}, {{{1, 0}, {{{1, 0}}}, {2, 0}, {{2, 0}}}, {{2, 0}, {{{2, 0}}}}, {{{{1, 0}, {{{1, 0}}}, {2, 0}, {{2, 0}}}}}, {{{2, 0}, {{{2, 0}}}}}}, {{{{0, {1, 0}, {2, 0}, {2, 1}}, {{1, 0}, {{{1, 0}}}, {2, 0}, {{2, 0}}}, {{{1, 0}}, {{{1, 0}}}}, {{0, {1, 0}, {2, 0}, {2, 1}}}}}}}, {{{{0, {1, 0}, {2, 0}, {2, 1}}, {{1, 0}, {{{1, 0}}}, {2, 0}, {{2, 0}}}, {{{1, 0}}, {{{1, 0}}}}, {{0, {1, 0}, {2, 0}, {2, 1}}}}}, {{{{0, {1, 0}, {2, 0}, {2, 1}}, {{1, 0}, {{{1, 0}}}, {2, 0}, {{2, 0}}}, {{{1, 0}}, {{{1, 0}}}}, {{0, {1, 0}, {2, 0}, {2, 1}}}}}}}}, {{{0, {1, 0}, {2, 0}, {2, 1}}, {{0, {1, 0}}, {{1, 0}, {{{1, 0}}}, {2, 0}, {{2, 0}}}, {{{0, {1, 0}}}}}, {{0, {1, 0}, {2, 0}, {2, 1}}, {{1, 0}, {{{1, 0}}}, {2, 0}, {{2, 0}}}, {{{1, 0}}, {{{1, 0}}}}, {{0, {1, 0}, {2, 0}, {2, 1}}}}, {{{0, {1, 0}}}, {{{0, {1, 0}}}}}}, {{{0, {1, 0}, {2, 0}, {2, 1}}, {{1, 0}, {{{1, 0}}}, {2, 0}, {{2, 0}}}, {{{1, 0}}, {{{1, 0}}}}, {{0, {1, 0}, {2, 0}, {2, 1}}}}, {{0, {1, 0}, {2, 0}, {2, 1}}, {{1, 0}, {{{1, 0}}}, {2, 0}, {{2, 0}}}, {{2, 0}, {{{2, 0}}}}, {{{0, {1, 0}, {2, 0}, {2, 1}}}}}, {{{0, {1, 0}, {2, 0}, {2, 1}}, {{1, 0}, {{{1, 0}}}, {2, 0}, {{2, 0}}}, {{2, 0}, {{{2, 0}}}}, {{{0, {1, 0}, {2, 0}, {2, 1}}}}}}, {{{1, 0}, {{{1, 0}}}, {2, 0}, {{2, 0}}}, {{2, 0}, {{{2, 0}}}}, {{{{1, 0}, {{{1, 0}}}, {2, 0}, {{2, 0}}}}}, {{{2, 0}, {{{2, 0}}}}}}, {{{{0, {1, 0}, {2, 0}, {2, 1}}, {{1, 0}, {{{1, 0}}}, {2, 0}, {{2, 0}}}, {{{1, 0}}, {{{1, 0}}}}, {{0, {1, 0}, {2, 0}, {2, 1}}}}}}}, {{{0, {1, 0}}, {{1, 0}, {{{1, 0}}}, {2, 0}, {{2, 0}}}, {{{0, {1, 0}}}}}, {{{1, 0}, {{{1, 0}}}, {2, 0}, {{2, 0}}}, {{2, 0}, {{{2, 0}}}}, {{{{1, 0}, {{{1, 0}}}, {2, 0}, {{2, 0}}}}}, {{{2, 0}, {{{2, 0}}}}}}, {{{{0, {1, 0}}, {{1, 0}, {{{1, 0}}}, {2, 0}, {{2, 0}}}, {{{0, {1, 0}}}}}}}}, {{{{0, {1, 0}, {2, 0}, {2, 1}}, {{0, {1, 0}}, {{1, 0}, {{{1, 0}}}, {2, 0}, {{2, 0}}}, {{{0, {1, 0}}}}}, {{0, {1, 0}, {2, 0}, {2, 1}}, {{1, 0}, {{{1, 0}}}, {2, 0}, {{2, 0}}}, {{{1, 0}}, {{{1, 0}}}}, {{0, {1, 0}, {2, 0}, {2, 1}}}}, {{{0, {1, 0}}}, {{{0, {1, 0}}}}}}}}}, {{{{0, {1, 0}, {2, 0}, {2, 1}}, {{0, {1, 0}}, {{1, 0}, {{{1, 0}}}, {2, 0}, {{2, 0}}}, {{{0, {1, 0}}}}}, {{0, {1, 0}, {2, 0}, {2, 1}}, {{1, 0}, {{{1, 0}}}, {2, 0}, {{2, 0}}}, {{{1, 0}}, {{{1, 0}}}}, {{0, {1, 0}, {2, 0}, {2, 1}}}}, {{{0, {1, 0}}}, {{{0, {1, 0}}}}}}}, {{{{0, {1, 0}, {2, 0}, {2, 1}}, {{0, {1, 0}}, {{1, 0}, {{{1, 0}}}, {2, 0}, {{2, 0}}}, {{{0, {1, 0}}}}}, {{0, {1, 0}, {2, 0}, {2, 1}}, {{1, 0}, {{{1, 0}}}, {2, 0}, {{2, 0}}}, {{{1, 0}}, {{{1, 0}}}}, {{0, {1, 0}, {2, 0}, {2, 1}}}}, {{{0, {1, 0}}}, {{{0, {1, 0}}}}}}}}}}";

    let interner = Interner::new();
    let g = interner.new_from_string(input).unwrap();
    assert_eq!(interner.birthday(g), 11);
}

#[test]
fn factors() {
    let interner = Interner::new();
    let g = interner.new_from_string("{{{{1, 0}, {{{1, 0}}}, {2, 0}, {{2, 0}}}}, {{{1, 0}}, {{{1, 0}}}}, {{{{1, 0}, {{{1, 0}}}, {2, 0}, {{2, 0}}}}}, {{{{{1, 0}}, {{{1, 0}}}}}}}").unwrap();

    let mut expected_factors = vec![
        (
            interner.new_from_string("0").unwrap(),
            interner.new_from_string("{{{{1, 0}, {{{1, 0}}}, {2, 0}, {{2, 0}}}}, {{{1, 0}}, {{{1, 0}}}}, {{{{1, 0}, {{{1, 0}}}, {2, 0}, {{2, 0}}}}}, {{{{{1, 0}}, {{{1, 0}}}}}}}").unwrap(),
        ),
        (
            interner.new_from_string("{1, 0}").unwrap(),
            interner.new_from_string("{{{2, 0}}, {2, 1}, {{{2, 0}}}, {{{2, 1}}}}").unwrap()
        ),
        (
            interner.new_from_string("{2, 0}").unwrap(
            ),
            interner.new_from_string("{{{1, 0}}, {{{1, 0}}}}").unwrap()
        ),
        (
            interner.new_from_string("{{1, 0}, {{{1, 0}}}, {2, 0}, {{2, 0}}}").unwrap(),
            interner.new_from_string("{2, 1}").unwrap()
        ),
        (
            interner.new_from_string("{{{1, 0}}, {{{1, 0}}}}").unwrap(),
            interner.new_from_string("{2, 0}").unwrap()
        ),
        (
            interner.new_from_string("{{{{1, 0}, {{{1, 0}}}, {2, 0}, {{2, 0}}}}, {{{1, 0}}, {{{1, 0}}}}, {{{{1, 0}, {{{1, 0}}}, {2, 0}, {{2, 0}}}}}, {{{{{1, 0}}, {{{1, 0}}}}}}}").unwrap(),
            interner.new_from_string("0").unwrap()
        ),
        (
            interner.new_from_string("{2, 1}").unwrap(),
            interner.new_from_string("{{1, 0}, {{{1, 0}}}, {2, 0}, {{2, 0}}}").unwrap()
        ),
        (
            interner.new_from_string("{{{2, 0}}, {2, 1}, {{{2, 0}}}, {{{2, 1}}}}").unwrap(),
            interner.new_from_string("{1, 0}").unwrap()
        ),
    ];
    expected_factors.sort();

    assert_eq!(interner.factors(g), expected_factors);
}
