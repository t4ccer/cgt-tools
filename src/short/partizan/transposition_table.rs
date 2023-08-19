//! Thread safe transposition table for game values

// TODO: Move to short positional game module
use crate::{rw_hash_map::RwHashMap, short::partizan::canonical_form::CanonicalForm};
use std::hash::Hash;

/// Transaction table (cache) of game positions and canonical forms.
pub struct TranspositionTable<G> {
    grids: RwHashMap<G, CanonicalForm>,
}

#[cfg_attr(feature = "cargo-clippy", allow(clippy::new_without_default))]
impl<G> TranspositionTable<G>
where
    G: Eq + Hash + Clone + Sync + Send,
{
    /// Create new empty transposition table.
    #[inline]
    pub fn new() -> Self {
        Self {
            grids: RwHashMap::new(),
        }
    }

    // TODO: more generic names

    /// Lookup a position
    #[inline]
    pub fn grids_get(&self, grid: &G) -> Option<CanonicalForm> {
        self.grids.get(grid)
    }

    /// Save position and its game value
    #[inline]
    pub fn grids_insert(&self, grid: G, game: CanonicalForm) {
        self.grids.insert(grid, game);
    }

    /// Get number of saved games
    #[inline]
    pub fn grids_saved(&self) -> usize {
        self.grids.len()
    }
}
