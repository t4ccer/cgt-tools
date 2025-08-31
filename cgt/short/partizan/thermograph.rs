//! Thermograph constructed from scaffolds with support for subzero thermography

use crate::{
    display,
    drawing::{BoundingBox, Canvas, Color, Draw, TextAlignment},
    numeric::{dyadic_rational_number::DyadicRationalNumber, rational::Rational, v2f::V2f},
    short::partizan::{Player, canonical_form::CanonicalForm, trajectory::Trajectory},
};
use core::fmt;
use std::{cmp::Ordering, fmt::Display};

/// See [thermograph](self) header
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Thermograph {
    /// Left wall of the thermograph
    pub left_wall: Trajectory,

    /// Right wall of the thermograph
    pub right_wall: Trajectory,
}

impl Thermograph {
    /// Construct a thermograph with only a mast at given value
    pub fn with_mast(mast: Rational) -> Self {
        let t = Trajectory::new_constant(mast);
        Self {
            left_wall: t.clone(),
            right_wall: t,
        }
    }

    /// Construct thermograph from left and right moves
    pub fn with_moves<LeftIter, LeftItem, RightIter, RightItem>(
        left_moves: LeftIter,
        right_moves: RightIter,
    ) -> Thermograph
    where
        LeftIter: Iterator<Item = LeftItem>,
        LeftItem: AsRef<CanonicalForm>,
        RightIter: Iterator<Item = RightItem>,
        RightItem: AsRef<CanonicalForm>,
    {
        Thermograph::with_trajectories(
            left_moves.map(|left_move| left_move.as_ref().thermograph().right_wall),
            right_moves.map(|right_move| right_move.as_ref().thermograph().left_wall),
        )
    }

    /// Construct thermograph from left and right trajectories
    pub fn with_trajectories<LeftIter, RightIter>(
        left_moves: LeftIter,
        right_moves: RightIter,
    ) -> Thermograph
    where
        LeftIter: Iterator<Item = Trajectory>,
        RightIter: Iterator<Item = Trajectory>,
    {
        let mut left_scaffold = left_moves.fold(
            Trajectory::new_constant(Rational::NegativeInfinity),
            |left_scaffold, left_move| Trajectory::max(&left_scaffold, &left_move),
        );
        left_scaffold.tilt(Rational::from(-1));

        let mut right_scaffold = right_moves.fold(
            Trajectory::new_constant(Rational::PositiveInfinity),
            |right_scaffold, right_move| Trajectory::min(&right_scaffold, &right_move),
        );
        right_scaffold.tilt(Rational::from(1));

        Thermograph::thermographic_intersection(left_scaffold, right_scaffold)
    }

    /// Get the temperature of the thermograph where both scaffolds merge into a mast
    #[allow(clippy::missing_panics_doc)]
    pub fn temperature(&self) -> DyadicRationalNumber {
        let left = self.get_left_temperature();
        let right = self.get_right_temperature();

        assert!(self.left_wall.value_at(left) <= self.right_wall.value_at(right),);

        DyadicRationalNumber::from_rational(left.max(right))
            .expect("unreachable: finite thermograph should give finite temperature")
    }

    fn get_left_temperature(&self) -> Rational {
        if self.left_wall.critical_points.is_empty() {
            Rational::from(-1)
        } else {
            self.left_wall.critical_points[0]
        }
    }

    fn get_right_temperature(&self) -> Rational {
        if self.right_wall.critical_points.is_empty() {
            Rational::from(-1)
        } else {
            self.right_wall.critical_points[0]
        }
    }

    /// Get the mast value of the thermograph
    pub fn get_mast(&self) -> Rational {
        let temperature = self.temperature().to_rational();

        if self.left_wall == Trajectory::new_constant(Rational::PositiveInfinity) {
            if self.right_wall.slopes[0] == Rational::from(0) {
                self.right_wall.value_at(temperature)
            } else {
                Rational::PositiveInfinity
            }
        } else if self.right_wall == Trajectory::new_constant(Rational::NegativeInfinity) {
            if self.left_wall.slopes[0] == Rational::from(0) {
                self.left_wall.value_at(temperature)
            } else {
                Rational::NegativeInfinity
            }
        } else {
            self.left_wall.value_at(temperature)
        }
    }

