use std::{
    fmt::Display,
    ops::{Add, AddAssign, Neg, Sub, SubAssign},
};

#[derive(Debug, Hash, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Nimber(pub u32);

impl Nimber {
    pub fn get(&self) -> u32 {
        self.0
    }
}

impl From<u32> for Nimber {
    fn from(value: u32) -> Self {
        Nimber(value)
    }
}

impl Add for Nimber {
    type Output = Nimber;

    fn add(self, rhs: Nimber) -> Nimber {
        Nimber(self.0 ^ rhs.0)
    }
}

impl AddAssign for Nimber {
    fn add_assign(&mut self, rhs: Self) {
        self.0 ^= rhs.0
    }
}

impl Neg for Nimber {
    type Output = Nimber;

    fn neg(self) -> Nimber {
        self
    }
}

impl Sub for Nimber {
    type Output = Nimber;

    fn sub(self, rhs: Nimber) -> Nimber {
        Nimber::add(self, rhs)
    }
}

impl SubAssign for Nimber {
    fn sub_assign(&mut self, rhs: Self) {
        Nimber::add_assign(self, rhs)
    }
}

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
    pub fn mex(mut nimbers: Vec<Self>) -> Self {
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
}
