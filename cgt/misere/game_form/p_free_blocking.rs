#![allow(missing_docs)]

use crate::{
    misere::game_form::{
        BlockingContext, ConstructionError, GameFormContext, Outcome, PFreeContext,
    },
    result::{UnwrapInfallible, Void},
    short::partizan::Player,
    total::{TotalWrappable, TotalWrapper},
};
use std::{collections::HashMap, error::Error, fmt, sync::RwLock};

/// Comparison of P-free blocking forms
///
/// Dead-ending forms are blocking so the same maintenance and proviso checks decide the order
/// modulo pf(E) when the inner context is dead-ending and modulo pf(B) when it is blocking
pub trait PFreeBlockingContext: BlockingContext + PFreeContext
where
    Self::IntegerConstructionError: Void,
{
    fn ge_mod_p_free_blocking(&self, g: &Self::Form, h: &Self::Form) -> bool;

    fn satisfy_maintenance(&self, g: &Self::Form, h: &Self::Form) -> bool {
        let a = self.moves(g, Player::Right).all(|gr| {
            self.moves(gr, Player::Left)
                .any(|grl| self.ge_mod_p_free_blocking(grl, h))
                || self
                    .moves(h, Player::Right)
                    .any(|hr| self.ge_mod_p_free_blocking(gr, hr))
        });
        let b = self.moves(h, Player::Left).all(|hl| {
            self.moves(hl, Player::Right)
                .any(|hlr| self.ge_mod_p_free_blocking(g, hlr))
                || self
                    .moves(g, Player::Left)
                    .any(|gl| self.ge_mod_p_free_blocking(gl, hl))
        });

        a && b
    }

    fn satisfy_proviso(&self, g: &Self::Form, h: &Self::Form) -> bool {
        (!self.is_end(g, Player::Right) || self.outcome(h) != Outcome::L)
            && (!self.is_end(h, Player::Left) || self.outcome(g) != Outcome::R)
    }

    fn eq_mod_p_free_blocking(&self, g: &Self::Form, h: &Self::Form) -> bool {
        self.ge_mod_p_free_blocking(g, h) && self.ge_mod_p_free_blocking(h, g)
    }

    fn incomp_mod_p_free_blocking(&self, g: &Self::Form, h: &Self::Form) -> bool {
        !self.ge_mod_p_free_blocking(g, h) && !self.ge_mod_p_free_blocking(h, g)
    }

    fn bypass_reversible_moves_l(&self, g: &Self::Form) -> Vec<Self::Form> {
        let mut i: i64 = 0;

        let mut left_moves: Vec<Option<Self::Form>> =
            self.moves(g, Player::Left).cloned().map(Some).collect();

        loop {
            if (i as usize) >= left_moves.len() {
                break;
            }
            let g_l = match &left_moves[i as usize] {
                None => {
                    i += 1;
                    continue;
                }
                Some(g) => g.clone(),
            };
            for g_lr in self.moves(&g_l, Player::Right) {
                if self.ge_mod_p_free_blocking(g, g_lr) {
                    let mut end_reversible = true;
                    for g_lrl in self.moves(g_lr, Player::Left) {
                        end_reversible = false;
                        left_moves.push(Some(g_lrl.clone()));
                    }

                    if end_reversible {
                        if self.to_integer(&g_l).is_none_or(|n| n != -1) {
                            left_moves.push(Some(self.new_integer(-1).unwrap_infallible()));
                            left_moves[i as usize] = None;
                        }
                    } else {
                        left_moves[i as usize] = None;
                    }

                    break;
                }
            }

            i += 1;
        }

        left_moves.into_iter().flatten().collect()
    }

    fn bypass_reversible_moves_r(&self, g: &Self::Form) -> Vec<Self::Form> {
        let mut i: i64 = 0;

        let mut right_moves: Vec<Option<Self::Form>> =
            self.moves(g, Player::Right).cloned().map(Some).collect();

        loop {
            if (i as usize) >= right_moves.len() {
                break;
            }
            let g_r = match &right_moves[i as usize] {
                None => {
                    i += 1;
                    continue;
                }
                Some(g) => g.clone(),
            };

            for g_rl in self.moves(&g_r, Player::Left) {
                if self.ge_mod_p_free_blocking(g_rl, g) {
                    let mut end_reversible = true;
                    for g_rlr in self.moves(g_rl, Player::Right) {
                        end_reversible = false;
                        right_moves.push(Some(g_rlr.clone()));
                    }

                    if end_reversible {
                        if self.to_integer(&g_r).is_none_or(|n| n != 1) {
                            right_moves.push(Some(self.new_integer(1).unwrap_infallible()));
                            right_moves[i as usize] = None;
                        }
                    } else {
                        right_moves[i as usize] = None;
                    }

                    break;
                }
            }

            i += 1;
        }

        right_moves.into_iter().flatten().collect()
    }

    fn eliminate_dominated_moves(&self, moves: &mut Vec<Self::Form>, player: Player) {
        let mut i = 0;
        'loop_i: while i < moves.len() {
            let mut j = i + 1;
            'loop_j: while i < moves.len() && j < moves.len() {
                let move_i = &moves[i];
                let move_j = &moves[j];

                let remove_i = match player {
                    Player::Left => self.ge_mod_p_free_blocking(move_j, move_i),
                    Player::Right => self.ge_mod_p_free_blocking(move_i, move_j),
                };

                if remove_i {
                    moves.swap_remove(i);
                    continue 'loop_i;
                }

                let remove_j = match player {
                    Player::Left => self.ge_mod_p_free_blocking(move_i, move_j),
                    Player::Right => self.ge_mod_p_free_blocking(move_j, move_i),
                };

                if remove_j {
                    moves.swap_remove(j);
                    continue 'loop_j;
                }

                j += 1;
            }

            i += 1;
        }
    }

    fn reduced(&self, game: &Self::Form) -> Self::Form {
        let mut left = self.bypass_reversible_moves_l(game);
        self.eliminate_dominated_moves(&mut left, Player::Left);

        let mut right = self.bypass_reversible_moves_r(game);
        self.eliminate_dominated_moves(&mut right, Player::Right);

        if let [gl] = left.as_slice()
            && let Some(a) = self.to_integer(gl)
            && let [gr] = right.as_slice()
            && let Some(b) = self.to_integer(gr)
        {
            // {-1|1} = 0
            if a == -1 && b == 1 {
                return self.new_integer(0).unwrap_infallible();
            }

            // {a|b} = a+1
            if a >= 0 && b <= a + 2 {
                return self.new_integer(a + 1).unwrap_infallible();
            }

            if b <= 0 && a >= b - 2 {
                return self.new_integer(b - 1).unwrap_infallible();
            }
        }

        match self.new(left, right) {
            Ok(g) => {
                // TODO: Find a better way of doing it, I'm not even sure why this happens but it does
                if self.total_eq(game, &g) {
                    g
                } else {
                    self.reduced(&g)
                }
            }
            Err(err) => {
                unreachable!(
                    "Reduction of `{}` is `{}` which is not P-free blocking",
                    self.display(game),
                    self.base_context().display(&err.recover())
                )
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum SeenZero {
    Once,
    Multiple,
}

#[derive(Debug)]
pub struct PFreeBlockingFormContext<C>
where
    C: GameFormContext,
{
    not_ge_zero: RwLock<HashMap<TotalWrapper<PFreeBlockingForm<C::Form>>, SeenZero>>,
    not_zero_ge: RwLock<HashMap<TotalWrapper<PFreeBlockingForm<C::Form>>, SeenZero>>,
    context: C,
}

impl<C> PFreeBlockingFormContext<C>
where
    C: GameFormContext,
{
    pub fn new(context: C) -> Self {
        Self {
            context,
            not_ge_zero: RwLock::new(HashMap::new()),
            not_zero_ge: RwLock::new(HashMap::new()),
        }
    }
}

#[derive(Debug, Clone)]
#[repr(transparent)]
pub struct PFreeBlockingForm<G> {
    underlying: G,
}

impl<G> PFreeBlockingForm<G> {
    pub(crate) const fn new_unchecked(underlying: G) -> PFreeBlockingForm<G> {
        PFreeBlockingForm { underlying }
    }

    pub(crate) const fn new_ref_unchecked(underlying: &G) -> &PFreeBlockingForm<G> {
        // SAFETY: We are #[repr(transparent)] so reference cast is safe
        unsafe { &*(::std::ptr::from_ref(underlying).cast::<Self>()) }
    }

    pub const fn underlying(&self) -> &G {
        &self.underlying
    }

    pub fn to_underlying(self) -> G {
        self.underlying
    }
}

impl<G> TotalWrappable for PFreeBlockingForm<G>
where
    G: TotalWrappable,
{
    fn total_cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.underlying.total_cmp(&other.underlying)
    }

    fn total_hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.underlying.total_hash(state);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PFreeBlockingConstructionError<E> {
    Underlying(E),
}

impl<E> std::fmt::Display for PFreeBlockingConstructionError<E>
where
    E: std::fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PFreeBlockingConstructionError::Underlying(_) => {
                write!(f, "could not construct the underlying form")
            }
        }
    }
}

