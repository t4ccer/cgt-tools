//! Left Dead Ends with interned storage

use crate::{
    misere::left_dead_end::LeftDeadEndContext,
    total::{TotalWrapper, impl_total_wrapper},
};
use append_only_vec::AppendOnlyVec;
use dashmap::DashMap;
use std::{cmp::Ordering, iter::FusedIterator};

impl_total_wrapper! {
    /// Left dead end is a game where every follower is a left end (there is no move for Left)
    #[derive(Debug, Clone, Copy)]
    LeftDeadEnd => idx => i64
}

impl LeftDeadEnd {
    pub const fn new_integer(integer: u32) -> LeftDeadEnd {
        LeftDeadEnd {
            idx: -(integer as i64) - 1,
        }
    }

    pub const fn to_integer(&self) -> Option<u32> {
        if self.idx < 0 {
            Some((-self.idx - 1) as u32)
        } else {
            None
        }
    }
}

/// Left Dead End Interner
///
/// Interner acts as a storage of Left Dead Ends. It stores only games that are not integers
#[derive(Debug)]
pub struct Interner {
    /// Storage of game moves
    games: AppendOnlyVec<Box<[TotalWrapper<LeftDeadEnd>]>>,

    /// Mapping from game moves to its index in `games` vector
    table: DashMap<Box<[TotalWrapper<LeftDeadEnd>]>, usize, ahash::RandomState>,
}

impl LeftDeadEndContext for Interner {
    type LeftDeadEnd = LeftDeadEnd;

    fn new_integer(&self, integer: u32) -> Self::LeftDeadEnd {
        LeftDeadEnd::new_integer(integer)
    }

    fn new_moves(&self, moves: Vec<Self::LeftDeadEnd>) -> Self::LeftDeadEnd {
        if moves.is_empty() {
            self.new_integer(0)
        } else if moves.len() == 1 && self.to_integer(&moves[0]).is_some() {
            self.new_integer(self.to_integer(&moves[0]).unwrap() + 1)
        } else {
            let mut moves = TotalWrapper::from_inner_vec(moves);
            moves.sort();
            let moves = moves.into_boxed_slice();

            let idx = self
                .table
                .entry(moves.clone())
                .or_insert_with(|| self.games.push(moves));
            LeftDeadEnd { idx: *idx as i64 }
        }
    }

    fn moves(&self, game: &Self::LeftDeadEnd) -> impl ExactSizeIterator<Item = Self::LeftDeadEnd> {
        if self.is_zero(game) {
            MovesIter {
                game: None,
                moves: [].iter(),
            }
        } else if self.to_integer(game).is_some() {
            MovesIter {
                game: Some(*game),
                moves: [].iter(),
            }
        } else {
            MovesIter {
                game: Some(*game),
                moves: self.games[game.idx as usize].iter(),
            }
        }
    }

    fn to_integer(&self, g: &Self::LeftDeadEnd) -> Option<u32> {
        g.to_integer()
    }

    fn is_zero(&self, g: &Self::LeftDeadEnd) -> bool {
        g.idx == -1
    }

    fn total_cmp(&self, lhs: &Self::LeftDeadEnd, rhs: &Self::LeftDeadEnd) -> Ordering {
        lhs.idx.cmp(&rhs.idx)
    }
}

impl Interner {
    /// Construct new interner with empty storage
    pub fn new() -> Interner {
        Interner {
            games: AppendOnlyVec::new(),
            table: DashMap::default(),
        }
    }

    /// Get count of interned games
    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> usize {
        self.games.len()
    }
}

#[derive(Debug, Clone)]
struct MovesIter<'a> {
    game: Option<LeftDeadEnd>,
    moves: core::slice::Iter<'a, TotalWrapper<LeftDeadEnd>>,
}

impl Iterator for MovesIter<'_> {
    type Item = LeftDeadEnd;

    fn next(&mut self) -> Option<Self::Item> {
        let game = self.game?;
        if let Some(integer) = game.to_integer() {
            self.game = None;
            Some(LeftDeadEnd::new_integer(integer - 1))
        } else {
            self.moves.next().copied().map(TotalWrapper::get)
        }
    }

    fn count(self) -> usize {
        self.len()
    }
}

impl FusedIterator for MovesIter<'_> {}

