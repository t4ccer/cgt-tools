//! Thread safe transposition table

use crate::{
    short::partizan::{canonical_form::CanonicalForm, transposition_table::TranspositionTable},
    total::TotalWrapper,
};
use append_only_vec::AppendOnlyVec;
use dashmap::DashMap;
use std::{fmt::Debug, hash::Hash};

/// Transaction table (cache) of game positions and canonical forms.
pub struct ParallelTranspositionTable<G> {
    values: AppendOnlyVec<CanonicalForm>,
    positions: DashMap<G, usize, crate::hash::RandomState>,
    known_values: DashMap<TotalWrapper<CanonicalForm>, usize, crate::hash::RandomState>,
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

impl<G> Debug for ParallelTranspositionTable<G>
where
    G: Debug + Hash + Eq,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let ParallelTranspositionTable {
            values,
            positions,
            known_values,
        } = self;

        f.debug_struct("ParallelTranspositionTable")
            .field("values", values)
            .field("positions", positions)
            .field("known_values", known_values)
            .finish()
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
    #[allow(clippy::missing_panics_doc)]
    #[inline]
    fn lookup_position(&self, position: &G) -> Option<CanonicalForm> {
        self.positions
            .get(position)
            .map(|id| self.values[*id].clone())
    }

    #[allow(clippy::missing_panics_doc)]
    #[inline]
    fn insert_position(&self, position: G, value: CanonicalForm) {
        if let Some(known) = self.known_values.get(TotalWrapper::from_ref(&value)) {
            self.positions.insert(position, *known);
        } else {
            let inserted = self.values.push(value.clone());
            self.known_values.insert(TotalWrapper::new(value), inserted);
            self.positions.insert(position, inserted);
        }
    }
}