impl<E> Error for PFreeBlockingConstructionError<E>
where
    E: std::fmt::Debug + Error + 'static,
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            PFreeBlockingConstructionError::Underlying(err) => Some(err),
        }
    }
}

impl<E, G> ConstructionError<G> for PFreeBlockingConstructionError<E>
where
    E: ConstructionError<G>,
{
    fn recover(self) -> G {
        match self {
            PFreeBlockingConstructionError::Underlying(err) => err.recover(),
        }
    }
}

impl<E> Void for PFreeBlockingConstructionError<E>
where
    E: Void,
{
    fn absurd<T>(self) -> T {
        match self {
            PFreeBlockingConstructionError::Underlying(err) => err.absurd(),
        }
    }
}

impl<C> GameFormContext for PFreeBlockingFormContext<C>
where
    C: GameFormContext,
{
    type Form = PFreeBlockingForm<C::Form>;

    type BaseForm = C::BaseForm;

    type DicoticConstructionError = PFreeBlockingConstructionError<C::DicoticConstructionError>;

    type IntegerConstructionError = PFreeBlockingConstructionError<C::IntegerConstructionError>;

    type ConjugateConstructionError = PFreeBlockingConstructionError<C::ConjugateConstructionError>;

    type SumConstructionError = PFreeBlockingConstructionError<C::SumConstructionError>;

    fn new(
        &self,
        left: impl IntoIterator<Item = Self::Form>,
        right: impl IntoIterator<Item = Self::Form>,
    ) -> Result<Self::Form, Self::DicoticConstructionError> {
        self.context
            .new(
                left.into_iter().map(|g| g.underlying),
                right.into_iter().map(|g| g.underlying),
            )
            .map(PFreeBlockingForm::new_unchecked)
            .map_err(PFreeBlockingConstructionError::Underlying)
    }

    fn moves<'a>(
        &self,
        game: &'a Self::Form,
        player: Player,
    ) -> impl Iterator<Item = &'a Self::Form> {
        self.context
            .moves(&game.underlying, player)
            .map(PFreeBlockingForm::new_ref_unchecked)
    }

    fn total_cmp(&self, lhs: &Self::Form, rhs: &Self::Form) -> std::cmp::Ordering {
        self.context.total_cmp(&lhs.underlying, &rhs.underlying)
    }

    fn total_eq(&self, lhs: &Self::Form, rhs: &Self::Form) -> bool {
        self.context.total_eq(&lhs.underlying, &rhs.underlying)
    }

    fn is_p_free(&self, game: &Self::Form) -> bool {
        self.context.is_p_free(&game.underlying)
    }

    fn is_dead_ending(&self, game: &Self::Form) -> bool {
        self.context.is_dead_ending(&game.underlying)
    }

    fn is_blocking(&self, game: &Self::Form) -> bool {
        self.context.is_blocking(&game.underlying)
    }

    fn sum(
        &self,
        g: &Self::Form,
        h: &Self::Form,
    ) -> Result<Self::Form, Self::SumConstructionError> {
        self.context
            .sum(&g.underlying, &h.underlying)
            .map(PFreeBlockingForm::new_unchecked)
            .map_err(PFreeBlockingConstructionError::Underlying)
    }

    fn base(&self, game: Self::Form) -> Self::BaseForm {
        self.context.base(game.underlying)
    }

    fn base_context(&self) -> &impl GameFormContext<Form = Self::BaseForm> {
        self.context.base_context()
    }
}

