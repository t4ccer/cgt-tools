#![allow(missing_docs)]

use crate::{
    misere::game_form::{
        BlockingContext, ConstructionError, GameFormContext, Outcome, PFreeContext,
    },
    result::{UnwrapInfallible, Void},
    short::partizan::Player,
    total::TotalWrappable,
};
use std::{error::Error, fmt};

/// Comparison of P-free forms of a blocking universe
///
/// The order modulo pf(U) is decided by the recursive test of "Recursive Comparison Test for
/// Invertible Subgroups of Blocking Universes" (Theorem `misereGE_iff_promain`), which applies
/// to any blocking universe containing integers, so it decides the order modulo pf(E) when the
/// inner context is dead-ending and modulo pf(B) when it is blocking
pub trait PFreeBlockingContext: BlockingContext + PFreeContext
where
    Self::IntegerConstructionError: Void,
{
    fn ge_mod_p_free_blocking(&self, g: &Self::Form, h: &Self::Form) -> bool;

    fn satisfy_promain(&self, g: &Self::Form, h: &Self::Form) -> bool {
        self.satisfy_proviso(g, h) && self.satisfy_maintenance(g, h)
    }

    fn satisfy_maintenance(&self, g: &Self::Form, h: &Self::Form) -> bool {
        self.satisfy_maintenance_with(g, h, |x, y| self.ge_mod_p_free_blocking(x, y))
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
        // A non-zero end is equal to the form with the missing side filled in by -1 or 1
        // (Lemma `reduction_plug_end_not_isEnd_left` and its conjugate), and unlike the end
        // itself that form can be simplified by the reductions below, e.g. {|1} = {-1|1} = 0
        if self.to_integer(game).is_none() {
            if self.is_end(game, Player::Left) {
                let plugged = self
                    .new(
                        [self.new_integer(-1).unwrap_infallible()],
                        self.moves(game, Player::Right).cloned(),
                    )
                    .unwrap();
                return self.reduced(&plugged);
            }

            if self.is_end(game, Player::Right) {
                let plugged = self
                    .new(
                        self.moves(game, Player::Left).cloned(),
                        [self.new_integer(1).unwrap_infallible()],
                    )
                    .unwrap();
                return self.reduced(&plugged);
            }
        }

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PFreeBlockingFormContext<C> {
    context: C,
}

impl<C> PFreeBlockingFormContext<C> {
    pub const fn new(context: C) -> Self {
        Self { context }
    }

    pub const fn underlying(&self) -> &C {
        &self.context
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
{
    fn ge_mod_p_free_blocking(&self, g: &Self::Form, h: &Self::Form) -> bool {
        let g_left_end = self.is_end(g, Player::Left);
        let g_right_end = self.is_end(g, Player::Right);
        let h_left_end = self.is_end(h, Player::Left);
        let h_right_end = self.is_end(h, Player::Right);

        // The cases of the theorem are read in order

        if g_left_end && g_right_end {
            return match self.outcome(h) {
                Outcome::R => true,
                Outcome::N => {
                    let h_not_right_end = self
                        .new(
                            self.moves(h, Player::Left)
                                .filter(|hl| !self.is_end(hl, Player::Right))
                                .cloned(),
                            self.moves(h, Player::Right).cloned(),
                        )
                        .unwrap();
                    self.satisfy_promain(g, &h_not_right_end)
                }
                Outcome::L => false,
                Outcome::P => unreachable!("P-free form has outcome P"),
            };
        }

        if h_left_end && h_right_end {
            return match self.outcome(g) {
                Outcome::L => true,
                Outcome::N => {
                    let g_not_left_end = self
                        .new(
                            self.moves(g, Player::Left).cloned(),
                            self.moves(g, Player::Right)
                                .filter(|gr| !self.is_end(gr, Player::Left))
                                .cloned(),
                        )
                        .unwrap();
                    self.satisfy_promain(&g_not_left_end, h)
                }
                Outcome::R => false,
                Outcome::P => unreachable!("P-free form has outcome P"),
            };
        }

        if g_left_end && h_right_end {
            return true;
        }

        if g_left_end {
            let g_plugged = self
                .new(
                    [self.new_integer(-1).unwrap_infallible()],
                    self.moves(g, Player::Right).cloned(),
                )
                .unwrap();
            return self.satisfy_promain(&g_plugged, h);
        }

        if h_right_end {
            let h_plugged = self
                .new(
                    self.moves(h, Player::Left).cloned(),
                    [self.new_integer(1).unwrap_infallible()],
                )
                .unwrap();
            return self.satisfy_promain(g, &h_plugged);
        }

        self.satisfy_promain(g, h)
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

    #[test]
    fn blocking_end_reductions() {
        let context = PFreeBlockingFormContext::new(PFreeFormContext::new(
            BlockingFormContext::new(StandardFormContext),
        ));

        for (end, integer) in [
            ("{|1}", "0"),
            ("{-1|}", "0"),
            ("{|{-1|2}}", "-1"),
            ("{{-2|1}|}", "1"),
            ("{|1,{-1|2}}", "0"),
            ("{|{-2|1}}", "-2"),
            ("{|0,{-1|2}}", "-1"),
        ] {
            let g = context.from_str(end).unwrap();
            let n = context.from_str(integer).unwrap();
            assert!(!context.is_dead_ending(&g), "{end} is dead-ending");
            assert!(
                context.eq_mod_p_free_blocking(&g, &n),
                "{end} != {integer} (mod pf(B))"
            );
            assert!(
                context.total_eq(&context.reduced(&g), &n),
                "reduced {end} = {} is not {integer}",
                context.display(&context.reduced(&g))
            );
        }
    }
}
