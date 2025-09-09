//! Canonical form of a short game

use crate::{
    display,
    numeric::{dyadic_rational_number::DyadicRationalNumber, nimber::Nimber, rational::Rational},
    parsing::{Parser, impl_from_str_via_parser, lexeme, try_option},
    short::partizan::thermograph::Thermograph,
};
use auto_ops::impl_op_ex;
use nus::Nus;
use std::{
    borrow::Cow,
    cmp::Ordering,
    fmt::{self, Display},
    hash::Hash,
    iter::FusedIterator,
    iter::Sum,
};

pub mod nus;

/// Left and Right moves from a given position
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct Moves {
    /// Left player's moves
    left: Vec<CanonicalForm>,

    /// Right player's moves
    right: Vec<CanonicalForm>,
}

impl PartialOrd for Moves {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Moves {
    fn cmp(&self, other: &Self) -> Ordering {
        let left = self
            .left
            .iter()
            .map(|cf| &cf.inner)
            .cmp(other.left.iter().map(|cf| &cf.inner));

        if left.is_eq() {
            self.right
                .iter()
                .map(|cf| &cf.inner)
                .cmp(other.right.iter().map(|cf| &cf.inner))
        } else {
            left
        }
    }
}

impl Moves {
    #[inline]
    const fn empty() -> Self {
        Self {
            left: vec![],
            right: vec![],
        }
    }

    #[inline]
    fn eliminate_duplicates(&mut self) {
        self.left.sort_by(|lhs, rhs| lhs.inner.cmp(&rhs.inner));
        self.left.dedup_by(|lhs, rhs| lhs.inner == rhs.inner);

        self.right.sort_by(|lhs, rhs| lhs.inner.cmp(&rhs.inner));
        self.right.dedup_by(|lhs, rhs| lhs.inner == rhs.inner);
    }

    /// Construct a canoical form of arbitrary moves.
    /// It is an alias of [`CanonicalForm::new_from_moves`]
    #[inline]
    pub fn canonical_form(self) -> CanonicalForm {
        CanonicalForm::new_from_moves(self.left, self.right)
    }

    /// Try converting moves to NUS. Returns [None] if moves do not form a NUS
    #[allow(clippy::cognitive_complexity)]
    pub fn to_nus(&self) -> Option<Nus> {
        let num_lo = self.left.len();
        let num_ro = self.right.len();

        if num_lo == 0 && num_ro == 0 {
            // Case: {|}
            // No left or right moves so the game is 0
            Some(Nus {
                number: DyadicRationalNumber::from(0),
                up_multiple: 0,
                nimber: Nimber::from(0),
            })
        } else if num_lo == 0 {
            // Case: n-1 = {|n}
            // We assume that entry is normalized, no left moves, thus there must be only one
            // right entry that's a number
            debug_assert!(num_ro == 1, "Entry not normalized");
            Some(Nus {
                number: self.right[0].to_nus_unchecked().number() - DyadicRationalNumber::from(1),
                up_multiple: 0,
                nimber: Nimber::from(0),
            })
        } else if num_ro == 0 {
            // Case: n+1 = {n|}
            // We assume that entry is normalized, no left moves, thus there must be only one
            // right entry that's a number
            debug_assert!(num_lo == 1, "Entry not normalized");
            Some(Nus {
                number: self.left[0].to_nus_unchecked().number() + DyadicRationalNumber::from(1),
                up_multiple: 0,
                nimber: Nimber::from(0),
            })
        } else if let [left_move] = &self.left[..]
            && let [right_move] = &self.right[..]
            && let Some(left_number) = left_move.to_number()
            && let Some(right_number) = right_move.to_number()
            && left_number < right_number
        {
            // Case: {n|m}, n < m
            // We're a number but not an integer.  Conveniently, since the option lists are
            // canonicalized, the value of this game is the mean of its left & right moves.

            Some(Nus {
                number: DyadicRationalNumber::mean(&left_number, &right_number),
                up_multiple: 0,
                nimber: Nimber::from(0),
            })
        } else if let [left_move1, left_move2] = &self.left[..]
            && let [right_move] = &self.right[..]
            && let Some(left_number) = left_move1.to_number()
            && left_move1 == right_move
            && let Some(left_nus) = left_move2.to_nus()
            && left_number == left_nus.number()
            && left_nus.up_multiple() == 0
            && left_nus.nimber() == Nimber::new(1)
        {
            // Case: {n,n*|n}
            Some(Nus {
                number: left_number,
                up_multiple: 1,
                nimber: Nimber::from(1),
            })
        } else if let [left_move] = &self.left[..]
            && let [right_move1, right_move2] = &self.right[..]
            && let Some(right_number) = right_move1.to_number()
            && left_move == right_move1
            && let Some(right_nus) = right_move2.to_nus()
            && right_number == right_nus.number()
            && right_nus.up_multiple() == 0
            && right_nus.nimber() == Nimber::new(1)
        {
            // Inverse of the previous one
            Some(Nus {
                number: right_number,
                up_multiple: -1,
                nimber: Nimber::from(1),
            })
        } else if let [left_move] = &self.left[..]
            && let [right_move] = &self.right[..]
            && let Some(left_number) = left_move.to_number()
            && let Some(right_nus) = right_move.to_nus()
            && !right_nus.is_number()
            && left_number == right_nus.number()
            && right_nus.up_multiple() >= 0
        {
            // Case: n + {0|G}, G is a number-up-star of up multiple >= 0
            Some(Nus {
                number: right_nus.number(),
                up_multiple: right_nus.up_multiple() + 1,
                nimber: right_nus.nimber() + Nimber::from(1),
            })
        } else if let [left_move] = &self.left[..]
            && let [right_move] = &self.right[..]
            && let Some(left_nus) = left_move.to_nus()
            && let Some(right_number) = right_move.to_number()
            && !left_nus.is_number()
            && right_number == left_nus.number()
            && left_nus.up_multiple() <= 0
        {
            // Inverse of the previous one
            Some(Nus {
                number: left_nus.number(),
                up_multiple: left_nus.up_multiple() - 1,
                nimber: left_nus.nimber() + Nimber::from(1),
            })
        } else if num_lo >= 1
            && num_lo == num_ro
            && let Some(left_number) = self.left[0].to_number()
            && self.left[0] == self.right[0]
        {
            // Case: n + *k
            // If doesn't hold then it's not a NUS

            for i in 0..num_lo {
                let l = &self.left[i];
                let r = &self.right[i];

                if l != r
                    || !l.is_number_up_star()
                    || l.to_nus_unchecked().number() != r.to_nus_unchecked().number()
                {
                    return None;
                }

                if l.to_nus_unchecked().up_multiple() != 0
                    || l.to_nus_unchecked().nimber().value() != (i as u32)
                {
                    return None;
                }
            }
            // It's a nimber
            Some(Nus {
                number: left_number,
                up_multiple: 0,
                nimber: Nimber::from(num_lo as u32),
            })
        } else {
            None
        }
    }

