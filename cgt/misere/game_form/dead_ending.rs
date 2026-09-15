#![allow(missing_docs)]

use crate::{
    misere::game_form::{
        BlockingContext, ConstructionError, GameFormContext, Outcome, StandardForm,
        StandardFormContext,
    },
    short::partizan::Player,
    total::TotalWrappable,
};
use std::{error::Error, fmt};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DeadEndingFormContext<C> {
    context: C,
}

impl<C> DeadEndingFormContext<C> {
    pub const fn new(context: C) -> Self {
        Self { context }
    }

    pub const fn underlying(&self) -> &C {
        &self.context
    }
}

#[derive(Debug, Clone)]
#[repr(transparent)]
pub struct DeadEndingForm<G> {
    underlying: G,
}

impl<G> DeadEndingForm<G> {
    pub(crate) const fn new_unchecked(underlying: G) -> DeadEndingForm<G> {
        DeadEndingForm { underlying }
    }

    pub(crate) const fn new_ref_unchecked(underlying: &G) -> &DeadEndingForm<G> {
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

impl<G> TotalWrappable for DeadEndingForm<G>
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
pub enum DeadEndingConstructionError<G, E> {
    NotDeadEnding(G),
    Underlying(E),
}

// NOTE: G: Display will be annoying for interner context
impl<G, E> std::fmt::Display for DeadEndingConstructionError<G, E>
where
    G: std::fmt::Display,
    E: std::fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DeadEndingConstructionError::NotDeadEnding(g) => {
                write!(f, "form `{}` is not dead-ending", g)
            }
            DeadEndingConstructionError::Underlying(_) => {
                write!(f, "could not construct the underlying form")
            }
        }
    }
}

impl<G, E> Error for DeadEndingConstructionError<G, E>
where
    G: std::fmt::Debug + std::fmt::Display,
    E: std::fmt::Debug + Error + 'static,
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            DeadEndingConstructionError::NotDeadEnding(_) => None,
            DeadEndingConstructionError::Underlying(err) => Some(err),
        }
    }
}

impl<G, E> ConstructionError<G> for DeadEndingConstructionError<G, E>
where
    E: ConstructionError<G>,
    G: std::fmt::Debug,
{
    fn recover(self) -> G {
        match self {
            DeadEndingConstructionError::NotDeadEnding(g) => g,
            DeadEndingConstructionError::Underlying(err) => err.recover(),
        }
    }
}

