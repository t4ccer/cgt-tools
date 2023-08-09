//! Shared canonical form backend

// TODO: Find better module name

use crate::{
    numeric::dyadic_rational_number::DyadicRationalNumber, numeric::nimber::Nimber,
    numeric::rational::Rational, rw_hash_map::RwHashMap, short::partizan::thermograph::Thermograph,
    short::partizan::trajectory::Trajectory,
};
use elsa::sync::FrozenVec;
use std::{
    cmp::Ordering,
    fmt::{self, Display, Write},
    hash::Hash,
    ops::{Add, Neg},
    str::FromStr,
    sync::Mutex,
};

/// A pointer to a game form. Must be used with backend that created it
#[derive(Debug, Hash, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct GamePtr(usize);

/// Canonical game form
#[derive(Debug, Hash, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Game {
    /// Number Up Star sum
    Nus(Nus),

    /// Not a NUS - pointer to list of left/right moves
    MovesPtr(GamePtr),
}

impl Game {
    #[inline]
    fn get_nus_unchecked(self) -> Nus {
        match self {
            Game::Nus(nus) => nus,
            Game::MovesPtr(_) => panic!("Not a nus"),
        }
    }

    /// Check if game is a Number Up Star sum
    pub fn is_number_up_star(&self) -> bool {
        matches!(self, Game::Nus(_))
    }

    /// Check if a game is only a number
    pub fn is_number(&self) -> bool {
        match self {
            Game::Nus(nus) => nus.is_number(),
            Game::MovesPtr(_) => false,
        }
    }

    /// Check if a game is only a nimber
    pub fn is_nimber(&self) -> bool {
        match self {
            Game::Nus(nus) => nus.is_nimber(),
            Game::MovesPtr(_) => false,
        }
    }
}

/// A number-up-star game position that is a sum of a number, up and, nimber.
#[derive(Debug, Hash, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Nus {
    number: DyadicRationalNumber,
    up_multiple: i32,
    nimber: Nimber,
}

impl Nus {
    pub(crate) fn parse(input: &str) -> nom::IResult<&str, Self> {
        use nom::{
            character::complete::{char, i32, one_of, u32},
            error::ErrorKind,
        };

        let (input, number) = match DyadicRationalNumber::parser(input) {
            Ok((input, number)) => (input, number),
            Err(_) => (input, DyadicRationalNumber::from(0)),
        };

        let (input, up_multiple) = match one_of::<_, _, (&str, ErrorKind)>("^v")(input) {
            Ok((input, chr)) => {
                let (input, up_multiple) = i32::<_, (&str, ErrorKind)>(input).unwrap_or((input, 1));
                (
                    input,
                    if chr == 'v' {
                        -up_multiple
                    } else {
                        up_multiple
                    },
                )
            }
            Err(_) => (input, 0),
        };

        let (input, star_multiple) = match char::<_, (&str, ErrorKind)>('*')(input) {
            Ok((input, _)) => u32::<_, (&str, ErrorKind)>(input).unwrap_or((input, 1)),
            Err(_) => (input, 0),
        };

        Ok((
            input,
            Nus {
                number,
                up_multiple,
                nimber: Nimber::from(star_multiple),
            },
        ))
    }
}

impl FromStr for Nus {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Nus::parse(s).map(|(_, nus)| nus).map_err(|_| ())
    }
}

#[cfg(test)]
fn test_nus(inp: &str) {
    assert_eq!(
        &format!(
            "{}",
            Nus::from_str(inp).expect(&format!("Could not parse: '{}'", inp))
        ),
        inp
    );
}

#[test]
fn parse_nus() {
    test_nus("42");
    test_nus("-8");
    test_nus("13^");
    test_nus("123v");
    test_nus("13^3");
    test_nus("123v58");
    test_nus("13^3*");
    test_nus("123v58*");
    test_nus("13^3*8");
    test_nus("123v58*34");
    test_nus("-13^3*");
    test_nus("-123v58*");
}

impl Nus {
    /// Create new number-up-star game equal to an integer.
    #[inline]
    pub fn integer(integer: i64) -> Self {
        Nus {
            number: DyadicRationalNumber::from(integer),
            up_multiple: 0,
            nimber: Nimber::from(0),
        }
    }

    /// Create new number-up-star game equal to an rational.
    #[inline]
    pub fn rational(number: DyadicRationalNumber) -> Self {
        Nus {
            number,
            up_multiple: 0,
            nimber: Nimber::from(0),
        }
    }

    /// Create new number-up-star game equal to an rational.
    #[inline]
    pub fn nimber(nimber: Nimber) -> Self {
        Nus {
            number: DyadicRationalNumber::from(0),
            up_multiple: 0,
            nimber,
        }
    }

    /// Check if the game has only number part (i.e. up multiple and nimber are zero).
    #[inline]
    pub fn is_number(&self) -> bool {
        self.up_multiple == 0 && self.nimber == Nimber::from(0)
    }

    /// Check if the game is a nimber.
    #[inline]
    pub fn is_nimber(&self) -> bool {
        self.number == DyadicRationalNumber::from(0) && self.up_multiple == 0
    }
}

impl Add for Nus {
    type Output = Nus;

    fn add(self, rhs: Nus) -> Nus {
        &self + &rhs
    }
}

impl Add for &Nus {
    type Output = Nus;

    fn add(self, rhs: &Nus) -> Nus {
        Nus {
            number: self.number + rhs.number,
            up_multiple: self.up_multiple + rhs.up_multiple,
            nimber: self.nimber + rhs.nimber,
        }
    }
}

impl Neg for Nus {
    type Output = Self;

    #[inline]
    fn neg(self) -> Self::Output {
        Nus {
            number: -self.number,
            up_multiple: -self.up_multiple,
            nimber: self.nimber, // Nimber is its own negative
        }
    }
}

