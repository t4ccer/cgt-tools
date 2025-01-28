//! Left dead end is a game where every follower is a left end (there is no move for Left)

use std::{cmp::Ordering, fmt::Display, mem::ManuallyDrop};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum LeftDeadEndInner {
    Integer(u32),
    Moves(Vec<LeftDeadEndInner>),
}

impl Display for LeftDeadEndInner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use crate::display::{braces, commas};

        match self {
            LeftDeadEndInner::Integer(n) => write!(f, "{}", n),
            LeftDeadEndInner::Moves(moves) => braces(f, |f| commas(f, moves.as_slice())),
        }
    }
}

impl LeftDeadEndInner {
    fn into_inner_vec(moves: Vec<LeftDeadEnd>) -> Vec<LeftDeadEndInner> {
        let mut md = ManuallyDrop::new(moves);
        let ptr: *mut LeftDeadEnd = md.as_mut_ptr();
        let len = md.len();
        let capacity = md.capacity();
        unsafe { Vec::from_raw_parts(ptr.cast::<LeftDeadEndInner>(), len, capacity) }
    }
}

/// Left dead end is a game where every follower is a left end (there is no move for Left)
#[derive(Debug, Clone)]
pub struct LeftDeadEnd {
    inner: LeftDeadEndInner,
}

impl Display for LeftDeadEnd {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.inner.fmt(f)
    }
}

impl PartialEq for LeftDeadEnd {
    fn eq(&self, rhs: &Self) -> bool {
        self.partial_cmp(rhs)
            .is_some_and(|cmp| matches!(cmp, Ordering::Equal))
    }
}

impl PartialOrd for LeftDeadEnd {
    fn partial_cmp(&self, rhs: &Self) -> Option<Ordering> {
        match (self >= rhs, self <= rhs) {
            (true, true) => Some(Ordering::Equal),
            (true, false) => Some(Ordering::Greater),
            (false, true) => Some(Ordering::Less),
            (false, false) => None,
        }
    }

    fn ge(&self, rhs: &LeftDeadEnd) -> bool {
        if self.to_integer().is_some_and(|int| int == 0) {
            return rhs.to_integer().is_some_and(|int| int == 0);
        }

        let rhs_options = rhs.clone().into_moves();
        self.clone().into_moves().iter().all(|left_option| {
            rhs_options
                .iter()
                .any(|right_option| left_option >= right_option)
        })
    }

    fn le(&self, rhs: &LeftDeadEnd) -> bool {
        if rhs.to_integer().is_some_and(|int| int == 0) {
            return self.to_integer().is_some_and(|int| int == 0);
        }

        let lhs_options = self.clone().into_moves();
        rhs.clone().into_moves().iter().all(|right_option| {
            lhs_options
                .iter()
                .any(|left_option| left_option <= right_option)
        })
    }
}

impl LeftDeadEnd {
    /// Construct new *non-positive* integer of given absolute value
    pub const fn new_integer(integer: u32) -> LeftDeadEnd {
        LeftDeadEnd {
            inner: LeftDeadEndInner::Integer(integer),
        }
    }

    /// Convert to absolute value of integer, if is an integer
    pub const fn to_integer(&self) -> Option<u32> {
        match self.inner {
            LeftDeadEndInner::Integer(n) => Some(n),
            LeftDeadEndInner::Moves(_) => None,
        }
    }

    /// Construct new position from Right's moves
    pub fn new_moves(moves: Vec<LeftDeadEnd>) -> LeftDeadEnd {
        let moves = LeftDeadEndInner::into_inner_vec(moves);
        LeftDeadEnd::normalize(LeftDeadEndInner::Moves(moves))
    }

    /// Convert position into Right's moves
    pub fn into_moves(self) -> Vec<LeftDeadEnd> {
        match self.inner {
            LeftDeadEndInner::Integer(0) => vec![],
            LeftDeadEndInner::Integer(n) => vec![LeftDeadEnd {
                inner: LeftDeadEndInner::Integer(n - 1),
            }],
            LeftDeadEndInner::Moves(moves) => LeftDeadEnd::from_inner_vec(moves),
        }
    }