    // TODO: Rewrite it to work on mutable vec and not clone
    fn eliminate_dominated_moves(
        moves: &[CanonicalForm],
        eliminate_smaller_moves: bool,
    ) -> Vec<CanonicalForm> {
        let mut moves: Vec<Option<CanonicalForm>> = moves.iter().cloned().map(Some).collect();

        'outer: for i in 0..moves.len() {
            'inner: for j in 0..i {
                let Some(move_i) = &moves[i] else {
                    continue 'outer;
                };
                let Some(move_j) = &moves[j] else {
                    continue 'inner;
                };

                // Split from ifs because borrow checker is sad
                let remove_i = (eliminate_smaller_moves && move_i <= move_j)
                    || (!eliminate_smaller_moves && move_j <= move_i);

                let remove_j = (eliminate_smaller_moves && move_j <= move_i)
                    || (!eliminate_smaller_moves && move_i <= move_j);

                if remove_i {
                    moves[i] = None;
                }

                if remove_j {
                    moves[j] = None;
                }
            }
        }

        moves.iter().flatten().cloned().collect()
    }

    /// Return false if `H <= GL` for some left option `GL` of `G` or `HR <= G` for some right
    /// option `HR` of `H`. Otherwise return true.
    fn leq_arrays(
        game: &CanonicalForm,
        left_moves: &[Option<CanonicalForm>],
        right_moves: &[Option<CanonicalForm>],
    ) -> bool {
        for r_opt in right_moves.iter().flatten() {
            if r_opt <= game {
                return false;
            }
        }

        for l_move in game.left_moves() {
            if Self::geq_arrays(&l_move, left_moves, right_moves) {
                return false;
            }
        }

        true
    }

    fn geq_arrays(
        game: &CanonicalForm,
        left_moves: &[Option<CanonicalForm>],
        right_moves: &[Option<CanonicalForm>],
    ) -> bool {
        for l_opt in left_moves.iter().flatten() {
            if game <= l_opt {
                return false;
            }
        }

        for r_move in game.right_moves() {
            if Self::leq_arrays(&r_move, left_moves, right_moves) {
                return false;
            }
        }

        true
    }

    fn bypass_reversible_moves_l(&self) -> Vec<CanonicalForm> {
        let mut i: i64 = 0;

        let mut left_moves: Vec<Option<CanonicalForm>> =
            self.left.iter().cloned().map(Some).collect();
        let right_moves: Vec<Option<CanonicalForm>> =
            self.right.iter().cloned().map(Some).collect();

        loop {
            if (i as usize) >= left_moves.len() {
                break;
            }
            let g_l = match &left_moves[i as usize] {
                None => {
                    i += 1;
                    continue;
                }
                Some(g) => g.clone(),
            };
            for g_lr in g_l.right_moves() {
                if Self::leq_arrays(&g_lr, &left_moves, &right_moves) {
                    let g_lr_moves = g_lr.left_moves();
                    let mut new_left_moves: Vec<Option<CanonicalForm>> =
                        vec![None; left_moves.len() + g_lr_moves.clone().len() - 1];
                    new_left_moves[..(i as usize)].clone_from_slice(&left_moves[..(i as usize)]);
                    new_left_moves[(i as usize)..(left_moves.len() - 1)]
                        .clone_from_slice(&left_moves[(i as usize + 1)..]);
                    for (k, g_lrl) in g_lr_moves.enumerate() {
                        let g_lrl = Some(g_lrl.into_owned());
                        if left_moves.contains(&g_lrl) {
                            new_left_moves[left_moves.len() + k - 1] = None;
                        } else {
                            new_left_moves[left_moves.len() + k - 1] = g_lrl;
                        }
                    }
                    left_moves = new_left_moves;
                    i -= 1;
                    break;
                }
            }

            i += 1;
        }

        left_moves.iter().flatten().cloned().collect()
    }

    fn bypass_reversible_moves_r(&self) -> Vec<CanonicalForm> {
        let mut i: i64 = 0;

        let left_moves: Vec<Option<CanonicalForm>> = self.left.iter().cloned().map(Some).collect();
        let mut right_moves: Vec<Option<CanonicalForm>> =
            self.right.iter().cloned().map(Some).collect();

        loop {
            if (i as usize) >= right_moves.len() {
                break;
            }
            let g_r = match &right_moves[i as usize] {
                None => {
                    i += 1;
                    continue;
                }
                Some(game) => game.clone(),
            };
            for g_rl in g_r.left_moves() {
                if Self::geq_arrays(&g_rl, &left_moves, &right_moves) {
                    let g_rl_moves = g_rl.right_moves();
                    let mut new_right_moves: Vec<Option<CanonicalForm>> =
                        vec![None; right_moves.len() + g_rl_moves.len() - 1];
                    new_right_moves[..(i as usize)].clone_from_slice(&right_moves[..(i as usize)]);
                    new_right_moves[(i as usize)..(right_moves.len() - 1)]
                        .clone_from_slice(&right_moves[(i as usize + 1)..]);
                    for (k, g_rlr) in g_rl_moves.enumerate() {
                        let g_rlr = Some(g_rlr.into_owned());
                        if right_moves.contains(&g_rlr) {
                            new_right_moves[right_moves.len() + k - 1] = None;
                        } else {
                            new_right_moves[right_moves.len() + k - 1] = g_rlr;
                        }
                    }
                    right_moves = new_right_moves;
                    i -= 1;
                    break;
                }
            }

            i += 1;
        }

        right_moves.iter().flatten().cloned().collect()
    }

    fn canonicalize(&self) -> Self {
        let left = self.bypass_reversible_moves_l();
        let left = Self::eliminate_dominated_moves(&left, true);

        let right = self.bypass_reversible_moves_r();
        let right = Self::eliminate_dominated_moves(&right, false);

        Self { left, right }
    }

    /// Parse comma-separated games, ie. the underlined part:
    ///
    /// `{a,b,...|c,d,...}`
    ///
    /// ` ^^^^^^^`
    fn parse_list(mut p: Parser<'_>) -> Option<(Parser<'_>, Vec<CanonicalForm>)> {
        let mut acc = Vec::new();
        loop {
            match lexeme!(p, CanonicalForm::parse) {
                Some((cf_p, cf)) => {
                    acc.push(cf);
                    p = cf_p;
                    p = p.trim_whitespace();
                    match p.parse_ascii_char(',') {
                        Some(pp) => {
                            p = pp.trim_whitespace();
                        }
                        None => return Some((p, acc)),
                    }
                }
                None => return Some((p, acc)),
            }
        }
    }

    fn parse(p: Parser<'_>) -> Option<(Parser<'_>, Moves)> {
        let p = try_option!(p.parse_ascii_char('{'));
        let (p, left) = try_option!(Moves::parse_list(p));
        let p = try_option!(p.parse_ascii_char('|'));
        let (p, right) = try_option!(Moves::parse_list(p));
        let p = try_option!(p.parse_ascii_char('}'));
        let moves = Self { left, right };
        Some((p, moves))
    }
}