impl Display for Nus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.number == DyadicRationalNumber::from(0)
            && self.up_multiple == 0
            && self.nimber == Nimber::from(0)
        {
            write!(f, "0")?;
            return Ok(());
        }

        if self.number != DyadicRationalNumber::from(0) {
            write!(f, "{}", self.number)?;
        }

        if self.up_multiple == 1 {
            write!(f, "^")?;
        } else if self.up_multiple == -1 {
            write!(f, "v")?;
        } else if self.up_multiple > 0 {
            write!(f, "^{}", self.up_multiple)?;
        } else if self.up_multiple < 0 {
            write!(f, "v{}", self.up_multiple.abs())?;
        }

        if self.nimber != Nimber::from(0) {
            write!(f, "{}", self.nimber)?;
        }

        Ok(())
    }
}

/// Left and Right moves from a given position
#[derive(Debug, Hash, Clone, PartialEq, Eq)]
pub struct Moves {
    /// Left player's moves
    pub left: Vec<Game>,

    /// Right player's moves
    pub right: Vec<Game>,
}

impl Moves {
    #[inline]
    fn empty() -> Self {
        Moves {
            left: vec![],
            right: vec![],
        }
    }

    fn eliminate_duplicates(&mut self) {
        self.left.sort();
        self.left.dedup();

        self.right.sort();
        self.right.dedup();
    }

    /// Try converting moves to NUS. Returns [None] if moves do not form a NUS
    pub fn to_nus(&self) -> Option<Nus> {
        let mut result = Nus::integer(0);

        let num_lo = self.left.len();
        let num_ro = self.right.len();

        if num_lo == 0 {
            if num_ro == 0 {
                // Case: {|}
                // No left or right moves so the game is 0
                result.number = DyadicRationalNumber::from(0);
            } else {
                // Case: n-1 = {|n}
                // We assume that entry is normalized, no left moves, thus there must be only one
                // right entry that's a number
                debug_assert!(num_ro == 1, "Entry not normalized");
                result.number =
                    self.right[0].get_nus_unchecked().number - DyadicRationalNumber::from(1);
            }
            result.up_multiple = 0;
            result.nimber = Nimber::from(0);
        } else if num_ro == 0 {
            // Case: n+1 = {n|}
            // No right options so there must be a left move that is a number
            debug_assert!(num_lo == 1, "Entry not normalized");
            result.number = self.left[0].get_nus_unchecked().number + DyadicRationalNumber::from(1);
            result.up_multiple = 0;
            result.nimber = Nimber::from(0);
        } else if num_lo == 1
            && num_ro == 1
            && self.left[0].is_number()
            && self.right[0].is_number()
            && self.left[0]
                .get_nus_unchecked()
                .number
                .cmp(&self.right[0].get_nus_unchecked().number)
                .is_lt()
        {
            // Case: {n|m}, n < m
            // We're a number but not an integer.  Conveniently, since the option lists are
            // canonicalized, the value of this game is the mean of its left & right moves.
            let l_num = self.left[0].get_nus_unchecked().number;
            let r_num = self.right[0].get_nus_unchecked().number;
            result.number = DyadicRationalNumber::mean(&l_num, &r_num);
            result.up_multiple = 0;
            result.nimber = Nimber::from(0);
        } else if num_lo == 2
            && num_ro == 1
            && self.left[0].is_number()
            && self.left[0] == self.right[0]
            && self.left[1].is_number_up_star()
            && self.left[0]
                .get_nus_unchecked()
                .number
                .cmp(&self.left[1].get_nus_unchecked().number)
                .is_eq()
            && self.left[1].get_nus_unchecked().up_multiple == 0
            && self.left[1].get_nus_unchecked().nimber == Nimber::from(1)
        {
            // Case: {G,H|G}
            result.number = self.left[0].get_nus_unchecked().number;
            result.up_multiple = 1;
            result.nimber = Nimber::from(1);
        } else if num_lo == 1
            && num_ro == 2
            && self.left[0].is_number()
            && self.left[0] == self.right[0]
            && self.right[1].is_number_up_star()
            && self.right[0]
                .get_nus_unchecked()
                .number
                .cmp(&self.right[1].get_nus_unchecked().number)
                .is_eq()
            && self.right[1].get_nus_unchecked().up_multiple == 0
            && self.right[1].get_nus_unchecked().nimber == Nimber::from(1)
        {
            // Inverse of the previous one
            result.number = self.right[0].get_nus_unchecked().number;
            result.up_multiple = -1;
            result.nimber = Nimber::from(1);
        } else if num_lo == 1
            && num_ro == 1
            && self.left[0].is_number()
            && self.right[0].is_number_up_star()
            && !self.right[0].is_number()
            && self.left[0]
                .get_nus_unchecked()
                .number
                .cmp(&self.right[0].get_nus_unchecked().number)
                .is_eq()
            && self.right[0].get_nus_unchecked().up_multiple >= 0
        {
            // Case: n + {0|G}, G is a number-up-star of up multiple >= 0
            result.number = self.left[0].get_nus_unchecked().number;
            result.up_multiple = self.right[0].get_nus_unchecked().up_multiple + 1;
            result.nimber = self.right[0].get_nus_unchecked().nimber + Nimber::from(1);
        } else if num_lo == 1
            && num_ro == 1
            && self.right[0].is_number()
            && self.left[0].is_number_up_star()
            && !self.left[0].is_number()
            && self.left[0]
                .get_nus_unchecked()
                .number
                .cmp(&self.right[0].get_nus_unchecked().number)
                .is_eq()
            && self.left[0].get_nus_unchecked().up_multiple <= 0
        {
            // Inverse of the previous one
            result.number = self.left[0].get_nus_unchecked().number;
            result.up_multiple = self.left[0].get_nus_unchecked().up_multiple - 1;
            result.nimber = self.left[0].get_nus_unchecked().nimber + Nimber::from(1);
        } else if num_lo >= 1
            && num_lo == num_ro
            && self.left[0].is_number()
            && self.left[0] == self.right[0]
        {
            // Case: n + *k
            // If doesn't hold then it's not a NUS
            for i in 0..num_lo {
                let l = self.left[i];
                let r = self.right[i];

                if l != r
                    || !l.is_number_up_star()
                    || l.get_nus_unchecked().number != r.get_nus_unchecked().number
                {
                    return None;
                }

                if l.get_nus_unchecked().up_multiple != 0
                    || l.get_nus_unchecked().nimber.value() != (i as u32)
                {
                    return None;
                }
            }
            // It's a nimber
            result.number = self.left[0].get_nus_unchecked().number;
            result.up_multiple = 0;
            result.nimber = Nimber::from(num_lo as u32);
        } else {
            return None;
        }

        Some(result)
    }
}

