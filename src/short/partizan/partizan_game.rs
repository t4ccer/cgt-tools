//! Shared traits for short partizan games

use crate::{
    numeric::rational::Rational,
    short::partizan::{
        canonical_form::{CanonicalForm, Moves},
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
    /// This is copy-pasted from [`super::canonical_form`] module, but works on positions rather than
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
    fn canonical_form(&self, cache: &TranspositionTable<Self>) -> CanonicalForm {
        if let Some(id) = cache.grids_get(self) {
            return id;
        }

        let mut result = CanonicalForm::new_integer(0);
        for position in self.decompositions() {
            let sub_result = cache.grids_get(&position).unwrap_or_else(|| {
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

                let canonical_form = CanonicalForm::new_from_moves(moves);
                cache.grids_insert(position, canonical_form.clone());
                canonical_form
            });

            result = sub_result + result;
        }

        cache.grids_insert(self.clone(), result.clone());
        result
    }

    // TODO: Find a way to reduce duplication - maybe macro?

    /// List of canonical moves for the Left player
    fn sensible_left_moves(&self, cache: &TranspositionTable<Self>) -> Vec<Self> {
        let canonical_form = self.canonical_form(cache);
        let moves = canonical_form.to_moves();
        let left_canonical = moves.left;

        self.left_moves()
            .into_iter()
            .filter(|m| {
                let move_game_form = m.canonical_form(cache);
                let res = left_canonical
                    .iter()
                    .any(|k| CanonicalForm::leq(k, &move_game_form));
                res
            })
            .collect::<Vec<_>>()
    }

    /// List of canonical moves for the Right player
    fn sensible_right_moves(&self, cache: &TranspositionTable<Self>) -> Vec<Self> {
        let canonical_form = self.canonical_form(cache);
        let moves = canonical_form.to_moves();
        let right_canonical = moves.right;

        self.right_moves()
            .into_iter()
            .filter(|m| {
                let move_game_form = m.canonical_form(cache);
                let res = right_canonical
                    .iter()
                    .any(|k| CanonicalForm::leq(&move_game_form, k));
                res
            })
            .collect::<Vec<_>>()
    }
}
