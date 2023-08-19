//! Canonical form of a short game

use crate::{
    display,
    nom_utils::{impl_from_str_via_nom, lexeme},
    numeric::dyadic_rational_number::DyadicRationalNumber,
    numeric::nimber::Nimber,
    numeric::rational::Rational,
    short::partizan::thermograph::Thermograph,
    short::partizan::trajectory::Trajectory,
};
use auto_ops::impl_op_ex;
use nom::{
    branch::alt,
    character::complete::{char, one_of, u32},
    error::ErrorKind,
    multi::separated_list0,
};
use std::{
    cmp::Ordering,
    fmt::{self, Display, Write},
    hash::Hash,
};

#[cfg(test)]
use std::str::FromStr;

/// A number-up-star game position that is a sum of a number, up and, nimber.
#[derive(Debug, Hash, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Nus {
    number: DyadicRationalNumber,
    up_multiple: i32,
    nimber: Nimber,
}

impl Nus {
    /// Parse nus from string, using notation without pluses between number, up, and star components
    ///
    /// Pattern: `\d*([v^]\d*)?(\*\d*)`
    pub fn parse(input: &str) -> nom::IResult<&str, Self> {
        let full_input = input;
        // This flag is set if we explicitly parse a number, rather than set it to zero if
        // it is omitted. It makes expressions like `*` a valid input, however it also makes
        // empty input parse to a zero game, which is undesired. We handle that case explicitly.
        let parsed_number: bool;

        let (input, number) = match lexeme(DyadicRationalNumber::parse)(input) {
            Ok((input, number)) => {
                parsed_number = true;
                (input, number)
            }
            Err(_) => {
                parsed_number = false;
                (input, DyadicRationalNumber::from(0))
            }
        };

        let (input, up_multiple) = match lexeme(one_of::<_, _, (&str, ErrorKind)>("^v"))(input) {
            Ok((input, chr)) => {
                let (input, up_multiple) =
                    lexeme(u32::<_, (&str, ErrorKind)>)(input).unwrap_or((input, 1));
                (
                    input,
                    if chr == 'v' {
                        -(up_multiple as i32)
                    } else {
                        up_multiple as i32
                    },
                )
            }
            Err(_) => (input, 0),
        };

        let (input, star_multiple) = match lexeme(char::<_, (&str, ErrorKind)>('*'))(input) {
            Ok((input, _)) => lexeme(u32::<_, (&str, ErrorKind)>)(input).unwrap_or((input, 1)),
            Err(_) => (input, 0),
        };

        let nus = Nus {
            number,
            up_multiple,
            nimber: Nimber::from(star_multiple),
        };

        if nus == Nus::new_integer(0) && !parsed_number {
            return Err(nom::Err::Error(nom::error::Error::new(
                full_input,
                ErrorKind::Fail,
            )));
        }

        Ok((input, nus))
    }
}

impl_from_str_via_nom!(Nus);

#[cfg(test)]
macro_rules! parse_nus_roundtrip {
    ($inp: expr) => {
        assert_eq!(
            &format!(
                "{}",
                Nus::from_str($inp).expect(&format!("Could not parse: '{}'", $inp))
            ),
            $inp
        );
    };
}

#[cfg(test)]
macro_rules! parse_nus_succeed {
    ($inp: expr) => {
        assert!(Nus::from_str($inp).is_ok(), "Parse should succeed");
    };
}

#[cfg(test)]
macro_rules! parse_nus_fail {
    ($inp: expr) => {
        assert!(Nus::from_str($inp).is_err(), "Parse should fail");
    };
}

#[test]
fn parse_nus() {
    parse_nus_fail!(""); // this shoult NOT parse to 0
    parse_nus_fail!("42 foo");
    parse_nus_roundtrip!("42");
    parse_nus_roundtrip!("1/2");
    parse_nus_roundtrip!("-8");
    parse_nus_roundtrip!("13^");
    parse_nus_roundtrip!("123v");
    parse_nus_roundtrip!("13^3");
    parse_nus_roundtrip!("123v58");
    parse_nus_roundtrip!("13^3*");
    parse_nus_roundtrip!("123v58*");
    parse_nus_roundtrip!("13^3*8");
    parse_nus_roundtrip!("123v58*34");
    parse_nus_roundtrip!("-13^3*");
    parse_nus_roundtrip!("-123v58*");
    parse_nus_succeed!("  123 v   58 *  43784");
}

impl Nus {
    /// Create new number-up-start sum
    #[inline]
    pub fn new(number: DyadicRationalNumber, up_multiple: i32, nimber: Nimber) -> Self {
        Nus {
            number,
            up_multiple,
            nimber,
        }
    }

