//! Canonical form of a short game

use crate::{
    display,
    numeric::{dyadic_rational_number::DyadicRationalNumber, nimber::Nimber, rational::Rational},
    parsing::{Parser, impl_from_str_via_parser, lexeme, try_option},
    short::partizan::{thermograph::Thermograph, trajectory::Trajectory},
};
use auto_ops::impl_op_ex;
use std::{
    borrow::Cow,
    cmp::Ordering,
    fmt::{self, Display, Write},
    hash::Hash,
    iter::Sum,
};

/// A number-up-star game position that is a sum of a number, up and, nimber.
#[derive(Debug, Hash, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Nus {
    number: DyadicRationalNumber,
    up_multiple: i32,
    nimber: Nimber,
}

impl Nus {
    /// Create new number-up-start sum
    #[inline]
    pub const fn new(number: DyadicRationalNumber, up_multiple: i32, nimber: Nimber) -> Self {
        Self {
            number,
            up_multiple,
            nimber,
        }
    }

    /// Create new number-up-star game equal to an integer.
    #[inline]
    pub const fn new_integer(integer: i64) -> Self {
        Self::new(
            DyadicRationalNumber::new_integer(integer),
            0,
            Nimber::new(0),
        )
    }

    /// Create new number-up-star game equal to an rational.
    #[inline]
    pub const fn new_number(number: DyadicRationalNumber) -> Self {
        Self::new(number, 0, Nimber::new(0))
    }

    /// Create new number-up-star game equal to an rational.
    #[inline]
    pub const fn new_nimber(nimber: Nimber) -> Self {
        Self::new(DyadicRationalNumber::new_integer(0), 0, nimber)
    }

    /// Get number part of the NUS sum
    #[inline]
    pub const fn number(self) -> DyadicRationalNumber {
        self.number
    }

    /// Get up/down part of the NUS sum. Positive for up, negative for down.
    #[inline]
    pub const fn up_multiple(self) -> i32 {
        self.up_multiple
    }

    /// Get nimber part of the NUS sum
    #[inline]
    pub const fn nimber(self) -> Nimber {
        self.nimber
    }

    /// Check if the game has only number part (i.e. up multiple and nimber are zero).
    #[inline]
    pub const fn is_number(self) -> bool {
        self.up_multiple() == 0 && self.nimber().value() == 0
    }

    /// Check if the game has only integer number part
    #[inline]
    pub const fn is_integer(self) -> bool {
        self.is_number() && self.number().to_integer().is_some()
    }

    /// Check if the game is a nimber.
    #[inline]
    pub const fn is_nimber(self) -> bool {
        self.number().eq_integer(0) && self.up_multiple() == 0
    }