#[cfg(feature = "statistics")]
#[derive(Debug)]
pub struct BoundsTracker<T> {
    min_value: T,
    max_value: T,
}

#[cfg(feature = "statistics")]
impl<T> BoundsTracker<T>
where
    T: Ord + Copy,
{
    fn new(init: T) -> Self {
        BoundsTracker {
            min_value: init,
            max_value: init,
        }
    }

    fn update(&mut self, new_value: T) {
        self.min_value = std::cmp::min(self.min_value, new_value);
        self.max_value = std::cmp::max(self.max_value, new_value);
    }
}

#[cfg(feature = "statistics")]
impl<T> Display for BoundsTracker<T>
where
    T: Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}, {}]", self.min_value, self.max_value)
    }
}

#[cfg(feature = "statistics")]
#[derive(Debug)]
pub struct Statistics {
    max_rational_num: BoundsTracker<i64>,
    max_rational_den_exp: BoundsTracker<u32>,
    max_up: BoundsTracker<i32>,
    max_nimber: BoundsTracker<Nimber>,
}

#[cfg(feature = "statistics")]
impl Statistics {
    pub fn new() -> Self {
        Statistics {
            max_rational_num: BoundsTracker::new(0),
            max_rational_den_exp: BoundsTracker::new(0),
            max_up: BoundsTracker::new(0),
            max_nimber: BoundsTracker::new(Nimber::from(0)),
        }
    }
}

#[cfg(feature = "statistics")]
impl Display for Statistics {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "max_rational_num: {}, max_rational_den_exp: {}, max_up: {}, max_nimber: {}",
            self.max_rational_num, self.max_rational_den_exp, self.max_up, self.max_nimber
        )
    }
}

/// Shared game backend
pub struct GameBackend {
    /// Lock that **MUST** be taken when adding new game
    add_game_lock: Mutex<()>,
    /// Games that were already constructed
    known_games: FrozenVec<Box<Moves>>,
    /// Lookup table for list of moves
    moves_index: RwHashMap<Moves, Game>,
    /// Lookup table for game addition
    add_index: RwHashMap<(Game, Game), Game>,
    /// Lookup table for comparison
    leq_index: RwHashMap<(Game, Game), bool>,
    /// Lookup table for already constructed thermographs of non-trivial games
    thermograph_index: RwHashMap<Game, Thermograph>,

    #[cfg(feature = "statistics")]
    pub statistics: Mutex<Statistics>,
}

impl GameBackend {
    /// Initialize new game backend
    pub fn new() -> Self {
        Self {
            add_game_lock: Mutex::new(()),
            known_games: FrozenVec::new(),
            moves_index: RwHashMap::new(),
            add_index: RwHashMap::new(),
            leq_index: RwHashMap::new(),
            thermograph_index: RwHashMap::new(),
            #[cfg(feature = "statistics")]
            statistics: Mutex::new(Statistics::new()),
        }
    }

    #[cfg(feature = "statistics")]
    fn update_statistics(&self, moves: &Moves) {
        let mut lock = self.statistics.lock().unwrap();
        for game in moves.left.iter().chain(moves.right.iter()) {
            if let Game::Nus(nus) = game {
                lock.max_rational_num.update(nus.number.numerator());
                lock.max_rational_den_exp
                    .update(nus.number.denominator_exponent());
                lock.max_up.update(nus.up_multiple);
                lock.max_nimber.update(nus.nimber);
            }
        }
    }

    fn add_new_game(&self, moves: Moves) -> Game {
        // What's going on here: we try to lookup game in cache without taking a lock (inserts are rare)
        // to allow for concurrent lookups. After taking write lock we need to lookup again to be
        // 100% sure we're not inserting two same games.
        if let Some(id) = self.moves_index.get(&moves) {
            return id;
        }

        // Locking here guarantees that no two threads will try to insert the same game
        let lock = self.add_game_lock.lock().unwrap();

        if let Some(id) = self.moves_index.get(&moves) {
            return id;
        }

        #[cfg(feature = "statistics")]
        self.update_statistics(&moves);

        let ptr = GamePtr(self.known_games.push_get_index(Box::new(moves.clone())));
        let game = Game::MovesPtr(ptr);
        self.moves_index.insert(moves, game);
        drop(lock);
        game
    }

    #[inline]
    fn get_moves(&self, ptr: &GamePtr) -> &Moves {
        self.known_games.get(ptr.0).unwrap()
    }

