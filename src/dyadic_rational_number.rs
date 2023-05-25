use std::{
    fmt::Display,
    ops::{Add, AddAssign, Neg, Sub},
};

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Hash, Clone, Copy, PartialEq, Eq)]
pub struct DyadicRationalNumber {
    numerator: i64,
    denominator_exponent: u32,
}

impl DyadicRationalNumber {
    pub fn new(numerator: i64, denominator_exponent: u32) -> DyadicRationalNumber {
        DyadicRationalNumber {
            numerator,
            denominator_exponent,
        }
        .normalized()
    }

    pub fn numerator(&self) -> i64 {
        self.numerator
    }

    pub fn denominator(&self) -> u128 {
        // 2^self.denominator_exponent, but as bitshift
        1 << self.denominator_exponent
    }

    pub fn denominator_exponent(&self) -> u32 {
        self.denominator_exponent
    }

    fn normalized(&self) -> Self {
        let mut res = self.clone();
        res.normalize();
        res
    }

    /// Internal function to normalize numbers
    fn normalize(&mut self) {
        // [2*(n)]/[2*d] = n/d
        while self.numerator % 2 == 0 && self.denominator_exponent != 0 {
            self.numerator >>= 1;
            self.denominator_exponent -= 1;
        }
    }

    /// Add to numerator. It is **NOT** addition function
    pub fn step(&self, n: i64) -> Self {
        DyadicRationalNumber {
            // numerator: self.numerator + (n << self.denominator_exponent),
            numerator: self.numerator + n,
            denominator_exponent: self.denominator_exponent,
        }
        .normalized()
    }

    /// Convert to intger if it's an integer
    pub fn to_integer(&self) -> Option<i64> {
        // exponent == 0 => denominator == 1 => It's an integer
        if self.denominator_exponent == 0 {
            Some(self.numerator)
        } else {
            None
        }
    }

    /// Arithmetic mean of two rationals
    pub fn mean(&self, rhs: &Self) -> Self {
        let mut res = *self + *rhs;
        res.denominator_exponent += 1; // divide by 2
        res.normalize();
        res
    }
}

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

impl Add for &DyadicRationalNumber {
    type Output = DyadicRationalNumber;

    fn add(self, rhs: &DyadicRationalNumber) -> DyadicRationalNumber {
        let denominator_exponent;
        let numerator;
        if self.denominator_exponent >= rhs.denominator_exponent {
            denominator_exponent = self.denominator_exponent;
            numerator = self.numerator
                + (rhs.numerator << (self.denominator_exponent - rhs.denominator_exponent))
        } else {
            denominator_exponent = rhs.denominator_exponent;
            numerator = rhs.numerator
                + (self.numerator << (rhs.denominator_exponent - self.denominator_exponent))
        }
        let mut res = DyadicRationalNumber {
            numerator,
            denominator_exponent,
        };
        res.normalize();
        res
    }
}

impl Add for DyadicRationalNumber {
    type Output = DyadicRationalNumber;

    fn add(self, rhs: DyadicRationalNumber) -> DyadicRationalNumber {
        &self + &rhs
    }
}

impl Sub for DyadicRationalNumber {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        self + (-rhs)
    }
}

impl AddAssign for DyadicRationalNumber {
    fn add_assign(&mut self, rhs: DyadicRationalNumber) {
        let lhs: &DyadicRationalNumber = self;
        let new: DyadicRationalNumber = lhs + &rhs;
        self.numerator = new.numerator;
        self.denominator_exponent = new.denominator_exponent;
    }
}

impl Neg for DyadicRationalNumber {
    type Output = Self;

    fn neg(self) -> Self::Output {
        DyadicRationalNumber {
            numerator: -self.numerator,
            denominator_exponent: self.denominator_exponent,
        }
    }
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
        .denominator(),
        8
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