    /// Calculate a thermograph given left and right scaffold. Note that scaffolds should be
    /// [tilted](Trajectory::tilt) before.
    #[allow(clippy::cognitive_complexity, clippy::missing_panics_doc)]
    pub fn thermographic_intersection(
        left_scaffold: Trajectory,
        right_scaffold: Trajectory,
    ) -> Self {
        if left_scaffold == Trajectory::new_constant(Rational::PositiveInfinity)
            || right_scaffold == Trajectory::new_constant(Rational::NegativeInfinity)
        {
            return Self {
                left_wall: left_scaffold,
                right_wall: right_scaffold,
            };
        }

        let mut left_wall_cps: Vec<Rational> = Vec::new();
        let mut left_wall_slopes: Vec<Rational> = Vec::new();
        let mut left_wall_x_intercepts: Vec<Rational> = Vec::new();
        let mut right_wall_cps: Vec<Rational> = Vec::new();
        let mut right_wall_slopes: Vec<Rational> = Vec::new();
        let mut right_wall_x_intercepts: Vec<Rational> = Vec::new();

        let minus_one = Rational::from(-1);
        let zero = Rational::from(0);

        let ls_at_base: Rational = left_scaffold.value_at(minus_one);
        let rs_at_base: Rational = right_scaffold.value_at(minus_one);

        let mut previous_cave_value: Option<Rational>;

        if ls_at_base < rs_at_base
            || (ls_at_base == rs_at_base
                && left_scaffold.slopes.last().unwrap() < right_scaffold.slopes.last().unwrap())
        {
            // The left scaffold is smaller than the right scaffold immediately
            // above the base.  So we start in a cave region.
            // The value of this cave region is 0 if 0 lies between the left
            // and right scaffolds at the base.  Otherwise it's the value of
            // the scaffold that lies *closer* to 0 at the base.

            if ls_at_base > zero {
                previous_cave_value = Some(ls_at_base);
            } else if rs_at_base < zero {
                previous_cave_value = Some(rs_at_base);
            } else {
                previous_cave_value = Some(zero);
            }
        } else {
            // We start in a hill region.
            previous_cave_value = None;
        }

        // We work bottom-up and reverse the lists at the end.

        let mut next_cp_left = left_scaffold.critical_points.len() as i32 - 1;
        let mut next_cp_right = right_scaffold.critical_points.len() as i32 - 1;

        while next_cp_left >= -1 || next_cp_right >= -1 {
            // <0 for left, 0 for both, >0 for Right
            let current_cp_owner: i32;
            let current_cp: Rational;

            if next_cp_left == -1 && next_cp_right == -1 {
                // We've reached the end of the "real" critical points.  Now we
                // need to consider infinity as an "artificial" critical point.

                current_cp_owner = 0;
                current_cp = Rational::PositiveInfinity;
            } else {
                if next_cp_left == -1 {
                    current_cp_owner = 1;
                } else if next_cp_right == -1 {
                    current_cp_owner = -1;
                } else {
                    current_cp_owner = left_scaffold.critical_points[next_cp_left as usize]
                        .cmp(&right_scaffold.critical_points[next_cp_right as usize])
                        as i32;
                }
                current_cp = if current_cp_owner <= 0 {
                    left_scaffold.critical_points[next_cp_left as usize]
                } else {
                    right_scaffold.critical_points[next_cp_right as usize]
                }
            }

            let now_in_hill_region: bool = matches!(
                left_scaffold.compare_to_at(&right_scaffold, current_cp),
                Ordering::Greater | Ordering::Equal
            );
            if previous_cave_value.is_none() && !now_in_hill_region {
                // We were previously in a hill region, but just entered a cave region.
                // Extend the hill to the crossover point.
                let crossover_point = Trajectory::intersection_point(
                    &left_scaffold.slopes[(next_cp_left + 1) as usize],
                    &left_scaffold.x_intercepts[(next_cp_left + 1) as usize],
                    &right_scaffold.slopes[(next_cp_right + 1) as usize],
                    &right_scaffold.x_intercepts[(next_cp_right + 1) as usize],
                );

                debug_assert_eq!(
                    left_scaffold.value_at(crossover_point),
                    right_scaffold.value_at(crossover_point),
                    "Invalid crossover point"
                );

                Trajectory::extend_trajectory(
                    true,
                    &mut left_wall_cps,
                    &mut left_wall_slopes,
                    &mut left_wall_x_intercepts,
                    &crossover_point,
                    &left_scaffold.slopes[(next_cp_left + 1) as usize],
                    &left_scaffold.x_intercepts[(next_cp_left + 1) as usize],
                );
                Trajectory::extend_trajectory(
                    true,
                    &mut right_wall_cps,
                    &mut right_wall_slopes,
                    &mut right_wall_x_intercepts,
                    &crossover_point,
                    &right_scaffold.slopes[(next_cp_right + 1) as usize],
                    &right_scaffold.x_intercepts[(next_cp_right + 1) as usize],
                );

                // Now add the cave mast.
                let cave_mast_slope: Rational;
                let cave_mast_intercept: Rational;
                if left_scaffold.value_at(current_cp) > left_scaffold.value_at(crossover_point) {
                    // The left scaffold moves to the left above the crossover point.
                    // The cave mast follows the left scaffold.
                    cave_mast_slope = left_scaffold.slopes[(next_cp_left + 1) as usize];
                    cave_mast_intercept = left_scaffold.x_intercepts[(next_cp_left + 1) as usize];
                    previous_cave_value = Some(left_scaffold.value_at(current_cp));
                } else if right_scaffold.value_at(current_cp)
                    < right_scaffold.value_at(crossover_point)
                {
                    // The right scaffold moves to the right above the crossover point.
                    // The cave mast follows the right scaffold.
                    cave_mast_slope = right_scaffold.slopes[(next_cp_right + 1) as usize];
                    cave_mast_intercept = right_scaffold.x_intercepts[(next_cp_right + 1) as usize];
                    previous_cave_value = Some(right_scaffold.value_at(current_cp));
                } else {
                    // Neither of the above.
                    // The cave mast extends vertically above the crossover point.
                    cave_mast_slope = Rational::from(0);
                    cave_mast_intercept = left_scaffold.value_at(crossover_point);
                    previous_cave_value = Some(cave_mast_intercept);
                }

                // Extend the trajectories according to the cave mast/intercept.
                Trajectory::extend_trajectory(
                    true,
                    &mut left_wall_cps,
                    &mut left_wall_slopes,
                    &mut left_wall_x_intercepts,
                    &current_cp,
                    &cave_mast_slope,
                    &cave_mast_intercept,
                );
                Trajectory::extend_trajectory(
                    true,
                    &mut right_wall_cps,
                    &mut right_wall_slopes,
                    &mut right_wall_x_intercepts,
                    &current_cp,
                    &cave_mast_slope,
                    &cave_mast_intercept,
                );
            } else if let Some(previous_cave_value_r) = &previous_cave_value {
                // We were previously in a cave region. There are three cases:
                // (i)   The left scaffold moves to the left of the previous cave value,
                // (ii)  The right scaffold moves to the right of the previous cave value,
                // (iii) The previous cave value remains between the left and right
                //       scaffolds.
                // If both scaffolds move past the previous cave value, then we favor
                // case (i) or (ii) depending on which happens *first*.

                // First determine which crossing points exist and find their values.
                let left_scaffold_crossing_point =
                    if &left_scaffold.value_at(current_cp) > previous_cave_value_r {
                        Some(
                            (previous_cave_value_r
                                - left_scaffold.x_intercepts[(next_cp_left + 1) as usize])
                                / left_scaffold.slopes[(next_cp_left + 1) as usize],
                        )
                    } else {
                        None
                    };
                let right_scaffold_crossing_point =
                    if &right_scaffold.value_at(current_cp) < previous_cave_value_r {
                        Some(
                            (previous_cave_value_r
                                - right_scaffold.x_intercepts[(next_cp_right + 1) as usize])
                                / right_scaffold.slopes[(next_cp_right + 1) as usize],
                        )
                    } else {
                        None
                    };

                if left_scaffold_crossing_point.is_some()
                    && (right_scaffold_crossing_point.is_none()
                        || left_scaffold_crossing_point.as_ref().unwrap()
                            <= right_scaffold_crossing_point.as_ref().unwrap())
                {
                    // We are in case (i). First add the truncated vertical mast.
                    Trajectory::extend_trajectory(
                        true,
                        &mut left_wall_cps,
                        &mut left_wall_slopes,
                        &mut left_wall_x_intercepts,
                        left_scaffold_crossing_point.as_ref().unwrap(),
                        &0.into(),
                        previous_cave_value_r,
                    );
                    Trajectory::extend_trajectory(
                        true,
                        &mut right_wall_cps,
                        &mut right_wall_slopes,
                        &mut right_wall_x_intercepts,
                        left_scaffold_crossing_point.as_ref().unwrap(), // it should be left
                        &0.into(),
                        previous_cave_value_r,
                    );

                    // Now add the tilted mast for the left wall. (The left wall follows the left
                    // scaffold up to currentCP even if the scaffolds enter a hill region.)
                    Trajectory::extend_trajectory(
                        true,
                        &mut left_wall_cps,
                        &mut left_wall_slopes,
                        &mut left_wall_x_intercepts,
                        &current_cp,
                        &left_scaffold.slopes[(next_cp_left + 1) as usize],
                        &left_scaffold.x_intercepts[(next_cp_left + 1) as usize],
                    );

                    // To handle the right wall we need to know whether we've re-entered a hill
                    // region or not.
                    let new_right_cp = if now_in_hill_region {
                        Trajectory::intersection_point(
                            &left_scaffold.slopes[(next_cp_left + 1) as usize],
                            &left_scaffold.x_intercepts[(next_cp_left + 1) as usize],
                            &right_scaffold.slopes[(next_cp_right + 1) as usize],
                            &right_scaffold.x_intercepts[(next_cp_right + 1) as usize],
                        )
                    } else {
                        previous_cave_value = Some(left_scaffold.value_at(current_cp));
                        current_cp
                    };

                    // Extend the right trajectory.
                    Trajectory::extend_trajectory(
                        true,
                        &mut right_wall_cps,
                        &mut right_wall_slopes,
                        &mut right_wall_x_intercepts,
                        &new_right_cp,
                        &left_scaffold.slopes[(next_cp_left + 1) as usize],
                        &left_scaffold.x_intercepts[(next_cp_left + 1) as usize],
                    );
                } else if let Some(right_scaffold_crossing_point_r) = &right_scaffold_crossing_point
                {
                    // We are in case (ii). First add the truncated vertical mast.
                    Trajectory::extend_trajectory(
                        true,
                        &mut left_wall_cps,
                        &mut left_wall_slopes,
                        &mut left_wall_x_intercepts,
                        right_scaffold_crossing_point_r, // it should be right
                        &Rational::from(0),
                        previous_cave_value_r,
                    );
                    Trajectory::extend_trajectory(
                        true,
                        &mut right_wall_cps,
                        &mut right_wall_slopes,
                        &mut right_wall_x_intercepts,
                        left_scaffold_crossing_point.as_ref().unwrap(),
                        &Rational::from(0),
                        previous_cave_value_r,
                    );

                    // Now add the tilted mast for the right wall. (The right wall follows the right
                    // scaffold up to currentCP even if the scaffolds enter a hill region.)
                    Trajectory::extend_trajectory(
                        true,
                        &mut right_wall_cps,
                        &mut right_wall_slopes,
                        &mut right_wall_x_intercepts,
                        &current_cp,
                        &right_scaffold.slopes[(next_cp_right + 1) as usize],
                        &right_scaffold.x_intercepts[(next_cp_right + 1) as usize],
                    );

                    // To handle the left wall we need to know whether we've re-entered a hill
                    // region or not.
                    let new_left_cp = if now_in_hill_region {
                        // A hill region is indeed re-entered.  So the tilted mast for Left extends
                        // just up to the scaffolds' next point of intersection.
                        Trajectory::intersection_point(
                            &left_scaffold.slopes[(next_cp_left + 1) as usize],
                            &left_scaffold.x_intercepts[(next_cp_left + 1) as usize],
                            &right_scaffold.slopes[(next_cp_right + 1) as usize],
                            &right_scaffold.x_intercepts[(next_cp_right + 1) as usize],
                        )
                    } else {
                        previous_cave_value = Some(right_scaffold.value_at(current_cp));
                        current_cp
                    };
                    Trajectory::extend_trajectory(
                        true,
                        &mut left_wall_cps,
                        &mut left_wall_slopes,
                        &mut left_wall_x_intercepts,
                        &new_left_cp,
                        &right_scaffold.slopes[(next_cp_right + 1) as usize],
                        &right_scaffold.x_intercepts[(next_cp_right + 1) as usize],
                    );
                } else {
                    // We are in case (iii).
                    Trajectory::extend_trajectory(
                        true,
                        &mut left_wall_cps,
                        &mut left_wall_slopes,
                        &mut left_wall_x_intercepts,
                        &current_cp,
                        &Rational::from(0),
                        previous_cave_value_r,
                    );
                    Trajectory::extend_trajectory(
                        true,
                        &mut right_wall_cps,
                        &mut right_wall_slopes,
                        &mut right_wall_x_intercepts,
                        &current_cp,
                        &Rational::from(0),
                        previous_cave_value_r,
                    );
                }
            }
            if now_in_hill_region {
                // We're in a hill region, so we need to add the critical point(s) for the hill,
                // regardless of what region we were in previously.
                if current_cp_owner <= 0 {
                    Trajectory::extend_trajectory(
                        true,
                        &mut left_wall_cps,
                        &mut left_wall_slopes,
                        &mut left_wall_x_intercepts,
                        &current_cp,
                        &left_scaffold.slopes[(next_cp_left + 1) as usize],
                        &left_scaffold.x_intercepts[(next_cp_left + 1) as usize],
                    );
                }
                if current_cp_owner >= 0 {
                    Trajectory::extend_trajectory(
                        true,
                        &mut right_wall_cps,
                        &mut right_wall_slopes,
                        &mut right_wall_x_intercepts,
                        &current_cp,
                        &right_scaffold.slopes[(next_cp_right + 1) as usize],
                        &right_scaffold.x_intercepts[(next_cp_right + 1) as usize],
                    );
                }
                previous_cave_value = None;
            }

            if current_cp_owner <= 0 {
                next_cp_left -= 1;
            }
            if current_cp_owner >= 0 {
                next_cp_right -= 1;
            }
        }

        // Now remove the "infinite" critical point from the end.
        left_wall_cps.pop();
        left_wall_cps.reverse();
        left_wall_slopes.reverse();
        left_wall_x_intercepts.reverse();
        let left_wall = Trajectory {
            critical_points: left_wall_cps,
            slopes: left_wall_slopes,
            x_intercepts: left_wall_x_intercepts,
        };

        right_wall_cps.pop();
        right_wall_cps.reverse();
        right_wall_slopes.reverse();
        right_wall_x_intercepts.reverse();
        let right_wall = Trajectory {
            critical_points: right_wall_cps,
            slopes: right_wall_slopes,
            x_intercepts: right_wall_x_intercepts,
        };

        Self {
            left_wall,
            right_wall,
        }
    }