    /// Get left and right moves from a canonical form
    pub fn get_game_moves(&self, game: &Game) -> Moves {
        match game {
            Game::Nus(nus) => {
                // Case: Just a number
                if nus.is_number() {
                    if nus.number == DyadicRationalNumber::from(0) {
                        return Moves {
                            left: vec![],
                            right: vec![],
                        };
                    }

                    if let Some(integer) = nus.number.to_integer() {
                        let sign = if integer >= 0 { 1 } else { -1 };
                        let prev = Game::Nus(Nus::integer(integer - sign));
                        return Moves {
                            left: (if sign > 0 { vec![prev] } else { vec![] }),
                            right: (if sign > 0 { vec![] } else { vec![prev] }),
                        };
                    } else {
                        let rational = nus.number;
                        let left_move = Game::Nus(Nus::rational(rational.step(-1)));
                        let right_move = Game::Nus(Nus::rational(rational.step(1)));
                        return Moves {
                            left: vec![left_move],
                            right: vec![right_move],
                        };
                    }
                }

                // Case: number + nimber but no up/down
                if nus.up_multiple == 0 {
                    let rational = nus.number;
                    let nimber = nus.nimber;

                    let mut moves = Moves::empty();
                    for i in 0..nimber.value() {
                        let new_nus = Nus {
                            number: rational,
                            up_multiple: 0,
                            nimber: Nimber::from(i),
                        };
                        moves.left.push(Game::Nus(new_nus));
                        moves.right.push(Game::Nus(new_nus));
                    }
                    return moves;
                }

                // Case: number-up-star
                let number_move = Game::Nus(Nus::rational(nus.number));

                let sign = if nus.up_multiple >= 0 { 1 } else { -1 };
                let prev_up = nus.up_multiple - sign;
                let up_parity: u32 = (nus.up_multiple & 1) as u32;
                let prev_nimber = nus.nimber.value() ^ up_parity ^ (prev_up as u32 & 1);
                let moves;

                if nus.up_multiple == 1 && nus.nimber == Nimber::from(1) {
                    // Special case: n^*
                    let star_move = Game::Nus(Nus {
                        number: nus.number,
                        up_multiple: 0,
                        nimber: Nimber::from(1),
                    });
                    moves = Moves {
                        left: vec![number_move, star_move],
                        right: vec![number_move],
                    };
                } else if nus.up_multiple == -1 && nus.nimber == Nimber::from(1) {
                    // Special case: nv*
                    let star_move = Game::Nus(Nus {
                        number: nus.number,
                        up_multiple: 0,
                        nimber: Nimber::from(1),
                    });
                    moves = Moves {
                        left: vec![number_move],
                        right: vec![number_move, star_move],
                    };
                } else if nus.up_multiple > 0 {
                    let prev_nus = Game::Nus(Nus {
                        number: nus.number,
                        up_multiple: prev_up,
                        nimber: Nimber::from(prev_nimber),
                    });
                    moves = Moves {
                        left: vec![number_move],
                        right: vec![prev_nus],
                    };
                } else {
                    let prev_nus = Game::Nus(Nus {
                        number: nus.number,
                        up_multiple: prev_up,
                        nimber: Nimber::from(prev_nimber),
                    });
                    moves = Moves {
                        left: vec![prev_nus],
                        right: vec![number_move],
                    };
                }

                moves
            }
            Game::MovesPtr(ptr) => self.get_moves(ptr).clone(),
        }
    }

    #[inline]
    fn get_game_by_moves(&self, moves: &Moves) -> Option<Game> {
        self.moves_index.get(moves)
    }

    /// Construct NUS with only integer
    #[inline]
    pub fn construct_integer(&self, integer: i64) -> Game {
        Game::Nus(Nus::integer(integer))
    }

    /// Construct NUS with only dyadic rational
    #[inline]
    pub fn construct_rational(&self, rational: DyadicRationalNumber) -> Game {
        Game::Nus(Nus::rational(rational))
    }

    /// Construct NUS with only nimber
    #[inline]
    pub fn construct_nimber(&self, number: DyadicRationalNumber, nimber: Nimber) -> Game {
        Game::Nus(Nus {
            number,
            up_multiple: 0,
            nimber,
        })
    }

    /// Construct NUS
    #[inline]
    pub fn construct_nus(&self, nus: Nus) -> Game {
        Game::Nus(nus)
    }

    /// Construct negative of a game
    pub fn construct_negative(&self, game: &Game) -> Game {
        match game {
            Game::Nus(nus) => Game::Nus(-*nus),
            Game::MovesPtr(ptr) => {
                // TODO: cache lookup

                let moves = self.get_moves(ptr);

                let new_left_moves = moves
                    .left
                    .iter()
                    .map(|left| self.construct_negative(left))
                    .collect::<Vec<_>>();
                let new_right_moves = moves
                    .right
                    .iter()
                    .map(|right| self.construct_negative(right))
                    .collect::<Vec<_>>();
                let new_moves = Moves {
                    left: new_left_moves,
                    right: new_right_moves,
                };
                self.construct_from_canonical_moves(new_moves)
            }
        }
    }

    /// Construct a sum of two games
    pub fn construct_sum(&self, g: Game, h: Game) -> Game {
        if let (Game::Nus(g_nus), Game::Nus(h_nus)) = (g, h) {
            return self.construct_nus(g_nus + h_nus);
        }

        if let Some(result) = self.add_index.get(&(g, h)) {
            return result;
        }

        // We want to return { GL+H, G+HL | GR+H, G+HR }

        // By the number translation theorem

        let mut moves = Moves::empty();

        if !g.is_number() {
            let g_moves = self.get_game_moves(&g);
            for g_l in &g_moves.left {
                moves.left.push(self.construct_sum(*g_l, h));
            }
            for g_r in &g_moves.right {
                moves.right.push(self.construct_sum(*g_r, h));
            }
        }
        if !h.is_number() {
            let h_moves = self.get_game_moves(&h);
            for h_l in &h_moves.left {
                moves.left.push(self.construct_sum(g, *h_l));
            }
            for h_r in &h_moves.right {
                moves.right.push(self.construct_sum(g, *h_r));
            }
        }

        let result = self.construct_from_moves(moves);
        self.add_index.insert((g, h), result);
        self.add_index.insert((h, g), result);
        result
    }

