use std::{
    fmt::Display,
    ops::{Add, AddAssign, Neg, Sub, SubAssign},
};

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Hash, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Nimber(u32);

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
        write!(f, "{}", self.0)
    }
}