    fn y_top_above_x_axis(&self) -> f32 {
        let y_top_above_x_axis_l = self
            .left_wall
            .critical_points
            .first()
            .copied()
            .and_then(Rational::as_f32)
            .unwrap_or(0.0);
        let y_top_above_x_axis_r = self
            .right_wall
            .critical_points
            .first()
            .copied()
            .and_then(Rational::as_f32)
            .unwrap_or(0.0);
        y_top_above_x_axis_l.max(y_top_above_x_axis_r)
    }

    /// Draw thermograph with scale (length of one thermograph unit)
    pub fn draw_scaled<C>(&self, canvas: &mut C, scale: f32)
    where
        C: Canvas,
    {
        let padding: f32 = 0.5;
        let mast_height: f32 = 0.5;

        let left_x = self
            .left_wall
            .value_at(Rational::from(-1))
            .as_f32()
            .unwrap();
        let right_x = self
            .right_wall
            .value_at(Rational::from(-1))
            .as_f32()
            .unwrap();
        let y_top_above_x_axis = self.y_top_above_x_axis();

        canvas.line(
            scale
                * V2f {
                    x: -left_x,
                    y: y_top_above_x_axis,
                },
            scale
                * V2f {
                    x: padding.mul_add(2.0, -right_x),
                    y: y_top_above_x_axis,
                },
            C::thin_line_weight(),
            Color::LIGHT_GRAY,
        );

        if left_x >= 0.0 && right_x <= 0.0 {
            let y_axis_position_x = padding;
            canvas.line(
                scale
                    * V2f {
                        x: y_axis_position_x,
                        y: -1.0,
                    },
                scale
                    * V2f {
                        x: y_axis_position_x,
                        y: padding.mul_add(2.0, y_top_above_x_axis) + mast_height,
                    },
                C::thin_line_weight(),
                Color::LIGHT_GRAY,
            );
        }

        let mut draw_trajectory = |trajectory: &Trajectory, side: Player| {
            let mut prev_x = -trajectory.mast_x_intercept().as_f32().unwrap();
            let mut prev_y = y_top_above_x_axis;

            // TODO: Inject points for y=0 if do not exist
            for (point_idx, this_y_r) in trajectory
                .critical_points
                .iter()
                .copied()
                .chain(std::iter::once(Rational::from(-1)))
                .enumerate()
            {
                let this_x_r = trajectory.value_at(this_y_r);

                let this_x = -this_x_r.as_f32().unwrap();
                let this_y = this_y_r.as_f32().unwrap();

                let prev_point = scale
                    * V2f {
                        x: prev_x + padding,
                        y: y_top_above_x_axis - 1.0 + padding + mast_height - prev_y,
                    };
                let this_point = scale
                    * V2f {
                        x: this_x + padding,
                        y: y_top_above_x_axis - 1.0 + padding + mast_height - this_y,
                    };

                // TODO: Better heuristic if top-most point should be on the left or right
                if point_idx > 0 || matches!(side, Player::Right) {
                    let alignment = match side {
                        Player::Left => TextAlignment::Left,
                        Player::Right => TextAlignment::Right,
                    };
                    canvas.text(
                        this_point,
                        format_args!("({this_x_r}, {this_y_r})"),
                        alignment,
                        Color::BLACK,
                    );
                }

                canvas.line(prev_point, this_point, C::thick_line_weight(), Color::BLACK);

                prev_x = this_x;
                prev_y = this_y;
            }
        };

        draw_trajectory(&self.left_wall, Player::Left);
        draw_trajectory(&self.right_wall, Player::Right);

        let mast_x = -self.left_wall.mast_x_intercept().as_f32().unwrap();
        let mast_y = y_top_above_x_axis;
        canvas.line(
            scale
                * V2f {
                    x: mast_x + padding,
                    y: y_top_above_x_axis - 1.0 + padding + mast_height - mast_y,
                },
            scale
                * V2f {
                    x: mast_x + padding,
                    y: y_top_above_x_axis - 1.0 + padding - mast_y,
                },
            C::thick_line_weight(),
            Color::BLACK,
        );
        canvas.line(
            scale
                * V2f {
                    x: mast_x + padding + 0.2,
                    y: y_top_above_x_axis - 1.0 + padding - mast_y + 0.2,
                },
            scale
                * V2f {
                    x: mast_x + padding,
                    y: y_top_above_x_axis - 1.0 + padding - mast_y,
                },
            C::thick_line_weight(),
            Color::BLACK,
        );
        canvas.line(
            scale
                * V2f {
                    x: mast_x + padding - 0.2,
                    y: y_top_above_x_axis - 1.0 + padding - mast_y + 0.2,
                },
            scale
                * V2f {
                    x: mast_x + padding,
                    y: y_top_above_x_axis - 1.0 + padding - mast_y,
                },
            C::thick_line_weight(),
            Color::BLACK,
        );
    }

