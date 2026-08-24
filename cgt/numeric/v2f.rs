#![allow(dead_code)]

//! Two dimensional vector

use std::ops::{Add, AddAssign, Mul, Neg, Sub, SubAssign};

/// Two dimensional vector
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct V2f {
    /// Horizontal component
    pub x: f32,

    /// Vertical component
    pub y: f32,
}

#[cfg(feature = "mint")]
impl From<V2f> for mint::Vector2<f32> {
    fn from(value: V2f) -> Self {
        Self {
            x: value.x,
            y: value.y,
        }
    }
}

impl From<[f32; 2]> for V2f {
    fn from([x, y]: [f32; 2]) -> Self {
        Self { x, y }
    }
}

impl V2f {
    /// The zero vector
    pub const ZERO: V2f = V2f { x: 0.0, y: 0.0 };

    /// Square of the distance between two `V2f`s.
    ///
    /// Prefer this instead of [`V2f::distance`] if the exact value is not required (e.g. when sorting)
    #[must_use]
    pub fn distance_squared(u: V2f, v: V2f) -> f32 {
        (v.x - u.x).mul_add(v.x - u.x, (v.y - u.y) * (v.y - u.y))
    }

    /// Compute the distance between two vectors
    ///
    /// # Examples
    /// ```
    /// # use cgt::numeric::v2f::V2f;
    /// let u = V2f {x: 3.0, y: 0.0};
    /// let v = V2f {x: 0.0, y: 4.0};
    /// assert_eq!(V2f::distance(u, v), 5.0);
    /// ```
    #[must_use]
    pub fn distance(u: V2f, v: V2f) -> f32 {
        f32::sqrt((v.x - u.x).mul_add(v.x - u.x, (v.y - u.y) * (v.y - u.y)))
    }

    /// Get a normalized vector pointing in the direction from `u` to `v`
    #[must_use]
    pub fn direction(u: V2f, v: V2f) -> V2f {
        (V2f {
            x: v.x - u.x,
            y: v.y - u.y,
        })
        .normalized()
    }

    /// Compute the length of the vector i.e. the distance between itself and the origin
    #[must_use]
    pub fn length(self) -> f32 {
        f32::sqrt(self.x.mul_add(self.x, self.y * self.y))
    }

    /// Normalize the length to 1. Return the zero vector if the input was zero as well.
    ///
    /// # Examples
    /// ```
    /// # use cgt::numeric::v2f::V2f;
    /// let normalized = V2f {x: 4.0, y: 2.0}.normalized();
    /// assert!(normalized.length() > 0.99);
    /// assert!(normalized.length() < 1.01);
    ///
    /// assert_eq!(V2f::ZERO.normalized(), V2f::ZERO);
    /// ```
    #[must_use]
    pub fn normalized(self) -> V2f {
        let l = self.length();
        if l == 0.0 {
            return self;
        }

        V2f {
            x: self.x / l,
            y: self.y / l,
        }
    }

    /// Check if the point is inside the specified rectangle
    #[must_use]
    pub fn inside_rect(self, position: V2f, size: V2f) -> bool {
        self.x >= position.x
            && self.x <= position.x + size.x
            && self.y >= position.y
            && self.y <= position.y + size.y
    }

    /// Check if the point is inside the specified circle
    #[must_use]
    #[allow(clippy::suspicious_operation_groupings)] // False positive
    pub fn inside_circle(self, position: V2f, radius: f32) -> bool {
        (self.x - position.x).mul_add(
            self.x - position.x,
            (self.y - position.y) * (self.y - position.y),
        ) <= radius * radius
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

impl Sub for V2f {
    type Output = V2f;

    fn sub(self, rhs: Self) -> Self::Output {
        V2f {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

impl Sub<f32> for V2f {
    type Output = V2f;

    fn sub(self, rhs: f32) -> Self::Output {
        V2f {
            x: self.x - rhs,
            y: self.y - rhs,
        }
    }
}

impl SubAssign for V2f {
    fn sub_assign(&mut self, rhs: Self) {
        self.x -= rhs.x;
        self.y -= rhs.y;
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

impl Neg for V2f {
    type Output = V2f;

    fn neg(self) -> Self::Output {
        V2f {
            x: -self.x,
            y: -self.y,
        }
    }
}