impl ExactSizeIterator for MovesIter<'_> {
    fn len(&self) -> usize {
        match self.game {
            None => 0,
            Some(g) if g.to_integer().is_some() => 1,
            Some(_) => self.moves.len(),
        }
    }
}

#[test]
fn to_integer() {
    let interner = Interner::new();

    let zero = interner.new_moves(vec![]);
    assert_eq!(zero.to_integer(), Some(0));

    let three = interner.new_moves(vec![LeftDeadEnd::new_integer(2)]);
    assert_eq!(three.to_integer(), Some(3));
}

#[test]
fn identical() {
    let interner = Interner::new();
    let g = interner.new_moves(vec![
        LeftDeadEnd::new_integer(2),
        LeftDeadEnd::new_integer(3),
    ]);
    let h = interner.new_moves(vec![
        LeftDeadEnd::new_integer(3),
        LeftDeadEnd::new_integer(2),
    ]);

    // == for identical
    assert_eq!(g.idx, h.idx);
}

#[test]
fn partial_order() {
    let interner = Interner::new();

    assert_eq!(
        interner.game_cmp(&LeftDeadEnd::new_integer(0), &LeftDeadEnd::new_integer(0)),
        Some(Ordering::Equal)
    );

    assert_eq!(
        interner.game_cmp(&LeftDeadEnd::new_integer(5), &LeftDeadEnd::new_integer(5)),
        Some(Ordering::Equal)
    );

    assert_eq!(
        interner.game_cmp(&LeftDeadEnd::new_integer(3), &LeftDeadEnd::new_integer(2)),
        None
    );

    let g = interner.new_moves(vec![
        LeftDeadEnd::new_integer(1),
        LeftDeadEnd::new_integer(2),
    ]);
    assert_eq!(
        interner.game_cmp(&LeftDeadEnd::new_integer(3), &g),
        Some(Ordering::Greater)
    );

    let g = interner.new_moves(vec![
        LeftDeadEnd::new_integer(0),
        LeftDeadEnd::new_integer(1),
    ]);
    assert_eq!(
        interner.game_cmp(&LeftDeadEnd::new_integer(1), &g),
        Some(Ordering::Greater),
    );

    let g = interner.new_moves(vec![
        LeftDeadEnd::new_integer(0),
        LeftDeadEnd::new_integer(0),
    ]);
    assert_eq!(
        interner.game_cmp(&LeftDeadEnd::new_integer(1), &g),
        Some(Ordering::Equal),
    );

    let g = interner.new_sum(&LeftDeadEnd::new_integer(2), &LeftDeadEnd::new_integer(2));
    assert_eq!(interner.game_cmp(&LeftDeadEnd::new_integer(3), &g), None);
}