    /// Create new number-up-star game equal to an integer.
    #[inline]
    pub fn new_integer(integer: i64) -> Self {
        Nus::new(DyadicRationalNumber::from(integer), 0, Nimber::from(0))
    }

    /// Create new number-up-star game equal to an rational.
    #[inline]
    pub fn new_rational(number: DyadicRationalNumber) -> Self {
        Nus::new(number, 0, Nimber::from(0))
    }

    /// Create new number-up-star game equal to an rational.
    #[inline]
    pub fn new_nimber(nimber: Nimber) -> Self {
        Nus::new(DyadicRationalNumber::from(0), 0, nimber)
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

    fn to_moves(&self) -> Moves {
        // Case: Just a number
        if self.is_number() {
            if self.number == DyadicRationalNumber::from(0) {
                return Moves {
                    left: vec![],
                    right: vec![],
                };
            }

            if let Some(integer) = self.number.to_integer() {
                let sign = if integer >= 0 { 1 } else { -1 };
                let prev = CanonicalForm::new_nus(Nus::new_integer(integer - sign));

                if sign > 0 {
                    return Moves {
                        left: vec![prev],
                        right: vec![],
                    };
                } else if sign < 0 {
                    return Moves {
                        left: vec![],
                        right: vec![prev],
                    };
                }
            } else {
                let rational = self.number;
                let left_move = CanonicalForm::new_nus(Nus::new_rational(rational.step(-1)));
                let right_move = CanonicalForm::new_nus(Nus::new_rational(rational.step(1)));
                return Moves {
                    left: vec![left_move],
                    right: vec![right_move],
                };
            }
        }

        // Case: number + nimber but no up/down
        if self.up_multiple == 0 {
            let rational = self.number;
            let nimber = self.nimber;

            let mut moves = Moves::empty();
            for i in 0..nimber.value() {
                let new_nus = Nus {
                    number: rational,
                    up_multiple: 0,
                    nimber: Nimber::from(i),
                };
                moves.left.push(CanonicalForm::new_nus(new_nus));
                moves.right.push(CanonicalForm::new_nus(new_nus));
            }
            return moves;
        }

        // Case: number-up-star
        let number_move = Nus::new_rational(self.number);

        let sign = if self.up_multiple >= 0 { 1 } else { -1 };
        let prev_up = self.up_multiple - sign;
        let up_parity: u32 = (self.up_multiple & 1) as u32;
        let prev_nimber = self.nimber.value() ^ up_parity ^ (prev_up as u32 & 1);
        let moves;

        if self.up_multiple == 1 && self.nimber == Nimber::from(1) {
            // Special case: n^*
            let star_move = CanonicalForm::new_nus(Nus {
                number: self.number,
                up_multiple: 0,
                nimber: Nimber::from(1),
            });
            moves = Moves {
                left: vec![CanonicalForm::new_nus(number_move), star_move],
                right: vec![CanonicalForm::new_nus(number_move)],
            };
        } else if self.up_multiple == -1 && self.nimber == Nimber::from(1) {
            // Special case: nv*
            let star_move = CanonicalForm::new_nus(Nus {
                number: self.number,
                up_multiple: 0,
                nimber: Nimber::from(1),
            });
            moves = Moves {
                left: vec![CanonicalForm::new_nus(number_move)],
                right: vec![CanonicalForm::new_nus(number_move), star_move],
            };
        } else if self.up_multiple > 0 {
            let prev_nus = CanonicalForm::new_nus(Nus {
                number: self.number,
                up_multiple: prev_up,
                nimber: Nimber::from(prev_nimber),
            });
            moves = Moves {
                left: vec![CanonicalForm::new_nus(number_move)],
                right: vec![prev_nus],
            };
        } else {
            let prev_nus = CanonicalForm::new_nus(Nus {
                number: self.number,
                up_multiple: prev_up,
                nimber: Nimber::from(prev_nimber),
            });
            moves = Moves {
                left: vec![prev_nus],
                right: vec![CanonicalForm::new_nus(number_move)],
            };
        }

        moves
    }
}

impl_op_ex!(+|lhs: &Nus, rhs: &Nus| -> Nus {
    Nus {
        number: lhs.number + rhs.number,
        up_multiple: lhs.up_multiple + rhs.up_multiple,
        nimber: lhs.nimber + rhs.nimber,
    }
});

impl_op_ex!(-|lhs: &Nus| -> Nus {
    Nus {
        number: -lhs.number,
        up_multiple: -lhs.up_multiple,
        nimber: lhs.nimber, // Nimber is its own negative
    }
});

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
#[derive(Debug, Hash, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Moves {
    /// Left player's moves
    pub left: Vec<CanonicalForm>,

