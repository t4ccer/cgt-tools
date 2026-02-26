#![allow(missing_docs)]

use crate::{
    misere::game_form::{ConstructionError, DeadEndingContext, GameFormContext, PFreeContext},
    result::Void,
    short::partizan::Player,
    total::TotalWrappable,
};
use std::{error::Error, fmt};

pub trait PFreeDeadEndingContext: DeadEndingContext + PFreeContext
where
    Self::IntegerConstructionError: Void,
{
    fn ge_mod_p_free_dead_ending(&self, g: &Self::Form, h: &Self::Form) -> bool {
        if let Some(g) = self.to_integer(g)
            && let Some(h) = self.to_integer(h)
        {
            return g <= h;
        }
        self.ge_mod_dead_ending(g, h)
    }

    fn eq_mod_p_free_dead_ending(&self, g: &Self::Form, h: &Self::Form) -> bool {
        self.ge_mod_p_free_dead_ending(g, h) && self.ge_mod_p_free_dead_ending(h, g)
    }

    fn eliminate_dominated_moves(&self, moves: &mut Vec<Self::Form>, player: Player) {
        let mut i = 0;
        'loop_i: while i < moves.len() {
            let mut j = i + 1;
            'loop_j: while i < moves.len() && j < moves.len() {
                let move_i = &moves[i];
                let move_j = &moves[j];

                let remove_i = match player {
                    Player::Left => self.ge_mod_p_free_dead_ending(move_j, move_i),
                    Player::Right => self.ge_mod_p_free_dead_ending(move_i, move_j),
                };

                if remove_i {
                    moves.swap_remove(i);
                    continue 'loop_i;
                }

                let remove_j = match player {
                    Player::Left => self.ge_mod_p_free_dead_ending(move_i, move_j),
                    Player::Right => self.ge_mod_p_free_dead_ending(move_j, move_i),
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
        let mut left = self.moves(game, Player::Left).cloned().collect::<Vec<_>>();
        self.eliminate_dominated_moves(&mut left, Player::Left);

        let mut right = self.moves(game, Player::Right).cloned().collect::<Vec<_>>();
        self.eliminate_dominated_moves(&mut right, Player::Right);

        if let [gl] = left.as_slice()
            && let Some(a) = self.to_integer(gl)
            && let [gr] = right.as_slice()
            && let Some(b) = self.to_integer(gr)
        {
            if a == -1 && b == 1 {
                return self.new_integer(0).unwrap();
            }

            if a >= 0 && b <= a + 2 {
                return self.new_integer(a + 1).unwrap();
            }

            if b <= 0 && a >= b - 2 {
                return self.new_integer(b - 1).unwrap();
            }
        }

        self.new(left, right).unwrap()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PFreeDeadEndingFormContext<C> {
    context: C,
}

impl<C> PFreeDeadEndingFormContext<C> {
    pub fn new(context: C) -> Self {
        Self { context }
    }
}

#[derive(Debug, Clone)]
#[repr(transparent)]
pub struct PFreeDeadEndingForm<G> {
    underlying: G,
}

impl<G> PFreeDeadEndingForm<G> {
    pub(crate) const fn new_unchecked(underlying: G) -> PFreeDeadEndingForm<G> {
        PFreeDeadEndingForm { underlying }
    }

    pub(crate) const fn new_ref_unchecked(underlying: &G) -> &PFreeDeadEndingForm<G> {
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

impl<G> TotalWrappable for PFreeDeadEndingForm<G>
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
pub enum PFreeDeadEndingConstructionError<E> {
    Underlying(E),
}

impl<E> std::fmt::Display for PFreeDeadEndingConstructionError<E>
where
    E: std::fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PFreeDeadEndingConstructionError::Underlying(err) => {
                write!(f, "could not construct the underlying form: {}", err)
            }
        }
    }
}

impl<E> Error for PFreeDeadEndingConstructionError<E>
where
    E: std::fmt::Debug + Error + 'static,
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            PFreeDeadEndingConstructionError::Underlying(err) => Some(err),
        }
    }
}

