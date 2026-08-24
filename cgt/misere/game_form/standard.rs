#![allow(missing_docs)]

use crate::{
    atomic_enum::atomic_enum,
    misere::game_form::{GameFormContext, Outcome, ParseError},
    short::partizan::Player,
    total::{IgnoreOrder, TotalWrappable, impl_total_wrapper},
};
use std::{cmp::Ordering, convert::Infallible, str::FromStr, sync::atomic};

atomic_enum! {
    #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
    enum CachedBool {
        NotCached,
        True,
        False,
    }

    #[derive(Debug)]
    struct AtomicCachedBool;
}

impl From<bool> for CachedBool {
    fn from(value: bool) -> CachedBool {
        if value {
            CachedBool::True
        } else {
            CachedBool::False
        }
    }
}

impl AtomicCachedBool {
    fn load_or_store(&self, f: impl FnOnce() -> bool) -> bool {
        self.load_or_store_with_ordering(atomic::Ordering::Relaxed, atomic::Ordering::Relaxed, f)
    }

    fn load_or_store_with_ordering(
        &self,
        load_ordering: atomic::Ordering,
        store_ordering: atomic::Ordering,
        f: impl FnOnce() -> bool,
    ) -> bool {
        match self.load(load_ordering) {
            CachedBool::NotCached => {
                let res = f();
                self.store(CachedBool::from(res), store_ordering);
                res
            }
            CachedBool::True => true,
            CachedBool::False => false,
        }
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct StandardFormInner {
    left: Vec<StandardFormInner>,
    right: Vec<StandardFormInner>,
    is_p_free: IgnoreOrder<AtomicCachedBool>,
    is_dead_ending: IgnoreOrder<AtomicCachedBool>,
}

impl Clone for StandardFormInner {
    fn clone(&self) -> Self {
        Self {
            left: self.left.clone(),
            right: self.right.clone(),
            is_p_free: IgnoreOrder(AtomicCachedBool::new(
                self.is_p_free.load(atomic::Ordering::Relaxed),
            )),
            is_dead_ending: IgnoreOrder(AtomicCachedBool::new(
                self.is_dead_ending.load(atomic::Ordering::Relaxed),
            )),
        }
    }
}

impl_total_wrapper! {
    #[derive(Debug, Clone)]
    pub struct StandardForm {
        inner: StandardFormInner
    }
}

impl StandardForm {
    fn moves(&self, player: Player) -> impl Iterator<Item = &Self> {
        match player {
            Player::Left => StandardForm::from_inner_slice(self.inner.left.as_slice()).iter(),
            Player::Right => StandardForm::from_inner_slice(self.inner.right.as_slice()).iter(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StandardFormContext;

impl GameFormContext for StandardFormContext {
    type Form = StandardForm;
    type BaseForm = StandardForm;

    type DicoticConstructionError = Infallible;
    type IntegerConstructionError = Infallible;
    type ConjugateConstructionError = Infallible;
    type SumConstructionError = Infallible;

    fn new(
        &self,
        left: impl IntoIterator<Item = Self::Form>,
        right: impl IntoIterator<Item = Self::Form>,
    ) -> Result<Self::Form, Self::DicoticConstructionError> {
        let mut left = StandardForm::into_inner_vec(left.into_iter().collect());
        left.sort();
        left.dedup();

        let mut right = StandardForm::into_inner_vec(right.into_iter().collect());
        right.sort();
        right.dedup();

        Ok(StandardForm {
            inner: StandardFormInner {
                is_dead_ending: IgnoreOrder(AtomicCachedBool::new(CachedBool::NotCached)),
                is_p_free: IgnoreOrder(AtomicCachedBool::new(CachedBool::NotCached)),
                left,
                right,
            },
        })
    }

    fn moves<'a>(
        &self,
        game: &'a Self::Form,
        player: Player,
    ) -> impl Iterator<Item = &'a Self::Form> {
        game.moves(player)
    }

    fn total_cmp(&self, lhs: &Self::Form, rhs: &Self::Form) -> Ordering {
        TotalWrappable::total_cmp(lhs, rhs)
    }

    fn total_eq(&self, lhs: &Self::Form, rhs: &Self::Form) -> bool {
        TotalWrappable::total_eq(lhs, rhs)
    }

    fn is_p_free(&self, game: &Self::Form) -> bool {
        game.inner.is_p_free.load_or_store(|| {
            (self.outcome(game) != Outcome::P)
                && Player::forall(|p| self.moves(game, p).all(|g| self.is_p_free(g)))
        })
    }

    fn is_dead_ending(&self, game: &Self::Form) -> bool {
        game.inner.is_dead_ending.load_or_store(|| {
            Player::forall(|p| !self.is_end(game, p) || self.is_dead_end(game, p))
                && Player::forall(|p| self.moves(game, p).all(|g| self.is_dead_ending(g)))
        })
    }

    fn base(&self, game: Self::Form) -> Self::BaseForm {
        game
    }

    fn base_context(&self) -> &impl GameFormContext<Form = Self::BaseForm> {
        self
    }
}

impl FromStr for StandardForm {
    type Err = ParseError<Infallible, Infallible>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        StandardFormContext.from_str(s)
    }
}

impl std::fmt::Display for StandardForm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", StandardFormContext.display(self))
    }
}