#[test]
fn parsing() {
    let input = "{{{0, {1, 0}, {2, 0}, {2, 1}}, {{0, {1, 0}}, {{1, 0}, {{{1, 0}}}, {2, 0}, {{2, 0}}}, {{{0, {1, 0}}}}}, {{0, {1, 0}, {2, 0}, {2, 1}}, {{1, 0}, {{{1, 0}}}, {2, 0}, {{2, 0}}}, {{{1, 0}}, {{{1, 0}}}}, {{0, {1, 0}, {2, 0}, {2, 1}}}}, {{{0, {1, 0}}}, {{{0, {1, 0}}}}}}, {{0, {1, 0}, {2, 0}, {2, 1}}, {{0, {2, 0}, {2, 1}}, {{1, 0}, {{{1, 0}}}, {2, 0}, {{2, 0}}}, {{{1, 0}}, {{{1, 0}}}}, {{0, {2, 0}, {2, 1}}}}, {{0, {1, 0}, {2, 0}, {2, 1}}, {{1, 0}, {{{1, 0}}}, {2, 0}, {{2, 0}}}, {{2, 0}, {{{2, 0}}}}, {{{0, {1, 0}, {2, 0}, {2, 1}}}}}, {{{0, {1, 0}, {2, 0}, {2, 1}}}, {{{0, {1, 0}, {2, 0}, {2, 1}}}}}}, {{{0, {1, 0}, {2, 0}, {2, 1}}, {{0, {2, 0}, {2, 1}}, {{1, 0}, {{{1, 0}}}, {2, 0}, {{2, 0}}}, {{{1, 0}}, {{{1, 0}}}}, {{0, {2, 0}, {2, 1}}}}, {{0, {1, 0}, {2, 0}, {2, 1}}, {{1, 0}, {{{1, 0}}}, {2, 0}, {{2, 0}}}, {{2, 0}, {{{2, 0}}}}, {{{0, {1, 0}, {2, 0}, {2, 1}}}}}, {{{0, {1, 0}, {2, 0}, {2, 1}}}, {{{0, {1, 0}, {2, 0}, {2, 1}}}}}}, {{{0, {1, 0}, {2, 0}, {2, 1}}, {{0, {2, 0}, {2, 1}}, {{1, 0}, {{{1, 0}}}, {2, 0}, {{2, 0}}}, {{{1, 0}}, {{{1, 0}}}}, {{0, {2, 0}, {2, 1}}}}, {{0, {1, 0}, {2, 0}, {2, 1}}, {{1, 0}, {{{1, 0}}}, {2, 0}, {{2, 0}}}, {{2, 0}, {{{2, 0}}}}, {{{0, {1, 0}, {2, 0}, {2, 1}}}}}, {{{0, {1, 0}, {2, 0}, {2, 1}}}, {{{0, {1, 0}, {2, 0}, {2, 1}}}}}}}, {{{0, {1, 0}, {2, 0}, {2, 1}}, {{1, 0}, {{{1, 0}}}, {2, 0}, {{2, 0}}}, {{{1, 0}}, {{{1, 0}}}}, {{0, {1, 0}, {2, 0}, {2, 1}}}}, {{0, {1, 0}, {2, 0}, {2, 1}}, {{1, 0}, {{{1, 0}}}, {2, 0}, {{2, 0}}}, {{2, 0}, {{{2, 0}}}}, {{{0, {1, 0}, {2, 0}, {2, 1}}}}}, {{{0, {1, 0}, {2, 0}, {2, 1}}, {{1, 0}, {{{1, 0}}}, {2, 0}, {{2, 0}}}, {{2, 0}, {{{2, 0}}}}, {{{0, {1, 0}, {2, 0}, {2, 1}}}}}}, {{{1, 0}, {{{1, 0}}}, {2, 0}, {{2, 0}}}, {{2, 0}, {{{2, 0}}}}, {{{{1, 0}, {{{1, 0}}}, {2, 0}, {{2, 0}}}}}, {{{2, 0}, {{{2, 0}}}}}}, {{{{0, {1, 0}, {2, 0}, {2, 1}}, {{1, 0}, {{{1, 0}}}, {2, 0}, {{2, 0}}}, {{{1, 0}}, {{{1, 0}}}}, {{0, {1, 0}, {2, 0}, {2, 1}}}}}}}, {{{{0, {1, 0}, {2, 0}, {2, 1}}, {{1, 0}, {{{1, 0}}}, {2, 0}, {{2, 0}}}, {{{1, 0}}, {{{1, 0}}}}, {{0, {1, 0}, {2, 0}, {2, 1}}}}}, {{{{0, {1, 0}, {2, 0}, {2, 1}}, {{1, 0}, {{{1, 0}}}, {2, 0}, {{2, 0}}}, {{{1, 0}}, {{{1, 0}}}}, {{0, {1, 0}, {2, 0}, {2, 1}}}}}}}}, {{{0, {1, 0}, {2, 0}, {2, 1}}, {{0, {1, 0}}, {{1, 0}, {{{1, 0}}}, {2, 0}, {{2, 0}}}, {{{0, {1, 0}}}}}, {{0, {1, 0}, {2, 0}, {2, 1}}, {{1, 0}, {{{1, 0}}}, {2, 0}, {{2, 0}}}, {{{1, 0}}, {{{1, 0}}}}, {{0, {1, 0}, {2, 0}, {2, 1}}}}, {{{0, {1, 0}}}, {{{0, {1, 0}}}}}}, {{{0, {1, 0}, {2, 0}, {2, 1}}, {{1, 0}, {{{1, 0}}}, {2, 0}, {{2, 0}}}, {{{1, 0}}, {{{1, 0}}}}, {{0, {1, 0}, {2, 0}, {2, 1}}}}, {{0, {1, 0}, {2, 0}, {2, 1}}, {{1, 0}, {{{1, 0}}}, {2, 0}, {{2, 0}}}, {{2, 0}, {{{2, 0}}}}, {{{0, {1, 0}, {2, 0}, {2, 1}}}}}, {{{0, {1, 0}, {2, 0}, {2, 1}}, {{1, 0}, {{{1, 0}}}, {2, 0}, {{2, 0}}}, {{2, 0}, {{{2, 0}}}}, {{{0, {1, 0}, {2, 0}, {2, 1}}}}}}, {{{1, 0}, {{{1, 0}}}, {2, 0}, {{2, 0}}}, {{2, 0}, {{{2, 0}}}}, {{{{1, 0}, {{{1, 0}}}, {2, 0}, {{2, 0}}}}}, {{{2, 0}, {{{2, 0}}}}}}, {{{{0, {1, 0}, {2, 0}, {2, 1}}, {{1, 0}, {{{1, 0}}}, {2, 0}, {{2, 0}}}, {{{1, 0}}, {{{1, 0}}}}, {{0, {1, 0}, {2, 0}, {2, 1}}}}}}}, {{{0, {1, 0}}, {{1, 0}, {{{1, 0}}}, {2, 0}, {{2, 0}}}, {{{0, {1, 0}}}}}, {{{1, 0}, {{{1, 0}}}, {2, 0}, {{2, 0}}}, {{2, 0}, {{{2, 0}}}}, {{{{1, 0}, {{{1, 0}}}, {2, 0}, {{2, 0}}}}}, {{{2, 0}, {{{2, 0}}}}}}, {{{{0, {1, 0}}, {{1, 0}, {{{1, 0}}}, {2, 0}, {{2, 0}}}, {{{0, {1, 0}}}}}}}}, {{{{0, {1, 0}, {2, 0}, {2, 1}}, {{0, {1, 0}}, {{1, 0}, {{{1, 0}}}, {2, 0}, {{2, 0}}}, {{{0, {1, 0}}}}}, {{0, {1, 0}, {2, 0}, {2, 1}}, {{1, 0}, {{{1, 0}}}, {2, 0}, {{2, 0}}}, {{{1, 0}}, {{{1, 0}}}}, {{0, {1, 0}, {2, 0}, {2, 1}}}}, {{{0, {1, 0}}}, {{{0, {1, 0}}}}}}}}}, {{{{0, {1, 0}, {2, 0}, {2, 1}}, {{0, {1, 0}}, {{1, 0}, {{{1, 0}}}, {2, 0}, {{2, 0}}}, {{{0, {1, 0}}}}}, {{0, {1, 0}, {2, 0}, {2, 1}}, {{1, 0}, {{{1, 0}}}, {2, 0}, {{2, 0}}}, {{{1, 0}}, {{{1, 0}}}}, {{0, {1, 0}, {2, 0}, {2, 1}}}}, {{{0, {1, 0}}}, {{{0, {1, 0}}}}}}}, {{{{0, {1, 0}, {2, 0}, {2, 1}}, {{0, {1, 0}}, {{1, 0}, {{{1, 0}}}, {2, 0}, {{2, 0}}}, {{{0, {1, 0}}}}}, {{0, {1, 0}, {2, 0}, {2, 1}}, {{1, 0}, {{{1, 0}}}, {2, 0}, {{2, 0}}}, {{{1, 0}}, {{{1, 0}}}}, {{0, {1, 0}, {2, 0}, {2, 1}}}}, {{{0, {1, 0}}}, {{{0, {1, 0}}}}}}}}}}";

    let interner = Interner::new();
    let g = interner.new_from_string(input).unwrap();
    assert_eq!(interner.birthday(&g), 11);
}

#[test]
fn factors() {
    let interner = Interner::new();
    let g = interner.new_from_string("{{{{1, 0}, {{{1, 0}}}, {2, 0}, {{2, 0}}}}, {{{1, 0}}, {{{1, 0}}}}, {{{{1, 0}, {{{1, 0}}}, {2, 0}, {{2, 0}}}}}, {{{{{1, 0}}, {{{1, 0}}}}}}}").unwrap();

    let expected_factors = vec![
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

    for (actual, expected) in interner.factors(&g).iter().zip(expected_factors.iter()) {
        assert_eq!(actual.0.idx, expected.0.idx);
        assert_eq!(actual.1.idx, expected.1.idx);
    }
}
