//! A continuous piecewise linear trajectory with rational slopes. Usually used as
//! [thermograph](crate::short::partizan::thermograph) scaffolds.

use crate::{display, numeric::rational::Rational};
use itertools::Itertools;
use std::{cmp::Ordering, fmt::Display};

/// A continuous piecewise linear trajectory with rational slopes and critical points.
/// Each trajectory is defined for all rational numbers on the interval `-1 ≤ x < ∞`.
#[derive(Debug, Hash, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Trajectory {
    /// A `critical point` is a point at which the trajectory changes slope, and must be strictly
    /// between `-1` and `∞`.
    pub(crate) critical_points: Vec<Rational>,
    pub(crate) slopes: Vec<Rational>,
    pub(crate) x_intercepts: Vec<Rational>,
}

impl Trajectory {
    /// Constructs a new `Trajectory` with constant value `r`
    pub fn new_constant(r: Rational) -> Self {
        Self {
            critical_points: vec![],
            slopes: vec![Rational::from(0)],
            x_intercepts: vec![r],
        }
    }

    /// Tilts this trajectory by `r`.
    /// If this trajectory has value `a(x)` at `x`, then the tilted trajectory has value `a(x) + rx`
    pub fn tilt(&mut self, r: Rational) {
        if self.is_infinite() {
            return;
        }

        for slope in &mut self.slopes {
            *slope += r;
        }
    }

    /// Create a new trajectory with given slopes and critical points. Returns [None] if input
    /// violates the invariants.
    pub fn new(
        mast: Rational,
        critical_points: Vec<Rational>,
        slopes: Vec<Rational>,
    ) -> Option<Self> {
        // Input validation
        if slopes.len() != critical_points.len() + 1 {
            // Slopes must have length one greater than criticalPoints
            return None;
        }

        if critical_points
            .iter()
            .tuples()
            .any(|(prev, next)| prev <= next)
        {
            // The critical points must be strictly decreasing
            return None;
        }

        let minus_one = Rational::from(-1);
        if critical_points.iter().any(|c| c <= &minus_one) {
            // All critical points must be strictly greater than -1
            return None;
        }

        // Actual construction
        let mut x_intercepts = Vec::with_capacity(slopes.len());
        if critical_points.is_empty() {
            x_intercepts[0] = mast;
        } else {
            let mut value = mast;
            let mut i = 0;
            for _ in 0..critical_points.len() {
                if i > 0 {
                    value -= (critical_points[i - 1] - critical_points[i]) * slopes[i];
                }
                x_intercepts[i] = value - (critical_points[i] * slopes[i]);
                i += 1;
            }
            x_intercepts[i] = value - (critical_points[i - 1] * slopes[i]);
        }

        Some(Self {
            critical_points,
            slopes,
            x_intercepts,
        })
    }

    /// Get intercept of mast and the x-axis
    pub fn mast_x_intercept(&self) -> Rational {
        self.x_intercepts[0]
    }

    /// Gets the value of this trajectory at the specified height (y value).
    pub fn value_at(&self, r: Rational) -> Rational {
        let i = self
            .critical_points
            .iter()
            .take_while(|critical_point| r < **critical_point)
            .count();
        if r.is_infinite() && self.slopes[i] == Rational::from(0) {
            self.x_intercepts[i]
        } else {
            (r * self.slopes[i]) + self.x_intercepts[i]
        }
    }

    /// # Panics
    /// - When `t < -1`
    pub(crate) fn compare_to_at(&self, other: &Self, t: Rational) -> Ordering {
        assert!(t >= Rational::from(-1), "t < -1");

        if t == Rational::PositiveInfinity {
            if self.slopes[0] == other.slopes[0] {
                self.x_intercepts[0].cmp(&other.x_intercepts[0])
            } else {
                self.slopes[0].cmp(&other.slopes[0])
            }
        } else {
            self.value_at(t).cmp(&other.value_at(t))
        }
    }

    #[inline]
    pub(crate) fn intersection_point(
        slope1: &Rational,
        x_intercept1: &Rational,
        slope2: &Rational,
        x_intercept2: &Rational,
    ) -> Rational {
        (x_intercept2 - x_intercept1) / (slope1 - slope2)
    }

