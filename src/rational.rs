use std::{
    fmt::Display,
    ops::{Add, Div, Mul, Sub, SubAssign},
};

use num_rational::Rational64;

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Hash, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Rational {
    NegativeInfinity,
    Value(Rational64),
    PositiveInfinity,
}

impl Rational {
    pub fn new(numerator: i64, denominator: u32) -> Self {
        Rational::Value(Rational64::new(numerator, denominator as i64))
    }

    pub fn is_infinite(&self) -> bool {
        match self {
            Rational::NegativeInfinity => true,
            Rational::Value(_) => false,
            Rational::PositiveInfinity => true,
        }
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

impl Display for Rational {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Rational::NegativeInfinity => write!(f, "-∞"),
            Rational::Value(val) => write!(f, "{}", val),
            Rational::PositiveInfinity => write!(f, "∞"),
        }
    }
}
