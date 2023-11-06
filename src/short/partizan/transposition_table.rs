//! Thread safe transposition table for game values

use crate::{rw_hash_map::RwHashMap, short::partizan::canonical_form::CanonicalForm};
use std::{hash::Hash, sync::RwLock};

/// Transaction table (cache) of game positions and canonical forms.
pub struct TranspositionTable<G> {
    values: RwLock<Vec<CanonicalForm>>,
    positions: RwHashMap<G, usize, ahash::RandomState>,
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
    #[inline]
    pub fn lookup_position(&self, position: &G) -> Option<CanonicalForm> {
        self.positions
            .get(position)
            .and_then(|id| self.values.read().unwrap().get(id).cloned())
    }

    /// Save position and its game value
    #[cfg_attr(feature = "cargo-clippy", allow(clippy::missing_panics_doc))]
    #[inline]
    pub fn insert_position(&self, position: G, value: CanonicalForm) {
        let mut arena = self.values.write().unwrap();
        let inserted = arena.len();
        arena.push(value);
        drop(arena);
        self.positions.insert(position, inserted);
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
            values: RwLock::default(),
            positions: RwHashMap::default(),
        }
    }
}
