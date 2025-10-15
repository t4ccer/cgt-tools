#![allow(missing_docs)]

use crate::{
    parsing::{Parser, impl_from_str_via_parser, lexeme, try_option},
    short::partizan::Player,
    total::impl_total_wrapper,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    L,
    N,
    P,
    R,
}

impl std::fmt::Display for Outcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Outcome::L => write!(f, "L"),
            Outcome::N => write!(f, "N"),
            Outcome::P => write!(f, "P"),
            Outcome::R => write!(f, "R"),
        }
    }
}

impl PartialOrd for Outcome {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        use std::cmp::Ordering;

        match (self, other) {
            (Outcome::L, Outcome::L) => Some(Ordering::Equal),
            (Outcome::L, Outcome::N) => Some(Ordering::Greater),
            (Outcome::L, Outcome::P) => Some(Ordering::Greater),
            (Outcome::L, Outcome::R) => Some(Ordering::Greater),
            (Outcome::N, Outcome::L) => Some(Ordering::Less),
            (Outcome::N, Outcome::N) => Some(Ordering::Equal),
            (Outcome::N, Outcome::P) => None,
            (Outcome::N, Outcome::R) => Some(Ordering::Greater),
            (Outcome::P, Outcome::L) => Some(Ordering::Less),
            (Outcome::P, Outcome::N) => None,
            (Outcome::P, Outcome::P) => Some(Ordering::Equal),
            (Outcome::P, Outcome::R) => Some(Ordering::Greater),
            (Outcome::R, Outcome::L) => Some(Ordering::Less),
            (Outcome::R, Outcome::N) => Some(Ordering::Less),
            (Outcome::R, Outcome::P) => Some(Ordering::Less),
            (Outcome::R, Outcome::R) => Some(Ordering::Equal),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct GameFormInner {
    left: Vec<GameFormInner>,
    right: Vec<GameFormInner>,
}

impl_total_wrapper! {
    #[derive(Debug, Clone)]
    struct GameForm {
        inner: GameFormInner
    }
}

impl std::fmt::Display for GameForm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.to_integer() {
            Some(n) => write!(f, "{n}"),
            None => {
                write!(f, "{{")?;
                for (idx, gl) in self.moves(Player::Left).iter().enumerate() {
                    if idx > 0 {
                        write!(f, ",")?;
                    }
                    write!(f, "{gl}")?;
                }
                write!(f, "|")?;
                for (idx, gr) in self.moves(Player::Right).iter().enumerate() {
                    if idx > 0 {
                        write!(f, ",")?;
                    }
                    write!(f, "{gr}")?;
                }
                write!(f, "}}")
            }
        }
    }
}

impl GameForm {
    pub fn new(left: Vec<GameForm>, right: Vec<GameForm>) -> GameForm {
        let mut left = GameForm::into_inner_vec(left);
        left.sort();
        left.dedup();

        let mut right = GameForm::into_inner_vec(right);
        right.sort();
        right.dedup();

        GameForm {
            inner: GameFormInner { left, right },
        }
    }

    pub fn new_integer(n: i32) -> GameForm {
        use std::cmp::Ordering;

        match n.cmp(&0) {
            Ordering::Less => GameForm::new(vec![], vec![GameForm::new_integer(n + 1)]),
            Ordering::Equal => GameForm::new(vec![], vec![]),
            Ordering::Greater => GameForm::new(vec![GameForm::new_integer(n - 1)], vec![]),
        }
    }

    pub fn to_integer(&self) -> Option<i32> {
        if self.moves(Player::Left).is_empty() && self.moves(Player::Right).is_empty() {
            Some(0)
        } else if let [gl] = &self.moves(Player::Left)
            && self.moves(Player::Right).is_empty()
        {
            let prev = gl.to_integer()?;
            (prev >= 0).then_some(prev + 1)
        } else if let [gr] = &self.moves(Player::Right)
            && self.moves(Player::Left).is_empty()
        {
            let prev = gr.to_integer()?;
            (prev <= 0).then_some(prev - 1)
        } else {
            None
        }
    }

    pub fn moves(&self, player: Player) -> &[GameForm] {
        match player {
            Player::Left => GameForm::from_inner_slice(self.inner.left.as_slice()),
            Player::Right => GameForm::from_inner_slice(self.inner.right.as_slice()),
        }
    }

    pub fn wins_going_first(&self, player: Player) -> bool {
        self.moves(player).is_empty()
            || self
                .moves(player)
                .iter()
                .any(|g| !g.wins_going_first(player.opposite()))
    }

    pub fn outcome(&self) -> Outcome {
        match (
            self.wins_going_first(Player::Left),
            self.wins_going_first(Player::Right),
        ) {
            (true, true) => Outcome::N,
            (true, false) => Outcome::L,
            (false, true) => Outcome::R,
            (false, false) => Outcome::P,
        }
    }

    pub fn is_p_free(&self) -> bool {
        (self.outcome() != Outcome::P)
            && Player::forall(|p| self.moves(p).iter().all(GameForm::is_p_free))
    }

    pub fn is_end(&self, player: Player) -> bool {
        self.moves(player).is_empty()
    }

    pub fn is_dead_end(&self, player: Player) -> bool {
        self.is_end(player)
            && self
                .moves(player.opposite())
                .iter()
                .all(|g| g.is_dead_end(player))
    }

    pub fn is_dead_ending(&self) -> bool {
        Player::forall(|p| !self.is_end(p) || self.is_dead_end(p))
            && Player::forall(|p| self.moves(p).iter().all(|g| g.is_dead_ending()))
    }

    pub fn is_blocked_end(&self, p: Player) -> bool {
        self.is_end(p)
            && self.moves(p.opposite()).iter().all(|gr| {
                gr.is_blocked_end(p) || gr.moves(p).iter().any(|grl| grl.is_blocked_end(p))
            })
    }