    fn to_moves(self) -> Moves {
        // Case: Just a number
        if self.is_number() {
            if self.number() == DyadicRationalNumber::from(0) {
                return Moves {
                    left: vec![],
                    right: vec![],
                };
            }

            if let Some(integer) = self.number().to_integer() {
                let sign = if integer >= 0 { 1 } else { -1 };
                let prev = CanonicalForm::new_nus(Self::new_integer(integer - sign));

                if integer >= 0 {
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
                let rational = self.number();
                let left_move = CanonicalForm::new_nus(Self::new_number(rational.step(-1)));
                let right_move = CanonicalForm::new_nus(Self::new_number(rational.step(1)));
                return Moves {
                    left: vec![left_move],
                    right: vec![right_move],
                };
            }
        }

        // Case: number + nimber but no up/down
        if self.up_multiple() == 0 {
            let rational = self.number();
            let nimber = self.nimber();

            let mut moves = Moves::empty();
            for i in 0..nimber.value() {
                let new_nus = Self {
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
        let number_move = Self::new_number(self.number());

        let sign = if self.up_multiple() >= 0 { 1 } else { -1 };
        let prev_up = self.up_multiple() - sign;
        let up_parity: u32 = (self.up_multiple() & 1) as u32;
        let prev_nimber = self.nimber().value() ^ up_parity ^ (prev_up as u32 & 1);
        let moves;

        if self.up_multiple() == 1 && self.nimber() == Nimber::from(1) {
            // Special case: n^*
            let star_move = CanonicalForm::new_nus(Self {
                number: self.number(),
                up_multiple: 0,
                nimber: Nimber::from(1),
            });
            moves = Moves {
                left: vec![CanonicalForm::new_nus(number_move), star_move],
                right: vec![CanonicalForm::new_nus(number_move)],
            };
        } else if self.up_multiple() == -1 && self.nimber() == Nimber::from(1) {
            // Special case: nv*
            let star_move = CanonicalForm::new_nus(Self {
                number: self.number(),
                up_multiple: 0,
                nimber: Nimber::from(1),
            });
            moves = Moves {
                left: vec![CanonicalForm::new_nus(number_move)],
                right: vec![CanonicalForm::new_nus(number_move), star_move],
            };
        } else if self.up_multiple() > 0 {
            let prev_nus = CanonicalForm::new_nus(Self {
                number: self.number(),
                up_multiple: prev_up,
                nimber: Nimber::from(prev_nimber),
            });
            moves = Moves {
                left: vec![CanonicalForm::new_nus(number_move)],
                right: vec![prev_nus],
            };
        } else {
            let prev_nus = CanonicalForm::new_nus(Self {
                number: self.number(),
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

    // TODO: Have iterators for these
    fn to_left_moves(self) -> Vec<CanonicalForm> {
        // Case: Just a number
        if self.is_number() {
            if self.number() == DyadicRationalNumber::from(0) {
                return vec![];
            }

            if let Some(integer) = self.number().to_integer() {
                let sign = if integer >= 0 { 1 } else { -1 };
                let prev = CanonicalForm::new_nus(Self::new_integer(integer - sign));

                if integer >= 0 {
                    return vec![prev];
                } else if sign < 0 {
                    return vec![];
                }
            } else {
                let rational = self.number();
                let left_move = CanonicalForm::new_nus(Self::new_number(rational.step(-1)));
                return vec![left_move];
            }
        }

        // Case: number + nimber but no up/down
        if self.up_multiple() == 0 {
            let rational = self.number();
            let nimber = self.nimber();

            let mut left = Vec::with_capacity(nimber.value() as usize);
            for i in 0..nimber.value() {
                let new_nus = Self {
                    number: rational,
                    up_multiple: 0,
                    nimber: Nimber::from(i),
                };
                left.push(CanonicalForm::new_nus(new_nus));
            }
            return left;
        }

        // Case: number-up-star
        let number_move = Self::new_number(self.number());

        let sign = if self.up_multiple() >= 0 { 1 } else { -1 };
        let prev_up = self.up_multiple() - sign;
        let up_parity: u32 = (self.up_multiple() & 1) as u32;
        let prev_nimber = self.nimber().value() ^ up_parity ^ (prev_up as u32 & 1);

        if self.up_multiple() == 1 && self.nimber() == Nimber::from(1) {
            // Special case: n^*
            let star_move = CanonicalForm::new_nus(Self {
                number: self.number(),
                up_multiple: 0,
                nimber: Nimber::from(1),
            });
            vec![CanonicalForm::new_nus(number_move), star_move]
        } else if self.up_multiple() == -1 && self.nimber() == Nimber::from(1) {
            // Special case: nv*
            vec![CanonicalForm::new_nus(number_move)]
        } else if self.up_multiple() > 0 {
            vec![CanonicalForm::new_nus(number_move)]
        } else {
            let prev_nus = CanonicalForm::new_nus(Self {
                number: self.number(),
                up_multiple: prev_up,
                nimber: Nimber::from(prev_nimber),
            });
            vec![prev_nus]
        }
    }

    // TODO: Have iterators for these
    fn to_right_moves(self) -> Vec<CanonicalForm> {
        // Case: Just a number
        if self.is_number() {
            if self.number() == DyadicRationalNumber::from(0) {
                return vec![];
            }

            if let Some(integer) = self.number().to_integer() {
                let sign = if integer >= 0 { 1 } else { -1 };
                let prev = CanonicalForm::new_nus(Self::new_integer(integer - sign));

                if integer >= 0 {
                    return vec![];
                } else if sign < 0 {
                    return vec![prev];
                }
            } else {
                let rational = self.number();
                let right_move = CanonicalForm::new_nus(Self::new_number(rational.step(1)));
                return vec![right_move];
            }
        }

        // Case: number + nimber but no up/down
        if self.up_multiple() == 0 {
            let rational = self.number();
            let nimber = self.nimber();

            let mut right = Vec::with_capacity(nimber.value() as usize);
            for i in 0..nimber.value() {
                let new_nus = Self {
                    number: rational,
                    up_multiple: 0,
                    nimber: Nimber::from(i),
                };
                right.push(CanonicalForm::new_nus(new_nus));
            }
            return right;
        }

        // Case: number-up-star
        let number_move = Self::new_number(self.number());

        let sign = if self.up_multiple() >= 0 { 1 } else { -1 };
        let prev_up = self.up_multiple() - sign;
        let up_parity: u32 = (self.up_multiple() & 1) as u32;
        let prev_nimber = self.nimber().value() ^ up_parity ^ (prev_up as u32 & 1);

        if self.up_multiple() == 1 && self.nimber() == Nimber::from(1) {
            // Special case: n^*
            vec![CanonicalForm::new_nus(number_move)]
        } else if self.up_multiple() == -1 && self.nimber() == Nimber::from(1) {
            // Special case: nv*
            let star_move = CanonicalForm::new_nus(Self {
                number: self.number(),
                up_multiple: 0,
                nimber: Nimber::from(1),
            });
            vec![CanonicalForm::new_nus(number_move), star_move]
        } else if self.up_multiple() > 0 {
            let prev_nus = CanonicalForm::new_nus(Self {
                number: self.number(),
                up_multiple: prev_up,
                nimber: Nimber::from(prev_nimber),
            });
            vec![prev_nus]
        } else {
            vec![CanonicalForm::new_nus(number_move)]
        }
    }

    /// Parse nus from string, using notation without pluses between number, up, and star components
    ///
    /// Pattern: `\d*([v^]\d*)?(\*\d*)`
    pub const fn parse(p: Parser<'_>) -> Option<(Parser<'_>, Nus)> {
        // This flag is set if we explicitly parse a number, rather than set it to zero if
        // it is omitted. It makes expressions like `*` a valid input, however it also makes
        // empty input parse to a zero game, which is undesired. We handle that case explicitly.
        let parsed_number: bool;

        let (p, number) = if let Some((p, number)) = lexeme!(p, DyadicRationalNumber::parse) {
            parsed_number = true;
            (p, number)
        } else {
            parsed_number = false;
            (p, DyadicRationalNumber::new_integer(0))
        };

        let p = p.trim_whitespace();
        let (p, up_multiple) = match lexeme!(p, Parser::parse_any_ascii_char) {
            Some((p, c)) if c == '^' || c == 'v' => {
                // TODO: add parse_i32
                let (p, up_multiple) = match lexeme!(p, Parser::parse_i64) {
                    Some((p, up_multiple)) => (p, up_multiple),
                    None => (p, 1),
                };
                (
                    p,
                    if c == 'v' {
                        -(up_multiple as i32)
                    } else {
                        up_multiple as i32
                    },
                )
            }
            _ => (p, 0),
        };

        let (p, star_multiple) = match lexeme!(p, Parser::parse_any_ascii_char) {
            Some((p, '*')) => match lexeme!(p, Parser::parse_u32) {
                Some((p, star_multiple)) => (p, star_multiple),
                None => (p, 1),
            },
            _ => (p, 0),
        };

        if number.eq_integer(0) && up_multiple == 0 && star_multiple == 0 && !parsed_number {
            None
        } else {
            Some((
                p,
                Self {
                    number,
                    up_multiple,
                    nimber: Nimber::new(star_multiple),
                },
            ))
        }
    }
}

impl_from_str_via_parser!(Nus);

impl_op_ex!(+|lhs: &Nus, rhs: &Nus| -> Nus {
    Nus {
        number: lhs.number() + rhs.number(),
        up_multiple: lhs.up_multiple() + rhs.up_multiple(),
        nimber: lhs.nimber() + rhs.nimber(),
    }
});

impl_op_ex!(-|lhs: &Nus| -> Nus {
    Nus {
        number: -lhs.number(),
        up_multiple: -lhs.up_multiple(),
        nimber: lhs.nimber(), // Nimber is its own negative
    }
});

impl Display for Nus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.number() == DyadicRationalNumber::from(0)
            && self.up_multiple() == 0
            && self.nimber() == Nimber::from(0)
        {
            write!(f, "0")?;
            return Ok(());
        }

        if self.number() != DyadicRationalNumber::from(0) {
            write!(f, "{}", self.number())?;
        }

        match self.up_multiple() {
            1 => write!(f, "^")?,
            -1 => write!(f, "v")?,
            n if n > 0 => write!(f, "^{}", n)?,
            n if n < 0 => write!(f, "v{}", -n)?,
            _ => {}
        }

        if self.nimber() != Nimber::from(0) {
            write!(f, "{}", self.nimber())?;
        }

        Ok(())
    }
}

/// Left and Right moves from a given position
#[derive(Debug, Hash, Clone, PartialEq, Eq)]
pub struct Moves {
    /// Left player's moves
    pub left: Vec<CanonicalForm>,

    /// Right player's moves
    pub right: Vec<CanonicalForm>,
}

impl PartialOrd for Moves {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Moves {
    fn cmp(&self, other: &Self) -> Ordering {
        let left = self
            .left
            .iter()
            .map(|cf| &cf.inner)
            .cmp(other.left.iter().map(|cf| &cf.inner));

        if left.is_eq() {
            self.right
                .iter()
                .map(|cf| &cf.inner)
                .cmp(other.right.iter().map(|cf| &cf.inner))
        } else {
            left
        }
    }
}

impl Moves {
    #[inline]
    const fn empty() -> Self {
        Self {
            left: vec![],
            right: vec![],
        }
    }

    #[inline]
    fn eliminate_duplicates(&mut self) {
        self.left.sort_by(|lhs, rhs| lhs.inner.cmp(&rhs.inner));
        self.left.dedup_by(|lhs, rhs| lhs.inner == rhs.inner);

        self.right.sort_by(|lhs, rhs| lhs.inner.cmp(&rhs.inner));
        self.right.dedup_by(|lhs, rhs| lhs.inner == rhs.inner);
    }

    /// Construct a canoical form of arbitrary moves.
    /// It is an alias of [`CanonicalForm::new_from_moves`]
    #[inline]
    pub fn canonical_form(self) -> CanonicalForm {
        CanonicalForm::new_from_moves(self)
    }

    /// Try converting moves to NUS. Returns [None] if moves do not form a NUS
    #[allow(clippy::cognitive_complexity)]
    pub fn to_nus(&self) -> Option<Nus> {
        let num_lo = self.left.len();
        let num_ro = self.right.len();

        if num_lo == 0 && num_ro == 0 {
            // Case: {|}
            // No left or right moves so the game is 0
            Some(Nus {
                number: DyadicRationalNumber::from(0),
                up_multiple: 0,
                nimber: Nimber::from(0),
            })
        } else if num_lo == 0 {
            // Case: n-1 = {|n}
            // We assume that entry is normalized, no left moves, thus there must be only one
            // right entry that's a number
            debug_assert!(num_ro == 1, "Entry not normalized");
            Some(Nus {
                number: self.right[0].to_nus_unchecked().number() - DyadicRationalNumber::from(1),
                up_multiple: 0,
                nimber: Nimber::from(0),
            })
        } else if num_ro == 0 {
            // Case: n+1 = {n|}
            // We assume that entry is normalized, no left moves, thus there must be only one
            // right entry that's a number
            debug_assert!(num_lo == 1, "Entry not normalized");
            Some(Nus {
                number: self.left[0].to_nus_unchecked().number() + DyadicRationalNumber::from(1),
                up_multiple: 0,
                nimber: Nimber::from(0),
            })
        } else if let [left_move] = &self.left[..]
            && let [right_move] = &self.right[..]
            && let Some(left_number) = left_move.to_number()
            && let Some(right_number) = right_move.to_number()
            && left_number < right_number
        {
            // Case: {n|m}, n < m
            // We're a number but not an integer.  Conveniently, since the option lists are
            // canonicalized, the value of this game is the mean of its left & right moves.

            Some(Nus {
                number: DyadicRationalNumber::mean(&left_number, &right_number),
                up_multiple: 0,
                nimber: Nimber::from(0),
            })
        } else if let [left_move1, left_move2] = &self.left[..]
            && let [right_move] = &self.right[..]
            && let Some(left_number) = left_move1.to_number()
            && left_move1 == right_move
            && let Some(left_nus) = left_move2.to_nus()
            && left_number == left_nus.number()
            && left_nus.up_multiple() == 0
            && left_nus.nimber() == Nimber::new(1)
        {
            // Case: {n,n*|n}
            Some(Nus {
                number: left_number,
                up_multiple: 1,
                nimber: Nimber::from(1),
            })
        } else if let [left_move] = &self.left[..]
            && let [right_move1, right_move2] = &self.right[..]
            && let Some(right_number) = right_move1.to_number()
            && left_move == right_move1
            && let Some(right_nus) = right_move2.to_nus()
            && right_number == right_nus.number()
            && right_nus.up_multiple() == 0
            && right_nus.nimber() == Nimber::new(1)
        {
            // Inverse of the previous one
            Some(Nus {
                number: right_number,
                up_multiple: -1,
                nimber: Nimber::from(1),
            })
        } else if let [left_move] = &self.left[..]
            && let [right_move] = &self.right[..]
            && let Some(left_number) = left_move.to_number()
            && let Some(right_nus) = right_move.to_nus()
            && !right_nus.is_number()
            && left_number == right_nus.number()
            && right_nus.up_multiple() >= 0
        {
            // Case: n + {0|G}, G is a number-up-star of up multiple >= 0
            Some(Nus {
                number: right_nus.number(),
                up_multiple: right_nus.up_multiple() + 1,
                nimber: right_nus.nimber() + Nimber::from(1),
            })
        } else if let [left_move] = &self.left[..]
            && let [right_move] = &self.right[..]
            && let Some(left_nus) = left_move.to_nus()
            && let Some(right_number) = right_move.to_number()
            && !left_nus.is_number()
            && right_number == left_nus.number()
            && left_nus.up_multiple() <= 0
        {
            // Inverse of the previous one
            Some(Nus {
                number: left_nus.number(),
                up_multiple: left_nus.up_multiple() - 1,
                nimber: left_nus.nimber() + Nimber::from(1),
            })
        } else if num_lo >= 1
            && num_lo == num_ro
            && let Some(left_number) = self.left[0].to_number()
            && self.left[0] == self.right[0]
        {
            // Case: n + *k
            // If doesn't hold then it's not a NUS

            for i in 0..num_lo {
                let l = &self.left[i];
                let r = &self.right[i];

                if l != r
                    || !l.is_number_up_star()
                    || l.to_nus_unchecked().number() != r.to_nus_unchecked().number()
                {
                    return None;
                }

                if l.to_nus_unchecked().up_multiple() != 0
                    || l.to_nus_unchecked().nimber().value() != (i as u32)
                {
                    return None;
                }
            }
            // It's a nimber
            Some(Nus {
                number: left_number,
                up_multiple: 0,
                nimber: Nimber::from(num_lo as u32),
            })
        } else {
            None
        }
    }

    // TODO: Rewrite it to work on mutable vec and not clone
    fn eliminate_dominated_moves(
        moves: &[CanonicalForm],
        eliminate_smaller_moves: bool,
    ) -> Vec<CanonicalForm> {
        let mut moves: Vec<Option<CanonicalForm>> = moves.iter().cloned().map(Some).collect();

        'outer: for i in 0..moves.len() {
            'inner: for j in 0..i {
                let Some(move_i) = &moves[i] else {
                    continue 'outer;
                };
                let Some(move_j) = &moves[j] else {
                    continue 'inner;
                };

                // Split from ifs because borrow checker is sad
                let remove_i = (eliminate_smaller_moves && move_i <= move_j)
                    || (!eliminate_smaller_moves && move_j <= move_i);

                let remove_j = (eliminate_smaller_moves && move_j <= move_i)
                    || (!eliminate_smaller_moves && move_i <= move_j);

                if remove_i {
                    moves[i] = None;
                }

                if remove_j {
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
        for r_opt in right_moves.iter().flatten() {
            if r_opt <= game {
                return false;
            }
        }

        for l_move in game.to_left_moves().iter() {
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
        for l_opt in left_moves.iter().flatten() {
            if game <= l_opt {
                return false;
            }
        }

        for r_move in game.to_right_moves().iter() {
            if Self::leq_arrays(r_move, left_moves, right_moves) {
                return false;
            }
        }

        true
    }

    fn bypass_reversible_moves_l(&self) -> Self {
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
            for g_lr in g_l.to_right_moves().iter() {
                if Self::leq_arrays(g_lr, &left_moves, &right_moves) {
                    let g_lr_moves = g_lr.to_left_moves();
                    let mut new_left_moves: Vec<Option<CanonicalForm>> =
                        vec![None; left_moves.len() + g_lr_moves.len() - 1];
                    new_left_moves[..(i as usize)].clone_from_slice(&left_moves[..(i as usize)]);
                    new_left_moves[(i as usize)..(left_moves.len() - 1)]
                        .clone_from_slice(&left_moves[(i as usize + 1)..]);
                    for (k, g_lrl) in g_lr_moves.iter().enumerate() {
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
        Self {
            left: left_moves.iter().flatten().cloned().collect(),
            right: self.right.clone(),
        }
    }

    fn bypass_reversible_moves_r(&self) -> Self {
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
            for g_rl in g_r.to_left_moves().iter() {
                if Self::geq_arrays(g_rl, &left_moves, &right_moves) {
                    let g_rl_moves = g_rl.to_right_moves();
                    let mut new_right_moves: Vec<Option<CanonicalForm>> =
                        vec![None; right_moves.len() + g_rl_moves.len() - 1];
                    new_right_moves[..(i as usize)].clone_from_slice(&right_moves[..(i as usize)]);
                    new_right_moves[(i as usize)..(right_moves.len() - 1)]
                        .clone_from_slice(&right_moves[(i as usize + 1)..]);
                    for (k, g_rlr) in g_rl_moves.iter().enumerate() {
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
        Self {
            left: self.left.clone(),
            right: right_moves.iter().flatten().cloned().collect(),
        }
    }

    fn canonicalize(&self) -> Self {
        let moves = self.bypass_reversible_moves_l();
        let moves = moves.bypass_reversible_moves_r();

        let left = Self::eliminate_dominated_moves(&moves.left, true);
        let right = Self::eliminate_dominated_moves(&moves.right, false);

        Self { left, right }
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
    #[allow(clippy::missing_errors_doc)]
    pub fn print_deep(&self, f: &mut impl Write) -> fmt::Result {
        display::braces(f, |f| {
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
            Ok(())
        })
    }

    /// Print moves to string with NUS unwrapped using `{G^L | G^R}` notation
    // Write to `String` never panics
    #[allow(clippy::missing_panics_doc)]
    pub fn print_deep_to_str(&self) -> String {
        let mut buf = String::new();
        Self::print_deep(self, &mut buf).unwrap();
        buf
    }

    /// Parse comma-separated games, ie. the underlined part:
    ///
    /// `{a,b,...|c,d,...}`
    ///
    /// ` ^^^^^^^`
    fn parse_list2(mut p: Parser<'_>) -> Option<(Parser<'_>, Vec<CanonicalForm>)> {
        let mut acc = Vec::new();
        loop {
            match lexeme!(p, CanonicalForm::parse) {
                Some((cf_p, cf)) => {
                    acc.push(cf);
                    p = cf_p;
                    p = p.trim_whitespace();
                    match p.parse_ascii_char(',') {
                        Some(pp) => {
                            p = pp.trim_whitespace();
                        }
                        None => return Some((p, acc)),
                    }
                }
                None => return Some((p, acc)),
            }
        }
    }

    fn parse(p: Parser<'_>) -> Option<(Parser<'_>, Moves)> {
        let p = try_option!(p.parse_ascii_char('{'));
        let (p, left) = try_option!(Moves::parse_list2(p));
        let p = try_option!(p.parse_ascii_char('|'));
        let (p, right) = try_option!(Moves::parse_list2(p));
        let p = try_option!(p.parse_ascii_char('}'));
        let moves = Self { left, right };
        Some((p, moves))
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

impl_from_str_via_parser!(Moves);

/// A game `G` even-tempered if, no matter how `G` is played, the first player will have the move
/// when `G` reaches a number.
#[derive(Debug, Hash, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Temper {
    /// `G` is even-tempered if `G` a number, or every option of `G` is odd-tempered
    Even,

    /// `G` is odd-tempered if `G` is not a number and every option of `G` is even-tempered
    Odd,
}

/// Canonical game form
///
/// Note that ordering is defined structurally for the sake of data structures. For proper partial
/// ordering see instance for [`CanonicalForm`].
#[derive(Debug, Hash, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
enum CanonicalFormInner {
    /// Number Up Star sum
    Nus(Nus),

    /// Not a NUS - list of left/right moves
    Moves(Moves),
}

/// Canonical game form
#[repr(transparent)]
#[derive(Debug, Hash, Clone, PartialEq, Eq)]
pub struct CanonicalForm {
    inner: CanonicalFormInner,
}

impl CanonicalForm {
    /// Construct NUS with only integer
    #[inline]
    pub const fn new_integer(integer: i64) -> Self {
        Self::new_nus(Nus::new_integer(integer))
    }

    /// Construct NUS with only dyadic rational
    #[inline]
    pub const fn new_dyadic(dyadic: DyadicRationalNumber) -> Self {
        Self::new_nus(Nus::new_number(dyadic))
    }

    /// Construct NUS with only nimber
    #[inline]
    pub const fn new_nimber(number: DyadicRationalNumber, nimber: Nimber) -> Self {
        Self::new_nus(Nus {
            number,
            up_multiple: 0,
            nimber,
        })
    }

    /// Construct NUS
    #[inline]
    #[must_use]
    pub const fn new_nus(nus: Nus) -> Self {
        Self::from_inner(CanonicalFormInner::Nus(nus))
    }

    /// Construct negative.0 of a game. Alias for negation [`-`] operator
    #[must_use]
    pub fn construct_negative(&self) -> Self {
        match &self.inner {
            CanonicalFormInner::Nus(nus) => Self::new_nus(-nus),
            CanonicalFormInner::Moves(moves) => {
                let new_left_moves = moves
                    .left
                    .iter()
                    .map(Self::construct_negative)
                    .collect::<Vec<_>>();
                let new_right_moves = moves
                    .right
                    .iter()
                    .map(Self::construct_negative)
                    .collect::<Vec<_>>();
                let new_moves = Moves {
                    left: new_left_moves,
                    right: new_right_moves,
                };
                Self::construct_from_canonical_moves(new_moves)
            }
        }
    }

    /// Construct a sum of two games. Alias for [`+`] operator
    pub fn construct_sum(g: &Self, h: &Self) -> Self {
        if let (CanonicalFormInner::Nus(g_nus), CanonicalFormInner::Nus(h_nus)) =
            (&g.inner, &h.inner)
        {
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
        moves.left.sort_by(|lhs, rhs| lhs.inner.cmp(&rhs.inner));
        moves.right.sort_by(|lhs, rhs| lhs.inner.cmp(&rhs.inner));

        if let Some(nus) = moves.to_nus() {
            return Self::new_nus(nus);
        }

        // Game is not a nus
        Self::from_inner(CanonicalFormInner::Moves(moves))
    }

    /// Safe function to construct a game from possible moves
    pub fn new_from_moves(mut moves: Moves) -> Self {
        moves.eliminate_duplicates();
        moves = moves.canonicalize();

        Self::construct_from_canonical_moves(moves)
    }

    #[inline]
    const fn from_inner(inner: CanonicalFormInner) -> Self {
        Self { inner }
    }

    /// Get left and right moves from a canonical form
    pub fn to_moves(&self) -> Moves {
        match &self.inner {
            CanonicalFormInner::Nus(nus) => nus.to_moves(),
            CanonicalFormInner::Moves(moves) => moves.clone(),
        }
    }

    /// Get left moves from a canonical form
    pub fn to_left_moves(&self) -> Cow<'_, [CanonicalForm]> {
        match &self.inner {
            CanonicalFormInner::Nus(nus) => Cow::Owned(nus.to_left_moves()),
            CanonicalFormInner::Moves(moves) => Cow::Borrowed(&moves.left),
        }
    }

    /// Get right moves from a canonical form
    pub fn to_right_moves(&self) -> Cow<'_, [CanonicalForm]> {
        match &self.inner {
            CanonicalFormInner::Nus(nus) => Cow::Owned(nus.to_right_moves()),
            CanonicalFormInner::Moves(moves) => Cow::Borrowed(&moves.right),
        }
    }

    /// Check if game is a Number Up Star sum
    #[inline]
    pub const fn is_number_up_star(&self) -> bool {
        matches!(self.inner, CanonicalFormInner::Nus(_))
    }

    /// Check if a game is only a number
    #[inline]
    pub const fn is_number(&self) -> bool {
        matches!(self.inner, CanonicalFormInner::Nus(nus) if nus.is_number())
    }

    /// Check if a game is only a nimber
    #[inline]
    pub const fn is_nimber(&self) -> bool {
        matches!(self.inner, CanonicalFormInner::Nus(nus) if nus.is_nimber())
    }

    /// Convert game to NUS if it is a NUS
    #[inline]
    pub const fn to_nus(&self) -> Option<Nus> {
        match self.inner {
            CanonicalFormInner::Nus(nus) => Some(nus),
            // Don't call Moves::to_nus here, because (a) it's already canonical and (b)
            // it calls here.
            CanonicalFormInner::Moves(_) => None,
        }
    }

    #[inline]
    const fn to_nus_unchecked(&self) -> Nus {
        self.to_nus().expect("Not a nus")
    }

    /// Convert game to number if it is only a number (i.e. [`Self::is_number`])
    #[inline]
    pub const fn to_number(&self) -> Option<DyadicRationalNumber> {
        match self.to_nus() {
            Some(nus) if nus.is_number() => Some(nus.number()),
            _ => None,
        }
    }

    /// Less than or equals comparison on two games
    pub fn leq(lhs_game: &Self, rhs_game: &Self) -> bool {
        // NOTE: There is a possible optimization.
        // Lessons in Play: Lemma 5.35: Let x be a number and G and H be games. If G.right_stop() > x
        // then G > x. If RS(G) > LS(H) then G > H.

        if lhs_game == rhs_game {
            return true;
        }

        if let (Some(lhs_nus), Some(rhs_nus)) = (&lhs_game.to_nus(), &rhs_game.to_nus()) {
            match lhs_nus.number().cmp(&rhs_nus.number()) {
                Ordering::Less => return true,
                Ordering::Greater => return false,
                Ordering::Equal => {
                    if lhs_nus.up_multiple() < rhs_nus.up_multiple() - 1 {
                        return true;
                    } else if lhs_nus.up_multiple() < rhs_nus.up_multiple() {
                        return (lhs_nus.nimber() + rhs_nus.nimber()) != Nimber::from(1);
                    }
                    return false;
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

    /// Calculate temperature of the game. Avoids computing a thermograph is game is a NUS
    #[allow(clippy::missing_panics_doc)]
    pub fn temperature(&self) -> DyadicRationalNumber {
        match self.inner {
            CanonicalFormInner::Nus(nus) => {
                if nus.is_number() {
                    // It's a number k/2^n, so the temperature is -1/2^n
                    DyadicRationalNumber::new(-1, nus.number().denominator_exponent())
                } else {
                    // It's a number plus a nonzero infinitesimal, thus the temperature is 0
                    DyadicRationalNumber::from(0)
                }
            }
            CanonicalFormInner::Moves(ref moves) => moves.thermograph().temperature(),
        }
    }

    /// Construct a thermograph of a game, using thermographic intersection of
    /// left and right scaffolds
    pub fn thermograph(&self) -> Thermograph {
        match self.inner {
            CanonicalFormInner::Moves(ref moves) => moves.thermograph(),
            CanonicalFormInner::Nus(nus) => {
                if let Some(nus_integer) = nus.number().to_integer() {
                    if nus.is_number() {
                        return Thermograph::with_mast(Rational::new(nus_integer, 1));
                    }
                }

                if nus.up_multiple() == 0
                    || (nus.nimber() == Nimber::from(1) && nus.up_multiple().abs() == 1)
                {
                    // This looks like 0 or * (depending on whether nimberPart is 0 or 1).
                    let new_game = Self::new_nus(Nus {
                        number: nus.number(),
                        up_multiple: 0,
                        nimber: Nimber::from(nus.nimber().value().cmp(&0) as u32), // signum(nus.nimber)
                    });
                    let new_game_moves = new_game.to_moves();
                    new_game_moves.thermograph()
                } else {
                    let new_game = Self::new_nus(Nus {
                        number: nus.number(),
                        up_multiple: nus.up_multiple().cmp(&0) as i32, // signum(nus.up_multiple)
                        nimber: Nimber::from(0),
                    });
                    let new_game_moves = new_game.to_moves();
                    new_game_moves.thermograph()
                }
            }
        }
    }

    /// The number reached when Left plays first.
    pub fn left_stop(&self) -> DyadicRationalNumber {
        if let Some(number) = self.to_number() {
            return number;
        }

        self.to_moves()
            .left
            .iter()
            .map(Self::right_stop)
            .max()
            .expect("Not a number so must have moves")
    }

    /// The number reached when Right plays first.
    pub fn right_stop(&self) -> DyadicRationalNumber {
        if let Some(number) = self.to_number() {
            return number;
        }

        self.to_moves()
            .right
            .iter()
            .map(Self::left_stop)
            .max()
            .expect("Not a number so must have moves")
    }

    /// Confusion interval is the region between Left and Right stops
    pub fn confusion_interval(&self) -> (DyadicRationalNumber, DyadicRationalNumber) {
        (self.left_stop(), self.right_stop())
    }

    /// Compute the mean value of the position
    ///
    /// Mean value is the result of cooling a position by value greater than temperature
    pub fn mean(&self) -> DyadicRationalNumber {
        match self.inner {
            CanonicalFormInner::Nus(nus) => nus.number(),
            CanonicalFormInner::Moves(ref moves) => {
                let mast = moves.thermograph().get_mast();
                DyadicRationalNumber::from_rational(mast)
                    .expect("Thermograph mast to have a finite dyadic value")
            }
        }
    }

    /// Cool the position by `temperature`
    ///
    /// Position `G` cooled by `t` is `G_t = {G^L_t - t | G^R_t + t}` unless there exists a
    /// temperature `t' < t` for which `G_t'` is infinitesimally close to a number
    #[must_use]
    pub fn cool(&self, temperature: DyadicRationalNumber) -> Self {
        if let Some(nus) = self.to_nus() {
            if nus.is_integer() {
                return self.clone();
            }
        }

        if self.temperature() < temperature {
            return Self::new_dyadic(self.mean());
        }

        let temperature_game = Self::new_dyadic(temperature);

        let moves = self.to_moves();

        let mut new_left_moves = Vec::with_capacity(moves.left.len());
        for left_move in moves.left {
            new_left_moves.push(left_move.cool(temperature) - &temperature_game);
        }

        let mut new_right_moves = Vec::with_capacity(moves.right.len());
        for right_move in moves.right {
            new_right_moves.push(right_move.cool(temperature) + &temperature_game);
        }

        let new_moves = Moves {
            left: new_left_moves,
            right: new_right_moves,
        };

        new_moves.canonical_form()
    }

    /// Heat position by given `temperature`.
    ///
    /// Heating is the inverse of cooling, defined as `\int^t G = G` if `G` is a number, or
    /// `\int^t G = {\int^t G^L + t | \int^t G^R - t}` otherwise
    #[must_use]
    pub fn heat(&self, temperature: &CanonicalForm) -> Self {
        if let Some(nus) = self.to_nus() {
            if nus.is_number() {
                return self.clone();
            }
        }

        let moves = self.to_moves();

        let mut new_left_moves = Vec::with_capacity(moves.left.len());
        for left_move in moves.left {
            new_left_moves.push(left_move.heat(temperature) + temperature);
        }

        let mut new_right_moves = Vec::with_capacity(moves.right.len());
        for right_move in moves.right {
            new_right_moves.push(right_move.heat(temperature) - temperature);
        }

        let new_moves = Moves {
            left: new_left_moves,
            right: new_right_moves,
        };

        new_moves.canonical_form()
    }

    /// A remote star of game `g` is a nimber `*N` if no position of `g` including `g` has value `N*`
    #[must_use]
    #[allow(clippy::or_fun_call)]
    pub fn far_star(&self) -> Nimber {
        if let CanonicalFormInner::Nus(ref nus) = self.inner {
            if nus.is_nimber() {
                return Nimber::from(nus.nimber().value() + 1);
            }
        }

        let moves = self.to_moves();
        moves
            .left
            .iter()
            .chain(moves.right.iter())
            .map(Self::far_star)
            .max()
            .unwrap_or(Nimber::from(1))
    }

    // FIXME: Handle cases when atomic weight does not exist
    /// Atmoic weight of a position, sometimes called "uppitiness"
    #[must_use]
    pub fn atomic_weight(&self) -> Self {
        match self.inner {
            CanonicalFormInner::Nus(nus) => Self::new_integer(nus.up_multiple() as i64),
            CanonicalFormInner::Moves(ref moves) => {
                let new_moves = Moves {
                    left: moves
                        .left
                        .iter()
                        .map(|left_move| left_move.atomic_weight() - Self::new_integer(2))
                        .collect::<Vec<_>>(),
                    right: moves
                        .right
                        .iter()
                        .map(|right_move| right_move.atomic_weight() + Self::new_integer(2))
                        .collect::<Vec<_>>(),
                };
                let new_game = Self::new_from_moves(new_moves.clone());

                let CanonicalFormInner::Nus(new_nus) = new_game.inner else {
                    return new_game;
                };

                if !new_nus.is_integer() {
                    return new_game;
                }

                let far_star = Self::new_nimber(DyadicRationalNumber::from(0), self.far_star());

                let less_than_far_star = self <= &far_star;
                let greater_than_far_star = self >= &far_star;

                if less_than_far_star && !greater_than_far_star {
                    let max_least = new_moves
                        .left
                        .iter()
                        .map(|left_move| {
                            let least = left_move.right_stop().ceil();
                            if &Self::new_integer(least) <= left_move {
                                least + 1
                            } else {
                                least
                            }
                        })
                        .max()
                        .unwrap_or(0);
                    Self::new_integer(max_least)
                } else if !less_than_far_star && greater_than_far_star {
                    let min_greatest = new_moves
                        .right
                        .iter()
                        .map(|right_move| {
                            let greatest = right_move.left_stop().round();
                            if right_move <= &Self::new_integer(greatest) {
                                greatest - 1
                            } else {
                                greatest
                            }
                        })
                        .min()
                        .unwrap_or(0);
                    Self::new_integer(min_greatest)
                } else {
                    new_game
                }
            }
        }
    }

    /// See: The Reduced Canonical Form Of a Game p. 411
    #[must_use]
    pub fn star_projection(&self) -> Self {
        if let Some(nus) = self.to_nus() {
            if (nus.nimber() == Nimber::new(0) || nus.nimber() == Nimber::new(1))
                && nus.up_multiple() == 0
            {
                return CanonicalForm::new_nus(Nus::new_number(nus.number()));
            }
        }

        let moves = self.to_moves();
        CanonicalForm::new_from_moves(Moves {
            left: moves
                .left
                .iter()
                .map(CanonicalForm::star_projection)
                .collect(),
            right: moves
                .right
                .iter()
                .map(CanonicalForm::star_projection)
                .collect(),
        })
    }

    /// A reduced canonical form of `G` is `\bar{G}`, such that `\bar{G} = \bar{H}`
    /// whenever `G - H` is infinitesimal.
    #[must_use]
    pub fn reduced(&self) -> Self {
        self.heat(&CanonicalForm::new_nus(Nus::new_nimber(Nimber::new(1))))
            .star_projection()
    }

    /// Get temper of the game
    #[must_use]
    pub fn temper(&self) -> Option<Temper> {
        if self.to_nus().is_some_and(Nus::is_number) {
            return Some(Temper::Even);
        }

        let moves = self.to_moves();
        if moves
            .left
            .iter()
            .chain(moves.right.iter())
            .all(|m| CanonicalForm::temper(m).is_some_and(|temper| matches!(temper, Temper::Even)))
        {
            return Some(Temper::Odd);
        }

        if moves
            .left
            .iter()
            .chain(moves.right.iter())
            .all(|m| CanonicalForm::temper(m).is_some_and(|temper| matches!(temper, Temper::Odd)))
        {
            return Some(Temper::Even);
        }

        None
    }

    /// Parse game using `{a,b,...|c,d,...}` notation
    #[allow(clippy::missing_errors_doc)]
    fn parse(p: Parser<'_>) -> Option<(Parser<'_>, CanonicalForm)> {
        match lexeme!(p, Nus::parse) {
            Some((p, nus)) => Some((p, CanonicalForm::new_nus(nus))),
            None => {
                let (p, moves) = try_option!(lexeme!(p, Moves::parse));
                Some((p, CanonicalForm::new_from_moves(moves)))
            }
        }
    }
}

impl PartialOrd for CanonicalForm {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self == other {
            Some(Ordering::Equal)
        } else if Self::leq(self, other) {
            Some(Ordering::Less)
        } else if Self::leq(other, self) {
            Some(Ordering::Greater)
        } else {
            None
        }
    }

    fn le(&self, other: &Self) -> bool {
        Self::leq(self, other)
    }

    fn ge(&self, other: &Self) -> bool {
        Self::leq(other, self)
    }
}

impl_op_ex!(+|g: &CanonicalForm, h: &CanonicalForm| -> CanonicalForm { CanonicalForm::construct_sum(g, h) });
impl_op_ex!(+=|g: &mut CanonicalForm, h: &CanonicalForm| { *g = CanonicalForm::construct_sum(g, h) });
impl_op_ex!(-|g: &CanonicalForm| -> CanonicalForm { CanonicalForm::construct_negative(g) });
impl_op_ex!(-|g: &CanonicalForm, h: &CanonicalForm| -> CanonicalForm {
    CanonicalForm::construct_sum(g, &CanonicalForm::construct_negative(h))
});
impl_op_ex!(-=|g: &mut CanonicalForm, h: &CanonicalForm| {
    *g = CanonicalForm::construct_sum(g, &CanonicalForm::construct_negative(h));
});

impl Display for CanonicalForm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.inner {
            CanonicalFormInner::Nus(nus) => nus.fmt(f),
            CanonicalFormInner::Moves(moves) => moves.fmt(f),
        }
    }
}

impl_from_str_via_parser!(CanonicalForm);

impl Sum for CanonicalForm {
    fn sum<I: Iterator<Item = CanonicalForm>>(iter: I) -> CanonicalForm {
        iter.fold(CanonicalForm::new_integer(0), |acc, v| acc + v)
    }
}

impl<'a> Sum<&'a CanonicalForm> for CanonicalForm {
    fn sum<I: Iterator<Item = &'a CanonicalForm>>(iter: I) -> CanonicalForm {
        iter.fold(CanonicalForm::new_integer(0), |acc, v| acc + v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quickcheck::{Arbitrary, Gen, QuickCheck};
    use std::{ops::Neg, str::FromStr};

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

    macro_rules! parse_nus_succeed {
        ($inp: expr) => {
            let res = Nus::from_str($inp);
            if let Err(err) = res {
                panic!("Parse should succeed, error: {}", err);
            }
        };
    }

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
        parse_nus_succeed!("  123 v   58 *  43784  ");
    }

    // TODO: Rewrite with proptest

    impl Arbitrary for Nus {
        fn arbitrary(g: &mut quickcheck::Gen) -> Self {
            macro_rules! arbitrary_mod {
                ($n: expr, $g: expr) => {{
                    let res: i64 = Arbitrary::arbitrary($g);
                    res.rem_euclid($n).try_into().unwrap()
                }};
            }

            Self {
                number: arbitrary_sign(
                    DyadicRationalNumber::new(arbitrary_mod!(1000, g), arbitrary_mod!(16, g)),
                    g,
                ),
                up_multiple: arbitrary_sign(arbitrary_mod!(1000, g), g),
                nimber: Nimber::new(arbitrary_mod!(1000, g)),
            }
        }
    }

    #[test]
    fn nus_moves_nus_roundtrip() {
        // We really need to stress test it to hit all branches.
        // Confirmed with
        // cargo tarpaulin --out html -- short::partizan::canonical_form::tests --nocapture
        let tests = 250_000;
        let mut qc = QuickCheck::new()
            .max_tests(tests)
            .min_tests_passed(tests)
            .tests(tests);
        qc.quickcheck(nus_moves_nus_roundtrip_impl as fn(Nus));
    }

    fn nus_moves_nus_roundtrip_impl(nus: Nus) {
        let moves = nus.to_moves();
        let nus_from_moves = moves.to_nus().expect("Should be a NUS");
        assert_eq!(nus, nus_from_moves, "Should be equal");
    }

    fn arbitrary_sign<T>(n: T, g: &mut Gen) -> T
    where
        T: Neg<Output = T>,
    {
        if Arbitrary::arbitrary(g) { n } else { -n }
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
        let three_sixteenth = CanonicalForm::new_dyadic(rational);
        assert_eq!(&three_sixteenth.to_string(), "3/16");

        let duplicate = CanonicalForm::new_dyadic(rational);
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

    macro_rules! assert_negative_eq {
        ($inp:expr, $out:expr) => {
            let inp = CanonicalForm::from_str($inp).unwrap();
            assert_eq!((-inp).to_string(), $out);
        };
    }

    #[test]
    fn negative() {
        assert_negative_eq!("0", "0");
        assert_negative_eq!("42", "-42");
        assert_negative_eq!("-42", "42");
        assert_negative_eq!("{^|*}", "{v|*}");
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
        assert_eq!(g.temperature(), DyadicRationalNumber::from(1));
    }

    #[test]
    fn parse_games() {
        macro_rules! test_game_parse {
            ($inp: expr, $expected: expr) => {{
                let g = CanonicalForm::parse(Parser::new($inp))
                    .expect("Could not parse")
                    .1;
                assert_eq!($expected, g.to_string());
            }};
        }

        // test_game_parse!("{|}", "0");
        test_game_parse!("{1,2|}", "3");
        test_game_parse!("{42|*}", "{42|*}");
        test_game_parse!("123", "123");
        test_game_parse!("{1/2|2}", "1");
        test_game_parse!("{3/4|7/8}", "13/16");
        test_game_parse!("{6/8|7/8}", "13/16");
        test_game_parse!("{12/16|14/16}", "13/16");
        test_game_parse!("{0|2}", "1");
        test_game_parse!("{0,*,*2|0,*,*2}", "*3");
    }

    #[test]
    fn ordering_works() {
        macro_rules! test_ordering {
            ($lhs:expr, $rhs:expr, $expected:expr) => {
                assert_eq!(
                    PartialOrd::partial_cmp(
                        &CanonicalForm::from_str($lhs).unwrap(),
                        &CanonicalForm::from_str($rhs).unwrap()
                    ),
                    $expected
                )
            };
        }

        test_ordering!("0", "*", None);
        test_ordering!("*", "*", Some(Ordering::Equal));
        test_ordering!("*2", "*", None);
        test_ordering!("*2", "*2", Some(Ordering::Equal));
        test_ordering!("*", "*2", None);
        test_ordering!("1", "2", Some(Ordering::Less));
        test_ordering!("-1", "*", Some(Ordering::Less));
        test_ordering!("1", "*", Some(Ordering::Greater));
    }

    macro_rules! assert_stops {
        ($cf:expr, $left:expr, $right:expr) => {
            let g = CanonicalForm::from_str($cf).unwrap();
            let (left_stop, right_stop) = g.confusion_interval();
            assert_eq!(
                left_stop,
                DyadicRationalNumber::from_str($left).expect("Could not parse left stop"),
                "Invalid left stop"
            );
            assert_eq!(
                right_stop,
                DyadicRationalNumber::from_str($right).expect("Could not parse right stop"),
                "Invalid right stop"
            );
            assert!(
                left_stop >= right_stop,
                "Left stop shold be geq than right stop"
            );
        };
    }

    #[test]
    fn stops_work() {
        assert_stops!("{{3|2}|0}", "2", "0");
        assert_stops!("v", "0", "0");
        assert_stops!("*", "0", "0");
        assert_stops!("^", "0", "0");
    }

    macro_rules! assert_cooled {
        ($cf:expr, $temp:expr, $expected:expr) => {
            let g = CanonicalForm::from_str($cf).unwrap();
            let temp = DyadicRationalNumber::from_str($temp).unwrap();
            let cooled = g.cool(temp);
            assert_eq!(
                cooled.to_string(),
                CanonicalForm::from_str($expected).unwrap().to_string()
            );
        };
    }

    #[test]
    fn cooling_works() {
        assert_cooled!("{2|-1}", "0", "{2|-1}");
        assert_cooled!("{2|-1}", "1/2", "{3/2|-1/2}");
        assert_cooled!("{2|-1}", "1", "{1|0}");
        assert_cooled!("{2|-1}", "3/2", "1/2*");
        assert_cooled!("{2|-1}", "2", "1/2");
        assert_cooled!("{2|-1}", "3", "1/2");
        assert_cooled!("{2|-1}", "42", "1/2");
    }

    #[test]
    fn heating_numbers() {
        let g = CanonicalForm::new_dyadic(DyadicRationalNumber::from(42));
        let heated = g.heat(&CanonicalForm::new_integer(1));
        assert_eq!(g, heated);
    }

    #[test]
    fn cooling_heating_roundtrip() {
        let g = CanonicalForm::from_str("{2|-1}").unwrap();
        let t = DyadicRationalNumber::from_str("3/2").unwrap();
        let cooled = g.cool(t);
        let frozen = g.cool(t + DyadicRationalNumber::from(1));
        let particle = &cooled - &frozen;
        let heated = particle.heat(&CanonicalForm::new_dyadic(t));
        assert_eq!(heated.to_string(), "{3/2|-3/2}");
        assert_eq!(g, &frozen + &heated);
    }

    macro_rules! assert_atomic_weight_eq {
        ($inp:expr, $atomic:expr) => {
            let cf = CanonicalForm::from_str($inp).unwrap();
            let atomic = CanonicalForm::from_str($atomic).unwrap();
            assert_eq!(cf.atomic_weight().to_string(), atomic.to_string());
        };
    }

    #[test]
    fn atomic_weight() {
        assert_atomic_weight_eq!("*3", "0");
        assert_atomic_weight_eq!("^", "1");
        assert_atomic_weight_eq!("v", "-1");
        assert_atomic_weight_eq!("v2", "-2");
        assert_atomic_weight_eq!("{^2|v}", "1/2");
        assert_atomic_weight_eq!("{^2|v2}", "*");
        assert_atomic_weight_eq!("{^3|v3}", "{1|-1}");
        assert_atomic_weight_eq!("{^2|*}", "1");
        assert_atomic_weight_eq!("{^2,{^|*}|*}", "1");
        assert_atomic_weight_eq!("{*|v2}", "-1");
    }

    #[test]
    fn reduced() {
        let cf = CanonicalForm::from_str("{{2|0}, 1*|*}").unwrap();
        assert_eq!(cf.reduced().to_string(), "{1|0}");

        let cf = CanonicalForm::from_str("{{3/2*|1/2}|{0|-3},{-1*,{-1/2|-1*}|-5/2}}").unwrap();
        assert_eq!(
            cf.reduced().to_string(),
            "{{3/2|1/2}|{0|-3}, {{-1/2|-1}|-5/2}}"
        );
    }

    #[test]
    fn temper() {
        let cf = CanonicalForm::from_str("2").unwrap();
        assert_eq!(cf.temper(), Some(Temper::Even));

        let cf = CanonicalForm::from_str("{2|0}").unwrap();
        assert_eq!(cf.temper(), Some(Temper::Odd));

        let cf = CanonicalForm::from_str("{2|1,{*|0}}").unwrap();
        assert_eq!(cf.temper(), None);
    }

    #[test]
    fn to_moves() {
        let cf = CanonicalForm::from_str("{{3/2*|1/2}|{0|-3},{-1*,{-1/2|-1*}|-5/2}}").unwrap();
        let moves = cf.to_moves();

        let left: &[CanonicalForm] = &cf.to_left_moves();
        assert_eq!(moves.left.as_slice(), left);

        let right: &[CanonicalForm] = &cf.to_right_moves();
        assert_eq!(moves.right.as_slice(), right);
    }
}