impl Display for Moves {
    /// Print moves using `{G^L | G^R}` notation
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        display::braces(f, |f| {
            display::commas(f, &self.left)?;
            write!(f, "|")?;
            display::commas(f, &self.right)
        })
    }
}

impl_from_str_via_parser!(Moves);

/// A game `G` even-tempered if, no matter how `G` is played, the first player will have the move
/// when `G` reaches a number.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Temper {
    /// `G` is even-tempered if `G` a number, or every option of `G` is odd-tempered
    Even,

    /// `G` is odd-tempered if `G` is not a number and every option of `G` is even-tempered
    Odd,
}

/// Canonical game form
///
/// Note that ordering is defined structurally for the sake of data structures. For proper partial
/// ordering see instance for [`CanonicalForm`].
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
enum CanonicalFormInner {
    /// Number Up Star sum
    Nus(Nus),

    /// Not a NUS - list of left/right moves
    Moves(Moves),
}

/// Canonical game form
#[repr(transparent)]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CanonicalForm {
    inner: CanonicalFormInner,
}

impl CanonicalForm {
    /// Construct NUS with only integer
    #[inline]
    pub const fn new_integer(integer: i64) -> Self {
        Self::new_nus(Nus::new_integer(integer))
    }

    /// Construct NUS with only dyadic rational
    #[inline]
    pub const fn new_dyadic(dyadic: DyadicRationalNumber) -> Self {
        Self::new_nus(Nus::new_number(dyadic))
    }

    /// Construct NUS with only nimber
    #[inline]
    pub const fn new_nimber(number: DyadicRationalNumber, nimber: Nimber) -> Self {
        Self::new_nus(Nus {
            number,
            up_multiple: 0,
            nimber,
        })
    }

    /// Construct NUS
    #[inline]
    #[must_use]
    pub const fn new_nus(nus: Nus) -> Self {
        Self::from_inner(CanonicalFormInner::Nus(nus))
    }

    /// Construct negative.0 of a game. Alias for negation [`-`] operator
    #[must_use]
    pub fn construct_negative(&self) -> Self {
        match &self.inner {
            CanonicalFormInner::Nus(nus) => Self::new_nus(-nus),
            CanonicalFormInner::Moves(moves) => {
                let new_left_moves = moves
                    .left
                    .iter()
                    .map(Self::construct_negative)
                    .collect::<Vec<_>>();
                let new_right_moves = moves
                    .right
                    .iter()
                    .map(Self::construct_negative)
                    .collect::<Vec<_>>();
                let new_moves = Moves {
                    left: new_left_moves,
                    right: new_right_moves,
                };
                Self::construct_from_canonical_moves(new_moves)
            }
        }
    }

    /// Construct a sum of two games. Alias for [`+`] operator
    pub fn construct_sum(g: &Self, h: &Self) -> Self {
        if let (CanonicalFormInner::Nus(g_nus), CanonicalFormInner::Nus(h_nus)) =
            (&g.inner, &h.inner)
        {
            return Self::new_nus(g_nus + h_nus);
        }

        // We want to return { GL+H, G+HL | GR+H, G+HR }

        // By the number translation theorem

        let mut moves = Moves::empty();

        if !g.is_number() {
            for g_l in g.left_moves() {
                moves.left.push(Self::construct_sum(&g_l, h));
            }
            for g_r in g.right_moves() {
                moves.right.push(Self::construct_sum(&g_r, h));
            }
        }
        if !h.is_number() {
            for h_l in h.left_moves() {
                moves.left.push(Self::construct_sum(g, &h_l));
            }
            for h_r in h.right_moves() {
                moves.right.push(Self::construct_sum(g, &h_r));
            }
        }

        Self::new_from_moves(moves.left, moves.right)
    }

    /// VERY INTERNAL
    fn construct_from_canonical_moves(mut moves: Moves) -> Self {
        moves.left.sort_by(|lhs, rhs| lhs.inner.cmp(&rhs.inner));
        moves.right.sort_by(|lhs, rhs| lhs.inner.cmp(&rhs.inner));

        if let Some(nus) = moves.to_nus() {
            return Self::new_nus(nus);
        }

        // Game is not a nus
        Self::from_inner(CanonicalFormInner::Moves(moves))
    }

    /// Safe function to construct a game from possible moves
    pub fn new_from_moves(left: Vec<CanonicalForm>, right: Vec<CanonicalForm>) -> Self {
        let mut moves = Moves { left, right };
        moves.eliminate_duplicates();
        moves = moves.canonicalize();

        Self::construct_from_canonical_moves(moves)
    }

    #[inline]
    const fn from_inner(inner: CanonicalFormInner) -> Self {
        Self { inner }
    }

