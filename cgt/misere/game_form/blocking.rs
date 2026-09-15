#![allow(missing_docs)]

use crate::{
    misere::game_form::{ConstructionError, GameFormContext, StandardForm, StandardFormContext},
    short::partizan::Player,
    total::TotalWrappable,
};
use std::{error::Error, fmt};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BlockingFormContext<C> {
    context: C,
}

impl<C> BlockingFormContext<C> {
    pub const fn new(context: C) -> Self {
        Self { context }
    }

    pub const fn underlying(&self) -> &C {
        &self.context
    }
}

#[derive(Debug, Clone)]
#[repr(transparent)]
pub struct BlockingForm<G> {
    underlying: G,
}

impl<G> BlockingForm<G> {
    pub(crate) const fn new_unchecked(underlying: G) -> BlockingForm<G> {
        BlockingForm { underlying }
    }

    pub(crate) const fn new_ref_unchecked(underlying: &G) -> &BlockingForm<G> {
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

impl<G> TotalWrappable for BlockingForm<G>
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
pub enum BlockingConstructionError<G, E> {
    NotBlocking(G),
    Underlying(E),
}

// NOTE: G: Display will be annoying for interner context
impl<G, E> std::fmt::Display for BlockingConstructionError<G, E>
where
    G: std::fmt::Display,
    E: std::fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BlockingConstructionError::NotBlocking(g) => {
                write!(f, "form `{}` is not blocking", g)
            }
            BlockingConstructionError::Underlying(_) => {
                write!(f, "could not construct the underlying form")
            }
        }
    }
}

impl<G, E> Error for BlockingConstructionError<G, E>
where
    G: std::fmt::Debug + std::fmt::Display,
    E: std::fmt::Debug + Error + 'static,
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            BlockingConstructionError::NotBlocking(_) => None,
            BlockingConstructionError::Underlying(err) => Some(err),
        }
    }
}

impl<G, E> ConstructionError<G> for BlockingConstructionError<G, E>
where
    E: ConstructionError<G>,
    G: std::fmt::Debug,
{
    fn recover(self) -> G {
        match self {
            BlockingConstructionError::NotBlocking(g) => g,
            BlockingConstructionError::Underlying(err) => err.recover(),
        }
    }
}

impl<C> GameFormContext for BlockingFormContext<C>
where
    C: GameFormContext,
{
    type Form = BlockingForm<C::Form>;
    type BaseForm = C::BaseForm;

    type DicoticConstructionError =
        BlockingConstructionError<Self::BaseForm, C::DicoticConstructionError>;

    type IntegerConstructionError = C::IntegerConstructionError;

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
            .map_err(BlockingConstructionError::Underlying)?;
        if self.context.is_blocking(&g) {
            Ok(BlockingForm::new_unchecked(g))
        } else {
            Err(BlockingConstructionError::NotBlocking(
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
            .map(BlockingForm::new_ref_unchecked)
    }

    fn is_p_free(&self, game: &Self::Form) -> bool {
        self.context.is_p_free(&game.underlying)
    }

    fn is_dead_ending(&self, game: &Self::Form) -> bool {
        self.context.is_dead_ending(&game.underlying)
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

/// Context in which every form is blocking
pub trait BlockingContext: GameFormContext {}

impl<C> BlockingContext for BlockingFormContext<C> where C: GameFormContext {}

impl std::fmt::Display for BlockingForm<StandardForm> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", StandardFormContext.display(&self.underlying))
    }
}

#[test]
fn parsing() {
    use crate::misere::game_form::{DeadEndingFormContext, ParseError};
    let context = BlockingFormContext::new(StandardFormContext);

    assert!(
        context
            .from_str("{|{1|}}")
            .is_err_and(|err| matches!(err, ParseError::Dicotic(_)))
    );

    let g = context.from_str("{|{0|}}").unwrap();
    assert!(!context.is_dead_ending(&g));
    assert!(
        DeadEndingFormContext::new(StandardFormContext)
            .from_str("{|{0|}}")
            .is_err()
    );
}
