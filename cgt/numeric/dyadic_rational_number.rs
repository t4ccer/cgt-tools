//! Number in form `n/2^m`

use crate::{
    numeric::rational::Rational,
    parsing::{Parser, impl_from_str_via_parser, lexeme, try_option},
};
use auto_ops::impl_op_ex;
use std::{
    fmt::Display,
    ops::{Add, Sub},
};

/// Number in form `n/2^m`
#[derive(Debug, Hash, Clone, Copy, PartialEq, Eq)]
pub struct DyadicRationalNumber {
    numerator: i64,
    denominator_exponent: u32,
}

impl DyadicRationalNumber {
    /// Create a new dyadic
    pub const fn new(numerator: i64, denominator_exponent: u32) -> Self {
        Self {
            numerator,
            denominator_exponent,
        }
        .normalized()
    }

    /// Create a new integer
    pub const fn new_integer(number: i64) -> Self {
        Self {
            numerator: number,
            denominator_exponent: 0,
        }
    }

    /// Create a new fraction. Returns [None] if denominator is zero, or the number is not dyadic
    #[must_use]
    pub const fn new_fraction(numerator: i64, mut denominator: u32) -> Option<Self> {
        let mut denominator_exponent = 0;

        if denominator == 0 {
            return None;
        }

        while denominator % 2 == 0 {
            denominator /= 2;
            denominator_exponent += 1;
        }

        if denominator == 1 {
            Some(
                Self {
                    numerator,
                    denominator_exponent,
                }
                .normalized(),
            )
        } else {
            None
        }
    }

    /// Get the numerator (`n` from `n/2^m`)
    #[must_use]
    pub const fn numerator(&self) -> i64 {
        self.numerator
    }

    /// Get the denominator (`2^m` from `n/2^m`) if it fits in [u128]
    #[must_use]
    pub const fn denominator(&self) -> Option<u128> {
        if self.denominator_exponent as usize >= std::mem::size_of::<u128>() * 8 {
            None
        } else {
            // 2^self.denominator_exponent, but as bitshift
            Some(1 << self.denominator_exponent)
        }
    }

    /// Get denominator exponent (`m` from `n/2^m`)
    pub const fn denominator_exponent(&self) -> u32 {
        self.denominator_exponent
    }

    #[must_use]
    const fn normalized(mut self) -> Self {
        while self.numerator % 2 == 0 && self.denominator_exponent != 0 {
            self.numerator >>= 1_i32;
            self.denominator_exponent -= 1;
        }
        self
    }

    /// Add to numerator. It is **NOT** addition function
    #[must_use]
    pub const fn step(&self, n: i64) -> Self {
        Self {
            // numerator: self.numerator + (n << self.denominator_exponent),
            numerator: self.numerator + n,
            denominator_exponent: self.denominator_exponent,
        }
        .normalized()
    }

    /// Convert to intger if it's an integer
    #[must_use]
    pub const fn to_integer(&self) -> Option<i64> {
        // exponent == 0 => denominator == 1 => It's an integer
        if self.denominator_exponent == 0 {
            Some(self.numerator)
        } else {
            None
        }
    }

    /// Check if number equals to given integer
    ///
    /// This is a useful util in `const` context
    #[must_use]
    pub const fn eq_integer(&self, rhs: i64) -> bool {
        // exponent == 0 => denominator == 1 => It's an integer
        if self.denominator_exponent == 0 {
            self.numerator == rhs
        } else {
            false
        }
    }

    /// Ceil division
    pub const fn ceil(self) -> i64 {
        // TODO: use `div_ceil` when `int_roundings` lands in stable
        let n = self.numerator();
        let d = self
            .denominator()
            .expect("unreachable: denominator cannot be zero") as i64;
        (n + d - 1) / d
    }

    /// Round a dyadic to the nearest integer
    pub const fn round(self) -> i64 {
        self.numerator()
            / self
                .denominator()
                .expect("unreachable: denominator cannot be zero") as i64
    }

    /// Arithmetic mean of two rationals
    #[must_use]
    pub fn mean(&self, rhs: &Self) -> Self {
        let mut res = *self + *rhs;
        res.denominator_exponent += 1; // divide by 2
        res.normalized()
    }

    /// Parse dyadic number
    pub(crate) const fn parse(p: Parser<'_>) -> Option<(Parser<'_>, DyadicRationalNumber)> {
        let (p, numerator) = try_option!(lexeme!(p, Parser::parse_i64));
        match p.parse_any_ascii_char() {
            Some((p, '/')) => {
                let p = p.trim_whitespace();
                let (p, denominator) = try_option!(lexeme!(p, Parser::parse_u32));
                let dyadic =
                    try_option!(DyadicRationalNumber::new_fraction(numerator, denominator));
                Some((p, dyadic))
            }
            _ => Some((p, DyadicRationalNumber::new_integer(numerator))),
        }
    }

    /// Convert rational to dyadic
    ///
    /// # Errors
    /// - Rational is infinite
    /// - Rational is not dyadic
    pub fn from_rational(rational: Rational) -> Option<Self> {
        let (numerator, denominator) = rational.to_fraction()?;
        Self::new_fraction(numerator, denominator)
    }

