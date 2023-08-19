//! Nimber is a number that represents a Nim heap of a given size.

use auto_ops::impl_op_ex;
use std::fmt::Display;

/// Number that represents a Nim heap of given size.
///
/// Addition is overloaded to Nim sum.
#[repr(transparent)]
#[derive(Debug, Hash, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Nimber(u32);

impl Nimber {
    /// Construct new nimber
    pub fn new(value: u32) -> Nimber {
        Nimber(value)
    }

    /// Get the underlying nimber value
    pub fn value(&self) -> u32 {
        self.0
    }
}

impl From<u32> for Nimber {
    fn from(value: u32) -> Self {
        Nimber(value)
    }
}

// xor is correct, that's how nimbers additon works
impl_op_ex!(+|lhs: &Nimber, rhs: &Nimber| -> Nimber { Nimber(lhs.0 ^ rhs.0) });
impl_op_ex!(+=|lhs: &mut Nimber, rhs: &Nimber| { lhs.0 ^= rhs.0 });

// Subtraction is the same as addition
impl_op_ex!(-|lhs: &Nimber, rhs: &Nimber| -> Nimber { Nimber(lhs.0 ^ rhs.0) });
impl_op_ex!(-=|lhs: &mut Nimber, rhs: &Nimber| { lhs.0 ^= rhs.0 });

// Nimber is its own negative
impl_op_ex!(-|lhs: &Nimber| -> Nimber { *lhs });

impl Display for Nimber {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.0 == 0 {
            write!(f, "0")
        } else if self.0 == 1 {
            write!(f, "*")
        } else {
            write!(f, "*{}", self.0)
        }
    }
}

impl Nimber {
    /// Compute the minimum excluded value from a vector of nimbers.
    /// See <https://en.wikipedia.org/wiki/Mex_(mathematics)>
    pub fn mex(mut nimbers: Vec<Nimber>) -> Nimber {
        nimbers.sort();
        let mut current = 0;
        for n in nimbers {
            if current < n.0 {
                return Nimber(current);
            } else if current == n.0 {
                current += 1;
            }
        }
        return Nimber(current);
    }
}

#[test]
fn mex_works() {
    assert_eq!(
        Nimber(3),
        Nimber::mex(vec![Nimber(0), Nimber(0), Nimber(2), Nimber(5), Nimber(1)])
    );

    assert_eq!(
        Nimber(3),
        Nimber::mex(vec![Nimber(0), Nimber(1), Nimber(2)])
    );

    assert_eq!(
        Nimber(2),
        Nimber::mex(vec![Nimber(0), Nimber(1), Nimber(1)])
    );

    assert_eq!(Nimber(0), Nimber::mex(vec![]));
}