    /// Measure thermograph with scale (length of one thermograph unit)
    pub fn required_canvas_scaled<C>(&self, scale: f32) -> BoundingBox
    where
        C: Canvas,
    {
        let padding: f32 = 0.5;
        let mast_height: f32 = 0.5;

        let left_x = self.left_wall.value_at(Rational::from(-1));
        let right_x = self.right_wall.value_at(Rational::from(-1));
        let y_top_above_x_axis = self.y_top_above_x_axis();

        BoundingBox {
            top_left: scale
                * V2f {
                    x: -left_x.as_f32().unwrap(),
                    y: -1.0,
                },
            bottom_right: scale
                * V2f {
                    x: padding.mul_add(2.0, -right_x.as_f32().unwrap()),
                    y: padding.mul_add(2.0, y_top_above_x_axis) + mast_height,
                },
        }
    }
}

impl Draw for Thermograph {
    fn draw<C>(&self, canvas: &mut C)
    where
        C: Canvas,
    {
        self.draw_scaled(canvas, 64.0);
    }

    fn required_canvas<C>(&self) -> BoundingBox
    where
        C: Canvas,
    {
        self.required_canvas_scaled::<C>(64.0)
    }
}

impl Display for Thermograph {
    /// Follows cgsuite format
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Thermograph")?;
        display::parens(f, |f| write!(f, "{}, {}", self.left_wall, self.right_wall))
    }
}

#[test]
fn thermograph() {
    use super::canonical_form::CanonicalForm;
    use std::str::FromStr;

    let cf = CanonicalForm::from_str("{{2|0}|-1}").unwrap();
    let t = cf.thermograph();
    assert_eq!(
        t.to_string(),
        "Thermograph(Trajectory(0, [], [0]), Trajectory(0, [1], [0, 1]))"
    );
}
