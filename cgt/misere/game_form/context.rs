use crate::{
    misere::game_form::Outcome,
    parsing::{Parser, lexeme},
    short::partizan::Player,
};
use std::{cmp::Ordering, convert::Infallible, error::Error};

/// Propagated error that occurred during [`GameFormContext::arbitrary`]
#[cfg(any(test, feature = "quickcheck"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ArbitraryError<Dicotic, Integer> {
    /// Generated game form could not be constructed from Left and Right options
    Dicotic(Dicotic),

    /// Generated integer could be be constructed as a game form
    Integer(Integer),
}

#[cfg(any(test, feature = "quickcheck"))]
impl<G, Dicotic, Integer> ConstructionError<G> for ArbitraryError<Dicotic, Integer>
where
    Dicotic: ConstructionError<G>,
    Integer: ConstructionError<G>,
{
    fn recover(self) -> G {
        match self {
            ArbitraryError::Dicotic(err) => err.recover(),
            ArbitraryError::Integer(err) => err.recover(),
        }
    }
}

#[cfg(any(test, feature = "quickcheck"))]
impl<Dicotic, Integer> crate::result::Void for ArbitraryError<Dicotic, Integer>
where
    Dicotic: crate::result::Void,
    Integer: crate::result::Void,
{
    fn absurd<T>(self) -> T {
        match self {
            ArbitraryError::Dicotic(err) => crate::result::Void::absurd(err),
            ArbitraryError::Integer(err) => crate::result::Void::absurd(err),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ParseError<Dicotic, Integer> {
    /// Parsed form is invalid
    Dicotic(Dicotic),

    /// Parsed integer would be an invalid form
    Integer(Integer),

    /// Malformed input
    MalformedInput,
}

impl<Dicotic, Integer> std::fmt::Display for ParseError<Dicotic, Integer>
where
    Dicotic: std::fmt::Display,
    Integer: std::fmt::Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::Dicotic(err) => write!(f, "parse error: {}", err),
            ParseError::Integer(err) => write!(f, "parse error: {}", err),
            ParseError::MalformedInput => write!(f, "parse error: malformed input"),
        }
    }
}

impl<Dicotic, Integer> Error for ParseError<Dicotic, Integer>
where
    Dicotic: Error + 'static,
    Integer: Error + 'static,
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            ParseError::Dicotic(err) => Some(err),
            ParseError::Integer(err) => Some(err),
            ParseError::MalformedInput => None,
        }
    }
}

/// Error that occurred when constructing a game form
///
/// It allows to recover the underlying game form if the construction would result in a game that
/// is not included in the restricted set we are working in.
pub trait ConstructionError<G>: std::fmt::Debug {
    fn recover(self) -> G;
}

impl<G> ConstructionError<G> for Infallible {
    fn recover(self) -> G {
        match self {}
    }
}

/// Context for unrestricted game forms
///
/// Context pattern allows to abstract away how the game forms are stored and managed in memory
/// (inline, interning strategy, etc.) and to cache operation results if desired
#[allow(clippy::missing_errors_doc)]
pub trait GameFormContext {
    /// Game form, potentially restricted to some subset
    type Form: Clone;

    /// Unrestricted game form, closed under all usual game operations
    type BaseForm: Clone + std::fmt::Debug;

    // TODO: Rename as it is not actually dicotic as we allow empty iterators in `new`
    type DicoticConstructionError: ConstructionError<Self::BaseForm>;
    type IntegerConstructionError: ConstructionError<Self::BaseForm>;
    type ConjugateConstructionError: ConstructionError<Self::BaseForm>;
    type SumConstructionError: ConstructionError<Self::BaseForm>;

    /// Construct new game form from Left and Right options
    ///
    /// # Example
    /// ```
    /// # use crate::cgt::misere::game_form::GameFormContext;
    /// # let context = &cgt::misere::game_form::StandardFormContext;
    /// let g = context
    ///     .new(
    ///         [context.from_str("{|0}").unwrap()],
    ///         [context.from_str("{1,2|}").unwrap()],
    ///     )
    ///     .unwrap();    
    /// assert_eq!(context.to_string(&g), "{-1|{1,2|}}");
    /// ```
    #[allow(clippy::wrong_self_convention)]
    fn new(
        &self,
        left: impl IntoIterator<Item = Self::Form>,
        right: impl IntoIterator<Item = Self::Form>,
    ) -> Result<Self::Form, Self::DicoticConstructionError>;

