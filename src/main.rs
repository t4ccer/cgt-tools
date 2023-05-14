use std::{fmt::Display, fs::remove_dir, ops::Not};

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
    fn parse(input: &str) -> IResult<&str, Self> {
        let (input, _) = parser::lexeme(tag("{"))(input)?;
        let (input, left) =
            separated_list0(parser::lexeme(tag(",")), parser::lexeme(Game::parse))(input)?;
        let (input, _) = parser::lexeme(tag("|"))(input)?;
        let (input, right) =
            separated_list0(parser::lexeme(tag(",")), parser::lexeme(Game::parse))(input)?;
        let (input, _) = parser::lexeme(tag("}"))(input)?;
        Ok((input, Game { left, right }))
    }

    fn zero() -> Game {
        Game {
            left: vec![],
            right: vec![],
        }
    }

    fn one() -> Game {
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
    assert_eq!(exp, Game::parse(inp).unwrap());
}

#[test]
fn parse_left_game() {
    let inp = "{{|},{|}|}";
    let exp = (
        "",
        Game {
            left: vec![Game::zero(), Game::zero()],
            right: vec![],
        },
    );
    assert_eq!(exp, Game::parse(inp).unwrap());
}

#[test]
fn parse_right_game() {
    let inp = "{|{|}}";
    let exp = (
        "",
        Game {
            left: vec![],
            right: vec![Game::zero()],
        },
    );
    assert_eq!(exp, Game::parse(inp).unwrap());
}

#[test]
fn parse_nested_game() {
    let inp = "{ {|} | {|} }";
    let exp = (
        "",
        Game {
            left: vec![Game::zero()],
            right: vec![Game::zero()],
        },
    );
    assert_eq!(exp, Game::parse(inp).unwrap());
}

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
        let g = self.remove_dominated();
        todo!()
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
        Game {
            left: vec![Game::zero(), Game::one()],
            right: vec![]
        }
        .remove_dominated(),
        Game {
            left: vec![Game::one()],
            right: vec![]
        }
    )
}

fn main() {
    // let g = Game::parse("{{{|}|},{|}|}").unwrap().1;
    // println!("{}", g);

    // let g = g.remove_dominated();
    // println!("{}", g);

    println!("{}", Game::zero());
    println!("{}", Game::one());
}
