#![allow(missing_docs, dead_code)]

use std::ops::{Add, AddAssign, Mul};

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct V2f {
    pub x: f32,
    pub y: f32,
}

impl From<V2f> for mint::Vector2<f32> {
    fn from(value: V2f) -> Self {
        Self {
            x: value.x,
            y: value.y,
        }
    }
}

impl V2f {
    pub fn distance_squared(u: V2f, v: V2f) -> f32 {
        (v.x - u.x) * (v.x - u.x) + (v.y - u.y) * (v.y - u.y)
    }

    pub fn distance(u: V2f, v: V2f) -> f32 {
        f32::sqrt((v.x - u.x) * (v.x - u.x) + (v.y - u.y) * (v.y - u.y))
    }

    pub fn direction(u: V2f, v: V2f) -> V2f {
        (V2f {
            x: v.x - u.x,
            y: v.y - u.y,
        })
        .normalized()
    }

    pub fn length(self) -> f32 {
        f32::sqrt(self.x * self.x + self.y * self.y)
    }

    pub fn normalized(self) -> V2f {
        let l = self.length();
        V2f {
            x: self.x / l,
            y: self.y / l,
        }
    }
}

impl Add for V2f {
    type Output = V2f;

    fn add(self, rhs: Self) -> Self::Output {
        V2f {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

impl AddAssign for V2f {
    fn add_assign(&mut self, rhs: Self) {
        self.x += rhs.x;
        self.y += rhs.y;
    }
}

impl Mul<f32> for V2f {
    type Output = V2f;

    fn mul(self, rhs: f32) -> Self::Output {
        V2f {
            x: self.x * rhs,
            y: self.y * rhs,
        }
    }
}

impl Mul<V2f> for f32 {
    type Output = V2f;

    fn mul(self, rhs: V2f) -> Self::Output {
        V2f {
            x: self * rhs.x,
            y: self * rhs.y,
        }
    }
}