    fn construct_from_canonical_moves(&self, mut moves: Moves) -> Game {
        moves.left.sort();
        moves.right.sort();

        if let Some(game) = self.get_game_by_moves(&moves) {
            return game;
        }

        if let Some(nus) = moves.to_nus() {
            return Game::Nus(nus);
        }

        // Game is not a nus
        self.add_new_game(moves)
    }

    /// Safe function to construct a game from possible moves
    pub fn construct_from_moves(&self, mut moves: Moves) -> Game {
        moves.eliminate_duplicates();

        let left_mex = self.mex(&moves.left);
        let right_mex = self.mex(&moves.right);
        if let (Some(left_mex), Some(right_mex)) = (left_mex, right_mex) {
            if left_mex == right_mex {
                let nus = Nus {
                    number: DyadicRationalNumber::from(0),
                    up_multiple: 0,
                    nimber: Nimber::from(left_mex),
                };
                return Game::Nus(nus);
            }
        }

        moves = self.canonicalize_moves(moves);

        self.construct_from_canonical_moves(moves)
    }

    fn canonicalize_moves(&self, moves: Moves) -> Moves {
        let moves = self.bypass_reversible_moves_l(moves);
        let moves = self.bypass_reversible_moves_r(moves);

        let left = self.eliminate_dominated_moves(&moves.left, true);
        let right = self.eliminate_dominated_moves(&moves.right, false);

        Moves { left, right }
    }

    fn eliminate_dominated_moves(
        &self,
        moves: &[Game],
        eliminate_smaller_moves: bool,
    ) -> Vec<Game> {
        let mut moves: Vec<Option<Game>> = moves.iter().cloned().map(Some).collect();

        for i in 0..moves.len() {
            let move_i = match moves[i] {
                None => continue,
                Some(id) => id,
            };
            for j in 0..i {
                let move_j = match moves[j] {
                    None => continue,
                    Some(id) => id,
                };

                if (eliminate_smaller_moves && self.leq(&move_i, &move_j))
                    || (!eliminate_smaller_moves && self.leq(&move_j, &move_i))
                {
                    moves[i] = None;
                }
                if (eliminate_smaller_moves && self.leq(&move_j, &move_i))
                    || (!eliminate_smaller_moves && self.leq(&move_i, &move_j))
                {
                    moves[j] = None;
                }
            }
        }

        moves.iter().flatten().copied().collect()
    }

    /// Return false if `H <= GL` for some left option `GL` of `G` or `HR <= G` for some right
    /// option `HR` of `H`. Otherwise return true.
    fn leq_arrays(
        &self,
        game: &Game,
        left_moves: &[Option<Game>],
        right_moves: &[Option<Game>],
    ) -> bool {
        for r_move in right_moves {
            if let Some(r_opt) = r_move {
                if self.leq(r_opt, game) {
                    return false;
                }
            }
        }

        let game_moves = self.get_game_moves(game);
        for l_move in &game_moves.left {
            if self.geq_arrays(l_move, left_moves, right_moves) {
                return false;
            }
        }

        true
    }

    fn geq_arrays(
        &self,
        game: &Game,
        left_moves: &[Option<Game>],
        right_moves: &[Option<Game>],
    ) -> bool {
        for l_move in left_moves {
            if let Some(l_opt) = l_move {
                if self.leq(game, l_opt) {
                    return false;
                }
            }
        }

        let game_moves = self.get_game_moves(game);
        for r_move in &game_moves.right {
            if self.leq_arrays(r_move, left_moves, right_moves) {
                return false;
            }
        }

        true
    }

    fn leq(&self, lhs_game: &Game, rhs_game: &Game) -> bool {
        if lhs_game == rhs_game {
            return true;
        }

        if let (Game::Nus(lhs_nus), Game::Nus(rhs_nus)) = (lhs_game, rhs_game) {
            match lhs_nus.number.cmp(&rhs_nus.number) {
                Ordering::Less => return true,
                Ordering::Greater => return false,
                Ordering::Equal => {
                    if lhs_nus.up_multiple < rhs_nus.up_multiple - 1 {
                        return true;
                    } else if lhs_nus.up_multiple < rhs_nus.up_multiple {
                        return (lhs_nus.nimber + rhs_nus.nimber) != Nimber::from(1);
                    } else {
                        return false;
                    }
                }
            }
        }

        if let Some(leq) = self.leq_index.get(&(*lhs_game, *rhs_game)) {
            return leq;
        }

        let mut leq = true;

        if !lhs_game.is_number() {
            let lhs_game_moves = self.get_game_moves(lhs_game);
            for lhs_l in &lhs_game_moves.left {
                if self.leq(rhs_game, lhs_l) {
                    leq = false;
                    break;
                }
            }
        }

        if leq && !rhs_game.is_number() {
            let rhs_game_moves = self.get_game_moves(rhs_game);
            for rhs_r in &rhs_game_moves.right {
                if self.leq(rhs_r, lhs_game) {
                    leq = false;
                    break;
                }
            }
        }

        self.leq_index.insert((*lhs_game, *rhs_game), leq);

        leq
    }

