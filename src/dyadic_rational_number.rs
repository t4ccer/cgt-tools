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

    pub fn rational(numerator: i32, denominator: i32) -> Option<Self> {
        if denominator == 0 {
            return None;
        }
        // FIXME: Check if fraction is dyadic
        Some(DyadicRationalNumber {
            numerator,
            denominator,
        })
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