    /// Get iterator over left moves from a canonical form
    pub fn left_moves(&self) -> LeftMovesIter<'_> {
        LeftMovesIter {
            inner: match &self.inner {
                CanonicalFormInner::Nus(nus) => MovesIterInner::Nus(nus.left_moves()),
                CanonicalFormInner::Moves(moves) => {
                    MovesIterInner::Moves(moves.left.as_slice().iter())
                }
            },
        }
    }

    /// Get iterator over right moves from a canonical form
    pub fn right_moves(&self) -> RightMovesIter<'_> {
        RightMovesIter {
            inner: match &self.inner {
                CanonicalFormInner::Nus(nus) => MovesIterInner::Nus(nus.right_moves()),
                CanonicalFormInner::Moves(moves) => {
                    MovesIterInner::Moves(moves.right.as_slice().iter())
                }
            },
        }
    }

    /// Check if game is a Number Up Star sum
    #[inline]
    pub const fn is_number_up_star(&self) -> bool {
        matches!(self.inner, CanonicalFormInner::Nus(_))
    }

    /// Check if a game is only a number
    #[inline]
    pub const fn is_number(&self) -> bool {
        matches!(self.inner, CanonicalFormInner::Nus(nus) if nus.is_number())
    }

    /// Check if a game is only a nimber
    #[inline]
    pub const fn is_nimber(&self) -> bool {
        matches!(self.inner, CanonicalFormInner::Nus(nus) if nus.is_nimber())
    }

    /// Convert game to NUS if it is a NUS
    #[inline]
    pub const fn to_nus(&self) -> Option<Nus> {
        match self.inner {
            CanonicalFormInner::Nus(nus) => Some(nus),
            // Don't call Moves::to_nus here, because (a) it's already canonical and (b)
            // it calls here.
            CanonicalFormInner::Moves(_) => None,
        }
    }

    #[inline]
    const fn to_nus_unchecked(&self) -> Nus {
        self.to_nus().expect("Not a nus")
    }

    /// Convert game to number if it is only a number (i.e. [`Self::is_number`])
    #[inline]
    pub const fn to_number(&self) -> Option<DyadicRationalNumber> {
        match self.to_nus() {
            Some(nus) if nus.is_number() => Some(nus.number()),
            _ => None,
        }
    }

    /// Less than or equals comparison on two games
    pub fn leq(lhs_game: &Self, rhs_game: &Self) -> bool {
        // NOTE: There is a possible optimization.
        // Lessons in Play: Lemma 5.35: Let x be a number and G and H be games. If G.right_stop() > x
        // then G > x. If RS(G) > LS(H) then G > H.

        if lhs_game == rhs_game {
            return true;
        }

        if let (Some(lhs_nus), Some(rhs_nus)) = (&lhs_game.to_nus(), &rhs_game.to_nus()) {
            match lhs_nus.number().cmp(&rhs_nus.number()) {
                Ordering::Less => return true,
                Ordering::Greater => return false,
                Ordering::Equal => {
                    if lhs_nus.up_multiple() < rhs_nus.up_multiple() - 1 {
                        return true;
                    } else if lhs_nus.up_multiple() < rhs_nus.up_multiple() {
                        return (lhs_nus.nimber() + rhs_nus.nimber()) != Nimber::from(1);
                    }
                    return false;
                }
            }
        }

        if !lhs_game.is_number() {
            for lhs_l in lhs_game.left_moves() {
                if Self::leq(rhs_game, &lhs_l) {
                    return false;
                }
            }
        }

        if !rhs_game.is_number() {
            for rhs_r in rhs_game.right_moves() {
                if Self::leq(&rhs_r, lhs_game) {
                    return false;
                }
            }
        }

        true
    }

    /// Calculate temperature of the game. Avoids computing a thermograph is game is a NUS
    #[allow(clippy::missing_panics_doc)]
    pub fn temperature(&self) -> DyadicRationalNumber {
        match self.inner {
            CanonicalFormInner::Nus(nus) => {
                if nus.is_number() {
                    // It's a number k/2^n, so the temperature is -1/2^n
                    DyadicRationalNumber::new(-1, nus.number().denominator_exponent())
                } else {
                    // It's a number plus a nonzero infinitesimal, thus the temperature is 0
                    DyadicRationalNumber::from(0)
                }
            }
            CanonicalFormInner::Moves(_) => self.thermograph().temperature(),
        }
    }

    /// Construct a thermograph of a game, using thermographic intersection of
    /// left and right scaffolds
    pub fn thermograph(&self) -> Thermograph {
        match self.inner {
            CanonicalFormInner::Moves(_) => {
                Thermograph::with_moves(self.left_moves(), self.right_moves())
            }
            CanonicalFormInner::Nus(nus) => {
                if let Some(nus_integer) = nus.number().to_integer() {
                    if nus.is_number() {
                        return Thermograph::with_mast(Rational::new_integer(nus_integer));
                    }
                }

                if nus.up_multiple() == 0
                    || (nus.nimber() == Nimber::from(1) && nus.up_multiple().abs() == 1)
                {
                    // This looks like 0 or * (depending on whether nimberPart is 0 or 1).
                    let new_game = Self::new_nus(Nus {
                        number: nus.number(),
                        up_multiple: 0,
                        nimber: Nimber::from(nus.nimber().value().cmp(&0) as u32), // signum(nus.nimber)
                    });
                    Thermograph::with_moves(new_game.left_moves(), new_game.right_moves())
                } else {
                    let new_game = Self::new_nus(Nus {
                        number: nus.number(),
                        up_multiple: nus.up_multiple().cmp(&0) as i32, // signum(nus.up_multiple)
                        nimber: Nimber::from(0),
                    });
                    Thermograph::with_moves(new_game.left_moves(), new_game.right_moves())
                }
            }
        }
    }

    /// The number reached when Left plays first.
    pub fn left_stop(&self) -> DyadicRationalNumber {
        if let Some(number) = self.to_number() {
            return number;
        }

        self.left_moves()
            .map(|gl| gl.right_stop())
            .max()
            .expect("Not a number so must have moves")
    }

    /// The number reached when Right plays first.
    pub fn right_stop(&self) -> DyadicRationalNumber {
        if let Some(number) = self.to_number() {
            return number;
        }

        self.right_moves()
            .map(|gr| gr.left_stop())
            .max()
            .expect("Not a number so must have moves")
    }

    /// Confusion interval is the region between Left and Right stops
    pub fn confusion_interval(&self) -> (DyadicRationalNumber, DyadicRationalNumber) {
        (self.left_stop(), self.right_stop())
    }

    /// Compute the mean value of the position
    ///
    /// Mean value is the result of cooling a position by value greater than temperature
    pub fn mean(&self) -> DyadicRationalNumber {
        match self.inner {
            CanonicalFormInner::Nus(nus) => nus.number(),
            CanonicalFormInner::Moves(_) => {
                let mast = self.thermograph().get_mast();
                DyadicRationalNumber::from_rational(mast)
                    .expect("Thermograph mast to have a finite dyadic value")
            }
        }
    }

    /// Cool the position by `temperature`
    ///
    /// Position `G` cooled by `t` is `G_t = {G^L_t - t | G^R_t + t}` unless there exists a
    /// temperature `t' < t` for which `G_t'` is infinitesimally close to a number
    #[must_use]
    pub fn cool(&self, temperature: DyadicRationalNumber) -> Self {
        if let Some(nus) = self.to_nus() {
            if nus.is_integer() {
                return self.clone();
            }
        }

        if self.temperature() < temperature {
            return Self::new_dyadic(self.mean());
        }

        let temperature_game = Self::new_dyadic(temperature);

        let mut new_left_moves = Vec::with_capacity(self.left_moves().len());
        for left_move in self.left_moves() {
            new_left_moves.push(left_move.cool(temperature) - &temperature_game);
        }

        let mut new_right_moves = Vec::with_capacity(self.right_moves().len());
        for right_move in self.right_moves() {
            new_right_moves.push(right_move.cool(temperature) + &temperature_game);
        }

        let new_moves = Moves {
            left: new_left_moves,
            right: new_right_moves,
        };

        new_moves.canonical_form()
    }

    /// Heat position by given `temperature`.
    ///
    /// Heating is the inverse of cooling, defined as `\int^t G = G` if `G` is a number, or
    /// `\int^t G = {\int^t G^L + t | \int^t G^R - t}` otherwise
    #[must_use]
    pub fn heat(&self, temperature: &CanonicalForm) -> Self {
        if let Some(nus) = self.to_nus() {
            if nus.is_number() {
                return self.clone();
            }
        }

        let mut new_left_moves = Vec::with_capacity(self.left_moves().len());
        for left_move in self.left_moves() {
            new_left_moves.push(left_move.heat(temperature) + temperature);
        }

        let mut new_right_moves = Vec::with_capacity(self.right_moves().len());
        for right_move in self.right_moves() {
            new_right_moves.push(right_move.heat(temperature) - temperature);
        }

        let new_moves = Moves {
            left: new_left_moves,
            right: new_right_moves,
        };

        new_moves.canonical_form()
    }

    /// A remote star of game `g` is a nimber `*N` if no position of `g` including `g` has value `N*`
    #[must_use]
    #[allow(clippy::or_fun_call)]
    pub fn far_star(&self) -> Nimber {
        if let CanonicalFormInner::Nus(ref nus) = self.inner {
            if nus.is_nimber() {
                return Nimber::from(nus.nimber().value() + 1);
            }
        }

        self.left_moves()
            .chain(self.right_moves())
            .map(|g| g.far_star())
            .max()
            .unwrap_or(Nimber::from(1))
    }

    // FIXME: Handle cases when atomic weight does not exist
    /// Atmoic weight of a position, sometimes called "uppitiness"
    #[must_use]
    pub fn atomic_weight(&self) -> Self {
        match self.inner {
            CanonicalFormInner::Nus(nus) => Self::new_integer(nus.up_multiple() as i64),
            CanonicalFormInner::Moves(ref moves) => {
                let new_moves = Moves {
                    left: moves
                        .left
                        .iter()
                        .map(|left_move| left_move.atomic_weight() - Self::new_integer(2))
                        .collect::<Vec<_>>(),
                    right: moves
                        .right
                        .iter()
                        .map(|right_move| right_move.atomic_weight() + Self::new_integer(2))
                        .collect::<Vec<_>>(),
                };
                let new_game =
                    Self::new_from_moves(new_moves.left.clone(), new_moves.right.clone());

                let CanonicalFormInner::Nus(new_nus) = new_game.inner else {
                    return new_game;
                };

                if !new_nus.is_integer() {
                    return new_game;
                }

                let far_star = Self::new_nimber(DyadicRationalNumber::from(0), self.far_star());

                let less_than_far_star = self <= &far_star;
                let greater_than_far_star = self >= &far_star;

                if less_than_far_star && !greater_than_far_star {
                    let max_least = new_moves
                        .left
                        .iter()
                        .map(|left_move| {
                            let least = left_move.right_stop().ceil();
                            if &Self::new_integer(least) <= left_move {
                                least + 1
                            } else {
                                least
                            }
                        })
                        .max()
                        .unwrap_or(0);
                    Self::new_integer(max_least)
                } else if !less_than_far_star && greater_than_far_star {
                    let min_greatest = new_moves
                        .right
                        .iter()
                        .map(|right_move| {
                            let greatest = right_move.left_stop().round();
                            if right_move <= &Self::new_integer(greatest) {
                                greatest - 1
                            } else {
                                greatest
                            }
                        })
                        .min()
                        .unwrap_or(0);
                    Self::new_integer(min_greatest)
                } else {
                    new_game
                }
            }
        }
    }

    /// See: The Reduced Canonical Form Of a Game p. 411
    #[must_use]
    pub fn star_projection(&self) -> Self {
        if let Some(nus) = self.to_nus() {
            if (nus.nimber() == Nimber::new(0) || nus.nimber() == Nimber::new(1))
                && nus.up_multiple() == 0
            {
                return CanonicalForm::new_nus(Nus::new_number(nus.number()));
            }
        }

        CanonicalForm::new_from_moves(
            self.left_moves().map(|gl| gl.star_projection()).collect(),
            self.right_moves().map(|gr| gr.star_projection()).collect(),
        )
    }

    /// A reduced canonical form of `G` is `\bar{G}`, such that `\bar{G} = \bar{H}`
    /// whenever `G - H` is infinitesimal.
    #[must_use]
    pub fn reduced(&self) -> Self {
        self.heat(&CanonicalForm::new_nus(Nus::new_nimber(Nimber::new(1))))
            .star_projection()
    }

    /// Get temper of the game
    #[must_use]
    pub fn temper(&self) -> Option<Temper> {
        if self.to_nus().is_some_and(Nus::is_number) {
            return Some(Temper::Even);
        }

        if self.left_moves().chain(self.right_moves()).all(|m| {
            m.temper()
                .is_some_and(|temper| matches!(temper, Temper::Even))
        }) {
            return Some(Temper::Odd);
        }

        if self.left_moves().chain(self.right_moves()).all(|m| {
            m.temper()
                .is_some_and(|temper| matches!(temper, Temper::Odd))
        }) {
            return Some(Temper::Even);
        }

        None
    }

    /// Parse game using `{a,b,...|c,d,...}` notation
    #[allow(clippy::missing_errors_doc)]
    fn parse(p: Parser<'_>) -> Option<(Parser<'_>, CanonicalForm)> {
        match lexeme!(p, Nus::parse) {
            Some((p, nus)) => Some((p, CanonicalForm::new_nus(nus))),
            None => {
                let (p, Moves { left, right }) = try_option!(lexeme!(p, Moves::parse));
                Some((p, CanonicalForm::new_from_moves(left, right)))
            }
        }
    }
}

