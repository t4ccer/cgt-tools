//! Infinite rational number.

use crate::nom_utils::{self, impl_from_str_via_nom};
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
    #[inline]
    pub fn new(numerator: i64, denominator: u32) -> Self {
        Self::Value(Rational64::new(numerator, denominator as i64))
    }

    /// Check if value is infinite
    #[inline]
    pub const fn is_infinite(&self) -> bool {
        !matches!(self, Self::Value(_))
    }

    // TODO: Handle infinities
    fn parse(input: &str) -> nom::IResult<&str, Self> {
        let (input, numerator) = nom_utils::lexeme(nom::character::complete::i64)(input)?;
        match nom_utils::lexeme(nom::bytes::complete::tag::<&str, &str, ()>("/"))(input) {
            Ok((input, _)) => {
                let (input, denominator) = nom_utils::lexeme(nom::character::complete::u32)(input)?;
                if denominator == 0 {
                    return Err(nom::Err::Error(nom::error::Error::new(
                        input,
                        nom::error::ErrorKind::Verify,
                    )));
                }
                Ok((input, Self::new(numerator, denominator)))
            }
            Err(_) => Ok((input, Self::from(numerator))),
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

impl_from_str_via_nom!(Rational);

#[cfg(test)]
fn test_parsing_works(inp: &str) {
    let number = Rational::from_str(inp).unwrap();
    assert_eq!(inp, &format!("{number}"));
}

#[test]
fn parsing_works_positive() {
    test_parsing_works("3/16");
    test_parsing_works("42");
    test_parsing_works("-1/2");
    test_parsing_works("2/3");
}
