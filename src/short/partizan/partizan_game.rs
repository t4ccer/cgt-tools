//! Shared traits for short partizan games

use crate::{
    numeric::rational::Rational,
    short::partizan::{
        short_canonical_game::{Game, Moves},
        thermograph::Thermograph,
        trajectory::Trajectory,
        transposition_table::TranspositionTable,
    },
};
use std::hash::Hash;

/// A short partizan game
pub trait PartizanGame: Sized + Clone + Hash + Send + Sync + Eq {
    /// List of all moves for the Left player
    fn left_moves(&self) -> Vec<Self>;

    /// List of all moves for the Right player
    fn right_moves(&self) -> Vec<Self>;

    /// Split game into disjoint sum
    ///
    /// Note that default implementation doesn't find any decompisitons and may lead to performance
    /// issues
    fn decompositions(&self) -> Vec<Self> {
        vec![self.clone()]
    }

    /// Compute the thermograph without going through canonical form
    /// Taken from Elwyn Berlekamp - The Economist’s View of Combinatorial Games
    /// This is copy-pasted from short_canonical_form module, but works on positions rather than
    /// it's canonical forms, although the algorithm is the same
    ///
    /// Note that for some games, going through canonical form to compute the thermograph may
    /// be faster
    ///
    /// See: zubzero-thermography
    fn thermograph_direct(&self) -> Thermograph {
        let left_moves = self.left_moves();
        let right_moves = self.right_moves();
        if left_moves.is_empty() && right_moves.is_empty() {
            return Thermograph::with_mast(Rational::from(0));
        }

        let mut left_scaffold = Trajectory::new_constant(Rational::NegativeInfinity);
        let mut right_scaffold = Trajectory::new_constant(Rational::PositiveInfinity);

        for left_move in &left_moves {
            left_scaffold = left_scaffold.max(&left_move.thermograph_direct().right_wall);
        }
        for right_move in &right_moves {
            right_scaffold = right_scaffold.min(&right_move.thermograph_direct().left_wall);
        }

        left_scaffold.tilt(Rational::from(-1));
        right_scaffold.tilt(Rational::from(1));

        Thermograph::thermographic_intersection(left_scaffold, right_scaffold)
    }

    /// Get the canonical form of the game position
    fn canonical_form(&self, cache: &TranspositionTable<Self>) -> Game {
        if let Some(id) = cache.grids_get(self) {
            return id;
        }

        let mut result = cache.game_backend().construct_integer(0);
        for position in self.decompositions() {
            let sub_result = match cache.grids_get(&position) {
                Some(canonical_form) => canonical_form,
                None => {
                    let moves = Moves {
                        left: position
                            .left_moves()
                            .iter()
                            .map(|o| o.canonical_form(cache))
                            .collect(),
                        right: position
                            .right_moves()
                            .iter()
                            .map(|o| o.canonical_form(cache))
                            .collect(),
                    };

                    let canonical_form = cache.game_backend().construct_from_moves(moves);
                    cache.grids_insert(position, canonical_form);
                    canonical_form
                }
            };

            result = cache.game_backend().construct_sum(sub_result, result);
        }

        cache.grids_insert(self.clone(), result);
        result
    }

    // TODO: Find a way to reduce duplication - maybe macro?

    /// List of canonical moves for the Left player
    fn sensible_left_moves(&self, cache: &TranspositionTable<Self>) -> Vec<Self> {
        let canonical_form = self.canonical_form(cache);
        let left_canonical = cache.game_backend().get_game_moves(&canonical_form).left;

        self.left_moves()
            .into_iter()
            .filter(|m| {
                let move_game_form = m.canonical_form(cache);
                left_canonical.contains(&move_game_form)
            })
            .collect::<Vec<_>>()
    }

    /// List of canonical moves for the Right player
    fn sensible_right_moves(&self, cache: &TranspositionTable<Self>) -> Vec<Self> {
        let canonical_form = self.canonical_form(cache);
        let right_canonical = cache.game_backend().get_game_moves(&canonical_form).right;

        self.right_moves()
            .into_iter()
            .filter(|m| {
                let move_game_form = m.canonical_form(cache);
                right_canonical.contains(&move_game_form)
            })
            .collect::<Vec<_>>()
    }
}