impl PartialOrd for CanonicalForm {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self == other {
            Some(Ordering::Equal)
        } else if Self::leq(self, other) {
            Some(Ordering::Less)
        } else if Self::leq(other, self) {
            Some(Ordering::Greater)
        } else {
            None
        }
    }

    fn le(&self, other: &Self) -> bool {
        Self::leq(self, other)
    }

    fn ge(&self, other: &Self) -> bool {
        Self::leq(other, self)
    }
}

impl_op_ex!(+|g: &CanonicalForm, h: &CanonicalForm| -> CanonicalForm { CanonicalForm::construct_sum(g, h) });
impl_op_ex!(+=|g: &mut CanonicalForm, h: &CanonicalForm| { *g = CanonicalForm::construct_sum(g, h) });
impl_op_ex!(-|g: &CanonicalForm| -> CanonicalForm { CanonicalForm::construct_negative(g) });
impl_op_ex!(-|g: &CanonicalForm, h: &CanonicalForm| -> CanonicalForm {
    CanonicalForm::construct_sum(g, &CanonicalForm::construct_negative(h))
});
impl_op_ex!(-=|g: &mut CanonicalForm, h: &CanonicalForm| {
    *g = CanonicalForm::construct_sum(g, &CanonicalForm::construct_negative(h));
});