    pub(crate) fn extend_trajectory(
        upwards: bool,
        cps: &mut Vec<Rational>,
        slopes: &mut Vec<Rational>,
        x_intercepts: &mut Vec<Rational>,
        new_cp: &Rational,
        new_slope: &Rational,
        new_x_intercept: &Rational,
    ) {
        if new_cp == &Rational::from(-1) || (!cps.is_empty() && cps.last().unwrap() == new_cp) {
        } else if !slopes.is_empty() && slopes.last().unwrap() == new_slope {
            // The x-intercept must also be the same (since the trajectory is connected).
            // So just set the critical point higher.
            debug_assert_eq!(new_x_intercept, &x_intercepts[slopes.len() - 1]);
            if upwards {
                // You cannot inline it becasue borrow checker...
                let last_idx = cps.len() - 1;
                cps[last_idx] = *new_cp;
            }
        } else {
            cps.push(*new_cp);
            slopes.push(*new_slope);
            x_intercepts.push(*new_x_intercept);
        }
    }

    #[inline]
    fn is_infinite(&self) -> bool {
        self.x_intercepts[0].is_infinite()
    }

    #[inline]
    pub(crate) fn max(&self, other: &Self) -> Self {
        self.minmax::<true>(other)
    }

    #[inline]
    pub(crate) fn min(&self, other: &Self) -> Self {
        self.minmax::<false>(other)
    }