    /// Get player moves
    ///
    /// # Example
    /// ```
    /// # use crate::cgt::{misere::game_form::GameFormContext, short::partizan::Player};
    /// # let context = &cgt::misere::game_form::StandardFormContext;
    /// let g = context.from_str("{1,2|{0|0}}").unwrap();
    /// assert_eq!(
    ///     context
    ///         .moves(&g, Player::Left)
    ///         .map(|gl| context.to_string(gl))
    ///         .collect::<Vec<_>>(),
    ///     vec!["1", "2"]
    /// );
    /// assert_eq!(
    ///     context
    ///         .moves(&g, Player::Right)
    ///         .map(|gl| context.to_string(gl))
    ///         .collect::<Vec<_>>(),
    ///     vec!["{0|0}"]
    /// );
    /// ```
    fn moves<'a>(
        &self,
        game: &'a Self::Form,
        player: Player,
    ) -> impl Iterator<Item = &'a Self::Form>;

    /// Construct new game form equal to the given integer i.e. for `n > 0` `n + 1 = {n|}`
    /// and analogously for Right.
    ///
    /// # Example
    /// ```
    /// # use crate::cgt::{misere::game_form::GameFormContext, short::partizan::Player};
    /// # let context = &cgt::misere::game_form::StandardFormContext;
    /// let g = context.new_integer(42).unwrap();
    /// assert_eq!(
    ///     context
    ///         .moves(&g, Player::Left)
    ///         .map(|gl| context.to_string(gl))
    ///         .collect::<Vec<_>>(),
    ///     vec!["41"]
    /// );
    /// assert_eq!(context.moves(&g, Player::Right).count(), 0);
    /// ```
    fn new_integer(&self, n: i32) -> Result<Self::Form, Self::IntegerConstructionError> {
        use std::cmp::Ordering;

        match n.cmp(&0) {
            Ordering::Less => {
                let previous = self.new_integer(n + 1)?;
                Ok(self.new([], [previous]).unwrap())
            }
            Ordering::Equal => Ok(self.new([], []).unwrap()),
            Ordering::Greater => {
                let previous = self.new_integer(n - 1)?;
                Ok(self.new([previous], []).unwrap())
            }
        }
    }

    /// If the game is an integer, extract it.
    ///
    /// # Example
    /// ```
    /// # use crate::cgt::misere::game_form::GameFormContext;
    /// # let context = &cgt::misere::game_form::StandardFormContext;
    /// let g = context.from_str("{{1|}|}").unwrap();
    /// assert_eq!(context.to_integer(&g), Some(3));
    /// ```
    fn to_integer(&self, game: &Self::Form) -> Option<i32> {
        let mut left = self.moves(game, Player::Left);
        let l1 = left.next();
        let l2 = left.next();

        let mut right = self.moves(game, Player::Right);
        let r1 = right.next();
        let r2 = right.next();

        match (l1, l2, r1, r2) {
            (None, _, None, _) => Some(0),
            (Some(gl), None, None, None) => {
                let prev = self.to_integer(gl)?;
                (prev >= 0).then_some(prev + 1)
            }
            (None, None, Some(gr), None) => {
                let prev = self.to_integer(gr)?;
                (prev <= 0).then_some(prev - 1)
            }
            _ => None,
        }
    }

    /// Return which player wins if passed `player` would to go first
    ///
    /// # Example
    /// ```
    /// # use cgt::{misere::game_form::GameFormContext, short::partizan::Player};
    /// # let context = &cgt::misere::game_form::StandardFormContext;
    /// let g = context.from_str("{1,2|{0|0}}").unwrap();
    /// assert_eq!(context.player_outcome(&g, Player::Left), Player::Right);
    /// assert_eq!(context.player_outcome(&g, Player::Right), Player::Right);
    /// ```
    fn player_outcome(&self, game: &Self::Form, player: Player) -> Player {
        if self.wins_going_first(game, player) {
            player
        } else {
            player.opposite()
        }
    }

    /// Check if passed `player` would win when going first
    ///
    /// # Example
    /// ```
    /// # use cgt::{misere::game_form::GameFormContext, short::partizan::Player};
    /// # let context = &cgt::misere::game_form::StandardFormContext;
    /// let g = context.from_str("{1,2|{0|0}}").unwrap();
    /// assert!(!context.wins_going_first(&g, Player::Left));
    /// assert!(context.wins_going_first(&g, Player::Right));
    /// ```
    fn wins_going_first(&self, game: &Self::Form, player: Player) -> bool {
        self.moves(game, player).count() == 0
            || self
                .moves(game, player)
                .any(|g| !self.wins_going_first(g, player.opposite()))
    }

    /// Return outcome of the game, no matter who goes first
    ///
    /// # Example
    /// ```
    /// # use cgt::{misere::game_form::{GameFormContext, Outcome}, short::partizan::Player};
    /// # let context = &cgt::misere::game_form::StandardFormContext;
    /// let g = context.from_str("{1,2|{0|0}}").unwrap();
    /// assert_eq!(context.outcome(&g), Outcome::R);
    /// ```
    fn outcome(&self, game: &Self::Form) -> Outcome {
        match (
            self.wins_going_first(game, Player::Left),
            self.wins_going_first(game, Player::Right),
        ) {
            (true, true) => Outcome::N,
            (true, false) => Outcome::L,
            (false, true) => Outcome::R,
            (false, false) => Outcome::P,
        }
    }

    fn is_p_free(&self, game: &Self::Form) -> bool {
        (self.outcome(game) != Outcome::P)
            && Player::forall(|p| self.moves(game, p).all(|g| self.is_p_free(g)))
    }

    fn is_end(&self, game: &Self::Form, player: Player) -> bool {
        self.moves(game, player).count() == 0
    }

    fn is_dead_end(&self, game: &Self::Form, player: Player) -> bool {
        self.is_end(game, player)
            && self
                .moves(game, player.opposite())
                .all(|g| self.is_dead_end(g, player))
    }

    fn is_dead_ending(&self, game: &Self::Form) -> bool {
        Player::forall(|p| !self.is_end(game, p) || self.is_dead_end(game, p))
            && Player::forall(|p| self.moves(game, p).all(|g| self.is_dead_ending(g)))
    }

    fn is_blocked_end(&self, game: &Self::Form, p: Player) -> bool {
        self.is_end(game, p)
            && self.moves(game, p.opposite()).all(|gr| {
                self.is_blocked_end(gr, p)
                    || self.moves(gr, p).any(|grl| self.is_blocked_end(grl, p))
            })
    }

    fn is_blocking(&self, game: &Self::Form) -> bool {
        Player::forall(|p| !self.is_end(game, p) || self.is_blocked_end(game, p))
            && Player::forall(|p| self.moves(game, p).all(|g| self.is_blocking(g)))
    }

    /// Implementation specific total comparison function
    ///
    /// Intended to be used when storing game forms in data structures that require
    /// total ordering that can be different from game equality
    fn total_cmp(&self, lhs: &Self::Form, rhs: &Self::Form) -> Ordering;

    /// Implementation specific total equality function
    fn total_eq(&self, lhs: &Self::Form, rhs: &Self::Form) -> bool;

    /// Construct all games born in the next day from the specified options.
    ///
    /// Note that the implementation may filter out forms that would be invalid.
    ///
    /// Example
    /// ```
    /// # use cgt::misere::game_form::GameFormContext;
    /// # let context = &cgt::misere::game_form::StandardFormContext;
    /// let day1 = context
    ///     .next_day(&[context.new_integer(0).unwrap()])
    ///     .map(|g| context.to_string(&g))
    ///     .collect::<Vec<_>>();
    /// assert_eq!(day1, vec!["0", "-1", "1", "{0|0}"]);
    /// ```
    fn next_day(&self, day: &[Self::Form]) -> impl Iterator<Item = Self::Form> {
        use itertools::Itertools;

        day.iter().powerset().flat_map(move |left_moves| {
            day.iter().powerset().filter_map(move |right_moves| {
                self.new(
                    left_moves.clone().into_iter().cloned().collect::<Vec<_>>(),
                    right_moves.into_iter().cloned().collect::<Vec<_>>(),
                )
                .ok()
            })
        })
    }

    fn conjugate(&self, game: &Self::Form) -> Result<Self::Form, Self::ConjugateConstructionError> {
        Ok(self
            .new(
                self.moves(game, Player::Right)
                    .map(|gl| self.conjugate(gl))
                    .collect::<Result<Vec<_>, _>>()?,
                self.moves(game, Player::Left)
                    .map(|gr| self.conjugate(gr))
                    .collect::<Result<Vec<_>, _>>()?,
            )
            .unwrap())
    }

    fn sum(
        &self,
        g: &Self::Form,
        h: &Self::Form,
    ) -> Result<Self::Form, Self::SumConstructionError> {
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

        Ok(self.new(left, right).unwrap())
    }

    fn birthday(&self, game: &Self::Form) -> u32 {
        self.moves(game, Player::Left)
            .chain(self.moves(game, Player::Right))
            .map(|g| self.birthday(g) + 1)
            .max()
            .unwrap_or(0)
    }

    fn parse_list<'a>(
        &'a self,
        mut p: Parser<'a>,
    ) -> Result<
        (Parser<'a>, Vec<Self::Form>),
        ParseError<Self::DicoticConstructionError, Self::IntegerConstructionError>,
    > {
        let mut acc = Vec::new();
        loop {
            p = p.trim_whitespace();
            match self.parse(p) {
                Ok((cf_p, cf)) => {
                    acc.push(cf);
                    p = cf_p;
                    p = p.trim_whitespace();
                    match p.parse_ascii_char(',') {
                        Some(pp) => {
                            p = pp.trim_whitespace();
                        }
                        None => return Ok((p, acc)),
                    }
                }
                Err(ParseError::MalformedInput) => return Ok((p, acc)),
                Err(err) => return Err(err),
            }
        }
    }

    fn parse<'a>(
        &'a self,
        p: Parser<'a>,
    ) -> Result<
        (Parser<'a>, Self::Form),
        ParseError<Self::DicoticConstructionError, Self::IntegerConstructionError>,
    > {
        let p = p.trim_whitespace();
        if let Some(p) = p.parse_ascii_char('{') {
            let (p, left) = self.parse_list(p)?;
            let p = p.parse_ascii_char('|').ok_or(ParseError::MalformedInput)?;
            let (p, right) = self.parse_list(p)?;
            let p = p.parse_ascii_char('}').ok_or(ParseError::MalformedInput)?;
            let p = p.trim_whitespace();
            Ok((p, self.new(left, right).map_err(ParseError::Dicotic)?))
        } else {
            // TODO: Generalize number parsers
            let (p, integer) = lexeme!(p, Parser::parse_i64).ok_or(ParseError::MalformedInput)?;
            Ok((
                p,
                self.new_integer(integer as i32)
                    .map_err(ParseError::Integer)?,
            ))
        }
    }

    #[allow(clippy::wrong_self_convention)]
    fn from_str(
        &self,
        input: &str,
    ) -> Result<
        Self::Form,
        ParseError<Self::DicoticConstructionError, Self::IntegerConstructionError>,
    > {
        let (_, g) = self.parse(Parser::new(input))?;
        Ok(g)
    }

    fn to_string(&self, game: &Self::Form) -> String {
        self.display(game).to_string()
    }

    fn display<'a>(&'a self, game: &'a Self::Form) -> impl std::fmt::Display + 'a {
        DisplayGame {
            context: self,
            form: game,
            format: DisplayFormat::Plain,
        }
    }

    fn display_tex<'a>(&'a self, game: &'a Self::Form) -> impl std::fmt::Display + 'a {
        DisplayGame {
            context: self,
            form: game,
            format: DisplayFormat::Tex,
        }
    }

    fn base(&self, game: Self::Form) -> Self::BaseForm;

    fn base_context(&self) -> &impl GameFormContext<Form = Self::BaseForm>;

    #[cfg(any(test, feature = "quickcheck"))]
    fn arbitrary_sized(
        &self,
        g: &mut quickcheck::Gen,
        size: i64,
    ) -> Result<
        Self::Form,
        ArbitraryError<Self::DicoticConstructionError, Self::IntegerConstructionError>,
    > {
        use quickcheck::Arbitrary;

        if size < 1 {
            return self.new_integer(0).map_err(ArbitraryError::Integer);
        }

        let is_integer = u8::arbitrary(g) < 64;
        if is_integer {
            let n = i64::arbitrary(g).rem_euclid(size);
            if bool::arbitrary(g) {
                self.new_integer(n as i32).map_err(ArbitraryError::Integer)
            } else {
                self.new_integer(-n as i32).map_err(ArbitraryError::Integer)
            }
        } else {
            let mut mk_player = || {
                let size = i64::arbitrary(g).rem_euclid(size);
                (0..size)
                    .filter_map(|_| self.arbitrary_sized(g, size - 1).ok())
                    .collect::<Vec<_>>()
            };
            let left = mk_player();
            let right = mk_player();

            self.new(left, right).map_err(ArbitraryError::Dicotic)
        }
    }

    #[cfg(any(test, feature = "quickcheck"))]
    fn arbitrary(
        &self,
        g: &mut quickcheck::Gen,
    ) -> Result<
        Self::Form,
        ArbitraryError<Self::DicoticConstructionError, Self::IntegerConstructionError>,
    > {
        self.arbitrary_sized(g, g.size() as i64)
    }

    #[cfg(any(test, feature = "quickcheck"))]
    #[allow(clippy::option_if_let_else)] // Needed for Box<&dyn>
    fn shrink<'a>(&'a self, game: &'a Self::Form) -> Box<dyn Iterator<Item = Self::Form> + 'a> {
        use itertools::Itertools;
        use std::cmp::Ordering;

        match self.to_integer(game) {
            Some(n) => match n.cmp(&0) {
                Ordering::Less => Box::new(((n + 1)..=0).filter_map(|n| self.new_integer(n).ok())),
                Ordering::Equal => Box::new(std::iter::empty()),
                Ordering::Greater => {
                    Box::new((0..n).rev().filter_map(|n| self.new_integer(n).ok()))
                }
            },
            None => {
                let left = (0..self.moves(game, Player::Left).count()).flat_map(|n| {
                    self.moves(game, Player::Left)
                        .collect::<Vec<_>>()
                        .into_iter()
                        .combinations(n)
                });
                let right = (0..self.moves(game, Player::Right).count()).flat_map(|n| {
                    self.moves(game, Player::Right)
                        .collect::<Vec<_>>()
                        .into_iter()
                        .combinations(n)
                });
                Box::new(left.cartesian_product(right).filter_map(|(left, right)| {
                    self.new(left.into_iter().cloned(), right.into_iter().cloned())
                        .ok()
                }))
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum DisplayFormat {
    Plain,
    Tex,
}

/// Opaque helper type to use game forms in format strings
struct DisplayGame<'a, C>
where
    C: GameFormContext + ?Sized,
{
    context: &'a C,
    form: &'a C::Form,
    format: DisplayFormat,
}

impl<C> std::fmt::Display for DisplayGame<'_, C>
where
    C: GameFormContext + ?Sized,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.context.to_integer(self.form) {
            Some(n) => write!(f, "{n}"),
            None => {
                match self.format {
                    DisplayFormat::Plain => write!(f, "{{")?,
                    DisplayFormat::Tex => write!(f, "\\left\\{{")?,
                }

                for (idx, gl) in self.context.moves(self.form, Player::Left).enumerate() {
                    if idx > 0 {
                        write!(f, ",")?;
                    }
                    DisplayGame {
                        context: self.context,
                        form: gl,
                        format: self.format,
                    }
                    .fmt(f)?;
                }

                match self.format {
                    DisplayFormat::Plain => write!(f, "|")?,
                    DisplayFormat::Tex => write!(f, " \\mid ")?,
                }

                for (idx, gr) in self.context.moves(self.form, Player::Right).enumerate() {
                    if idx > 0 {
                        write!(f, ",")?;
                    }
                    DisplayGame {
                        context: self.context,
                        form: gr,
                        format: self.format,
                    }
                    .fmt(f)?;
                }

                match self.format {
                    DisplayFormat::Plain => write!(f, "}}")?,
                    DisplayFormat::Tex => write!(f, "\\right\\}}")?,
                }

                Ok(())
            }
        }
    }
}

#[test]
fn parsing() {
    use crate::misere::game_form::GameFormContext;
    let context = &crate::misere::game_form::StandardFormContext;

    assert!(context.from_str("{{2|{|}},1|{0|0}}").is_ok());
    assert!(context.from_str("{|{|1}}").is_ok());
    assert!(
        context
            .from_str("  {  {  2  |  {  | }  }  ,  1  |  {  0  |  0  }  }  ")
            .is_ok()
    );
}
