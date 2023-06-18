// TODO: Move to short positional game module
use crate::short_canonical_game::{Game, GameBackend, PlacementGame};
use concurrent_lru::sharded::LruCache;
use std::hash::Hash;

/// Transaction table (cache) of game positions and canonical forms.
pub struct TranspositionTable<G> {
    grids: LruCache<G, Game>,
    game_backend: GameBackend,
}

impl<G> TranspositionTable<G>
where
    G: Eq + Hash + Clone + Sync + Send + PlacementGame,
{
    /// Create new empty transposition table.
    #[inline]
    pub fn new() -> Self {
        TranspositionTable::with_game_backend(GameBackend::new())
    }

    /// Create new transposition table with pre-existing game backend.
    /// Useful if you load game backend from file, or re-use it from earlier computations.
    #[inline]
    pub fn with_game_backend(game_backend: GameBackend) -> Self {
        TranspositionTable {
            grids: LruCache::new(1 << 23),
            game_backend,
        }
    }

    /// Get the underlying game storage backend.
    #[inline]
    pub fn game_backend(&self) -> &GameBackend {
        &self.game_backend
    }

    #[inline]
    pub fn grids_get(&self, grid: &G) -> Option<Game> {
        self.grids.get(grid.clone()).map(|c| c.value().clone())
    }

    #[inline]
    pub fn grids_insert(&self, grid: G, game: Game) {
        self.grids.get_or_init(grid, 1, |_| game);
    }

    #[inline]
    pub fn grids_saved(&self) -> usize {
        self.grids.total_charge() as usize
    }
}