impl Display for CanonicalForm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.inner {
            CanonicalFormInner::Nus(nus) => nus.fmt(f),
            CanonicalFormInner::Moves(moves) => moves.fmt(f),
        }
    }
}

impl_from_str_via_parser!(CanonicalForm);

impl Sum for CanonicalForm {
    fn sum<I: Iterator<Item = CanonicalForm>>(iter: I) -> CanonicalForm {
        iter.fold(CanonicalForm::new_integer(0), |acc, v| acc + v)
    }
}

impl<'a> Sum<&'a CanonicalForm> for CanonicalForm {
    fn sum<I: Iterator<Item = &'a CanonicalForm>>(iter: I) -> CanonicalForm {
        iter.fold(CanonicalForm::new_integer(0), |acc, v| acc + v)
    }
}

#[derive(Debug, Clone)]
enum MovesIterInner<'a, I> {
    Moves(core::slice::Iter<'a, CanonicalForm>),
    Nus(I),
}

macro_rules! dispatch_moves_iter {
    ($fname:ident (&mut self $(, $arg_name:ident: $arg_ty:ty)* $(,)?) -> Option<Self::Item>) => {
        #[inline]
        fn $fname(&mut self $(, $arg_name: $arg_ty)*) -> Option<Self::Item> {
            match self {
                MovesIterInner::Moves(iter) => iter.$fname($($arg_name,)?)
                    .map(Cow::Borrowed),
                MovesIterInner::Nus(iter) => iter.$fname($($arg_name,)?)
                    .map(|nus| Cow::Owned(CanonicalForm::new_nus(nus))),
            }
        }
    };

    ($fname:ident (self $(, $arg_name:ident: $arg_ty:ty)* $(,)?) -> $ret:tt) => {
        #[inline]
        fn $fname(self $(, $arg_name: $arg_ty)*) -> $ret {
            match self {
                MovesIterInner::Moves(iter) => iter.$fname($($arg_name,)?),
                MovesIterInner::Nus(iter) => iter.$fname($($arg_name,)?),
            }
        }
    };

    ($fname:ident (&self $(, $arg_name:ident: $arg_ty:ty)* $(,)?) -> $ret:tt) => {
        #[inline]
        fn $fname(&self $(, $arg_name: $arg_ty)*) -> $ret {
            match self {
                MovesIterInner::Moves(iter) => iter.$fname($($arg_name,)?),
                MovesIterInner::Nus(iter) => iter.$fname($($arg_name,)?),
            }
        }
    };
}

impl<'a, I> Iterator for MovesIterInner<'a, I>
where
    I: Iterator<Item = Nus>,
{
    type Item = Cow<'a, CanonicalForm>;

    dispatch_moves_iter!(next(&mut self) -> Option<Self::Item>);
    dispatch_moves_iter!(nth(&mut self, n: usize) -> Option<Self::Item>);
    dispatch_moves_iter!(size_hint(&self) -> (usize, Option<usize>));
    dispatch_moves_iter!(count(self) -> usize);
}

impl<I> ExactSizeIterator for MovesIterInner<'_, I>
where
    I: Iterator<Item = Nus> + ExactSizeIterator,
{
    dispatch_moves_iter!(len(&self) -> usize);
}

impl<I> FusedIterator for MovesIterInner<'_, I> where I: Iterator<Item = Nus> + FusedIterator {}