    fn bypass_reversible_moves_l(&self, moves: Moves) -> Moves {
        let mut i: i64 = 0;

        let mut left_moves: Vec<Option<Game>> = moves.left.iter().cloned().map(Some).collect();
        let right_moves: Vec<Option<Game>> = moves.right.iter().cloned().map(Some).collect();

        loop {
            if (i as usize) >= left_moves.len() {
                break;
            }
            let g_l = match left_moves[i as usize] {
                None => {
                    i += 1;
                    continue;
                }
                Some(g) => g,
            };
            for g_lr in self.get_game_moves(&g_l).right {
                if self.leq_arrays(&g_lr, &left_moves, &right_moves) {
                    let g_lr_moves = self.get_game_moves(&g_lr);
                    let mut new_left_moves: Vec<Option<Game>> =
                        vec![None; left_moves.len() + g_lr_moves.left.len() as usize - 1];
                    for k in 0..(i as usize) {
                        new_left_moves[k] = left_moves[k];
                    }
                    for k in (i as usize + 1)..left_moves.len() {
                        new_left_moves[k - 1] = left_moves[k];
                    }
                    for (k, g_lrl) in g_lr_moves.left.iter().enumerate() {
                        if left_moves.contains(&Some(*g_lrl)) {
                            new_left_moves[left_moves.len() + k - 1] = None;
                        } else {
                            new_left_moves[left_moves.len() + k - 1] = Some(*g_lrl);
                        }
                    }
                    left_moves = new_left_moves;
                    i -= 1;
                    break;
                }
            }

            i += 1;
        }
        Moves {
            left: left_moves.iter().flatten().copied().collect(),
            right: moves.right,
        }
    }

    fn bypass_reversible_moves_r(&self, moves: Moves) -> Moves {
        let mut i: i64 = 0;

        let left_moves: Vec<Option<Game>> = moves.left.iter().cloned().map(Some).collect();
        let mut right_moves: Vec<Option<Game>> = moves.right.iter().cloned().map(Some).collect();

        loop {
            if (i as usize) >= right_moves.len() {
                break;
            }
            let g_r = match right_moves[i as usize] {
                None => {
                    i += 1;
                    continue;
                }
                Some(game) => game,
            };
            for g_rl in self.get_game_moves(&g_r).left {
                if self.geq_arrays(&g_rl, &left_moves, &right_moves) {
                    let g_rl_moves = self.get_game_moves(&g_rl);
                    let mut new_right_moves: Vec<Option<Game>> =
                        vec![None; right_moves.len() + g_rl_moves.right.len() as usize - 1];
                    for k in 0..(i as usize) {
                        new_right_moves[k] = right_moves[k];
                    }
                    for k in (i as usize + 1)..right_moves.len() {
                        new_right_moves[k - 1] = right_moves[k];
                    }
                    for (k, g_rlr) in g_rl_moves.right.iter().enumerate() {
                        if right_moves.contains(&Some(*g_rlr)) {
                            new_right_moves[right_moves.len() + k - 1] = None;
                        } else {
                            new_right_moves[right_moves.len() + k - 1] = Some(*g_rlr);
                        }
                    }
                    right_moves = new_right_moves;
                    i -= 1;
                    break;
                }
            }

            i += 1;
        }
        Moves {
            left: moves.left,
            right: right_moves.iter().flatten().copied().collect(),
        }
    }

    /// Calculate mex if possible. Assumes that input is sorted
    fn mex(&self, moves: &[Game]) -> Option<u32> {
        let mut i = 0;
        let mut mex = 0;
        loop {
            if i >= moves.len() {
                break;
            }

            match moves[i] {
                Game::Nus(nus) => {
                    if !nus.is_nimber() {
                        return None;
                    }

                    if nus.nimber == Nimber::from(mex) {
                        mex += 1;
                    } else {
                        break;
                    }
                    i += 1;
                }
                Game::MovesPtr(_) => return None,
            }
        }

        for m in &moves[i..] {
            if !m.is_nimber() {
                return None;
            }
        }

        Some(mex)
    }

    /// Calculate temperature of the game. Avoids computing a thermograph is game is a NUS
    pub fn temperature(&self, game: &Game) -> Rational {
        match game {
            Game::Nus(nus) => {
                if nus.is_number() {
                    // It's a number k/2^n, so the temperature is -1/2^n
                    // DyadicRationalNumber::new(-1, nus.number.denominator_exponent())
                    Rational::new(-1, nus.number.denominator().unwrap() as u32)
                } else {
                    // It's a number plus a nonzero infinitesimal, thus the temperature is 0
                    // DyadicRationalNumber::from(0)
                    Rational::from(0)
                }
            }
            Game::MovesPtr(_) => self.thermograph(game).get_temperature(),
        }
    }

    /// Construct a thermograph of a game, using thermographic intersection of
    /// left and right scaffolds
    pub fn thermograph(&self, game: &Game) -> Thermograph {
        let thermograph = match game {
            Game::MovesPtr(ptr) => {
                if let Some(thermograph) = self.thermograph_index.get(&game) {
                    return thermograph.clone();
                }
                let moves = self.get_moves(ptr);
                self.thermograph_from_moves(&moves)
            }
            Game::Nus(nus) => {
                if nus.number.to_integer().is_some() && nus.is_number() {
                    Thermograph::with_mast(Rational::new(nus.number.to_integer().unwrap(), 1))
                } else {
                    if nus.up_multiple == 0
                        || (nus.nimber == Nimber::from(1) && nus.up_multiple.abs() == 1)
                    {
                        // This looks like 0 or * (depending on whether nimberPart is 0 or 1).
                        let new_game = self.construct_nus(Nus {
                            number: nus.number,
                            up_multiple: 0,
                            nimber: Nimber::from(nus.nimber.value().cmp(&0) as u32), // signum(nus.nimber)
                        });
                        let new_game_moves = self.get_game_moves(&new_game);
                        self.thermograph_from_moves(&new_game_moves)
                    } else {
                        let new_game = self.construct_nus(Nus {
                            number: nus.number,
                            up_multiple: nus.up_multiple.cmp(&0) as i32, // signum(nus.up_multiple)
                            nimber: Nimber::from(0),
                        });
                        let new_game_moves = self.get_game_moves(&new_game);
                        self.thermograph_from_moves(&new_game_moves)
                    }
                }
            }
        };
        self.thermograph_index.insert(*game, thermograph.clone());

        thermograph
    }

