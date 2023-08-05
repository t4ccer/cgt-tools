//! Infinite rational number.

use std::{
    fmt::Display,
    ops::{Add, AddAssign, Div, Mul, Neg, Sub, SubAssign},
    str::FromStr,
};

use num_rational::Rational64;

use crate::nom_utils;

#[derive(Debug, Hash, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Rational {
    NegativeInfinity,
    Value(Rational64),
    PositiveInfinity,
}

impl Rational {
    #[inline]
    pub fn new(numerator: i64, denominator: u32) -> Self {
        Rational::Value(Rational64::new(numerator, denominator as i64))
    }

    #[inline]
    pub fn is_infinite(&self) -> bool {
        !matches!(self, Rational::Value(_))
    }
}

impl From<Rational64> for Rational {
    fn from(value: Rational64) -> Self {
        Rational::Value(value)
    }
}

impl From<i64> for Rational {
    fn from(value: i64) -> Self {
        Rational::from(Rational64::from(value))
    }
}

impl From<i32> for Rational {
    fn from(value: i32) -> Self {
        Rational::from(value as i64)
    }
}

impl Add for Rational {
    type Output = Rational;

    fn add(self, rhs: Self) -> Self::Output {
        Add::add(&self, &rhs)
    }
}

impl Add for &Rational {
    type Output = Rational;

    fn add(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Rational::Value(lhs), Rational::Value(rhs)) => Rational::from(lhs + rhs),
            (Rational::Value(_), Rational::PositiveInfinity) => Rational::PositiveInfinity,
            (Rational::PositiveInfinity, Rational::Value(_)) => Rational::PositiveInfinity,
            (Rational::Value(_), Rational::NegativeInfinity) => Rational::NegativeInfinity,
            (Rational::NegativeInfinity, Rational::Value(_)) => Rational::NegativeInfinity,
            _ => {
                dbg!(self, rhs);
                unimplemented!()
            }
        }
    }
}

impl AddAssign for Rational {
    fn add_assign(&mut self, rhs: Self) {
        *self = self.add(rhs);
    }
}

impl Sub for Rational {
    type Output = Rational;

    fn sub(self, rhs: Self) -> Self::Output {
        Sub::sub(&self, &rhs)
    }
}

impl Sub for &Rational {
    type Output = Rational;

    fn sub(self, rhs: Self) -> Self::Output {
        if let (Rational::Value(lhs), Rational::Value(rhs)) = (self, rhs) {
            Rational::from(lhs - rhs)
        } else {
            unimplemented!()
        }
    }
}

impl SubAssign for Rational {
    fn sub_assign(&mut self, rhs: Self) {
        if let (Rational::Value(lhs), Rational::Value(rhs)) = (self, rhs) {
            *lhs -= rhs;
        } else {
            unimplemented!();
        }
    }
}

impl Mul for Rational {
    type Output = Rational;

    fn mul(self, rhs: Self) -> Self::Output {
        Mul::mul(&self, &rhs)
    }
}

impl Mul for &Rational {
    type Output = Rational;

    fn mul(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
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
                dbg!(&self, &rhs);
                unimplemented!()
            }
            (rhs, lhs) => Mul::mul(lhs, rhs), // NOTE: Be careful here not to loop
        }
    }
}

impl Div for Rational {
    type Output = Rational;

    fn div(self, rhs: Self) -> Self::Output {
        Div::div(&self, &rhs)
    }
}

impl Div for &Rational {
    type Output = Rational;

    fn div(self, rhs: Self) -> Self::Output {
        if let (Rational::Value(lhs), Rational::Value(rhs)) = (self, rhs) {
            Rational::from(lhs / rhs)
        } else {
            unimplemented!()
        }
    }
}

impl Neg for Rational {
    type Output = Rational;

    fn neg(self) -> Self::Output {
        match self {
            Rational::NegativeInfinity => Rational::PositiveInfinity,
            Rational::Value(val) => Rational::Value(-val),
            Rational::PositiveInfinity => Rational::NegativeInfinity,
        }
    }
}

impl Display for Rational {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Rational::NegativeInfinity => write!(f, "-∞"),
            Rational::Value(val) => write!(f, "{}", val),
            Rational::PositiveInfinity => write!(f, "∞"),
        }
    }
}

impl Rational {
    // NOTE: Doesn't handle infinities
    fn parser(input: &str) -> nom::IResult<&str, Rational> {
        let (input, numerator) = nom_utils::lexeme(nom::character::complete::i64)(input)?;
        match nom_utils::lexeme(nom::bytes::complete::tag::<&str, &str, ()>("/"))(input) {
            Ok((input, _)) => {
                let (input, denominator) = nom_utils::lexeme(nom::character::complete::u32)(input)?;
                let (input, _eof) = nom_utils::lexeme(nom::combinator::eof)(input)?;
                // NOTE: zero denominator not handled
                Ok((input, Rational::new(numerator, denominator)))
            }
            Err(_) => Ok((input, Rational::from(numerator))),
        }
    }
}

impl FromStr for Rational {
    type Err = &'static str;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        Rational::parser(input)
            .map_err(|_| "Invalid rational")
            .map(|(_, d)| d)
    }
}

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
