//! Number-up-star special case

use crate::{
    numeric::{dyadic_rational_number::DyadicRationalNumber, nimber::Nimber},
    parsing::{Parser, impl_from_str_via_parser, lexeme},
    short::partizan::canonical_form::{CanonicalForm, Hash, Moves},
};
use auto_ops::impl_op_ex;
use std::fmt::Display;

/// A number-up-star game position that is a sum of a number, up and, nimber.
#[derive(Debug, Hash, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Nus {
    pub(crate) number: DyadicRationalNumber,
    pub(crate) up_multiple: i32,
    pub(crate) nimber: Nimber,
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

    pub(crate) fn to_moves(self) -> Moves {
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
    pub(crate) fn to_left_moves(self) -> Vec<CanonicalForm> {
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
    pub(crate) fn to_right_moves(self) -> Vec<CanonicalForm> {
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