impl<E, G> ConstructionError<G> for PFreeDeadEndingConstructionError<E>
where
    E: ConstructionError<G>,
{
    fn recover(self) -> G {
        match self {
            PFreeDeadEndingConstructionError::Underlying(err) => err.recover(),
        }
    }
}

impl<E> Void for PFreeDeadEndingConstructionError<E>
where
    E: Void,
{
    fn absurd<T>(self) -> T {
        match self {
            PFreeDeadEndingConstructionError::Underlying(err) => err.absurd(),
        }
    }
}

impl<C> GameFormContext for PFreeDeadEndingFormContext<C>
where
    C: GameFormContext,
{
    type Form = PFreeDeadEndingForm<C::Form>;

    type BaseForm = C::BaseForm;

    type DicoticConstructionError = PFreeDeadEndingConstructionError<C::DicoticConstructionError>;

    type IntegerConstructionError = PFreeDeadEndingConstructionError<C::IntegerConstructionError>;

    type ConjugateConstructionError =
        PFreeDeadEndingConstructionError<C::ConjugateConstructionError>;

    type SumConstructionError = PFreeDeadEndingConstructionError<C::SumConstructionError>;

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
            .map(PFreeDeadEndingForm::new_unchecked)
            .map_err(PFreeDeadEndingConstructionError::Underlying)
    }

    fn moves<'a>(
        &self,
        game: &'a Self::Form,
        player: Player,
    ) -> impl Iterator<Item = &'a Self::Form> {
        self.context
            .moves(&game.underlying, player)
            .map(PFreeDeadEndingForm::new_ref_unchecked)
    }

    fn total_cmp(&self, lhs: &Self::Form, rhs: &Self::Form) -> std::cmp::Ordering {
        self.context.total_cmp(&lhs.underlying, &rhs.underlying)
    }

    fn total_eq(&self, lhs: &Self::Form, rhs: &Self::Form) -> bool {
        self.context.total_eq(&lhs.underlying, &rhs.underlying)
    }

    fn base(&self, game: Self::Form) -> Self::BaseForm {
        self.context.base(game.underlying)
    }

    fn base_context(&self) -> &impl GameFormContext<Form = Self::BaseForm> {
        self.context.base_context()
    }
}

impl<C> PFreeContext for PFreeDeadEndingFormContext<C>
where
    C: PFreeContext,
    C::IntegerConstructionError: Void,
{
}

impl<C> DeadEndingContext for PFreeDeadEndingFormContext<C>
where
    C: DeadEndingContext + PFreeContext,
    C::IntegerConstructionError: Void,
{
    fn satisfy_maintenance(&self, g: &Self::Form, h: &Self::Form) -> bool {
        let a = self.moves(g, Player::Right).all(|gr| {
            self.moves(gr, Player::Left)
                .any(|grl| self.ge_mod_p_free_dead_ending(grl, h))
                || self
                    .moves(h, Player::Right)
                    .any(|hr| self.ge_mod_p_free_dead_ending(gr, hr))
        });
        let b = self.moves(h, Player::Left).all(|hl| {
            self.moves(hl, Player::Right)
                .any(|hlr| self.ge_mod_p_free_dead_ending(g, hlr))
                || self
                    .moves(g, Player::Left)
                    .any(|gl| self.ge_mod_p_free_dead_ending(gl, hl))
        });

        a && b
    }
}

impl<C> PFreeDeadEndingContext for PFreeDeadEndingFormContext<C>
where
    C: DeadEndingContext + PFreeContext,
    C::IntegerConstructionError: Void,
{
}

#[cfg(test)]
mod tests {
    use crate::{
        misere::game_form::{
            DeadEndingFormContext, GameFormContext, PFreeDeadEndingContext,
            PFreeDeadEndingFormContext, PFreeFormContext, StandardFormContext,
        },
        total::TotalWrappable,
    };