    /// Convert dyadic to rational
    ///
    /// # Panics
    /// - If denominator is too large to fit in [`Rational`]
    pub const fn to_rational(self) -> Rational {
        Rational::new_fraction(self.numerator(), self.denominator().unwrap() as u32).unwrap()
    }
}

impl_from_str_via_parser!(DyadicRationalNumber);

#[test]
fn step_works() {
    assert_eq!(
        DyadicRationalNumber {
            numerator: 1,
            denominator_exponent: 1,
        }
        .normalized()
        .step(1),
        DyadicRationalNumber {
            numerator: 1,
            denominator_exponent: 0,
        }
        .normalized()
    );
}

impl From<i64> for DyadicRationalNumber {
    fn from(value: i64) -> Self {
        Self::new(value, 0)
    }
}

impl PartialOrd for DyadicRationalNumber {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for DyadicRationalNumber {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        if self.denominator_exponent <= other.denominator_exponent {
            i64::cmp(
                &(self.numerator << (other.denominator_exponent - self.denominator_exponent)),
                &other.numerator,
            )
        } else {
            i64::cmp(
                &self.numerator,
                &(other.numerator << (self.denominator_exponent - other.denominator_exponent)),
            )
        }
    }
}

#[test]
fn half_is_less_than_forty_two() {
    let half = DyadicRationalNumber::new(1, 1);
    let forty_two = DyadicRationalNumber::new(42, 0);
    assert!(half <= forty_two);
    assert!(half < forty_two);
    assert!(half != forty_two);
    assert!(forty_two >= half);
    assert!(forty_two > half);
    assert!(forty_two != half);
}

impl_op_ex!(+|lhs: &DyadicRationalNumber, rhs: &DyadicRationalNumber| -> DyadicRationalNumber {
    let (numerator, denominator_exponent) =
    if lhs.denominator_exponent >= rhs.denominator_exponent {
            let denominator_exponent = lhs.denominator_exponent;
            let numerator = lhs.numerator
        + (rhs.numerator << (lhs.denominator_exponent - rhs.denominator_exponent));
        (numerator, denominator_exponent)
    } else {
            let denominator_exponent = rhs.denominator_exponent;
            let numerator = rhs.numerator
        + (lhs.numerator << (rhs.denominator_exponent - lhs.denominator_exponent));
            (numerator, denominator_exponent)
    };
    DyadicRationalNumber {
        numerator,
        denominator_exponent,
    }
    .normalized()
});

impl_op_ex!(+=|lhs: &mut DyadicRationalNumber, rhs: &DyadicRationalNumber| { *lhs = lhs.add(rhs); });

impl_op_ex!(
    -|lhs: &DyadicRationalNumber, rhs: &DyadicRationalNumber| -> DyadicRationalNumber {
        lhs + (-rhs)
    }
);

impl_op_ex!(-=|lhs: &mut DyadicRationalNumber, rhs: &DyadicRationalNumber| { *lhs = lhs.sub(rhs); });

impl_op_ex!(-|lhs: &DyadicRationalNumber| -> DyadicRationalNumber {
    DyadicRationalNumber {
        numerator: -lhs.numerator,
        denominator_exponent: lhs.denominator_exponent,
    }
});

impl Display for DyadicRationalNumber {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(int) = self.to_integer() {
            write!(f, "{}", int)?;
        } else if let Some(denum) = self.denominator() {
            write!(f, "{}/{}", self.numerator(), denum)?;
        } else {
            write!(f, "{}/2^{}", self.numerator(), self.denominator_exponent())?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn one_plus_half() {
        let one = DyadicRationalNumber::new(1, 0);
        let half = DyadicRationalNumber::new(1, 1);
        assert_eq!(one + half, DyadicRationalNumber::new(3, 1));
        assert_eq!(half + one, DyadicRationalNumber::new(3, 1));
    }

    #[test]
    fn denominator_works() {
        assert_eq!(
            DyadicRationalNumber {
                numerator: 0,
                denominator_exponent: 0
            }
            .denominator_exponent(),
            0
        );
        assert_eq!(
            DyadicRationalNumber {
                numerator: 3,
                denominator_exponent: 3
            }
            .denominator()
            .unwrap(),
            8
        );
    }

    #[test]
    fn dyadic_rationals_pretty() {
        assert_eq!(format!("{}", DyadicRationalNumber::new(3, 8)), "3/256");
        assert_eq!(
            format!("{}", DyadicRationalNumber::new(21, 200)),
            "21/2^200"
        );
    }

    #[cfg(test)]
    fn test_parsing_works(inp: &str) {
        let number = DyadicRationalNumber::from_str(inp).unwrap();
        assert_eq!(inp, &format!("{number}"));

        let (_, number) = DyadicRationalNumber::parse(crate::parsing::Parser::new(inp)).unwrap();
        assert_eq!(inp, &format!("{number}"));
    }

    #[test]
    fn parsing_works_positive() {
        test_parsing_works("3/16");
        test_parsing_works("42");
        test_parsing_works("-1/2");
    }

    #[test]
    #[should_panic]
    fn parsing_works_negative() {
        test_parsing_works("2/3");
    }
}