    /// Right player's moves
    pub right: Vec<CanonicalForm>,
}

impl Moves {
    #[inline]
    fn empty() -> Self {
        Moves {
            left: vec![],
            right: vec![],
        }
    }

    #[inline]
    fn eliminate_duplicates(&mut self) {
        self.left.sort();
        self.left.dedup();

        self.right.sort();
        self.right.dedup();
    }

    /// Construct a canoical form of arbitrary moves.
    /// It is an alias of [CanonicalForm::new_from_moves]
    pub fn canonical_form(self) -> CanonicalForm {
        CanonicalForm::new_from_moves(self)
    }

    /// Try converting moves to NUS. Returns [None] if moves do not form a NUS
    pub fn to_nus(&self) -> Option<Nus> {
        let mut result = Nus::new_integer(0);

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
                let l = &self.left[i];
                let r = &self.right[i];

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

    fn eliminate_dominated_moves(
        moves: &[CanonicalForm],
        eliminate_smaller_moves: bool,
    ) -> Vec<CanonicalForm> {
        let mut moves: Vec<Option<CanonicalForm>> = moves.iter().cloned().map(Some).collect();

        for i in 0..moves.len() {
            let move_i = match &moves[i] {
                None => continue,
                Some(id) => id.clone(),
            };
            for j in 0..i {
                let move_j = match &moves[j] {
                    None => continue,
                    Some(id) => id.clone(),
                };

                if (eliminate_smaller_moves && CanonicalForm::leq(&move_i, &move_j))
                    || (!eliminate_smaller_moves && CanonicalForm::leq(&move_j, &move_i))
                {
                    moves[i] = None;
                }
                if (eliminate_smaller_moves && CanonicalForm::leq(&move_j, &move_i))
                    || (!eliminate_smaller_moves && CanonicalForm::leq(&move_i, &move_j))
                {
                    moves[j] = None;
                }
            }
        }

        moves.iter().flatten().cloned().collect()
    }

    /// Return false if `H <= GL` for some left option `GL` of `G` or `HR <= G` for some right
    /// option `HR` of `H`. Otherwise return true.
    fn leq_arrays(
        game: &CanonicalForm,
        left_moves: &[Option<CanonicalForm>],
        right_moves: &[Option<CanonicalForm>],
    ) -> bool {
        for r_move in right_moves {
            if let Some(r_opt) = r_move {
                if CanonicalForm::leq(r_opt, game) {
                    return false;
                }
            }
        }

        let game_moves = game.to_moves();
        for l_move in &game_moves.left {
            if Self::geq_arrays(l_move, left_moves, right_moves) {
                return false;
            }
        }

        true
    }

    fn geq_arrays(
        game: &CanonicalForm,
        left_moves: &[Option<CanonicalForm>],
        right_moves: &[Option<CanonicalForm>],
    ) -> bool {
        for l_move in left_moves {
            if let Some(l_opt) = l_move {
                if CanonicalForm::leq(game, l_opt) {
                    return false;
                }
            }
        }

        let game_moves = game.to_moves();
        for r_move in &game_moves.right {
            if Self::leq_arrays(r_move, left_moves, right_moves) {
                return false;
            }
        }

        true
    }

    fn bypass_reversible_moves_l(&self) -> Moves {
        let mut i: i64 = 0;

        let mut left_moves: Vec<Option<CanonicalForm>> =
            self.left.iter().cloned().map(Some).collect();
        let right_moves: Vec<Option<CanonicalForm>> =
            self.right.iter().cloned().map(Some).collect();

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
            for g_lr in g_l.to_moves().right {
                if Self::leq_arrays(&g_lr, &left_moves, &right_moves) {
                    let g_lr_moves = g_lr.to_moves();
                    let mut new_left_moves: Vec<Option<CanonicalForm>> =
                        vec![None; left_moves.len() + g_lr_moves.left.len() as usize - 1];
                    for k in 0..(i as usize) {
                        new_left_moves[k] = left_moves[k].clone();
                    }
                    for k in (i as usize + 1)..left_moves.len() {
                        new_left_moves[k - 1] = left_moves[k].clone();
                    }
                    for (k, g_lrl) in g_lr_moves.left.iter().enumerate() {
                        if left_moves.contains(&Some(g_lrl.clone())) {
                            new_left_moves[left_moves.len() + k - 1] = None;
                        } else {
                            new_left_moves[left_moves.len() + k - 1] = Some(g_lrl.clone());
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
            left: left_moves.iter().flatten().cloned().collect(),
            right: self.right.clone(),
        }
    }

    fn bypass_reversible_moves_r(&self) -> Moves {
        let mut i: i64 = 0;

        let left_moves: Vec<Option<CanonicalForm>> = self.left.iter().cloned().map(Some).collect();
        let mut right_moves: Vec<Option<CanonicalForm>> =
            self.right.iter().cloned().map(Some).collect();

        loop {
            if (i as usize) >= right_moves.len() {
                break;
            }
            let g_r = match &right_moves[i as usize] {
                None => {
                    i += 1;
                    continue;
                }
                Some(game) => game.clone(),
            };
            for g_rl in g_r.to_moves().left {
                if Self::geq_arrays(&g_rl, &left_moves, &right_moves) {
                    let g_rl_moves = g_rl.to_moves();
                    let mut new_right_moves: Vec<Option<CanonicalForm>> =
                        vec![None; right_moves.len() + g_rl_moves.right.len() as usize - 1];
                    for k in 0..(i as usize) {
                        new_right_moves[k] = right_moves[k].clone();
                    }
                    for k in (i as usize + 1)..right_moves.len() {
                        new_right_moves[k - 1] = right_moves[k].clone();
                    }
                    for (k, g_rlr) in g_rl_moves.right.iter().enumerate() {
                        if right_moves.contains(&Some(g_rlr.clone())) {
                            new_right_moves[right_moves.len() + k - 1] = None;
                        } else {
                            new_right_moves[right_moves.len() + k - 1] = Some(g_rlr.clone());
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
            left: self.left.clone(),
            right: right_moves.iter().flatten().cloned().collect(),
        }
    }

    fn canonicalize(&self) -> Moves {
        let moves = self.bypass_reversible_moves_l();
        let moves = moves.bypass_reversible_moves_r();

        let left = Self::eliminate_dominated_moves(&moves.left, true);
        let right = Self::eliminate_dominated_moves(&moves.right, false);

        Moves { left, right }
    }

    fn thermograph(&self) -> Thermograph {
        let mut left_scaffold = Trajectory::new_constant(Rational::NegativeInfinity);
        let mut right_scaffold = Trajectory::new_constant(Rational::PositiveInfinity);

        for left_move in &self.left {
            left_scaffold = left_scaffold.max(&CanonicalForm::thermograph(left_move).right_wall);
        }
        for right_move in &self.right {
            right_scaffold = right_scaffold.min(&CanonicalForm::thermograph(right_move).left_wall);
        }

        left_scaffold.tilt(Rational::from(-1));
        right_scaffold.tilt(Rational::from(1));

        Thermograph::thermographic_intersection(left_scaffold, right_scaffold)
    }

    /// Print moves with NUS unwrapped using `{G^L | G^R}` notation
    pub fn print_deep(&self, f: &mut impl Write) -> fmt::Result {
        write!(f, "{{")?;
        for (idx, l) in self.left.iter().enumerate() {
            if idx != 0 {
                write!(f, ",")?;
            }
            Self::print_deep(&l.to_moves(), f)?;
        }
        write!(f, "|")?;
        for (idx, r) in self.right.iter().enumerate() {
            if idx != 0 {
                write!(f, ",")?;
            }
            Self::print_deep(&r.to_moves(), f)?;
        }
        write!(f, "}}")?;
        Ok(())
    }

    /// Print moves to string with NUS unwrapped using `{G^L | G^R}` notation
    pub fn print_deep_to_str(&self) -> String {
        let mut buf = String::new();
        Self::print_deep(self, &mut buf).unwrap();
        buf
    }
}

impl Display for Moves {
    /// Print moves using `{G^L | G^R}` notation
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        display::braces(f, |f| {
            display::commas(f, &self.left)?;
            write!(f, "|")?;
            display::commas(f, &self.right)
        })
    }
}

impl Moves {
    /// Parse comma-separated games
    fn parse_list(input: &str) -> nom::IResult<&str, Vec<CanonicalForm>> {
        separated_list0(lexeme(nom::bytes::complete::tag(",")), |input| {
            CanonicalForm::parse(input)
        })(input)
    }

    /// Parse game using `{a,b,...|c,d,...}` notation
    fn parse(input: &str) -> nom::IResult<&str, Self> {
        let (input, _) = lexeme(char('{'))(input)?;
        let (input, left) = Self::parse_list(input)?;
        let (input, _) = lexeme(char('|'))(input)?;
        let (input, right) = Self::parse_list(input)?;
        let (input, _) = lexeme(char('}'))(input)?;
        let moves = Moves { left, right };
        Ok((input, moves))
    }
}

impl_from_str_via_nom!(Moves);

// NOTE: Is there really no way to have an enum with private constructors?

/// Canonical game form
#[derive(Debug, Hash, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum CanonicalFormInner {
    /// Number Up Star sum
    Nus(Nus),

    /// Not a NUS - list of left/right moves
    Moves(Moves),
}

#[repr(transparent)]
#[derive(Debug, Hash, Clone, PartialEq, Eq, PartialOrd, Ord)]
/// Canonical game form
pub struct CanonicalForm(CanonicalFormInner);

impl CanonicalForm {
    /// Construct NUS with only integer
    #[inline]
    pub fn new_integer(integer: i64) -> Self {
        Self::new_nus(Nus::new_integer(integer))
    }

    /// Construct NUS with only dyadic rational
    #[inline]
    pub fn new_rational(rational: DyadicRationalNumber) -> Self {
        Self::new_nus(Nus::new_rational(rational))
    }

    /// Construct NUS with only nimber
    #[inline]
    pub fn new_nimber(number: DyadicRationalNumber, nimber: Nimber) -> Self {
        Self::new_nus(Nus {
            number,
            up_multiple: 0,
            nimber,
        })
    }

    /// Construct NUS
    #[inline]
    pub fn new_nus(nus: Nus) -> Self {
        CanonicalForm(CanonicalFormInner::Nus(nus))
    }

    /// Construct negative.0 of a game
    fn construct_negative(&self) -> Self {
        match &self.0 {
            CanonicalFormInner::Nus(nus) => CanonicalForm::new_nus(-nus),
            CanonicalFormInner::Moves(moves) => {
                let new_left_moves = moves
                    .left
                    .iter()
                    .map(|left| left.construct_negative())
                    .collect::<Vec<_>>();
                let new_right_moves = moves
                    .right
                    .iter()
                    .map(|right| right.construct_negative())
                    .collect::<Vec<_>>();
                let new_moves = Moves {
                    left: new_left_moves,
                    right: new_right_moves,
                };
                Self::construct_from_canonical_moves(new_moves)
            }
        }
    }

    /// Construct a sum of two games
    fn construct_sum(g: &CanonicalForm, h: &CanonicalForm) -> Self {
        if let (CanonicalFormInner::Nus(g_nus), CanonicalFormInner::Nus(h_nus)) = (&g.0, &h.0) {
            return Self::new_nus(g_nus + h_nus);
        }

        // We want to return { GL+H, G+HL | GR+H, G+HR }

        // By the number translation theorem

        let mut moves = Moves::empty();

        if !g.is_number() {
            let g_moves = g.to_moves();
            for g_l in &g_moves.left {
                moves.left.push(Self::construct_sum(g_l, h));
            }
            for g_r in &g_moves.right {
                moves.right.push(Self::construct_sum(g_r, h));
            }
        }
        if !h.is_number() {
            let h_moves = h.to_moves();
            for h_l in &h_moves.left {
                moves.left.push(Self::construct_sum(g, h_l));
            }
            for h_r in &h_moves.right {
                moves.right.push(Self::construct_sum(g, h_r));
            }
        }

        Self::new_from_moves(moves)
    }

    /// VERY INTERNAL
    fn construct_from_canonical_moves(mut moves: Moves) -> Self {
        moves.left.sort();
        moves.right.sort();

        if let Some(nus) = moves.to_nus() {
            return CanonicalForm::new_nus(nus);
        }

        // Game is not a nus
        CanonicalForm(CanonicalFormInner::Moves(moves))
    }

    /// Safe function to construct a game from possible moves
    pub fn new_from_moves(mut moves: Moves) -> Self {
        moves.eliminate_duplicates();

        let left_mex = CanonicalForm::mex(&moves.left);
        let right_mex = CanonicalForm::mex(&moves.right);
        if let (Some(left_mex), Some(right_mex)) = (left_mex, right_mex) {
            if left_mex == right_mex {
                let nus = Nus {
                    number: DyadicRationalNumber::from(0),
                    up_multiple: 0,
                    nimber: Nimber::from(left_mex),
                };
                return CanonicalForm::new_nus(nus);
            }
        }

        moves = moves.canonicalize();

        Self::construct_from_canonical_moves(moves)
    }

    /// Get left and right moves from a canonical form
    pub fn to_moves(&self) -> Moves {
        match &self.0 {
            CanonicalFormInner::Nus(nus) => nus.to_moves(),
            CanonicalFormInner::Moves(moves) => moves.clone(),
        }
    }

    /// Calculate mex if possible. Assumes that input is sorted
    fn mex(moves: &[CanonicalForm]) -> Option<u32> {
        let mut i = 0;
        let mut mex = 0;
        loop {
            if i >= moves.len() {
                break;
            }

            match moves[i].0 {
                CanonicalFormInner::Nus(nus) => {
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
                CanonicalFormInner::Moves(_) => return None,
            }
        }

        for m in &moves[i..] {
            if !m.is_nimber() {
                return None;
            }
        }

        Some(mex)
    }

    #[inline]
    fn get_nus_unchecked(&self) -> Nus {
        match self.0 {
            CanonicalFormInner::Nus(nus) => nus,
            CanonicalFormInner::Moves(_) => panic!("Not a nus"),
        }
    }

    /// Check if game is a Number Up Star sum
    #[inline]
    pub fn is_number_up_star(&self) -> bool {
        matches!(self.0, CanonicalFormInner::Nus(_))
    }

    /// Check if a game is only a number
    #[inline]
    pub fn is_number(&self) -> bool {
        matches!(self.0, CanonicalFormInner::Nus(nus) if nus.is_number())
    }

    /// Check if a game is only a nimber
    #[inline]
    pub fn is_nimber(&self) -> bool {
        matches!(self.0, CanonicalFormInner::Nus(nus) if nus.is_nimber())
    }

    /// Less than or equals comparison on two games
    pub fn leq(lhs_game: &Self, rhs_game: &Self) -> bool {
        if lhs_game == rhs_game {
            return true;
        }

        if let (CanonicalFormInner::Nus(lhs_nus), CanonicalFormInner::Nus(rhs_nus)) =
            (&lhs_game.0, &rhs_game.0)
        {
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

        if !lhs_game.is_number() {
            let lhs_game_moves = lhs_game.to_moves();
            for lhs_l in &lhs_game_moves.left {
                if Self::leq(rhs_game, lhs_l) {
                    return false;
                }
            }
        }

        if !rhs_game.is_number() {
            let rhs_game_moves = rhs_game.to_moves();
            for rhs_r in &rhs_game_moves.right {
                if Self::leq(rhs_r, lhs_game) {
                    return false;
                }
            }
        }

        true
    }

    // TODO: Should be dyadic but not sure how to handle infinities
    /// Calculate temperature of the game. Avoids computing a thermograph is game is a NUS
    pub fn temperature(&self) -> Rational {
        match self.0 {
            CanonicalFormInner::Nus(nus) => {
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
            CanonicalFormInner::Moves(_) => Self::thermograph(self).get_temperature(),
        }
    }

    /// Construct a thermograph of a game, using thermographic intersection of
    /// left and right scaffolds
    pub fn thermograph(&self) -> Thermograph {
        let thermograph = match self.0 {
            CanonicalFormInner::Moves(ref moves) => moves.thermograph(),
            CanonicalFormInner::Nus(nus) => {
                if nus.number.to_integer().is_some() && nus.is_number() {
                    Thermograph::with_mast(Rational::new(nus.number.to_integer().unwrap(), 1))
                } else {
                    if nus.up_multiple == 0
                        || (nus.nimber == Nimber::from(1) && nus.up_multiple.abs() == 1)
                    {
                        // This looks like 0 or * (depending on whether nimberPart is 0 or 1).
                        let new_game = Self::new_nus(Nus {
                            number: nus.number,
                            up_multiple: 0,
                            nimber: Nimber::from(nus.nimber.value().cmp(&0) as u32), // signum(nus.nimber)
                        });
                        let new_game_moves = new_game.to_moves();
                        new_game_moves.thermograph()
                    } else {
                        let new_game = Self::new_nus(Nus {
                            number: nus.number,
                            up_multiple: nus.up_multiple.cmp(&0) as i32, // signum(nus.up_multiple)
                            nimber: Nimber::from(0),
                        });
                        let new_game_moves = new_game.to_moves();
                        new_game_moves.thermograph()
                    }
                }
            }
        };
        thermograph
    }

    /// Parse game using `{a,b,...|c,d,...}` notation
    pub fn parse(input: &str) -> nom::IResult<&str, CanonicalForm> {
        alt((
            |input| Nus::parse(input).map(|(input, nus)| (input, CanonicalForm::new_nus(nus))),
            |input| {
                Moves::parse(input)
                    .map(|(input, moves)| (input, CanonicalForm::new_from_moves(moves)))
            },
        ))(input)
    }
}

impl_op_ex!(+|g: &CanonicalForm, h: &CanonicalForm| -> CanonicalForm { CanonicalForm::construct_sum(&g, &h) });
impl_op_ex!(-|g: &CanonicalForm| -> CanonicalForm { CanonicalForm::construct_negative(&g) });
impl_op_ex!(-|g: &CanonicalForm, h: &CanonicalForm| -> CanonicalForm {
    CanonicalForm::construct_sum(&g, &CanonicalForm::construct_negative(&h))
});

impl Display for CanonicalForm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.0 {
            CanonicalFormInner::Nus(nus) => nus.fmt(f),
            CanonicalFormInner::Moves(moves) => moves.fmt(f),
        }
    }
}

#[test]
fn constructs_integers() {
    let eight = CanonicalForm::new_integer(8);
    assert_eq!(&eight.to_string(), "8");
    let eight_moves = eight.to_moves();
    assert_eq!(&eight_moves.to_string(), "{7|}");
    assert_eq!(
        &eight_moves.print_deep_to_str(),
        "{{{{{{{{{|}|}|}|}|}|}|}|}|}"
    );

    let minus_forty_two = CanonicalForm::new_integer(-42);
    assert_eq!(&minus_forty_two.to_string(), "-42");
}

#[test]
fn constructs_rationals() {
    let rational = DyadicRationalNumber::new(3, 4);
    let three_sixteenth = CanonicalForm::new_rational(rational);
    assert_eq!(&three_sixteenth.to_string(), "3/16");

    let duplicate = CanonicalForm::new_rational(rational);
    assert_eq!(three_sixteenth, duplicate);
}

#[test]
fn constructs_nimbers() {
    let star = CanonicalForm::new_nus(Nus::new_nimber(Nimber::from(1)));
    assert_eq!(&star.to_string(), "*");
    let star_moves = star.to_moves();
    assert_eq!(&star_moves.to_string(), "{0|0}");
    assert_eq!(&star_moves.print_deep_to_str(), "{{|}|{|}}");

    let star_three = CanonicalForm::new_nus(Nus::new_nimber(Nimber::from(3)));
    assert_eq!(&star_three.to_string(), "*3");
    let star_three_moves = star_three.to_moves();
    assert_eq!(star_three_moves.to_string(), "{0, *, *2|0, *, *2}");

    let one_star_two = CanonicalForm::new_nus(Nus {
        number: DyadicRationalNumber::from(1),
        up_multiple: 0,
        nimber: (Nimber::from(2)),
    });
    assert_eq!(&one_star_two.to_string(), "1*2");
    let one_star_two_moves = one_star_two.to_moves();
    assert_eq!(&one_star_two_moves.to_string(), "{1, 1*|1, 1*}");
}

#[test]
fn constructs_up() {
    let up = CanonicalForm::new_nus(Nus {
        number: DyadicRationalNumber::from(0),
        up_multiple: 1,
        nimber: Nimber::from(0),
    });
    assert_eq!(&up.to_string(), "^");

    let up_star = CanonicalForm::new_nus(Nus {
        number: DyadicRationalNumber::from(0),
        up_multiple: 1,
        nimber: Nimber::from(1),
    });
    assert_eq!(&up_star.to_string(), "^*");

    let down = CanonicalForm::new_nus(Nus {
        number: DyadicRationalNumber::from(0),
        up_multiple: -3,
        nimber: Nimber::from(0),
    });
    assert_eq!(&down.to_string(), "v3");
}

#[test]
fn nimber_is_its_negative() {
    let star = CanonicalForm::new_nimber(DyadicRationalNumber::from(0), Nimber::from(4));
    assert_eq!(&star.to_string(), "*4");

    let neg_star = star.construct_negative();
    assert_eq!(star, neg_star);
}

#[test]
fn gets_moves() {
    let down_moves = CanonicalForm::new_nus(Nus::from_str("v").unwrap()).to_moves();
    assert_eq!(down_moves.to_string(), "{*|0}");
    assert_eq!(&down_moves.print_deep_to_str(), "{{{|}|{|}}|{|}}");

    let up_moves = CanonicalForm::new_nus(Nus::from_str("^").unwrap()).to_moves();
    assert_eq!(&up_moves.to_string(), "{0|*}");
    assert_eq!(up_moves.print_deep_to_str(), "{{|}|{{|}|{|}}}");

    let moves = Moves {
        left: vec![CanonicalForm::new_nus(Nus::from_str("v").unwrap())],
        right: vec![CanonicalForm::new_nus(Nus::from_str("-2").unwrap())],
    };
    assert_eq!(&moves.to_string(), "{v|-2}");
    assert_eq!(&moves.print_deep_to_str(), "{{{{|}|{|}}|{|}}|{|{|{|}}}}");
}

#[test]
fn simplifies_moves() {
    let one = CanonicalForm::new_nus(Nus::from_str("1").unwrap());
    let star = CanonicalForm::new_nus(Nus::from_str("*").unwrap());

    let moves_l = Moves {
        left: vec![one],
        right: vec![star],
    };
    let left_id = CanonicalForm::new_from_moves(moves_l);
    assert_eq!(&left_id.to_string(), "{1|*}");

    let weird = Moves {
        left: vec![CanonicalForm::new_nus(Nus::from_str("1v2*").unwrap())],
        right: vec![CanonicalForm::new_nus(Nus::from_str("1").unwrap())],
    };
    let weird = CanonicalForm::new_from_moves(weird);
    assert_eq!(&weird.to_string(), "1v3");
    let weird_moves = weird.to_moves();
    assert_eq!(&weird_moves.to_string(), "{1v2*|1}");
    assert_eq!(&weird_moves.left[0].to_string(), "1v2*");
    assert_eq!(&weird_moves.left[0].to_moves().to_string(), "{1v|1}");
    assert_eq!(
        &weird_moves.print_deep_to_str(),
        "{{{{{{|}|}|{{|}|}}|{{|}|}}|{{|}|}}|{{|}|}}"
    );

    // Another case:

    let weird_right = Moves {
        left: vec![CanonicalForm::new_nus(Nus::from_str("^").unwrap())],
        right: vec![CanonicalForm::new_nus(Nus::from_str("-2").unwrap())],
    };
    let weird_right = CanonicalForm::new_from_moves(weird_right);
    assert_eq!(&weird_right.to_string(), "{^|-2}");
    let weird_right_moves = weird_right.to_moves();
    dbg!(&weird_right);
    dbg!(&weird_right_moves);
    assert_eq!(&weird_right_moves.to_string(), "{^|-2}");
    assert_eq!(
        &weird_right_moves.print_deep_to_str(),
        "{{{|}|{{|}|{|}}}|{|{|{|}}}}"
    );

    let weird = Moves {
        left: vec![],
        right: vec![weird_right],
    };
    assert_ne!(
        &Moves::print_deep_to_str(&weird.canonicalize()),
        "{|{{{|}|{{|}|{|}}}|{|{|{|}}}}}"
    );
    assert_eq!(&weird.canonicalize().to_string(), "{|}");
    let weird = CanonicalForm::new_from_moves(weird);
    let weird_moves = weird.to_moves();
    assert_eq!(&weird_moves.to_string(), "{|}");
    assert_eq!(&weird.to_string(), "0");
}

#[test]
fn sum_works() {
    let zero = CanonicalForm::new_integer(0);
    let one = CanonicalForm::new_integer(1);

    let one_zero = CanonicalForm::new_from_moves(Moves {
        left: vec![one.clone()],
        right: vec![zero.clone()],
    });
    let zero_one = CanonicalForm::new_from_moves(Moves {
        left: vec![zero],
        right: vec![one],
    });
    // let sum = Game::construct_sum(&one_zero, &zero_one);
    let sum = one_zero + zero_one;
    assert_eq!(&sum.to_string(), "{3/2|1/2}");
}

#[test]
fn temp_of_one_minus_one_is_one() {
    let one = CanonicalForm::new_integer(1);
    let negative_one = CanonicalForm::new_integer(-1);

    let moves = Moves {
        left: vec![one],
        right: vec![negative_one],
    };
    let g = CanonicalForm::new_from_moves(moves);
    assert_eq!(g.temperature(), Rational::from(1));
}

impl_from_str_via_nom!(CanonicalForm);

#[cfg(test)]
macro_rules! test_game_parse {
    ($inp: expr, $expected: expr) => {{
        let g = CanonicalForm::parse($inp).expect("Could not parse").1;
        dbg!(&g);
        assert_eq!($expected, g.to_string());
    }};
}

#[test]
fn parse_games() {
    test_game_parse!("{|}", "0");
    test_game_parse!("{1,2|}", "3");
    test_game_parse!("{42|*}", "{42|*}");
    test_game_parse!("123", "123");
    test_game_parse!("{1/2|2}", "1");
}