    fn thermograph_from_moves(&self, moves: &Moves) -> Thermograph {
        let mut left_scaffold = Trajectory::new_constant(Rational::NegativeInfinity);
        let mut right_scaffold = Trajectory::new_constant(Rational::PositiveInfinity);

        for left_move in &moves.left {
            left_scaffold = left_scaffold.max(&self.thermograph(left_move).right_wall);
        }
        for right_move in &moves.right {
            right_scaffold = right_scaffold.min(&self.thermograph(right_move).left_wall);
        }

        left_scaffold.tilt(Rational::from(-1));
        right_scaffold.tilt(Rational::from(1));

        Thermograph::thermographic_intersection(left_scaffold, right_scaffold)
    }
}

// printing
impl GameBackend {
    /// Print game using `{G^L | G^R}` notation
    pub fn print_game(&self, game: &Game, f: &mut impl Write) -> fmt::Result {
        match game {
            Game::Nus(nus) => write!(f, "{}", nus),
            Game::MovesPtr(ptr) => {
                let moves = self.get_moves(ptr);
                self.print_moves(&moves, f)
            }
        }
    }

    /// Print game to string using `{G^L | G^R}` notation
    pub fn print_game_to_str(&self, id: &Game) -> String {
        let mut buf = String::new();
        self.print_game(id, &mut buf).unwrap();
        buf
    }

    /// Print moves using `{G^L | G^R}` notation
    pub fn print_moves(&self, moves: &Moves, f: &mut impl Write) -> fmt::Result {
        write!(f, "{{")?;
        for (idx, l) in moves.left.iter().enumerate() {
            if idx != 0 {
                write!(f, ",")?;
            }
            self.print_game(l, f)?;
        }
        write!(f, "|")?;
        for (idx, r) in moves.right.iter().enumerate() {
            if idx != 0 {
                write!(f, ",")?;
            }
            self.print_game(r, f)?;
        }
        write!(f, "}}")?;
        Ok(())
    }

    /// Print moves to string using `{G^L | G^R}` notation
    pub fn print_moves_to_str(&self, moves: &Moves) -> String {
        let mut buf = String::new();
        self.print_moves(moves, &mut buf).unwrap();
        buf
    }

    /// Print moves with NUS unwrapped using `{G^L | G^R}` notation
    pub fn print_moves_deep(&self, moves: &Moves, f: &mut impl Write) -> fmt::Result {
        write!(f, "{{")?;
        for (idx, l) in moves.left.iter().enumerate() {
            if idx != 0 {
                write!(f, ",")?;
            }
            self.print_moves_deep(&self.get_game_moves(l), f)?;
        }
        write!(f, "|")?;
        for (idx, r) in moves.right.iter().enumerate() {
            if idx != 0 {
                write!(f, ",")?;
            }
            self.print_moves_deep(&self.get_game_moves(r), f)?;
        }
        write!(f, "}}")?;
        Ok(())
    }

    /// Print moves to string with NUS unwrapped using `{G^L | G^R}` notation
    pub fn print_moves_deep_to_str(&self, moves: &Moves) -> String {
        let mut buf = String::new();
        self.print_moves_deep(moves, &mut buf).unwrap();
        buf
    }
}

#[test]
fn constructs_integers() {
    let b = GameBackend::new();

    let eight = b.construct_integer(8);
    assert_eq!(&b.print_game_to_str(&eight), "8");
    let eight_moves = b.get_game_moves(&eight);
    assert_eq!(&b.print_moves_to_str(&eight_moves), "{7|}");
    assert_eq!(
        &b.print_moves_deep_to_str(&eight_moves),
        "{{{{{{{{{|}|}|}|}|}|}|}|}|}"
    );

    let minus_forty_two = b.construct_integer(-42);
    assert_eq!(&b.print_game_to_str(&minus_forty_two), "-42");
}

#[test]
fn constructs_rationals() {
    let b = GameBackend::new();

    let rational = DyadicRationalNumber::new(3, 4);
    let three_sixteenth = b.construct_rational(rational);
    assert_eq!(&b.print_game_to_str(&three_sixteenth), "3/16");

    let duplicate = b.construct_rational(rational);
    assert_eq!(three_sixteenth, duplicate);
}

#[test]
fn constructs_nimbers() {
    let b = GameBackend::new();

    let star = Game::Nus(Nus::nimber(Nimber::from(1)));
    assert_eq!(&b.print_game_to_str(&star), "*");
    let star_moves = b.get_game_moves(&star);
    assert_eq!(&b.print_moves_to_str(&star_moves), "{0|0}");
    assert_eq!(&b.print_moves_deep_to_str(&star_moves), "{{|}|{|}}");

    let star_three = Game::Nus(Nus::nimber(Nimber::from(3)));
    assert_eq!(&b.print_game_to_str(&star_three), "*3");
    let star_three_moves = b.get_game_moves(&star_three);
    assert_eq!(&b.print_moves_to_str(&star_three_moves), "{0,*,*2|0,*,*2}");

    let one_star_two = Game::Nus(Nus {
        number: DyadicRationalNumber::from(1),
        up_multiple: 0,
        nimber: (Nimber::from(2)),
    });
    assert_eq!(&b.print_game_to_str(&one_star_two), "1*2");
    let one_star_two_moves = b.get_game_moves(&one_star_two);
    assert_eq!(&b.print_moves_to_str(&one_star_two_moves), "{1,1*|1,1*}");
}

