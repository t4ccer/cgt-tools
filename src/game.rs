use std::{fmt::Display, ops::Not};

use nom::{bytes::complete::tag, multi::separated_list0, IResult};

// TODO: Find rational library
type Num = i64;

mod parser {
    use nom::{
        character::complete::{self, multispace0},
        IResult, Parser,
    };

    pub fn i64(input: &str) -> IResult<&str, i64> {
        complete::i64(input)
    }

    pub fn lexeme<'a, Output, Error, F>(
        mut inner: F,
    ) -> impl FnMut(&'a str) -> IResult<&str, Output, Error>
    where
        F: Parser<&'a str, Output, Error>,
        Error: nom::error::ParseError<&'a str>,
    {
        move |input: &str| {
            let (input, _ws) = multispace0(input)?;
            let (input, res) = inner.parse(input)?;
            let (input, _ws) = multispace0(input)?;
            Ok((input, res))
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Game {
    pub left: Vec<Game>,
    pub right: Vec<Game>,
}

impl Game {
    fn parser(input: &str) -> IResult<&str, Self> {
        match parser::i64(input) {
            Ok((input, num)) => Ok((input, Game::num_to_game(num))),
            Err(_) => {
                let (input, _) = parser::lexeme(tag("{"))(input)?;
                let (input, left) =
                    separated_list0(parser::lexeme(tag(",")), parser::lexeme(Game::parser))(input)?;
                let (input, _) = parser::lexeme(tag("|"))(input)?;
                let (input, right) =
                    separated_list0(parser::lexeme(tag(",")), parser::lexeme(Game::parser))(input)?;
                let (input, _) = parser::lexeme(tag("}"))(input)?;
                Ok((input, Game { left, right }))
            }
        }
    }

    pub fn parse(input: &str) -> Option<Self> {
        Self::parser(input).ok().map(|(_, g)| g)
    }

    pub fn zero() -> Game {
        Game {
            left: vec![],
            right: vec![],
        }
    }

    pub fn one() -> Game {
        Game {
            left: vec![Game::zero()],
            right: vec![],
        }
    }
}

#[test]
fn parse_empty_game() {
    let inp = "{|}";
    let exp = ("", Game::zero());
    assert_eq!(exp, Game::parser(inp).unwrap());
}

#[test]
fn parse_left_game() {
    let inp = "{0,0|}";
    let exp = (
        "",
        Game {
            left: vec![Game::zero(), Game::zero()],
            right: vec![],
        },
    );
    assert_eq!(exp, Game::parser(inp).unwrap());
}

#[test]
fn parse_right_game() {
    let inp = "{|0}";
    let exp = (
        "",
        Game {
            left: vec![],
            right: vec![Game::zero()],
        },
    );
    assert_eq!(exp, Game::parser(inp).unwrap());
}

#[test]
fn parse_nested_game() {
    let inp = "{0|0}";
    let exp = (
        "",
        Game {
            left: vec![Game::zero()],
            right: vec![Game::zero()],
        },
    );
    assert_eq!(exp, Game::parser(inp).unwrap());
}

// TODO: Add support for "known values", like 0, 1, *, etc.
impl Display for Game {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{")?;
        for (idx, m) in self.left.iter().enumerate() {
            if idx != 0 {
                write!(f, ",")?;
            }
            write!(f, "{}", *m)?;
        }
        write!(f, "|")?;
        for (idx, m) in self.right.iter().enumerate() {
            if idx != 0 {
                write!(f, ",")?;
            }
            write!(f, "{}", *m)?;
        }
        write!(f, "}}")?;
        Ok(())
    }
}

impl Game {
    fn greater_eq(lhs: &Game, rhs: &Game) -> bool {
        Self::less_eq(rhs, lhs)
    }

    fn less_eq(g: &Game, h: &Game) -> bool {
        g.left.iter().any(|gl| Self::less_eq(h, gl)).not()
            && h.right.iter().any(|hr| Self::less_eq(hr, g)).not()
    }

    fn confused(lhs: &Game, rhs: &Game) -> bool {
        !(Self::less_eq(lhs, rhs)) && !(Self::less_eq(rhs, lhs))
    }

    fn remove_dominated_by<F>(comp: F, games: &[Game]) -> Vec<Game>
    where
        F: Fn(&Game, &Game) -> bool,
    {
        match games.get(0) {
            None => vec![],
            Some(g) => {
                // TODO: refactor
                let gs = &games[1..];
                if gs.iter().any(|gg| comp(gg, g)) {
                    Self::remove_dominated_by(comp, gs)
                } else {
                    let gs: Vec<Game> = gs
                        .iter()
                        .filter(|gg| Self::confused(g, gg))
                        .cloned()
                        .collect();
                    let mut res = Self::remove_dominated_by(comp, &gs);
                    res.push(g.clone());
                    res
                }
            }
        }
    }

    fn remove_dominated(&self) -> Game {
        Game {
            left: Self::remove_dominated_by(Self::greater_eq, &self.left),
            right: Self::remove_dominated_by(Self::less_eq, &self.right),
        }
    }

    pub fn canonical_form(self) -> Self {
        if self == Game::zero() {
            return self;
        }
        let g = self.remove_dominated();

        let left: Vec<Game> = g
            .left
            .clone()
            .into_iter()
            .map(Self::canonical_form)
            .collect();

        let right: Vec<Game> = g
            .right
            .clone()
            .into_iter()
            .map(Self::canonical_form)
            .collect();

        let left: Vec<Game> = left
            .into_iter()
            .flat_map(|l| Self::l_bypass_reversible(&g, &l))
            .collect();
        let right: Vec<Game> = right
            .into_iter()
            .flat_map(|r| Self::r_bypass_reversible(&g, &r))
            .collect();

        Game { left, right }
    }

    fn l_bypass_reversible(g: &Game, gl: &Game) -> Vec<Game> {
        match gl.right.iter().find(|r| Self::less_eq(r, g)) {
            None => vec![gl.clone()],
            Some(glr) => glr.left.clone(),
        }
    }

    fn r_bypass_reversible(g: &Game, gr: &Game) -> Vec<Game> {
        match gr.left.iter().find(|l| Self::greater_eq(l, g)) {
            None => vec![gr.clone()],
            Some(glr) => glr.right.clone(),
        }
    }

    pub fn num_to_game(num: Num) -> Self {
        if num == 0 {
            Game::zero()
        } else if num > 0 {
            Game {
                left: vec![Game::num_to_game(num - 1)],
                right: vec![],
            }
        } else {
            assert!(num < 0);
            Game {
                left: vec![],
                right: vec![Game::num_to_game(num + 1)],
            }
        }
    }
}

#[test]
fn zero_leq_zero() {
    assert!(Game::less_eq(&Game::zero(), &Game::zero()))
}

#[test]
fn zero_leq_one() {
    assert!(Game::less_eq(&Game::zero(), &Game::one()))
}

#[test]
fn one_dominates_zero() {
    assert_eq!(
        Game::parse("{0,1|}").unwrap().remove_dominated(),
        Game::parse("{1|}").unwrap()
    )
}

#[test]
fn compute_canonical_form() {
    assert_eq!(
        Game::parse("{0,1|}").unwrap().canonical_form(),
        Game::parse("2").unwrap()
    );

    assert_eq!(
        Game::parse("{2|}").unwrap().canonical_form(),
        Game::parse("3").unwrap()
    );

    assert_eq!(
        Game::parse("{|-2}").unwrap().canonical_form(),
        Game::parse("-3").unwrap()
    );

    assert_eq!(
        Game::parse("{1,2,3|1}").unwrap().canonical_form(),
        Game::parse("{3|1}").unwrap()
    );
}
