//! Infinite rational number.

use crate::parsing::{impl_from_str_via_parser, lexeme, try_option, Parser};
use auto_ops::impl_op_ex;
use num_rational::Rational64;
use std::{
    fmt::Display,
    ops::{Add, Div, Mul, Sub},
};

#[cfg(test)]
use std::str::FromStr;

/// Infinite rational number.
#[derive(Debug, Hash, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Rational {
    /// Negative infnity, smaller than all other values
    NegativeInfinity,

    /// A finite number
    Value(Rational64),

    /// Positive infnity, greater than all other values
    PositiveInfinity,
}

impl Rational {
    /// Create a new rational. Panics if denominator is zero.
    // TODO: Make it return option
    #[inline]
    pub const fn new(numerator: i64, denominator: u32) -> Self {
        assert!(denominator != 0);
        let g = gcd(numerator, denominator as i64).abs();
        Self::Value(Rational64::new_raw(numerator / g, denominator as i64 / g))
    }

    /// Check if value is infinite
    #[inline]
    pub const fn is_infinite(&self) -> bool {
        !matches!(self, Self::Value(_))
    }

    // TODO: Handle infinities
    const fn parse(p: Parser<'_>) -> Option<(Parser<'_>, Rational)> {
        let (p, numerator) = try_option!(lexeme!(p, Parser::parse_i64));
        match p.parse_ascii_char('/') {
            Some(p) => {
                let (p, denominator) = try_option!(lexeme!(p, Parser::parse_u32));
                let rational = Rational::new(numerator, denominator);
                Some((p, rational))
            }
            _ => Some((p, Rational::new(numerator, 1))),
        }
    }

    /// Rounding towards zero
    ///
    /// # Errors
    /// - Rational is infinite
    pub fn try_round(&self) -> Option<i64> {
        match self {
            Self::Value(val) => Some(val.to_integer()),
            Self::PositiveInfinity | Self::NegativeInfinity => None,
        }
    }

    /// Get fraction if rational is finite
    ///
    /// # Errors
    /// - Rational is infinite
    pub const fn to_fraction(self) -> Option<(i64, u32)> {
        if let Self::Value(r) = self {
            Some((*r.numer(), *r.denom() as u32))
        } else {
            None
        }
    }

    /// Get floating point approximation if rational is finite
    pub fn as_f32(self) -> Option<f32> {
        let (n, d) = self.to_fraction()?;
        Some(n as f32 / d as f32)
    }
}

impl From<Rational64> for Rational {
    fn from(value: Rational64) -> Self {
        Self::Value(value)
    }
}

impl From<i64> for Rational {
    fn from(value: i64) -> Self {
        Self::from(Rational64::from(value))
    }
}

impl From<i32> for Rational {
    fn from(value: i32) -> Self {
        Self::from(value as i64)
    }
}

impl_op_ex!(+|lhs: &Rational, rhs: &Rational| -> Rational {
    match (lhs, rhs) {
        (Rational::Value(lhs), Rational::Value(rhs)) => Rational::from(lhs + rhs),
        (Rational::Value(_), Rational::PositiveInfinity) |
        (Rational::PositiveInfinity, Rational::Value(_)) => Rational::PositiveInfinity,
        (Rational::Value(_), Rational::NegativeInfinity) |
        (Rational::NegativeInfinity, Rational::Value(_)) => Rational::NegativeInfinity,
        _ => {
            panic!()
        }
    }
});

impl_op_ex!(+=|lhs: &mut Rational, rhs: &Rational| {*lhs = lhs.add(rhs) });

impl_op_ex!(-|lhs: &Rational, rhs: &Rational| -> Rational {
    if let (Rational::Value(lhs), Rational::Value(rhs)) = (lhs, rhs) {
        Rational::from(lhs - rhs)
    } else {
        panic!()
    }
});

impl_op_ex!(-=|lhs: &mut Rational, rhs: &Rational| {*lhs = lhs.sub(rhs) });

impl_op_ex!(*|lhs: &Rational, rhs: &Rational| -> Rational {
    match (lhs, rhs) {
        (Rational::Value(lhs), Rational::Value(rhs)) => Rational::from(lhs * rhs),
        (Rational::Value(lhs), Rational::PositiveInfinity) if lhs > &0.into() => {
            Rational::PositiveInfinity
        }
        (Rational::Value(lhs), Rational::PositiveInfinity) if lhs < &0.into() => {
            Rational::NegativeInfinity
        }
        (Rational::Value(lhs), Rational::NegativeInfinity) if lhs > &0.into() => {
            Rational::NegativeInfinity
        }
        (Rational::Value(lhs), Rational::NegativeInfinity) if lhs < &0.into() => {
            Rational::PositiveInfinity
        }
        (Rational::Value(_), _) => {
            panic!()
        }
        (rhs, lhs) => Mul::mul(lhs, rhs), // NOTE: Be careful here not to loop
    }
});

impl_op_ex!(*=|lhs: &mut Rational, rhs: &Rational| {*lhs = lhs.mul(rhs) });

impl_op_ex!(/|lhs: &Rational, rhs: &Rational| -> Rational {
    if let (Rational::Value(lhs), Rational::Value(rhs)) = (lhs, rhs) {
        Rational::from(lhs / rhs)
    } else {
        panic!()
    }
});
impl_op_ex!(/=|lhs: &mut Rational, rhs: &Rational| {*lhs = lhs.div(rhs) });

impl_op_ex!(-|lhs: &Rational| -> Rational {
    match lhs {
        Rational::NegativeInfinity => Rational::PositiveInfinity,
        Rational::Value(val) => Rational::Value(-val),
        Rational::PositiveInfinity => Rational::NegativeInfinity,
    }
});

impl Display for Rational {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NegativeInfinity => write!(f, "-∞"),
            Self::Value(val) => write!(f, "{}", val),
            Self::PositiveInfinity => write!(f, "∞"),
        }
    }
}

impl_from_str_via_parser!(Rational);

#[cfg(test)]
fn test_parsing_works(inp: &str) {
    let number = Rational::from_str(inp).unwrap();
    assert_eq!(inp, &format!("{number}"));
}

#[test]
fn parsing_works_positive() {
    // test_parsing_works("3/16");
    // test_parsing_works("42");
    test_parsing_works("-1/2");
    // test_parsing_works("2/3");
}

const fn gcd(a: i64, b: i64) -> i64 {
    let (mut a, mut b) = if a > b { (a, b) } else { (b, a) };

    while b != 0 {
        let temp = a;
        a = b;
        b = temp;

        b %= a;
    }

    a
}
