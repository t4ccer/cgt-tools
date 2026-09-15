#![allow(missing_docs)]

use crate::{
    misere::game_form::{
        BlockingContext, BlockingForm, BlockingFormContext, ConstructionError, DeadEndingContext,
        DeadEndingForm, DeadEndingFormContext, GameFormContext, Outcome, StandardForm,
        StandardFormContext,
    },
    result::{UnwrapInfallible, Void},
    short::partizan::Player,
    total::TotalWrappable,
};
use std::{convert::Infallible, error::Error, fmt};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PFreeFormContext<C> {
    context: C,
}

impl<C> PFreeFormContext<C> {
    pub const fn new(context: C) -> Self {
        Self { context }
    }

    pub const fn underlying(&self) -> &C {
        &self.context
    }
}

#[derive(Debug, Clone)]
#[repr(transparent)]
pub struct PFreeForm<G> {
    underlying: G,
}

impl<G> PFreeForm<G> {
    pub(crate) const fn new_unchecked(underlying: G) -> PFreeForm<G> {
        PFreeForm { underlying }
    }

    pub(crate) const fn new_ref_unchecked(underlying: &G) -> &PFreeForm<G> {
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

impl<G> TotalWrappable for PFreeForm<G>
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

pub trait PFreeContext: GameFormContext
where
    Self::IntegerConstructionError: Void,
{
    // FIXME: Use base forms instead of unwraps

    fn tipping_point(&self, game: &Self::Form, player: Player) -> u32 {
        match player {
            Player::Left => {
                let mut n = 0;
                loop {
                    if self.outcome(
                        &self
                            .sum(game, &self.new_integer(-(n as i32)).unwrap_infallible())
                            .unwrap(),
                    ) == Outcome::L
                    {
                        break n;
                    }
                    n += 1;
                }
            }
            Player::Right => {
                let mut n = 0;
                loop {
                    if self.outcome(
                        &self
                            .sum(game, &self.new_integer(n as i32).unwrap_infallible())
                            .unwrap(),
                    ) == Outcome::R
                    {
                        break n;
                    }
                    n += 1;
                }
            }
        }
    }

    fn left_tipping_point(&self, game: &Self::Form) -> u32 {
        for n in 0.. {
            let g = self
                .sum(game, &self.new_integer(-(n as i32)).unwrap_infallible())
                .unwrap();
            if self.outcome(&g) == Outcome::L {
                return n;
            }
        }

        unreachable!()
    }

    fn right_tipping_point(&self, game: &Self::Form) -> u32 {
        for n in 0.. {
            let g = self
                .sum(game, &self.new_integer(n as i32).unwrap_infallible())
                .unwrap();
            if self.outcome(&g) == Outcome::R {
                return n;
            }
        }

        unreachable!()
    }

    fn next_tipping_point(&self, game: &Self::Form) -> u32 {
        for n in 0.. {
            let g = self
                .sum(game, &self.new_integer(n as i32).unwrap_infallible())
                .unwrap();
            if self.outcome(&g) == Outcome::N {
                return n;
            }

            let g = self
                .sum(game, &self.new_integer(-(n as i32)).unwrap_infallible())
                .unwrap();
            if self.outcome(&g) == Outcome::N {
                return n;
            }
        }

        unreachable!()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PFreeConstructionError<G, E> {
    NotPFree(G),
    Underlying(E),
}

// NOTE: G: Display will be annoying for interner context
impl<G, E> std::fmt::Display for PFreeConstructionError<G, E>
where
    G: std::fmt::Display,
    E: std::fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PFreeConstructionError::NotPFree(g) => {
                write!(f, "form `{}` is not P-free", g)
            }
            PFreeConstructionError::Underlying(_) => {
                write!(f, "could not construct the underlying form")
            }
        }
    }
}

impl<G, E> Error for PFreeConstructionError<G, E>
where
    G: std::fmt::Debug + std::fmt::Display,
    E: std::fmt::Debug + Error + 'static,
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            PFreeConstructionError::NotPFree(_) => None,
            PFreeConstructionError::Underlying(err) => Some(err),
        }
    }
}