    #[test]
    fn relations() {
        let context = PFreeDeadEndingFormContext::new(PFreeFormContext::new(
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

        macro_rules! assert_eq_mod_p_free_dead_ending {
            ($lhs:expr, $rhs:expr) => {
                assert_rel!(
                    $lhs,
                    $rhs,
                    eq_mod_p_free_dead_ending,
                    "Game forms are not = (mod pf(E))\n  left: {}\n right: {}"
                );
            };
        }

        macro_rules! assert_ge_mod_p_free_dead_ending {
            ($lhs:expr, $rhs:expr) => {
                assert_rel!(
                    $lhs,
                    $rhs,
                    ge_mod_p_free_dead_ending,
                    "Game forms are >= (mod pf(E))\n  left: {}\n right: {}"
                );
            };
        }

        assert_eq_mod_p_free_dead_ending!("1", "{0|1}");

        assert_eq_mod_p_free_dead_ending!("1", "{0,{-2|2}|1}");
        assert_eq_mod_p_free_dead_ending!("2", "{1,{-2|2}|1}");
        assert_eq_mod_p_free_dead_ending!("2", "{2,{-2|2}|1}");
        assert_eq_mod_p_free_dead_ending!("2", "{3,{-2|2}|1}");

        assert_eq_mod_p_free_dead_ending!("1", "{0,{-3|3}|1}");
        assert_eq_mod_p_free_dead_ending!("2", "{1,{-3|3}|1}");
        assert_eq_mod_p_free_dead_ending!("3", "{2,{-3|3}|1}");
        assert_eq_mod_p_free_dead_ending!("3", "{3,{-3|3}|1}");
        assert_eq_mod_p_free_dead_ending!("3", "{4,{-3|3}|1}");

        assert_eq_mod_p_free_dead_ending!("3", "{{-3|3}|1}");
        assert_eq_mod_p_free_dead_ending!("3", "{{-3|3}|2}");
        assert_eq_mod_p_free_dead_ending!("3", "{{-3|3}|3}");
        assert_eq_mod_p_free_dead_ending!("3", "{{-3|3}|4}");

        assert_ge_mod_p_free_dead_ending!("3", "{{-3|3}|5}");

        assert_eq_mod_p_free_dead_ending!("{{-3|3},{-4|4}|1}", "3");

        assert_ge_mod_p_free_dead_ending!("0", "1");

        assert_eq_mod_p_free_dead_ending!("{1|3}", "2");

        assert_ge_mod_p_free_dead_ending!("{-2|1}", "{-2|2}");

        assert_eq_mod_p_free_dead_ending!("{0, {-2|2}|1}", "{0|1}");
        assert_eq_mod_p_free_dead_ending!("{0, {-2|2}|2}", "{0|2}");
        assert_eq_mod_p_free_dead_ending!("{0, {-2|2}, {-3|3}|2}", "{0|2}");
    }

    #[test]
    fn reductions() {
        let context = PFreeDeadEndingFormContext::new(PFreeFormContext::new(
            DeadEndingFormContext::new(StandardFormContext),
        ));

        macro_rules! assert_identical {
            ($lhs:expr, $rhs:expr) => {
                let g = context.reduced(&context.from_str($lhs).unwrap());
                let h = context.from_str($rhs).unwrap();
                assert!(
                    TotalWrappable::total_eq(&g, &h),
                    "Game forms are not identical\n  left: {}\n right: {}",
                    context.display(&g),
                    context.display(&h)
                );

                let gc = context.conjugate(&g).unwrap();
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
        assert_identical!("{0,1|3}", "{0|3}");
        assert_identical!("{-2|0}", "-1");

        // assert_identical!("{0,{-2|2}|1}", "1");
        // assert_identical!("{3,{-2|2}|1}", "4");
    }
}
