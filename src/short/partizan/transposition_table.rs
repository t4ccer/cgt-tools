//! Thread safe transposition table for game values

use crate::{rw_hash_map::RwHashMap, short::partizan::canonical_form::CanonicalForm};
use id_arena::{Arena, Id};
use std::{hash::Hash, sync::Mutex};

/// Transaction table (cache) of game positions and canonical forms.
pub struct TranspositionTable<G> {
    known_games: Mutex<Arena<CanonicalForm>>,
    grids: RwHashMap<G, Id<CanonicalForm>, ahash::RandomState>,
}

#[cfg_attr(feature = "cargo-clippy", allow(clippy::new_without_default))]
impl<G> TranspositionTable<G>
where
    G: Eq + Hash + Sync + Send,
{
    /// Create new empty transposition table.
    #[inline]
    pub fn new() -> Self {
        Self {
            known_games: Mutex::new(Arena::new()),
            grids: RwHashMap::with_hasher(ahash::RandomState::new()),
        }
    }

    // TODO: more generic names

    /// Lookup a position
    #[inline]
    pub fn grids_get(&self, grid: &G) -> Option<CanonicalForm> {
        self.grids
            .get(grid)
            .and_then(|id| self.known_games.lock().unwrap().get(id).cloned())
    }

    /// Save position and its game value
    #[cfg_attr(feature = "cargo-clippy", allow(clippy::missing_panics_doc))]
    #[inline]
    pub fn grids_insert(&self, grid: G, game: CanonicalForm) {
        let mut arena = self.known_games.lock().unwrap();
        let inserted = arena.alloc(game);
        drop(arena);
        self.grids.insert(grid, inserted);
    }

    /// Get number of saved games
    #[inline]
    pub fn grids_saved(&self) -> usize {
        self.grids.len()
    }
}
