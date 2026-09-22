#![allow(missing_docs)]

use crate::{
    atomic_enum::atomic_enum,
    misere::game_form::{GameFormContext, Outcome},
    short::partizan::Player,
    total::{TotalWrappable, TotalWrapper, impl_total_wrapper},
};
use append_only_vec::AppendOnlyVec;
use dashmap::{DashMap, mapref::entry::Entry};
use std::{cmp::Ordering, convert::Infallible, sync::atomic};

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

impl AtomicCachedBool {
    fn load_or_store(&self, f: impl FnOnce() -> bool) -> bool {
        match self.load(atomic::Ordering::Relaxed) {
            CachedBool::NotCached => {
                let res = f();
                self.store(
                    if res {
                        CachedBool::True
                    } else {
                        CachedBool::False
                    },
                    atomic::Ordering::Relaxed,
                );
                res
            }
            CachedBool::True => true,
            CachedBool::False => false,
        }
    }
}

impl_total_wrapper! {
    /// Handle to a game form interned in [`InternedFormContext`]
    ///
    /// Handles are meaningful only in the context that created them
    #[derive(Debug, Clone, Copy)]
    pub struct InternedForm {
        idx: u32
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct Options {
    left: Box<[TotalWrapper<InternedForm>]>,
    right: Box<[TotalWrapper<InternedForm>]>,
}

#[derive(Debug)]
struct Node {
    // Option slices are leaked so that `GameFormContext::moves` can hand out references that
    // outlive the borrow of the context, as the trait ties them to the handle's lifetime only
    left: &'static [TotalWrapper<InternedForm>],
    right: &'static [TotalWrapper<InternedForm>],
    left_wins_going_first: AtomicCachedBool,
    right_wins_going_first: AtomicCachedBool,
    is_p_free: AtomicCachedBool,
    is_dead_ending: AtomicCachedBool,
    is_dicotic: AtomicCachedBool,
}

/// Hash-consing context for unrestricted game forms
///
/// Every form is stored once, so handle equality is form equality. Sums, conjugates, outcomes
/// and cached predicates are memoised per handle.
#[derive(Debug)]
pub struct InternedFormContext {
    nodes: AppendOnlyVec<Node>,
    table: DashMap<Options, u32, crate::hash::RandomState>,
    sums: DashMap<(u32, u32), u32, crate::hash::RandomState>,
    conjugates: DashMap<u32, u32, crate::hash::RandomState>,
}

impl InternedFormContext {
    pub fn new() -> InternedFormContext {
        InternedFormContext {
            nodes: AppendOnlyVec::new(),
            table: DashMap::default(),
            sums: DashMap::default(),
            conjugates: DashMap::default(),
        }
    }

    /// Get count of interned forms
    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    fn node(&self, game: InternedForm) -> &Node {
        &self.nodes[game.idx as usize]
    }

    fn intern(&self, options: Options) -> InternedForm {
        if let Some(idx) = self.table.get(&options) {
            return InternedForm { idx: *idx };
        }

        let idx = match self.table.entry(options) {
            Entry::Occupied(entry) => *entry.get(),
            Entry::Vacant(entry) => {
                let node = Node {
                    left: Box::leak(entry.key().left.clone()),
                    right: Box::leak(entry.key().right.clone()),
                    left_wins_going_first: AtomicCachedBool::new(CachedBool::NotCached),
                    right_wins_going_first: AtomicCachedBool::new(CachedBool::NotCached),
                    is_p_free: AtomicCachedBool::new(CachedBool::NotCached),
                    is_dead_ending: AtomicCachedBool::new(CachedBool::NotCached),
                    is_dicotic: AtomicCachedBool::new(CachedBool::NotCached),
                };
                let idx = self.nodes.push(node) as u32;
                entry.insert(idx);
                idx
            }
        };
        InternedForm { idx }
    }
}

impl GameFormContext for InternedFormContext {
    type Form = InternedForm;
    type BaseForm = InternedForm;

    type DicoticConstructionError = Infallible;
    type IntegerConstructionError = Infallible;
    type ConjugateConstructionError = Infallible;
    type SumConstructionError = Infallible;

    fn new(
        &self,
        left: impl IntoIterator<Item = Self::Form>,
        right: impl IntoIterator<Item = Self::Form>,
    ) -> Result<Self::Form, Self::DicoticConstructionError> {
        let mut left = TotalWrapper::from_inner_vec(left.into_iter().collect());
        left.sort_unstable();
        left.dedup();

        let mut right = TotalWrapper::from_inner_vec(right.into_iter().collect());
        right.sort_unstable();
        right.dedup();

        Ok(self.intern(Options {
            left: left.into_boxed_slice(),
            right: right.into_boxed_slice(),
        }))
    }

    fn moves<'a>(
        &self,
        game: &'a Self::Form,
        player: Player,
    ) -> impl Iterator<Item = &'a Self::Form> {
        let node = self.node(*game);
        let options: &'a [InternedForm] = TotalWrapper::into_inner_slice(match player {
            Player::Left => node.left,
            Player::Right => node.right,
        });
        options.iter()
    }

    fn wins_going_first(&self, game: &Self::Form, player: Player) -> bool {
        let node = self.node(*game);
        let cache = match player {
            Player::Left => &node.left_wins_going_first,
            Player::Right => &node.right_wins_going_first,
        };
        cache.load_or_store(|| {
            self.moves(game, player).count() == 0
                || self
                    .moves(game, player)
                    .any(|g| !self.wins_going_first(g, player.opposite()))
        })
    }

    fn is_p_free(&self, game: &Self::Form) -> bool {
        self.node(*game).is_p_free.load_or_store(|| {
            (self.outcome(game) != Outcome::P)
                && Player::forall(|p| self.moves(game, p).all(|g| self.is_p_free(g)))
        })
    }

    fn is_dead_ending(&self, game: &Self::Form) -> bool {
        self.node(*game).is_dead_ending.load_or_store(|| {
            Player::forall(|p| !self.is_end(game, p) || self.is_dead_end(game, p))
                && Player::forall(|p| self.moves(game, p).all(|g| self.is_dead_ending(g)))
        })
    }

    fn is_dicotic(&self, game: &Self::Form) -> bool {
        self.node(*game).is_dicotic.load_or_store(|| {
            Player::forall(|p| !self.is_end(game, p) || self.is_end(game, p.opposite()))
                && Player::forall(|p| self.moves(game, p).all(|g| self.is_dicotic(g)))
        })
    }

    fn total_cmp(&self, lhs: &Self::Form, rhs: &Self::Form) -> Ordering {
        TotalWrappable::total_cmp(lhs, rhs)
    }

    fn total_eq(&self, lhs: &Self::Form, rhs: &Self::Form) -> bool {
        TotalWrappable::total_eq(lhs, rhs)
    }

    fn conjugate(&self, game: &Self::Form) -> Result<Self::Form, Self::ConjugateConstructionError> {
        if let Some(conjugate) = self.conjugates.get(&game.idx) {
            return Ok(InternedForm { idx: *conjugate });
        }

        let left = self
            .moves(game, Player::Right)
            .map(|gr| self.conjugate(gr))
            .collect::<Result<Vec<_>, _>>()?;
        let right = self
            .moves(game, Player::Left)
            .map(|gl| self.conjugate(gl))
            .collect::<Result<Vec<_>, _>>()?;
        let conjugate = self.new(left, right)?;

        self.conjugates.insert(game.idx, conjugate.idx);
        self.conjugates.insert(conjugate.idx, game.idx);
        Ok(conjugate)
    }

    fn sum(
        &self,
        g: &Self::Form,
        h: &Self::Form,
    ) -> Result<Self::Form, Self::SumConstructionError> {
        let key = (Ord::min(g.idx, h.idx), Ord::max(g.idx, h.idx));
        if let Some(sum) = self.sums.get(&key) {
            return Ok(InternedForm { idx: *sum });
        }

        if self.is_end(g, Player::Left) && self.is_end(g, Player::Right) {
            return Ok(*h);
        }
        if self.is_end(h, Player::Left) && self.is_end(h, Player::Right) {
            return Ok(*g);
        }

        let mut left = Vec::with_capacity(
            self.moves(g, Player::Left).count() + self.moves(h, Player::Left).count(),
        );
        for gl in self.moves(g, Player::Left) {
            left.push(self.sum(gl, h)?);
        }
        for hl in self.moves(h, Player::Left) {
            left.push(self.sum(g, hl)?);
        }

        let mut right = Vec::with_capacity(
            self.moves(g, Player::Right).count() + self.moves(h, Player::Right).count(),
        );
        for gr in self.moves(g, Player::Right) {
            right.push(self.sum(gr, h)?);
        }
        for hr in self.moves(h, Player::Right) {
            right.push(self.sum(g, hr)?);
        }

        let sum = self.new(left, right)?;
        self.sums.insert(key, sum.idx);
        Ok(sum)
    }

    fn base(&self, game: Self::Form) -> Self::BaseForm {
        game
    }

    fn base_context(&self) -> &impl GameFormContext<Form = Self::BaseForm> {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::misere::game_form::{DeadEndingContext, DeadEndingFormContext, ParseError};

    #[test]
    fn context_is_sync() {
        fn assert_sync<T: Sync + Send>() {}
        assert_sync::<InternedFormContext>();
    }

    #[test]
    fn construction() {
        let context = &InternedFormContext::new();
        let g = context
            .new(
                [context.from_str("{|0}").unwrap()],
                [context.from_str("{1,2|}").unwrap()],
            )
            .unwrap();
        assert_eq!(context.to_string(&g), "{-1|{1,2|}}");
    }

    #[test]
    fn moves() {
        let context = &InternedFormContext::new();
        let g = context.from_str("{1,2|{0|0}}").unwrap();
        assert_eq!(
            context
                .moves(&g, Player::Left)
                .map(|gl| context.to_string(gl))
                .collect::<Vec<_>>(),
            vec!["1", "2"]
        );
        assert_eq!(
            context
                .moves(&g, Player::Right)
                .map(|gr| context.to_string(gr))
                .collect::<Vec<_>>(),
            vec!["{0|0}"]
        );

        let g = context.new_integer(42).unwrap();
        assert_eq!(
            context
                .moves(&g, Player::Left)
                .map(|gl| context.to_string(gl))
                .collect::<Vec<_>>(),
            vec!["41"]
        );
        assert_eq!(context.moves(&g, Player::Right).count(), 0);
    }

    #[test]
    fn integers() {
        let context = &InternedFormContext::new();
        let g = context.from_str("{{1|}|}").unwrap();
        assert_eq!(context.to_integer(&g), Some(3));
        assert!(context.total_eq(&g, &context.new_integer(3).unwrap()));
        assert_eq!(context.to_string(&context.new_integer(-2).unwrap()), "-2");
    }

    #[test]
    fn outcomes() {
        let context = &InternedFormContext::new();
        let g = context.from_str("{1,2|{0|0}}").unwrap();
        assert_eq!(context.player_outcome(&g, Player::Left), Player::Right);
        assert_eq!(context.player_outcome(&g, Player::Right), Player::Right);
        assert!(!context.wins_going_first(&g, Player::Left));
        assert!(context.wins_going_first(&g, Player::Right));
        assert_eq!(context.outcome(&g), Outcome::R);
    }

    #[test]
    fn next_day() {
        let context = &InternedFormContext::new();
        let day1 = context
            .next_day(&[context.new_integer(0).unwrap()])
            .map(|g| context.to_string(&g))
            .collect::<Vec<_>>();
        assert_eq!(day1, vec!["0", "-1", "1", "{0|0}"]);
    }

    #[test]
    fn interning() {
        let context = &InternedFormContext::new();
        let zero = context.new_integer(0).unwrap();
        let one = context.new_integer(1).unwrap();
        let star = context.new([zero], [zero]).unwrap();

        let g = context.new([zero, one], [star]).unwrap();
        let h = context.new([zero, one], [star]).unwrap();
        assert!(context.total_eq(&g, &h));

        let k = context.new([one, zero, one], [star, star]).unwrap();
        assert!(context.total_eq(&g, &k));
        assert!(context.total_eq(&g, &context.from_str("{0,1|{0|0}}").unwrap()));

        assert!(!context.total_eq(&g, &star));
        assert_eq!(context.len(), 4);
    }

    #[test]
    fn nimbers() {
        let context = &InternedFormContext::new();
        let star = context.from_str("{0|0}").unwrap();
        assert_eq!(context.outcome(&star), Outcome::P);

        let star2 = context.from_str("{0,{0|0}|0,{0|0}}").unwrap();
        assert_eq!(context.outcome(&star2), Outcome::N);

        let star2_star2 = context.sum(&star2, &star2).unwrap();
        assert_eq!(context.outcome(&star2_star2), Outcome::P);
    }

    #[test]
    fn sum_and_conjugate() {
        let context = &InternedFormContext::new();
        let g = context.from_str("{1,2|{0|0}}").unwrap();
        let h = context.from_str("{|{0|0},-1}").unwrap();

        assert!(context.total_eq(&context.sum(&g, &h).unwrap(), &context.sum(&h, &g).unwrap()));
        assert_eq!(
            context.to_string(&context.sum(&g, &h).unwrap()),
            context.to_string(&context.sum(&h, &g).unwrap())
        );

        let zero = context.new_integer(0).unwrap();
        assert!(context.total_eq(&context.sum(&g, &zero).unwrap(), &g));

        let g_conj = context.conjugate(&g).unwrap();
        assert_eq!(context.to_string(&g_conj), "{{0|0}|-1,-2}");
        assert!(context.total_eq(&context.conjugate(&g_conj).unwrap(), &g));

        let big = context.sum(&g, &context.new_integer(5).unwrap()).unwrap();
        assert!(
            context.total_eq(
                &context.conjugate(&context.sum(&big, &h).unwrap()).unwrap(),
                &context
                    .sum(
                        &context.conjugate(&big).unwrap(),
                        &context.conjugate(&h).unwrap()
                    )
                    .unwrap()
            )
        );
    }

    #[test]
    fn p_free_and_dead_ending() {
        let context = &InternedFormContext::new();
        assert!(context.is_p_free(&context.from_str("{1,2|3}").unwrap()));
        assert!(!context.is_p_free(&context.from_str("{1,2|{0|0}}").unwrap()));
        assert!(context.is_dead_ending(&context.from_str("{2|4}").unwrap()));
        assert!(!context.is_dead_ending(&context.from_str("{|{|1}}").unwrap()));
    }

    #[test]
    fn parsing() {
        let context = &InternedFormContext::new();

        assert!(context.from_str("{{2|{|}},1|{0|0}}").is_ok());
        assert!(context.from_str("{|{|1}}").is_ok());
        assert!(
            context
                .from_str("  {  {  2  |  {  | }  }  ,  1  |  {  0  |  0  }  }  ")
                .is_ok()
        );
        assert!(context.from_str("{0|").is_err());

        let context = DeadEndingFormContext::new(InternedFormContext::new());
        assert!(
            context
                .from_str("{|{|1}}")
                .is_err_and(|err| matches!(err, ParseError::Dicotic(_)))
        );
    }

    #[test]
    fn waiting_protected() {
        let context = DeadEndingFormContext::new(InternedFormContext::new());

        let m0 = context.waiting_protected(0);
        assert_eq!(context.to_string(&m0), "0");

        let m1 = context.waiting_protected(1);
        assert_eq!(context.to_string(&m1), "-1");

        let m4 = context.waiting_protected(4);
        assert_eq!(context.to_string(&m4), "{|0,{|0,{|0,-1}}}");

        let m4_conj = context.waiting_protected(-4);
        assert_eq!(context.to_string(&m4_conj), "{0,{0,{0,1|}|}|}");
    }

    #[test]
    fn relations() {
        let context = DeadEndingFormContext::new(InternedFormContext::new());

        let g = context.from_str("{2|4}").unwrap();
        let h = context.from_str("{2|}").unwrap();
        assert!(context.eq_mod_dead_ending(&g, &h));

        let g = context.from_str("1").unwrap();
        let h = context.from_str("{0|4}").unwrap();
        assert!(context.ge_mod_dead_ending(&g, &h));
    }

    #[test]
    fn agrees_with_standard() {
        use crate::misere::game_form::StandardFormContext;

        let interned = &InternedFormContext::new();
        let standard = &StandardFormContext;
        let inputs = [
            "{{2|{|}},1|{0|0}}",
            "{|{|1}}",
            "{0,{0|0}|0,{0|0}}",
            "{1,2|{0|0}}",
            "{{0|0},-1|{{1|}|}}",
        ];

        for g in inputs {
            for h in inputs {
                let gi = interned.from_str(g).unwrap();
                let hi = interned.from_str(h).unwrap();
                let gs = standard.from_str(g).unwrap();
                let hs = standard.from_str(h).unwrap();

                let sum_i = interned.sum(&gi, &hi).unwrap();
                let sum_s = standard.sum(&gs, &hs).unwrap();
                assert!(standard.total_eq(
                    &standard.from_str(&interned.to_string(&sum_i)).unwrap(),
                    &sum_s
                ));
                assert_eq!(interned.outcome(&sum_i), standard.outcome(&sum_s));
                assert_eq!(interned.is_p_free(&sum_i), standard.is_p_free(&sum_s));
                assert_eq!(
                    interned.is_dead_ending(&sum_i),
                    standard.is_dead_ending(&sum_s)
                );
                assert_eq!(interned.birthday(&sum_i), standard.birthday(&sum_s));
                assert!(
                    standard.total_eq(
                        &standard
                            .from_str(&interned.to_string(&interned.conjugate(&sum_i).unwrap()))
                            .unwrap(),
                        &standard.conjugate(&sum_s).unwrap()
                    )
                );
            }
        }
    }
}