    #[cfg_attr(
        feature = "cargo-clippy",
        allow(clippy::useless_let_if_seq, clippy::cognitive_complexity)
    )]
    fn minmax<const MAX: bool>(&self, other: &Self) -> Self {
        let max_multiplier = if MAX { -1 } else { 1 };
        // We scan down through the critical points.  We keep track of which
        // trajectory was dominant at the previous critical point:
        // <0 = this, 0 = both (equal), >0 = t.
        let mut next_critical_point_self = 0;
        let mut next_critical_point_other = 0;

        let mut new_critical_points = Vec::<Rational>::new();
        let mut new_slopes = Vec::<Rational>::new();
        let mut new_x_intercepts = Vec::<Rational>::new();

        // First handle the masts. We set dominantAtPrevCP to equal the trajectory that dominates
        // at infinity. This is the one with the lower mast slope (for min); if the mast slopes are
        // equal, it's the one with the lower mast x-intercept. Note that if either mast is
        // infinite, then we consider only the x-intercepts, *not* the slopes.

        let mut dominant_at_previous_critical_point = 0;
        if !self.is_infinite() && !other.is_infinite() {
            dominant_at_previous_critical_point =
                max_multiplier * self.slopes[0].cmp(&other.slopes[0]) as i32;
        }
        if dominant_at_previous_critical_point == 0 {
            dominant_at_previous_critical_point =
                max_multiplier * self.x_intercepts[0].cmp(&other.x_intercepts[0]) as i32;
        }

        loop {
            let current_critical_point_owner;
            let current_critical_point: Rational;

            if next_critical_point_self == self.critical_points.len()
                && next_critical_point_other == other.critical_points.len()
            {
                current_critical_point_owner = 0;
                current_critical_point = Rational::from(-1);
            } else {
                if next_critical_point_self == self.critical_points.len() {
                    current_critical_point_owner = 1;
                } else if next_critical_point_other == other.critical_points.len() {
                    current_critical_point_owner = -1;
                } else {
                    current_critical_point_owner = other.critical_points[next_critical_point_other]
                        .cmp(&self.critical_points[next_critical_point_self])
                        as i32;
                }
                current_critical_point = if current_critical_point_owner <= 0 {
                    self.critical_points[next_critical_point_self]
                } else {
                    other.critical_points[next_critical_point_other]
                }
            }

            let dominant_at_current_critical_point = max_multiplier
                * (self
                    .value_at(current_critical_point)
                    .cmp(&other.value_at(current_critical_point)) as i32);

            if (dominant_at_current_critical_point < 0 && dominant_at_previous_critical_point > 0)
                || (dominant_at_current_critical_point > 0
                    && dominant_at_previous_critical_point < 0)
            {
                // The dominant trajectory has changed.  This means there
                // must have been a crossover since the last critical point.
                // The crossover occurs at the intersection of the two line
                // segments above this critical point.
                let crossover_point = (other.x_intercepts[next_critical_point_other]
                    - self.x_intercepts[next_critical_point_self])
                    / (self.slopes[next_critical_point_self]
                        - other.slopes[next_critical_point_other]);
                new_critical_points.push(crossover_point);
                new_slopes.push(if dominant_at_previous_critical_point < 0 {
                    self.slopes[next_critical_point_self]
                } else {
                    other.slopes[next_critical_point_other]
                });
                new_x_intercepts.push(if dominant_at_previous_critical_point < 0 {
                    self.x_intercepts[next_critical_point_self]
                } else {
                    other.x_intercepts[next_critical_point_other]
                });
            }

            if current_critical_point == Rational::from(-1) {
                break;
            }

            // Now we need to determine whether `current_critical_point` is a critical point
            // of the new trajectory.  There are several ways this can happen:

            if dominant_at_current_critical_point < 0 && current_critical_point_owner <= 0 {
                // This trajectory is dominant at `current_critical_point` and its slope changes there.
                new_critical_points.push(current_critical_point);
                new_slopes.push(self.slopes[next_critical_point_self]);
                new_x_intercepts.push(self.x_intercepts[next_critical_point_self]);
            } else if dominant_at_current_critical_point > 0 && current_critical_point_owner >= 0 {
                // `other` is dominant at `current_critical_point` and its slope changes there.
                new_critical_points.push(current_critical_point);
                new_slopes.push(other.slopes[next_critical_point_other]);
                new_x_intercepts.push(other.x_intercepts[next_critical_point_other]);
            } else if dominant_at_current_critical_point == 0 {
                // The trajectories meet at `current_critical_point`. In this case we check which
                // *slope* dominates above and below `current_critical_point`, and add
                // `current_critical_point` if they differ. If we're finding the min, then the
                // dominant slope is the *smaller* slope above, *larger* slope below.
                let dominant_slope_above_current_critical_point = max_multiplier
                    * (self.slopes[next_critical_point_self]
                        .cmp(&other.slopes[next_critical_point_other])
                        as i32);
                let slope_above_current_critical_point =
                    if dominant_slope_above_current_critical_point < 0 {
                        self.slopes[next_critical_point_self]
                    } else {
                        other.slopes[next_critical_point_other]
                    };
                let self_slope_below_current_critical_point = if current_critical_point_owner <= 0 {
                    self.slopes[next_critical_point_self + 1]
                } else {
                    self.slopes[next_critical_point_self]
                };
                let other_slope_below_current_critical_point = if current_critical_point_owner >= 0
                {
                    other.slopes[next_critical_point_other + 1]
                } else {
                    other.slopes[next_critical_point_other]
                };

                let slope_below_current_critical_point = if MAX {
                    self_slope_below_current_critical_point
                        .min(other_slope_below_current_critical_point)
                } else {
                    self_slope_below_current_critical_point
                        .max(other_slope_below_current_critical_point)
                };
                if slope_above_current_critical_point != slope_below_current_critical_point {
                    new_critical_points.push(current_critical_point);
                    new_slopes.push(slope_above_current_critical_point);
                    new_x_intercepts.push(if dominant_slope_above_current_critical_point < 0 {
                        self.x_intercepts[next_critical_point_self]
                    } else {
                        other.x_intercepts[next_critical_point_other]
                    });
                }
            }

            if current_critical_point_owner <= 0 {
                next_critical_point_self += 1;
            }
            if current_critical_point_owner >= 0 {
                next_critical_point_other += 1;
            }

            dominant_at_previous_critical_point = dominant_at_current_critical_point;
        }
        // For the final slope / x-intercept, we use whichever dominates at -1.  If they're equal
        // at -1, then it's the one whose slope dominates just *above* -1 (the one with the lower
        // final slope in the case of min).

        let negative_one = Rational::from(-1);
        let mut dominant_at_tail = max_multiplier
            * (self
                .value_at(negative_one)
                .cmp(&other.value_at(negative_one)) as i32);
        if dominant_at_tail == 0 {
            dominant_at_tail = max_multiplier
                * (self
                    .slopes
                    .last()
                    .unwrap()
                    .cmp(other.slopes.last().unwrap()) as i32);
        }

        new_slopes.push(if dominant_at_tail < 0 {
            *self.slopes.last().unwrap()
        } else {
            *other.slopes.last().unwrap()
        });

        new_x_intercepts.push(if dominant_at_tail < 0 {
            *self.x_intercepts.last().unwrap()
        } else {
            *other.x_intercepts.last().unwrap()
        });

        Self {
            critical_points: new_critical_points,
            slopes: new_slopes,
            x_intercepts: new_x_intercepts,
        }
    }
}

impl Display for Trajectory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Trajectory")?;
        display::parens(f, |f| {
            write!(f, "{}, ", self.mast_x_intercept())?;
            display::brackets(f, |f| display::commas(f, &self.critical_points))?;
            write!(f, ", ")?;
            display::brackets(f, |f| display::commas(f, &self.slopes))
        })
    }
}
