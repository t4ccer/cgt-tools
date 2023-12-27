//! Thread safe transposition table for game values

use crate::short::partizan::canonical_form::CanonicalForm;
use append_only_vec::AppendOnlyVec;
use dashmap::DashMap;
use std::{hash::Hash, marker::PhantomData};

/// Interface of a transposition table
pub trait TranspositionTable<G> {
    /// Lookup a position value if exists
    fn lookup_position(&self, position: &G) -> Option<CanonicalForm>;

    /// Save position and its game value
    fn insert_position(&self, position: G, value: CanonicalForm);
}

/// Transaction table (cache) of game positions and canonical forms.
pub struct ParallelTranspositionTable<G> {
    values: AppendOnlyVec<CanonicalForm>,
    positions: DashMap<G, usize, ahash::RandomState>,
    known_values: DashMap<CanonicalForm, usize, ahash::RandomState>,
}

impl<G> ParallelTranspositionTable<G>
where
    G: Eq + Hash,
{
    /// Create new empty transposition table.
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// Get number of saved positions
    #[inline]
    pub fn len(&self) -> usize {
        self.positions.len()
    }

    /// Check if table stores any position
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.positions.is_empty()
    }
}

impl<G> Default for ParallelTranspositionTable<G>
where
    G: Hash + Eq,
{
    #[inline]
    fn default() -> Self {
        Self {
            values: AppendOnlyVec::new(),
            positions: DashMap::default(),
            known_values: DashMap::default(),
        }
    }
}

impl<G> TranspositionTable<G> for ParallelTranspositionTable<G>
where
    G: Eq + Hash,
{
    #[cfg_attr(feature = "cargo-clippy", allow(clippy::missing_panics_doc))]
    #[inline]
    fn lookup_position(&self, position: &G) -> Option<CanonicalForm> {
        self.positions
            .get(position)
            .map(|id| self.values[*id].clone())
    }

    #[cfg_attr(feature = "cargo-clippy", allow(clippy::missing_panics_doc))]
    #[inline]
    fn insert_position(&self, position: G, value: CanonicalForm) {
        if let Some(known) = self.known_values.get(&value) {
            self.positions.insert(position, *known);
        } else {
            let inserted = self.values.push(value.clone());
            self.known_values.insert(value, inserted);
            self.positions.insert(position, inserted);
        }
    }
}

/// Dummy transposition table that does not store anythning
pub struct NoTranspositionTable<G>(PhantomData<G>);

impl<G> NoTranspositionTable<G> {
    #[inline]
    /// Create new dummy transposition table
    pub const fn new() -> Self {
        Self(PhantomData)
    }
}

impl<G> Default for NoTranspositionTable<G> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<G> TranspositionTable<G> for NoTranspositionTable<G> {
    #[inline]
    fn lookup_position(&self, _position: &G) -> Option<CanonicalForm> {
        None
    }

    #[inline]
    fn insert_position(&self, _position: G, _value: CanonicalForm) {}
}