#[test]
fn constructs_up() {
    let b = GameBackend::new();

    let up = b.construct_nus(Nus {
        number: DyadicRationalNumber::from(0),
        up_multiple: 1,
        nimber: Nimber::from(0),
    });
    assert_eq!(&b.print_game_to_str(&up), "^");

    let up_star = b.construct_nus(Nus {
        number: DyadicRationalNumber::from(0),
        up_multiple: 1,
        nimber: Nimber::from(1),
    });
    assert_eq!(&b.print_game_to_str(&up_star), "^*");

    let down = b.construct_nus(Nus {
        number: DyadicRationalNumber::from(0),
        up_multiple: -3,
        nimber: Nimber::from(0),
    });
    assert_eq!(&b.print_game_to_str(&down), "v3");
}

#[test]
fn nimber_is_its_negative() {
    let b = GameBackend::new();

    let star = b.construct_nimber(DyadicRationalNumber::from(0), Nimber::from(4));
    assert_eq!(&b.print_game_to_str(&star), "*4");

    let neg_star = b.construct_negative(&star);
    assert_eq!(star, neg_star);
}

#[test]
fn gets_moves() {
    let b = GameBackend::new();

    let down_moves = b.get_game_moves(&Game::Nus(Nus::from_str("v").unwrap()));
    assert_eq!(b.print_moves_to_str(&down_moves), "{*|0}");
    assert_eq!(b.print_moves_deep_to_str(&down_moves), "{{{|}|{|}}|{|}}");

    let down_moves = b.get_game_moves(&Game::Nus(Nus::from_str("^").unwrap()));
    assert_eq!(b.print_moves_to_str(&down_moves), "{0|*}");
    assert_eq!(b.print_moves_deep_to_str(&down_moves), "{{|}|{{|}|{|}}}");

    let moves = Moves {
        left: vec![Game::Nus(Nus::from_str("v").unwrap())],
        right: vec![Game::Nus(Nus::from_str("-2").unwrap())],
    };
    assert_eq!(b.print_moves_to_str(&moves), "{v|-2}");
    assert_eq!(
        b.print_moves_deep_to_str(&moves),
        "{{{{|}|{|}}|{|}}|{|{|{|}}}}"
    );
}

#[test]
fn simplifies_moves() {
    let b = GameBackend::new();

    let one = Game::Nus(Nus::from_str("1").unwrap());
    let star = Game::Nus(Nus::from_str("*").unwrap());

    let moves_l = Moves {
        left: vec![one],
        right: vec![star],
    };
    let left_id = b.construct_from_moves(moves_l);
    assert_eq!(&b.print_game_to_str(&left_id), "{1|*}");

    let weird = Moves {
        left: vec![Game::Nus(Nus::from_str("1v2*").unwrap())],
        right: vec![Game::Nus(Nus::from_str("1").unwrap())],
    };
    let weird = b.construct_from_moves(weird);
    assert_eq!(&b.print_game_to_str(&weird), "1v3");
    let weird_moves = b.get_game_moves(&weird);
    assert_eq!(&b.print_moves_to_str(&weird_moves), "{1v2*|1}");
    assert_eq!(&b.print_game_to_str(&weird_moves.left[0]), "1v2*");
    assert_eq!(
        &b.print_moves_to_str(&b.get_game_moves(&weird_moves.left[0])),
        "{1v|1}"
    );
    assert_eq!(
        &b.print_moves_deep_to_str(&weird_moves),
        "{{{{{{|}|}|{{|}|}}|{{|}|}}|{{|}|}}|{{|}|}}"
    );

    // Another case:

    let weird_right = Moves {
        left: vec![Game::Nus(Nus::from_str("^").unwrap())],
        right: vec![Game::Nus(Nus::from_str("-2").unwrap())],
    };
    let weird_right = b.construct_from_moves(weird_right);
    assert_eq!(&b.print_game_to_str(&weird_right), "{^|-2}");
    let weird_right_moves = b.get_game_moves(&weird_right);
    dbg!(&weird_right);
    dbg!(&weird_right_moves);
    assert_eq!(&b.print_moves_to_str(&weird_right_moves), "{^|-2}");
    assert_eq!(
        &b.print_moves_deep_to_str(&weird_right_moves),
        "{{{|}|{{|}|{|}}}|{|{|{|}}}}"
    );

    let weird = Moves {
        left: vec![],
        right: vec![weird_right],
    };
    assert_ne!(
        &b.print_moves_deep_to_str(&b.canonicalize_moves(weird.clone())),
        "{|{{{|}|{{|}|{|}}}|{|{|{|}}}}}"
    );
    assert_eq!(
        &b.print_moves_to_str(&b.canonicalize_moves(weird.clone())),
        "{|}"
    );
    let weird = b.construct_from_moves(weird);
    let weird_moves = b.get_game_moves(&weird);
    assert_eq!(&b.print_moves_to_str(&weird_moves), "{|}");
    assert_eq!(&b.print_game_to_str(&weird), "0");
}

#[test]
fn sum_works() {
    let b = GameBackend::new();

    let zero = b.construct_integer(0);
    let one = b.construct_integer(1);

    let one_zero = b.construct_from_moves(Moves {
        left: vec![one],
        right: vec![zero],
    });
    let zero_one = b.construct_from_moves(Moves {
        left: vec![zero],
        right: vec![one],
    });
    let sum = b.construct_sum(one_zero, zero_one);
    assert_eq!(&b.print_game_to_str(&sum), "{3/2|1/2}");
}

#[test]
fn temp_of_one_minus_one_is_one() {
    let b = GameBackend::new();

    let one = b.construct_integer(1);
    let negative_one = b.construct_integer(-1);

    let moves = Moves {
        left: vec![one],
        right: vec![negative_one],
    };
    let g = b.construct_from_moves(moves);
    assert_eq!(b.temperature(&g), Rational::from(1));
}