macro_rules! impl_moves_iter {
    ($(#[$attr:meta])* $name:ident) => {
        $(#[$attr])*
        #[derive(Debug, Clone)]
        pub struct $name<'a> {
            inner: MovesIterInner<'a, nus::$name>,
        }

        impl<'a> Iterator for $name<'a> {
            type Item = Cow<'a, CanonicalForm>;

            #[inline]
            fn next(&mut self) -> Option<Self::Item> {
                self.inner.next()
            }

            #[inline]
            fn count(self) -> usize {
                self.inner.count()
            }

            #[inline]
            fn size_hint(&self) -> (usize, Option<usize>) {
                self.inner.size_hint()
            }

            #[inline]
            fn nth(&mut self, n: usize) -> Option<Self::Item> {
                self.inner.nth(n)
            }
        }

        impl ExactSizeIterator for $name<'_> {
            #[inline]
            fn len(&self) -> usize {
                self.inner.len()
            }
        }

        impl<'a> FusedIterator for $name<'_> {}
    };
}

impl_moves_iter! {
    /// Iterator over form's left moves
    ///
    /// Can be created by the [`CanonicalForm::left_moves`] method
    LeftMovesIter
}

impl_moves_iter! {
    /// Iterator over form's right moves
    ///
    /// Can be created by the [`CanonicalForm::right_moves`] method
    RightMovesIter
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn constructs_integers() {
        let eight = CanonicalForm::new_integer(8);
        assert_eq!(&eight.to_string(), "8");

        let minus_forty_two = CanonicalForm::new_integer(-42);
        assert_eq!(&minus_forty_two.to_string(), "-42");
    }

    #[test]
    fn constructs_rationals() {
        let rational = DyadicRationalNumber::new(3, 4);
        let three_sixteenth = CanonicalForm::new_dyadic(rational);
        assert_eq!(&three_sixteenth.to_string(), "3/16");

        let duplicate = CanonicalForm::new_dyadic(rational);
        assert_eq!(three_sixteenth, duplicate);
    }

    #[test]
    fn constructs_nimbers() {
        let star = CanonicalForm::new_nus(Nus::new_nimber(Nimber::from(1)));
        assert_eq!(&star.to_string(), "*");

        let star_three = CanonicalForm::new_nus(Nus::new_nimber(Nimber::from(3)));
        assert_eq!(&star_three.to_string(), "*3");

        let one_star_two = CanonicalForm::new_nus(Nus {
            number: DyadicRationalNumber::from(1),
            up_multiple: 0,
            nimber: (Nimber::from(2)),
        });
        assert_eq!(&one_star_two.to_string(), "1*2");
    }

    #[test]
    fn constructs_up() {
        let up = CanonicalForm::new_nus(Nus {
            number: DyadicRationalNumber::from(0),
            up_multiple: 1,
            nimber: Nimber::from(0),
        });
        assert_eq!(&up.to_string(), "^");

        let up_star = CanonicalForm::new_nus(Nus {
            number: DyadicRationalNumber::from(0),
            up_multiple: 1,
            nimber: Nimber::from(1),
        });
        assert_eq!(&up_star.to_string(), "^*");

        let down = CanonicalForm::new_nus(Nus {
            number: DyadicRationalNumber::from(0),
            up_multiple: -3,
            nimber: Nimber::from(0),
        });
        assert_eq!(&down.to_string(), "v3");
    }

    macro_rules! assert_negative_eq {
        ($inp:expr, $out:expr) => {
            let inp = CanonicalForm::from_str($inp).unwrap();
            assert_eq!((-inp).to_string(), $out);
        };
    }

    #[test]
    fn negative() {
        assert_negative_eq!("0", "0");
        assert_negative_eq!("42", "-42");
        assert_negative_eq!("-42", "42");
        assert_negative_eq!("{^|*}", "{v|*}");
    }

    #[test]
    fn nimber_is_its_negative() {
        let star = CanonicalForm::new_nimber(DyadicRationalNumber::from(0), Nimber::from(4));
        assert_eq!(&star.to_string(), "*4");

        let neg_star = star.construct_negative();
        assert_eq!(star, neg_star);
    }

    #[test]
    fn simplifies_moves() {
        let one = CanonicalForm::new_nus(Nus::from_str("1").unwrap());
        let star = CanonicalForm::new_nus(Nus::from_str("*").unwrap());

        let moves_l = Moves {
            left: vec![one],
            right: vec![star],
        };
        let left_id = CanonicalForm::new_from_moves(moves_l.left, moves_l.right);
        assert_eq!(&left_id.to_string(), "{1|*}");

        let weird = Moves {
            left: vec![CanonicalForm::new_nus(Nus::from_str("1v2*").unwrap())],
            right: vec![CanonicalForm::new_nus(Nus::from_str("1").unwrap())],
        };
        let weird = CanonicalForm::new_from_moves(weird.left, weird.right);
        assert_eq!(&weird.to_string(), "1v3");
        assert_eq!(&weird.left_moves().nth(0).unwrap().to_string(), "1v2*");

        // Another case:

        let weird_right = Moves {
            left: vec![CanonicalForm::new_nus(Nus::from_str("^").unwrap())],
            right: vec![CanonicalForm::new_nus(Nus::from_str("-2").unwrap())],
        };
        let weird_right = CanonicalForm::new_from_moves(weird_right.left, weird_right.right);
        assert_eq!(&weird_right.to_string(), "{^|-2}");

        let weird = Moves {
            left: vec![],
            right: vec![weird_right],
        };
        assert_eq!(&weird.canonicalize().to_string(), "{|}");
        let weird = CanonicalForm::new_from_moves(weird.left, weird.right);
        assert_eq!(&weird.to_string(), "0");
    }

    #[test]
    fn sum_works() {
        let zero = CanonicalForm::new_integer(0);
        let one = CanonicalForm::new_integer(1);

        let one_zero = CanonicalForm::new_from_moves(vec![one.clone()], vec![zero.clone()]);
        let zero_one = CanonicalForm::new_from_moves(vec![zero], vec![one]);

        let sum = one_zero + zero_one;
        assert_eq!(&sum.to_string(), "{3/2|1/2}");
    }

    #[test]
    fn temp_of_one_minus_one_is_one() {
        let one = CanonicalForm::new_integer(1);
        let negative_one = CanonicalForm::new_integer(-1);

        let g = CanonicalForm::new_from_moves(vec![one], vec![negative_one]);
        assert_eq!(g.temperature(), DyadicRationalNumber::from(1));
    }

    #[test]
    fn parse_games() {
        macro_rules! test_game_parse {
            ($inp: expr, $expected: expr) => {{
                let g = CanonicalForm::parse(Parser::new($inp))
                    .expect("Could not parse")
                    .1;
                assert_eq!($expected, g.to_string());
            }};
        }

        // test_game_parse!("{|}", "0");
        test_game_parse!("{1,2|}", "3");
        test_game_parse!("{42|*}", "{42|*}");
        test_game_parse!("123", "123");
        test_game_parse!("{1/2|2}", "1");
        test_game_parse!("{3/4|7/8}", "13/16");
        test_game_parse!("{6/8|7/8}", "13/16");
        test_game_parse!("{12/16|14/16}", "13/16");
        test_game_parse!("{0|2}", "1");
        test_game_parse!("{0,*,*2|0,*,*2}", "*3");
    }

    #[test]
    fn ordering_works() {
        macro_rules! test_ordering {
            ($lhs:expr, $rhs:expr, $expected:expr) => {
                assert_eq!(
                    PartialOrd::partial_cmp(
                        &CanonicalForm::from_str($lhs).unwrap(),
                        &CanonicalForm::from_str($rhs).unwrap()
                    ),
                    $expected
                )
            };
        }

        test_ordering!("0", "*", None);
        test_ordering!("*", "*", Some(Ordering::Equal));
        test_ordering!("*2", "*", None);
        test_ordering!("*2", "*2", Some(Ordering::Equal));
        test_ordering!("*", "*2", None);
        test_ordering!("1", "2", Some(Ordering::Less));
        test_ordering!("-1", "*", Some(Ordering::Less));
        test_ordering!("1", "*", Some(Ordering::Greater));
    }

    macro_rules! assert_stops {
        ($cf:expr, $left:expr, $right:expr) => {
            let g = CanonicalForm::from_str($cf).unwrap();
            let (left_stop, right_stop) = g.confusion_interval();
            assert_eq!(
                left_stop,
                DyadicRationalNumber::from_str($left).expect("Could not parse left stop"),
                "Invalid left stop"
            );
            assert_eq!(
                right_stop,
                DyadicRationalNumber::from_str($right).expect("Could not parse right stop"),
                "Invalid right stop"
            );
            assert!(
                left_stop >= right_stop,
                "Left stop shold be geq than right stop"
            );
        };
    }

    #[test]
    fn stops_work() {
        assert_stops!("{{3|2}|0}", "2", "0");
        assert_stops!("v", "0", "0");
        assert_stops!("*", "0", "0");
        assert_stops!("^", "0", "0");
    }

    macro_rules! assert_cooled {
        ($cf:expr, $temp:expr, $expected:expr) => {
            let g = CanonicalForm::from_str($cf).unwrap();
            let temp = DyadicRationalNumber::from_str($temp).unwrap();
            let cooled = g.cool(temp);
            assert_eq!(
                cooled.to_string(),
                CanonicalForm::from_str($expected).unwrap().to_string()
            );
        };
    }

    #[test]
    fn cooling_works() {
        assert_cooled!("{2|-1}", "0", "{2|-1}");
        assert_cooled!("{2|-1}", "1/2", "{3/2|-1/2}");
        assert_cooled!("{2|-1}", "1", "{1|0}");
        assert_cooled!("{2|-1}", "3/2", "1/2*");
        assert_cooled!("{2|-1}", "2", "1/2");
        assert_cooled!("{2|-1}", "3", "1/2");
        assert_cooled!("{2|-1}", "42", "1/2");
    }

    #[test]
    fn heating_numbers() {
        let g = CanonicalForm::new_dyadic(DyadicRationalNumber::from(42));
        let heated = g.heat(&CanonicalForm::new_integer(1));
        assert_eq!(g, heated);
    }

    #[test]
    fn cooling_heating_roundtrip() {
        let g = CanonicalForm::from_str("{2|-1}").unwrap();
        let t = DyadicRationalNumber::from_str("3/2").unwrap();
        let cooled = g.cool(t);
        let frozen = g.cool(t + DyadicRationalNumber::from(1));
        let particle = &cooled - &frozen;
        let heated = particle.heat(&CanonicalForm::new_dyadic(t));
        assert_eq!(heated.to_string(), "{3/2|-3/2}");
        assert_eq!(g, &frozen + &heated);
    }

    macro_rules! assert_atomic_weight_eq {
        ($inp:expr, $atomic:expr) => {
            let cf = CanonicalForm::from_str($inp).unwrap();
            let atomic = CanonicalForm::from_str($atomic).unwrap();
            assert_eq!(cf.atomic_weight().to_string(), atomic.to_string());
        };
    }

    #[test]
    fn atomic_weight() {
        assert_atomic_weight_eq!("*3", "0");
        assert_atomic_weight_eq!("^", "1");
        assert_atomic_weight_eq!("v", "-1");
        assert_atomic_weight_eq!("v2", "-2");
        assert_atomic_weight_eq!("{^2|v}", "1/2");
        assert_atomic_weight_eq!("{^2|v2}", "*");
        assert_atomic_weight_eq!("{^3|v3}", "{1|-1}");
        assert_atomic_weight_eq!("{^2|*}", "1");
        assert_atomic_weight_eq!("{^2,{^|*}|*}", "1");
        assert_atomic_weight_eq!("{*|v2}", "-1");
    }

    #[test]
    fn reduced() {
        let cf = CanonicalForm::from_str("{{2|0}, 1*|*}").unwrap();
        assert_eq!(cf.reduced().to_string(), "{1|0}");

        let cf = CanonicalForm::from_str("{{3/2*|1/2}|{0|-3},{-1*,{-1/2|-1*}|-5/2}}").unwrap();
        assert_eq!(
            cf.reduced().to_string(),
            "{{3/2|1/2}|{0|-3}, {{-1/2|-1}|-5/2}}"
        );
    }

    #[test]
    fn temper() {
        let cf = CanonicalForm::from_str("2").unwrap();
        assert_eq!(cf.temper(), Some(Temper::Even));

        let cf = CanonicalForm::from_str("{2|0}").unwrap();
        assert_eq!(cf.temper(), Some(Temper::Odd));

        let cf = CanonicalForm::from_str("{2|1,{*|0}}").unwrap();
        assert_eq!(cf.temper(), None);
    }
}