impl<G, E> ConstructionError<G> for PFreeConstructionError<G, E>
where
    G: std::fmt::Debug,
    E: ConstructionError<G>,
{
    fn recover(self) -> G {
        match self {
            PFreeConstructionError::NotPFree(g) => g,
            PFreeConstructionError::Underlying(err) => err.recover(),
        }
    }
}

impl<C> GameFormContext for PFreeFormContext<C>
where
    C: GameFormContext<IntegerConstructionError = Infallible>,
{
    type Form = PFreeForm<C::Form>;
    type BaseForm = C::BaseForm;

    type DicoticConstructionError =
        PFreeConstructionError<Self::BaseForm, C::DicoticConstructionError>;

    type IntegerConstructionError = Infallible;

    type ConjugateConstructionError = C::ConjugateConstructionError;

    type SumConstructionError = PFreeConstructionError<Self::BaseForm, C::SumConstructionError>;

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
            .map_err(PFreeConstructionError::Underlying)?;
        if self.underlying().is_p_free(&g) {
            Ok(PFreeForm::new_unchecked(g))
        } else {
            Err(PFreeConstructionError::NotPFree(self.underlying().base(g)))
        }
    }

    fn moves<'a>(
        &self,
        game: &'a Self::Form,
        player: Player,
    ) -> impl Iterator<Item = &'a Self::Form> {
        self.context
            .moves(&game.underlying, player)
            .map(PFreeForm::new_ref_unchecked)
    }

    fn is_p_free(&self, _game: &Self::Form) -> bool {
        true
    }

    fn is_dead_ending(&self, game: &Self::Form) -> bool {
        self.context.is_dead_ending(&game.underlying)
    }

    fn is_blocking(&self, game: &Self::Form) -> bool {
        self.context.is_blocking(&game.underlying)
    }

    fn total_cmp(&self, lhs: &Self::Form, rhs: &Self::Form) -> std::cmp::Ordering {
        self.context.total_cmp(&lhs.underlying, &rhs.underlying)
    }

    fn total_eq(&self, lhs: &Self::Form, rhs: &Self::Form) -> bool {
        self.context.total_eq(&lhs.underlying, &rhs.underlying)
    }

    fn sum(
        &self,
        g: &Self::Form,
        h: &Self::Form,
    ) -> Result<Self::Form, Self::SumConstructionError> {
        let g = self
            .context
            .sum(&g.underlying, &h.underlying)
            .map_err(PFreeConstructionError::Underlying)?;
        if self.underlying().is_p_free(&g) {
            Ok(PFreeForm::new_unchecked(g))
        } else {
            Err(PFreeConstructionError::NotPFree(self.underlying().base(g)))
        }
    }

    fn base(&self, game: Self::Form) -> Self::BaseForm {
        self.underlying().base(game.to_underlying())
    }

    fn base_context(&self) -> &impl GameFormContext<Form = Self::BaseForm> {
        self.underlying().base_context()
    }
}

impl<C> PFreeContext for PFreeFormContext<C> where
    C: GameFormContext<IntegerConstructionError = Infallible>
{
}

impl<C> DeadEndingContext for PFreeFormContext<C> where
    C: DeadEndingContext<IntegerConstructionError = Infallible>
{
}

impl<C> BlockingContext for PFreeFormContext<C> where
    C: BlockingContext<IntegerConstructionError = Infallible>
{
}

impl std::fmt::Display for PFreeForm<StandardForm> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", StandardFormContext.display(&self.underlying))
    }
}

impl std::fmt::Display for PFreeForm<DeadEndingForm<StandardForm>> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            DeadEndingFormContext::new(StandardFormContext).display(&self.underlying)
        )
    }
}

impl std::fmt::Display for PFreeForm<BlockingForm<StandardForm>> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            BlockingFormContext::new(StandardFormContext).display(&self.underlying)
        )
    }
}
