#![allow(missing_docs)]

use crate::{
    misere::game_form::{
        BlockingContext, ConstructionError, DeadEndingContext, GameFormContext, StandardForm,
        StandardFormContext,
    },
    short::partizan::Player,
    total::TotalWrappable,
};
use std::{error::Error, fmt};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DicoticFormContext<C> {
    context: C,
}

impl<C> DicoticFormContext<C> {
    pub const fn new(context: C) -> Self {
        Self { context }
    }

    pub const fn underlying(&self) -> &C {
        &self.context
    }
}

#[derive(Debug, Clone)]
#[repr(transparent)]
pub struct DicoticForm<G> {
    underlying: G,
}

impl<G> DicoticForm<G> {
    pub(crate) const fn new_unchecked(underlying: G) -> DicoticForm<G> {
        DicoticForm { underlying }
    }

    pub(crate) const fn new_ref_unchecked(underlying: &G) -> &DicoticForm<G> {
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

impl<G> TotalWrappable for DicoticForm<G>
where
    G: TotalWrappable,
{
    fn total_cmp(&self, other: &Self) -> std::cmp::Ordering {
        TotalWrappable::total_cmp(self.underlying(), other.underlying())
    }

    fn total_hash<H: std::hash::Hasher>(&self, state: &mut H) {
        TotalWrappable::total_hash(self.underlying(), state);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DicoticFormConstructionError<G, E> {
    NotDicotic(G),
    Underlying(E),
}

impl<G, E> std::fmt::Display for DicoticFormConstructionError<G, E>
where
    G: std::fmt::Display,
    E: std::fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DicoticFormConstructionError::NotDicotic(g) => {
                write!(f, "form `{}` is not dicotic", g)
            }
            DicoticFormConstructionError::Underlying(_) => {
                write!(f, "could not construct the underlying form")
            }
        }
    }
}

impl<G, E> Error for DicoticFormConstructionError<G, E>
where
    G: std::fmt::Debug + std::fmt::Display,
    E: std::fmt::Debug + Error + 'static,
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            DicoticFormConstructionError::NotDicotic(_) => None,
            DicoticFormConstructionError::Underlying(err) => Some(err),
        }
    }
}

impl<G, E> ConstructionError<G> for DicoticFormConstructionError<G, E>
where
    E: ConstructionError<G>,
    G: std::fmt::Debug,
{
    fn recover(self) -> G {
        match self {
            DicoticFormConstructionError::NotDicotic(g) => g,
            DicoticFormConstructionError::Underlying(err) => err.recover(),
        }
    }
}

impl<C> GameFormContext for DicoticFormContext<C>
where
    C: GameFormContext,
{
    type Form = DicoticForm<C::Form>;
    type BaseForm = C::BaseForm;

    type DicoticConstructionError =
        DicoticFormConstructionError<Self::BaseForm, C::DicoticConstructionError>;

    type IntegerConstructionError =
        DicoticFormConstructionError<Self::BaseForm, C::IntegerConstructionError>;

    type ConjugateConstructionError = C::ConjugateConstructionError;

    type SumConstructionError = C::SumConstructionError;

    fn new(
        &self,
        left: impl IntoIterator<Item = Self::Form>,
        right: impl IntoIterator<Item = Self::Form>,
    ) -> Result<Self::Form, Self::DicoticConstructionError> {
        let g = self
            .context
            .new(
                left.into_iter().map(|g| g.underlying),
                right.into_iter().map(|g| g.underlying),
            )
            .map_err(DicoticFormConstructionError::Underlying)?;
        if self.context.is_dicotic(&g) {
            Ok(DicoticForm::new_unchecked(g))
        } else {
            Err(DicoticFormConstructionError::NotDicotic(
                self.underlying().base(g),
            ))
        }
    }

    // The default builds the integer through `new` and unwraps, which panics here because only
    // `0` is dicotic.
    fn new_integer(&self, n: i32) -> Result<Self::Form, Self::IntegerConstructionError> {
        let g = self
            .context
            .new_integer(n)
            .map_err(DicoticFormConstructionError::Underlying)?;
        if self.context.is_dicotic(&g) {
            Ok(DicoticForm::new_unchecked(g))
        } else {
            Err(DicoticFormConstructionError::NotDicotic(
                self.underlying().base(g),
            ))
        }
    }

    fn moves<'a>(
        &self,
        game: &'a Self::Form,
        player: Player,
    ) -> impl Iterator<Item = &'a Self::Form> {
        self.context
            .moves(&game.underlying, player)
            .map(DicoticForm::new_ref_unchecked)
    }

    fn is_p_free(&self, game: &Self::Form) -> bool {
        self.context.is_p_free(&game.underlying)
    }

    fn is_dicotic(&self, _game: &Self::Form) -> bool {
        true
    }

    fn is_dead_ending(&self, _game: &Self::Form) -> bool {
        true
    }

    fn is_blocking(&self, _game: &Self::Form) -> bool {
        true
    }

    fn total_cmp(&self, lhs: &Self::Form, rhs: &Self::Form) -> std::cmp::Ordering {
        self.context.total_cmp(&lhs.underlying, &rhs.underlying)
    }

    fn total_eq(&self, lhs: &Self::Form, rhs: &Self::Form) -> bool {
        self.context.total_eq(&lhs.underlying, &rhs.underlying)
    }

    fn base(&self, game: Self::Form) -> Self::BaseForm {
        self.underlying().base(game.to_underlying())
    }

    fn base_context(&self) -> &impl GameFormContext<Form = Self::BaseForm> {
        self.underlying().base_context()
    }
}

