use crate::{numeric::rational::Rational, short::partizan::trajectory::Trajectory};
use std::cmp::Ordering;
use std::fmt::Display;

#[derive(Debug, Hash, Clone, PartialEq, Eq)]
pub struct Thermograph {
    pub(crate) left_wall: Trajectory,
    pub(crate) right_wall: Trajectory,
}

impl Thermograph {
    pub fn with_mast(mast: Rational) -> Self {
        let t = Trajectory::new_constant(mast);
        Thermograph {
            left_wall: t.clone(),
            right_wall: t,
        }
    }

    pub fn get_temperature(&self) -> Rational {
        let left = self.get_left_temperature();
        let right = self.get_right_temperature();

        if self.left_wall.value_at(left) > self.right_wall.value_at(right) {
            Rational::PositiveInfinity
        } else {
            left.max(right)
        }
    }

    pub fn get_left_temperature(&self) -> Rational {
        if self.left_wall.critical_points.is_empty() {
            Rational::from(-1)
        } else {
            self.left_wall.critical_points[0]
        }
    }

    pub fn get_right_temperature(&self) -> Rational {
        if self.right_wall.critical_points.is_empty() {
            Rational::from(-1)
        } else {
            self.right_wall.critical_points[0]
        }
    }

    pub fn thermographic_intersection(
        left_scaffold: Trajectory,
        right_scaffold: Trajectory,
    ) -> Self {
        if left_scaffold == Trajectory::new_constant(Rational::PositiveInfinity)
            || right_scaffold == Trajectory::new_constant(Rational::NegativeInfinity)
        {
            return Thermograph {
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
                    left_scaffold.critical_points[next_cp_left as usize].clone()
                } else {
                    right_scaffold.critical_points[next_cp_right as usize].clone()
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

                // debug_assert_eq!(
                //     left_scaffold.value_at(&crossover_point),
                //     right_scaffold.value_at(&crossover_point),
                //     "Invalid crossover point"
                // );

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
                    cave_mast_slope = left_scaffold.slopes[(next_cp_left + 1) as usize].clone();
                    cave_mast_intercept =
                        left_scaffold.x_intercepts[(next_cp_left + 1) as usize].clone();
                    previous_cave_value = Some(left_scaffold.value_at(current_cp));
                } else if right_scaffold.value_at(current_cp)
                    < right_scaffold.value_at(crossover_point)
                {
                    // The right scaffold moves to the right above the crossover point.
                    // The cave mast follows the right scaffold.
                    cave_mast_slope = right_scaffold.slopes[(next_cp_right + 1) as usize].clone();
                    cave_mast_intercept =
                        right_scaffold.x_intercepts[(next_cp_right + 1) as usize].clone();
                    previous_cave_value = Some(right_scaffold.value_at(current_cp));
                } else {
                    // Neither of the above.
                    // The cave mast extends vertically above the crossover point.
                    cave_mast_slope = Rational::from(0);
                    cave_mast_intercept = left_scaffold.value_at(crossover_point);
                    previous_cave_value = Some(cave_mast_intercept.clone());
                }

                // Extend the trajectories according to the cave mast/intercept.
                Trajectory::extend_trajectory(
                    true,
                    &mut left_wall_cps,
                    &mut left_wall_slopes,
                    &mut left_wall_x_intercepts,
                    &current_cp,
                    &cave_mast_slope,
                    &&cave_mast_intercept,
                );
                Trajectory::extend_trajectory(
                    true,
                    &mut right_wall_cps,
                    &mut right_wall_slopes,
                    &mut right_wall_x_intercepts,
                    &current_cp,
                    &cave_mast_slope,
                    &&cave_mast_intercept,
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
                            &(previous_cave_value_r
                                - &left_scaffold.x_intercepts[(next_cp_left + 1) as usize])
                                / &left_scaffold.slopes[(next_cp_left + 1) as usize],
                        )
                    } else {
                        None
                    };
                let right_scaffold_crossing_point =
                    if &right_scaffold.value_at(current_cp) < previous_cave_value_r {
                        Some(
                            &(previous_cave_value_r
                                - &right_scaffold.x_intercepts[(next_cp_right + 1) as usize])
                                / &right_scaffold.slopes[(next_cp_right + 1) as usize],
                        )
                    } else {
                        None
                    };

                if left_scaffold_crossing_point.is_some()
                    && (right_scaffold_crossing_point.is_none()
                        || left_scaffold_crossing_point.as_ref().unwrap()
                            <= &right_scaffold_crossing_point.as_ref().unwrap())
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
                    let new_right_cp: Rational;
                    if now_in_hill_region {
                        new_right_cp = Trajectory::intersection_point(
                            &left_scaffold.slopes[(next_cp_left + 1) as usize],
                            &left_scaffold.x_intercepts[(next_cp_left + 1) as usize],
                            &right_scaffold.slopes[(next_cp_right + 1) as usize],
                            &right_scaffold.x_intercepts[(next_cp_right + 1) as usize],
                        );
                    } else {
                        previous_cave_value = Some(left_scaffold.value_at(current_cp));
                        new_right_cp = current_cp.clone();
                    }

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
                    let new_left_cp: Rational;
                    if now_in_hill_region {
                        // A hill region is indeed re-entered.  So the tilted mast for Left extends
                        // just up to the scaffolds' next point of intersection.
                        new_left_cp = Trajectory::intersection_point(
                            &left_scaffold.slopes[(next_cp_left + 1) as usize],
                            &left_scaffold.x_intercepts[(next_cp_left + 1) as usize],
                            &right_scaffold.slopes[(next_cp_right + 1) as usize],
                            &right_scaffold.x_intercepts[(next_cp_right + 1) as usize],
                        );
                    } else {
                        previous_cave_value = Some(right_scaffold.value_at(current_cp));
                        new_left_cp = current_cp.clone();
                    }
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
                        &previous_cave_value_r,
                    );
                    Trajectory::extend_trajectory(
                        true,
                        &mut right_wall_cps,
                        &mut right_wall_slopes,
                        &mut right_wall_x_intercepts,
                        &current_cp,
                        &Rational::from(0),
                        &previous_cave_value_r,
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

        Thermograph {
            left_wall,
            right_wall,
        }
    }
}

impl Display for Thermograph {
    /// Follows cgsuite format
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fn fmt_trajectory(f: &mut std::fmt::Formatter<'_>, t: &Trajectory) -> std::fmt::Result {
            write!(f, "{},[", t.mast_x_intercept())?;
            for (idx, critical_point) in t.critical_points.iter().enumerate() {
                if idx != 0 {
                    write!(f, ",")?;
                }
                write!(f, "{}", critical_point)?;
            }
            write!(f, "],[")?;
            for (idx, slope) in t.slopes.iter().enumerate() {
                if idx != 0 {
                    write!(f, ",")?;
                }
                write!(f, "{}", slope)?;
            }
            write!(f, "]")?;
            Ok(())
        }

        write!(f, "Thermograph(")?;
        fmt_trajectory(f, &self.left_wall)?;
        write!(f, ",")?;
        fmt_trajectory(f, &self.right_wall)?;
        write!(f, ")")?;

        Ok(())
    }
}