    pub fn is_blocking(&self) -> bool {
        Player::forall(|p| !self.is_end(p) || self.is_blocked_end(p))
            && Player::forall(|p| self.moves(p).iter().all(|gp| gp.is_blocking()))
    }

    pub fn next_day(day: &[GameForm]) -> impl Iterator<Item = GameForm> {
        use itertools::Itertools;

        day.iter().powerset().flat_map(|left_moves| {
            day.iter().powerset().map(move |right_moves| {
                GameForm::new(
                    left_moves.clone().into_iter().cloned().collect(),
                    right_moves.into_iter().cloned().collect(),
                )
            })
        })
    }

    pub fn conjugate(&self) -> GameForm {
        GameForm::new(
            self.moves(Player::Right)
                .iter()
                .map(|gr| gr.conjugate())
                .collect(),
            self.moves(Player::Left)
                .iter()
                .map(|gl| gl.conjugate())
                .collect(),
        )
    }

    pub fn sum(g: &GameForm, h: &GameForm) -> GameForm {
        let mut left = Vec::new();
        for gl in g.moves(Player::Left) {
            left.push(GameForm::sum(gl, h));
        }
        for hl in h.moves(Player::Left) {
            left.push(GameForm::sum(g, hl));
        }

        let mut right = Vec::new();
        for gr in g.moves(Player::Right) {
            right.push(GameForm::sum(gr, h));
        }
        for hr in h.moves(Player::Right) {
            right.push(GameForm::sum(g, hr));
        }

        GameForm::new(left, right)
    }

    pub fn tipping_point(&self, player: Player) -> u32 {
        match player {
            Player::Left => {
                let mut n = 0;
                loop {
                    if GameForm::sum(self, &GameForm::new_integer(-(n as i32))).outcome()
                        == Outcome::L
                    {
                        break n;
                    }
                    n += 1;
                }
            }
            Player::Right => {
                let mut n = 0;
                loop {
                    if GameForm::sum(self, &GameForm::new_integer(n as i32)).outcome() == Outcome::R
                    {
                        break n;
                    }
                    n += 1;
                }
            }
        }
    }

    fn parse_list(mut p: Parser<'_>) -> Option<(Parser<'_>, Vec<GameForm>)> {
        let mut acc = Vec::new();
        loop {
            match lexeme!(p, GameForm::parse) {
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

    fn parse<'p>(p: Parser<'p>) -> Option<(Parser<'p>, GameForm)> {
        let p = p.trim_whitespace();
        if let Some(p) = p.parse_ascii_char('{') {
            let (p, left) = try_option!(GameForm::parse_list(p));
            let p = try_option!(p.parse_ascii_char('|'));
            let (p, right) = try_option!(GameForm::parse_list(p));
            let p = try_option!(p.parse_ascii_char('}'));
            let p = p.trim_whitespace();
            Some((p, GameForm::new(left, right)))
        } else {
            // TODO: Generalize number parsers
            let (p, integer) = try_option!(lexeme!(p, Parser::parse_i64));
            Some((p, GameForm::new_integer(integer as i32)))
        }
    }
}

impl_from_str_via_parser!(GameForm);

#[cfg(any(test, feature = "quickcheck"))]
impl GameForm {
    fn arbitrary_sized(g: &mut quickcheck::Gen, mut size: i64) -> GameForm {
        use quickcheck::Arbitrary;

        let mut left = Vec::new();
        let mut right = Vec::new();

        while size > 0 {
            let opt = if bool::arbitrary(g) {
                let n = i64::arbitrary(g).rem_euclid(size);
                size -= n + 1;
                if bool::arbitrary(g) {
                    GameForm::new_integer(n as i32)
                } else {
                    GameForm::new_integer(-n as i32)
                }
            } else {
                let n = i64::arbitrary(g) % size;
                size -= n;
                GameForm::arbitrary_sized(g, n)
            };

            if bool::arbitrary(g) {
                left.push(opt);
            } else {
                right.push(opt);
            }
        }

        GameForm::new(left, right)
    }
}

#[cfg(any(test, feature = "quickcheck"))]
impl quickcheck::Arbitrary for GameForm {
    fn arbitrary(g: &mut quickcheck::Gen) -> Self {
        let size = (g.size() / 2) as i64;
        GameForm::arbitrary_sized(g, size)
    }

    fn shrink(&self) -> Box<dyn Iterator<Item = Self>> {
        use itertools::Itertools;
        use std::cmp::Ordering;

        match self.to_integer() {
            Some(n) => match n.cmp(&0) {
                Ordering::Less => Box::new(((n + 1)..=0).map(|n| GameForm::new_integer(n))),
                Ordering::Equal => quickcheck::empty_shrinker(),
                Ordering::Greater => Box::new((0..n).rev().map(|n| GameForm::new_integer(n))),
            },
            None => {
                let this = self.clone();
                Box::new(
                    this.moves(Player::Left)
                        .to_vec()
                        .shrink()
                        .chain(std::iter::once(vec![]))
                        .cartesian_product(
                            this.moves(Player::Right)
                                .to_vec()
                                .shrink()
                                .chain(std::iter::once(vec![]))
                                .collect::<Vec<_>>(),
                        )
                        .map(|(left, right)| GameForm::new(left, right)),
                )
            }
        }
    }
}

#[test]
fn to_integer() {
    assert_eq!(GameForm::new_integer(1).to_integer(), Some(1));
    assert_eq!(GameForm::new_integer(-1).to_integer(), Some(-1));
    assert_eq!(
        GameForm::new(vec![], vec![GameForm::new_integer(1)],).to_integer(),
        None
    );
}