impl<C> GameFormContext for DeadEndingFormContext<C>
where
    C: GameFormContext,
{
    type Form = DeadEndingForm<C::Form>;
    type BaseForm = C::BaseForm;

    type DicoticConstructionError =
        DeadEndingConstructionError<Self::BaseForm, C::DicoticConstructionError>;

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
            .map_err(DeadEndingConstructionError::Underlying)?;
        if self.context.is_dead_ending(&g) {
            Ok(DeadEndingForm::new_unchecked(g))
        } else {
            Err(DeadEndingConstructionError::NotDeadEnding(
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
            .map(DeadEndingForm::new_ref_unchecked)
    }

    fn is_p_free(&self, game: &Self::Form) -> bool {
        self.context.is_p_free(&game.underlying)
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

pub trait DeadEndingContext: GameFormContext {
    #[doc(alias = "perfect_murder")]
    fn waiting_protected(&self, rank: i32) -> Self::Form {
        let zero = self.new_integer(0).unwrap();
        let mut acc = self.new_integer(0).unwrap();

        if rank >= 0 {
            for _ in 0..rank {
                acc = self.new([], [zero.clone(), acc]).unwrap();
            }
        } else {
            for _ in 0..rank.abs() {
                acc = self.new([zero.clone(), acc], []).unwrap();
            }
        }

        acc
    }

    fn strong_player_outcome(&self, game: &Self::Form, player: Player) -> Player {
        let k = self.birthday(game) as i32;

        if k == 0 {
            return player;
        }

        match player {
            Player::Left => Ord::min(
                self.player_outcome(game, player),
                self.player_outcome(
                    &self.sum(game, &self.waiting_protected(k - 1)).unwrap(),
                    player,
                ),
            ),
            Player::Right => Ord::max(
                self.player_outcome(game, player),
                self.player_outcome(
                    &self.sum(game, &self.waiting_protected(-(k - 1))).unwrap(),
                    player,
                ),
            ),
        }
    }

    fn strong_outcome(&self, game: &Self::Form) -> Outcome {
        match (
            self.strong_player_outcome(game, Player::Left),
            self.strong_player_outcome(game, Player::Right),
        ) {
            (Player::Left, Player::Left) => Outcome::L,
            (Player::Left, Player::Right) => Outcome::N,
            (Player::Right, Player::Left) => Outcome::P,
            (Player::Right, Player::Right) => Outcome::R,
        }
    }

    fn satisfy_maintenance(&self, g: &Self::Form, h: &Self::Form) -> bool {
        let a = self.moves(g, Player::Right).all(|gr| {
            self.moves(gr, Player::Left)
                .any(|grl| self.ge_mod_dead_ending(grl, h))
                || self
                    .moves(h, Player::Right)
                    .any(|hr| self.ge_mod_dead_ending(gr, hr))
        });
        let b = self.moves(h, Player::Left).all(|hl| {
            self.moves(hl, Player::Right)
                .any(|hlr| self.ge_mod_dead_ending(g, hlr))
                || self
                    .moves(g, Player::Left)
                    .any(|gl| self.ge_mod_dead_ending(gl, hl))
        });

        a && b
    }

    fn satisfy_proviso(&self, g: &Self::Form, h: &Self::Form) -> bool {
        self.strong_outcome(g) >= self.strong_outcome(h)
    }

    fn ge_mod_dead_ending(&self, g: &Self::Form, h: &Self::Form) -> bool {
        self.satisfy_proviso(g, h) && self.satisfy_maintenance(g, h)
    }

    fn eq_mod_dead_ending(&self, g: &Self::Form, h: &Self::Form) -> bool {
        self.ge_mod_dead_ending(g, h) && self.ge_mod_dead_ending(h, g)
    }
}

impl<C> DeadEndingContext for DeadEndingFormContext<C> where C: GameFormContext {}

impl<C> BlockingContext for DeadEndingFormContext<C> where C: GameFormContext {}

impl std::fmt::Display for DeadEndingForm<StandardForm> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", StandardFormContext.display(&self.underlying))
    }
}

#[test]
fn waiting_protected() {
    let context = DeadEndingFormContext::new(StandardFormContext);

    let m0 = context.waiting_protected(0);
    assert_eq!(m0.to_string(), "0");

    let m1 = context.waiting_protected(1);
    assert_eq!(m1.to_string(), "-1");

    let m4 = context.waiting_protected(4);
    assert_eq!(m4.to_string(), "{|0,{|0,{|0,-1}}}");

    let m4_conj = context.waiting_protected(-4);
    assert_eq!(m4_conj.to_string(), "{0,{0,{0,1|}|}|}");
}

#[test]
fn relations() {
    let context = DeadEndingFormContext::new(StandardFormContext);

    let g = context.from_str("{2|4}").unwrap();
    let h = context.from_str("{2|}").unwrap();
    assert!(context.eq_mod_dead_ending(&g, &h));

    let g = context.from_str("1").unwrap();
    let h = context.from_str("{0|4}").unwrap();
    assert!(context.ge_mod_dead_ending(&g, &h));
}

#[test]
fn parsing() {
    use crate::misere::game_form::ParseError;
    let context = DeadEndingFormContext::new(StandardFormContext);

    assert!(
        context
            .from_str("{|{|1}}")
            .is_err_and(|err| matches!(err, ParseError::Dicotic(_)))
    );
}
