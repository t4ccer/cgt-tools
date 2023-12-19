//! Thread safe transposition table for game values

use crate::short::partizan::canonical_form::CanonicalForm;
use append_only_vec::AppendOnlyVec;
use dashmap::DashMap;
use std::hash::Hash;

/// Transaction table (cache) of game positions and canonical forms.
pub struct TranspositionTable<G> {
    values: AppendOnlyVec<CanonicalForm>,
    positions: DashMap<G, usize, ahash::RandomState>,
    known_values: DashMap<CanonicalForm, usize, ahash::RandomState>,
}

impl<G> TranspositionTable<G>
where
    G: Eq + Hash,
{
    /// Create new empty transposition table.
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// Lookup a position
    #[cfg_attr(feature = "cargo-clippy", allow(clippy::missing_panics_doc))]
    #[inline]
    pub fn lookup_position(&self, position: &G) -> Option<CanonicalForm> {
        self.positions
            .get(position)
            .map(|id| self.values[*id].clone())
    }

    /// Save position and its game value
    #[cfg_attr(feature = "cargo-clippy", allow(clippy::missing_panics_doc))]
    #[inline]
    pub fn insert_position(&self, position: G, value: CanonicalForm) {
        if let Some(known) = self.known_values.get(&value) {
            self.positions.insert(position, *known);
        } else {
            let inserted = self.values.push(value.clone());
            self.known_values.insert(value, inserted);
            self.positions.insert(position, inserted);
        }
    }

    /// Get number of saved positions
    #[inline]
    pub fn len(&self) -> usize {
        self.positions.len()
    }
}

impl<G> Default for TranspositionTable<G>
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