impl<C> PFreeContext for PFreeBlockingFormContext<C>
where
    C: PFreeContext,
    C::IntegerConstructionError: Void,
{
}

impl<C> BlockingContext for PFreeBlockingFormContext<C> where C: BlockingContext {}

impl<C> PFreeBlockingContext for PFreeBlockingFormContext<C>
where
    C: BlockingContext + PFreeContext,
    C::IntegerConstructionError: Void,
    C::Form: TotalWrappable,
{
    fn ge_mod_p_free_blocking(&self, g: &Self::Form, h: &Self::Form) -> bool {
        // Relation on integers does not follow from maintenance/proviso so it is hardcoded
        if let Some(g) = self.to_integer(g)
            && let Some(h) = self.to_integer(h)
        {
            // The order of games is the opposite of order of integers so <= is correct for `ge` check
            return g <= h;
        }

        if self.satisfy_proviso(g, h) && self.satisfy_maintenance(g, h) {
            return true;
        }

        let plug_end = |g,
                        h,
                        seen_zero: &RwLock<
            HashMap<TotalWrapper<PFreeBlockingForm<C::Form>>, SeenZero>,
        >| {
            self.to_integer(g).and_then(|g| match g.cmp(&0) {
                // G = -n = {-1 | -(n - 1)}
                std::cmp::Ordering::Less => Some(
                    self.new(
                        [self.new_integer(-1).unwrap_infallible()],
                        [self.new_integer(g + 1).unwrap_infallible()],
                    )
                    .unwrap(),
                ),
                // We need to try plugging the G = 0 to G = {-1|1} but that may loop since G^RL = 0
                // in the maintenance check
                std::cmp::Ordering::Equal => {
                    // NOTE: Race may happen here but worst case we'll just do a redundant check
                    // and two threads will mark the same game as checked in the HashMap
                    let not_zero_ge = seen_zero.read().unwrap();
                    match not_zero_ge.get(TotalWrapper::from_ref(h)) {
                        // First time we see `h` compared against G = 0
                        // If we are here that means that maintenance/proviso check of `0 >= h`
                        // failed so we try again with `{-1|1} >= h`
                        None => {
                            drop(not_zero_ge);
                            seen_zero
                                .write()
                                .unwrap()
                                .insert(TotalWrapper::new(h.clone()), SeenZero::Once);
                            Some(
                                self.new(
                                    [self.new_integer(-1).unwrap_infallible()],
                                    [self.new_integer(1).unwrap_infallible()],
                                )
                                .unwrap(),
                            )
                        }
                        // If we are here that means that we are in the process of checking
                        // `{-1|1} >= h` holds since we got `0 >= h = false`
                        // already, so we break the recursion and note that in the HashMap to not take
                        // the write lock again for that game
                        Some(SeenZero::Once) => {
                            drop(not_zero_ge);
                            seen_zero
                                .write()
                                .unwrap()
                                .insert(TotalWrapper::new(h.clone()), SeenZero::Multiple);
                            None
                        }
                        Some(SeenZero::Multiple) => None,
                    }
                }
                // G = n = {n - 1 | 1}
                std::cmp::Ordering::Greater => Some(
                    self.new(
                        [self.new_integer(g - 1).unwrap_infallible()],
                        [self.new_integer(1).unwrap_infallible()],
                    )
                    .unwrap(),
                ),
            })
        };

        // No need to plug both cause then they are both integers and handled by the case above
        if let Some(g) = plug_end(g, h, &self.not_ge_zero) {
            return self.ge_mod_p_free_blocking(&g, h);
        }

        if let Some(h) = plug_end(h, g, &self.not_zero_ge) {
            return self.ge_mod_p_free_blocking(g, &h);
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        misere::game_form::{
            BlockingFormContext, DeadEndingFormContext, GameFormContext, PFreeBlockingContext,
            PFreeBlockingFormContext, PFreeFormContext, StandardFormContext,
        },
        total::TotalWrappable,
    };

    #[test]
    fn relations() {
        let context = PFreeBlockingFormContext::new(PFreeFormContext::new(
            DeadEndingFormContext::new(StandardFormContext),
        ));

        macro_rules! assert_rel {
            ($lhs:expr, $rhs:expr, $func:ident, $fmt:literal) => {
                let g = context.from_str($lhs).unwrap();
                let h = context.from_str($rhs).unwrap();
                assert!(
                    context.$func(&g, &h),
                    $fmt,
                    context.display(&g),
                    context.display(&h)
                );

                let gc = context.conjugate(&g).unwrap();
                let hc = context.conjugate(&h).unwrap();
                assert!(
                    context.$func(&hc, &gc),
                    $fmt,
                    context.display(&hc),
                    context.display(&gc)
                );
            };
        }

        macro_rules! assert_eq_mod_p_free_blocking {
            ($lhs:expr, $rhs:expr) => {
                assert_rel!(
                    $lhs,
                    $rhs,
                    eq_mod_p_free_blocking,
                    "Game forms are not = (mod pf(E))\n  left: {}\n right: {}"
                );
            };
        }

        macro_rules! assert_ge_mod_p_free_blocking {
            ($lhs:expr, $rhs:expr) => {
                assert_rel!(
                    $lhs,
                    $rhs,
                    ge_mod_p_free_blocking,
                    "Game forms are not >= (mod pf(E))\n  left: {}\n right: {}"
                );
            };
        }

        assert_eq_mod_p_free_blocking!("1", "{0|1}");

        assert_eq_mod_p_free_blocking!("1", "{0,{-2|2}|1}");
        assert_eq_mod_p_free_blocking!("2", "{1,{-2|2}|1}");
        assert_eq_mod_p_free_blocking!("2", "{2,{-2|2}|1}");
        assert_eq_mod_p_free_blocking!("2", "{3,{-2|2}|1}");

        assert_eq_mod_p_free_blocking!("1", "{0,{-3|3}|1}");
        assert_eq_mod_p_free_blocking!("2", "{1,{-3|3}|1}");
        assert_eq_mod_p_free_blocking!("3", "{2,{-3|3}|1}");
        assert_eq_mod_p_free_blocking!("3", "{3,{-3|3}|1}");
        assert_eq_mod_p_free_blocking!("3", "{4,{-3|3}|1}");

        assert_eq_mod_p_free_blocking!("3", "{{-3|3}|1}");
        assert_eq_mod_p_free_blocking!("3", "{{-3|3}|2}");
        assert_eq_mod_p_free_blocking!("3", "{{-3|3}|3}");
        assert_eq_mod_p_free_blocking!("3", "{{-3|3}|4}");

        assert_ge_mod_p_free_blocking!("3", "{{-3|3}|5}");

        assert_eq_mod_p_free_blocking!("{{-3|3},{-4|4}|1}", "3");

        assert_ge_mod_p_free_blocking!("0", "1");

        assert_eq_mod_p_free_blocking!("{1|3}", "2");

        assert_ge_mod_p_free_blocking!("{-2|1}", "{-2|2}");

        assert_eq_mod_p_free_blocking!("5", "{4|1,{-1|3}}");
        assert_ge_mod_p_free_blocking!("-1", "5");
        assert_ge_mod_p_free_blocking!("-1", "{4|1,{-1|3}}");
        assert_ge_mod_p_free_blocking!("{-1|0}", "5");
        assert_ge_mod_p_free_blocking!("{-1|0}", "{4|1,{-1|3}}");

        assert_eq_mod_p_free_blocking!("5", "{4|{0|3}}");
        assert_ge_mod_p_free_blocking!("0", "{4|{0|3}}");

        assert_eq_mod_p_free_blocking!("{0, {-2|2}|1}", "{0|1}");
        assert_eq_mod_p_free_blocking!("{0, {-2|2}|2}", "{0|2}");
        assert_eq_mod_p_free_blocking!("{0, {-2|2}, {-3|3}|2}", "{0|2}");
    }

    #[test]
    fn reductions() {
        let context = PFreeBlockingFormContext::new(PFreeFormContext::new(
            DeadEndingFormContext::new(StandardFormContext),
        ));

        macro_rules! assert_identical {
            ($lhs:expr, $rhs:expr) => {
                let g = context.from_str($lhs).unwrap();
                let h = context.from_str($rhs).unwrap();
                assert!(
                    context.eq_mod_p_free_blocking(&g, &h),
                    "SANITY CHECK: Games are not equal mod pf(E)\n  left: {}\n right: {}",
                    context.display(&g),
                    context.display(&h)
                );

                let gg = context.reduced(&g);

                assert!(
                    context.eq_mod_p_free_blocking(&g, &h),
                    "SANITY CHECK: Original and reduced are not equal mod pf(E)\n  left: {}\n right: {}",
                    context.display(&g),
                    context.display(&gg)
                );

                assert!(
                    TotalWrappable::total_eq(&gg, &h),
                    "Game forms are not identical\n  left: {}\n right: {}",
                    context.display(&gg),
                    context.display(&h)
                );

                let gc = context.conjugate(&gg).unwrap();
                let hc = context.conjugate(&h).unwrap();
                assert!(
                    TotalWrappable::total_eq(&hc, &gc),
                    "Conjugate game forms are not identical\n  left: {}\n right: {}",
                    context.display(&hc),
                    context.display(&gc)
                );
            };
        }

        assert_identical!("{0|2}", "1");
        assert_identical!("{0,1|2}", "1");
        assert_identical!("{0|3}", "{0|3}");
        assert_identical!("{0,1|3}", "{0|3}");
        assert_identical!("{-2|0}", "-1");

        assert_identical!("{{-2|1}|1}", "1");
        assert_identical!("{{-2|1}|2}", "1");

        assert_identical!("{{-1|2}|1}", "2");
        assert_identical!("{{-1|2}|2}", "2");
        assert_identical!("{{-1|2}|3}", "2");
        assert_identical!("{{-2|2}|1}", "2");
        assert_identical!("{{-2|2}|2}", "2");
        assert_identical!("{{-2|2}|3}", "2");

        assert_identical!("{-1|{-1|2}}", "-1");
        assert_identical!("{-2|{-1|2}}", "-1");

        assert_identical!("{-1|{-2|1}}", "-2");
        assert_identical!("{-1|{-2|2}}", "-2");
        assert_identical!("{-2|{-2|1}}", "-2");
        assert_identical!("{-2|{-2|2}}", "-2");
        assert_identical!("{-3|{-2|1}}", "-2");
        assert_identical!("{-3|{-2|2}}", "-2");

        assert_identical!("{0,{-2|2}|1}", "1");
        assert_identical!("{0,{-3|3}|1}", "1");
        assert_identical!("{0,{-2|2},{-3|3}|1}", "1");

        assert_identical!("{0,{-2|2}|3}", "{0|3}");
        assert_identical!("{-3|0,{-2|2}}", "{-3|0}");

        assert_identical!("{1,{-2|2},{-3|3}|1}", "2");

        assert_identical!("{-1|{0|3}}", "0");
    }

    #[test]
    fn blocking_relations() {
        let context = PFreeBlockingFormContext::new(PFreeFormContext::new(
            BlockingFormContext::new(StandardFormContext),
        ));

        let g = context.from_str("{|{0|}}").unwrap();
        assert!(!context.is_dead_ending(&g));
        assert!(context.eq_mod_p_free_blocking(&g, &g));

        let one = context.from_str("1").unwrap();
        let h = context.from_str("{0|1}").unwrap();
        assert!(context.eq_mod_p_free_blocking(&one, &h));
        assert!(context.total_eq(&context.reduced(&h), &one));
    }
}