/// Comparison modulo the dicotic universe: the outcome condition together with maintenance,
/// see Larsson, Milley, Nowakowski, Renault, Santos, *Recursive comparison tests for
/// dicot and dead-ending games under misère play* (Theorem 2).
pub trait DicoticContext: GameFormContext {
    fn satisfy_dicotic_maintenance(&self, g: &Self::Form, h: &Self::Form) -> bool {
        self.satisfy_maintenance_with(g, h, |x, y| self.ge_mod_dicotic(x, y))
    }

    fn satisfy_dicotic_proviso(&self, g: &Self::Form, h: &Self::Form) -> bool {
        self.outcome(g) >= self.outcome(h)
    }

    fn ge_mod_dicotic(&self, g: &Self::Form, h: &Self::Form) -> bool {
        self.satisfy_dicotic_proviso(g, h) && self.satisfy_dicotic_maintenance(g, h)
    }

    fn eq_mod_dicotic(&self, g: &Self::Form, h: &Self::Form) -> bool {
        self.ge_mod_dicotic(g, h) && self.ge_mod_dicotic(h, g)
    }
}

impl<C> DicoticContext for DicoticFormContext<C> where C: GameFormContext {}

impl<C> DeadEndingContext for DicoticFormContext<C> where C: GameFormContext {}

impl<C> BlockingContext for DicoticFormContext<C> where C: GameFormContext {}

impl std::fmt::Display for DicoticForm<StandardForm> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", StandardFormContext.display(&self.underlying))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::misere::game_form::{InternedFormContext, Outcome, ParseError};

    #[test]
    fn parsing() {
        let context = DicoticFormContext::new(StandardFormContext);

        assert!(
            context
                .from_str("1")
                .is_err_and(|err| matches!(err, ParseError::Integer(_)))
        );
        assert!(
            context
                .from_str("{0|}")
                .is_err_and(|err| matches!(err, ParseError::Dicotic(_)))
        );
        assert!(
            context
                .from_str("{*|{0|}}")
                .is_err_and(|err| matches!(err, ParseError::Dicotic(_)))
        );
        assert!(context.from_str("{0,*|0,*,^,v}").is_ok());
    }

    #[test]
    fn known_forms() {
        let context = DicoticFormContext::new(StandardFormContext);
        let zero = context.from_str("0").unwrap();
        let star = context.from_str("*").unwrap();
        let star2 = context.from_str("*2").unwrap();
        let star_star = context.sum(&star, &star).unwrap();
        let star2_star2 = context.sum(&star2, &star2).unwrap();
        let j = context.from_str("{*|0,*}").unwrap();
        let h0 = context.from_str("{*|{*|0,*}}").unwrap();
        let h0_star2 = context.sum(&h0, &star2).unwrap();

        assert_eq!(context.outcome(&star), Outcome::P);
        assert_eq!(context.outcome(&star2), Outcome::N);
        assert_eq!(context.outcome(&star2_star2), Outcome::P);
        assert_eq!(context.outcome(&h0), Outcome::L);
        assert!(context.eq_mod_dicotic(&star_star, &zero));
        assert!(context.ge_mod_dicotic(&zero, &j));
        assert!(context.ge_mod_dicotic(&star2, &h0_star2));
        assert!(!context.ge_mod_dicotic(&zero, &h0));
        assert!(!context.ge_mod_dicotic(&zero, &star2_star2));
        assert!(!context.ge_mod_dicotic(&star2_star2, &zero));
    }

    fn not_cancellative_with<C: DicoticContext>(context: &C) {
        let g = context.from_str("{0,*|0,*,^,v}").unwrap();
        let g_conjugate = context.conjugate(&g).unwrap();
        let g_sub_g = context.sum(&g, &g_conjugate).unwrap();
        let star2 = context.from_str("*2").unwrap();
        let zero = context.from_str("0").unwrap();
        let g_sub_g_add_star2 = context.sum(&g_sub_g, &star2).unwrap();

        assert_eq!(context.outcome(&g_sub_g), Outcome::P);
        assert!(!context.eq_mod_dicotic(&g_sub_g, &zero));
        assert!(context.eq_mod_dicotic(&g_sub_g_add_star2, &star2));
    }

    #[test]
    fn not_cancellative() {
        not_cancellative_with(&DicoticFormContext::new(StandardFormContext));
    }

    #[test]
    fn not_cancellative_interned() {
        not_cancellative_with(&DicoticFormContext::new(InternedFormContext::new()));
    }
}