    fn from_inner_vec(moves: Vec<LeftDeadEndInner>) -> Vec<LeftDeadEnd> {
        let mut md = ManuallyDrop::new(moves);
        let ptr: *mut LeftDeadEndInner = md.as_mut_ptr();
        let len = md.len();
        let capacity = md.capacity();
        unsafe { Vec::from_raw_parts(ptr.cast::<LeftDeadEnd>(), len, capacity) }
    }

    fn normalize(inner: LeftDeadEndInner) -> LeftDeadEnd {
        match inner {
            LeftDeadEndInner::Integer(_) => LeftDeadEnd { inner },
            LeftDeadEndInner::Moves(moves) => {
                let moves = moves
                    .into_iter()
                    .map(|inner| LeftDeadEnd::normalize(inner).inner)
                    .collect::<Vec<_>>();
                match moves.as_slice() {
                    [] => LeftDeadEnd {
                        inner: LeftDeadEndInner::Integer(0),
                    },
                    [LeftDeadEndInner::Integer(n)] => LeftDeadEnd {
                        inner: LeftDeadEndInner::Integer(n + 1),
                    },
                    _ => LeftDeadEnd {
                        inner: LeftDeadEndInner::Moves(moves),
                    },
                }
            }
        }
    }
}

#[test]
fn to_from_moves() {
    let five = LeftDeadEnd::new_integer(5);
    assert_eq!(five.to_string(), "5");

    let five_moves = five.into_moves();
    assert_eq!(
        LeftDeadEnd::new_moves(five_moves),
        LeftDeadEnd::new_integer(5)
    );
}

#[test]
fn cmp() {
    assert_eq!(
        LeftDeadEnd::new_integer(0).partial_cmp(&LeftDeadEnd::new_integer(0)),
        Some(Ordering::Equal)
    );

    assert_eq!(
        LeftDeadEnd::new_integer(5).partial_cmp(&LeftDeadEnd::new_integer(5)),
        Some(Ordering::Equal)
    );

    assert_eq!(
        LeftDeadEnd::new_integer(3).partial_cmp(&LeftDeadEnd::new_integer(2)),
        None
    );

    assert_eq!(
        LeftDeadEnd::new_integer(3).partial_cmp(&LeftDeadEnd::new_moves(vec![
            LeftDeadEnd::new_integer(1),
            LeftDeadEnd::new_integer(2)
        ])),
        Some(Ordering::Greater)
    );

    assert_eq!(
        LeftDeadEnd::new_integer(1).partial_cmp(&LeftDeadEnd::new_moves(vec![
            LeftDeadEnd::new_integer(0),
            LeftDeadEnd::new_integer(1)
        ])),
        Some(Ordering::Greater),
    );
}

#[cfg(test)]
fn next_day(day: Vec<LeftDeadEnd>) -> Vec<LeftDeadEnd> {
    use itertools::Itertools;

    day.into_iter()
        .powerset()
        .fold(Vec::new(), |mut seen, moves| {
            let g = LeftDeadEnd::new_moves(moves);
            if !seen.contains(&g) {
                seen.push(g);
            }
            seen
        })
}

#[test]
fn born_by_day() {
    let day0 = vec![LeftDeadEnd::new_integer(0)];

    let day1 = next_day(day0);
    assert_eq!(
        day1.iter().map(|g| g.to_string()).collect::<Vec<String>>(),
        vec!["0", "1"],
    );

    let day2 = next_day(day1);
    assert_eq!(
        day2.iter().map(|g| g.to_string()).collect::<Vec<String>>(),
        vec!["0", "1", "2", "{0, 1}"],
    );

    let day3 = next_day(day2);
    assert_eq!(day3.len(), 10);

    let day4 = next_day(day3);
    assert_eq!(day4.len(), 52);
}
