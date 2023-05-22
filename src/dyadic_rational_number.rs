use std::{
    fmt::Display,
    ops::{Add, AddAssign, DivAssign, Neg, Sub},
};

use gcd::Gcd;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DyadicRationalNumber {
    numerator: i32,
    denominator: i32,
}

impl DyadicRationalNumber {
    pub fn numerator(&self) -> i32 {
        self.numerator
    }

    pub fn denominator(&self) -> i32 {
        self.denominator
    }

    pub fn denominator_exponent(&self) -> i32 {
        self.denominator().trailing_zeros() as i32
    }

    fn normalized(&self) -> Self {
        let d = Gcd::gcd(
            self.numerator().abs() as u32,
            self.denominator().abs() as u32,
        );

        DyadicRationalNumber {
            numerator: self.numerator / (d as i32),
            denominator: self.denominator / (d as i32),
        }
    }

    fn normalize(&mut self) {
        let d = Gcd::gcd(
            self.numerator().abs() as u32,
            self.denominator().abs() as u32,
        );

        self.numerator.div_assign(d as i32);
        self.denominator.div_assign(d as i32);
    }

    pub fn rational(numerator: i32, denominator: i32) -> Option<Self> {
        if denominator == 0 {
            return None;
        }
        if numerator == 0 {
            return Some(DyadicRationalNumber {
                numerator: 0,
                denominator: 1,
            });
        }

        let sign = numerator.signum() * denominator.signum();

        // FIXME: Check if fraction is dyadic
        Some(
            DyadicRationalNumber {
                numerator: numerator.abs() * sign,
                denominator: denominator.abs(),
            }
            .normalized(),
        )
    }

    pub fn step(&self, n: i32) -> Self {
        DyadicRationalNumber {
            numerator: self.numerator + n,
            denominator: self.denominator,
        }
        .normalized()
    }

    /// Convert to intger if it's an integer
    pub fn to_integer(&self) -> Option<i32> {
        if self.denominator == 1 {
            Some(self.numerator)
        } else {
            None
        }
    }

    pub fn mean(&self, rhs: &Self) -> Self {
        let mut res = *self + *rhs;
        res.denominator *= 2;
        res.normalized()
    }
}

impl From<i32> for DyadicRationalNumber {
    fn from(value: i32) -> Self {
        Self {
            numerator: value,
            denominator: 1,
        }
    }
}

impl Add for DyadicRationalNumber {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        DyadicRationalNumber {
            numerator: self.numerator() * rhs.denominator + self.denominator * rhs.numerator,
            denominator: self.denominator() * rhs.denominator(),
        }
        .normalized()
    }
}

impl Sub for DyadicRationalNumber {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        self + (-rhs)
    }
}

impl AddAssign for DyadicRationalNumber {
    fn add_assign(&mut self, rhs: Self) {
        self.numerator = self.numerator() * rhs.denominator + self.denominator * rhs.numerator;
        self.denominator = self.denominator() * rhs.denominator();
        self.normalize();
    }
}

impl Neg for DyadicRationalNumber {
    type Output = Self;

    fn neg(self) -> Self::Output {
        DyadicRationalNumber {
            numerator: -self.numerator,
            denominator: self.denominator,
        }
    }
}

#[test]
fn denominator_exponent_works() {
    assert_eq!(
        DyadicRationalNumber::rational(52, 1)
            .unwrap()
            .denominator_exponent(),
        0
    );
    assert_eq!(
        DyadicRationalNumber::rational(1, 8)
            .unwrap()
            .denominator_exponent(),
        3
    );
}

impl Display for DyadicRationalNumber {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(int) = self.to_integer() {
            write!(f, "{}", int)?;
        } else {
            write!(f, "{}/{}", self.numerator(), self.denominator())?;
        }
        return Ok(());
    }
}
