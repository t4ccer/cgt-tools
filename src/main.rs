use std::fmt::Display;

use nom::{bytes::complete::tag, character::complete, multi::separated_list0, IResult};

// TODO: Find rational library
type Num = i64;

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
enum Value {
    Number(Num),
    Star(Num),
}

mod parser {

    use nom::{bytes::complete::tag, character::complete::multispace0, IResult, Parser};

    pub fn star(input: &str) -> IResult<&str, ()> {
        tag("*")(input).map(|(input, _)| (input, ()))
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
impl Value {
    fn parse(input: &str) -> IResult<&str, Self> {
        let (input, num) = complete::i64(input)?;

        match parser::star(input) {
            Ok((input, _star)) => Ok((input, Value::Star(num))),
            Err(_) => Ok((input, Value::Number(num))),
        }
    }
}

#[test]
fn parse_num_value() {
    let inp = "42";
    let exp = ("", Value::Number(42));
    assert_eq!(exp, Value::parse(inp).unwrap());
}

#[test]
fn parse_star_value() {
    let inp = "42*";
    let exp = ("", Value::Star(42));
    assert_eq!(exp, Value::parse(inp).unwrap());
}

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Number(n) => write!(f, "{}", n),
            Value::Star(n) => write!(f, "{}*", n),
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
enum Game {
    Value(Value),
    Moves(Vec<Game>, Vec<Game>),
}

impl Game {
    fn parse(input: &str) -> IResult<&str, Self> {
        match Value::parse(input) {
            Ok((input, val)) => Ok((input, Game::Value(val))),
            Err(_) => {
                let (input, _) = parser::lexeme(tag("{"))(input)?;
                let (input, left) =
                    separated_list0(parser::lexeme(tag(",")), parser::lexeme(Game::parse))(input)?;
                let (input, _) = parser::lexeme(tag("|"))(input)?;
                let (input, right) =
                    separated_list0(parser::lexeme(tag(",")), parser::lexeme(Game::parse))(input)?;
                let (input, _) = parser::lexeme(tag("}"))(input)?;
                Ok((input, Game::Moves(left, right)))
            }
        }
    }
}

#[test]
fn parse_value_game() {
    let inp = "42";
    let exp = ("", Game::Value(Value::Number(42)));
    assert_eq!(exp, Game::parse(inp).unwrap());
}

#[test]
fn parse_empty_game() {
    let inp = "{|}";
    let exp = ("", Game::Moves(vec![], vec![]));
    assert_eq!(exp, Game::parse(inp).unwrap());
}

#[test]
fn parse_left_game() {
    let inp = "{1,2*,3*|}";
    let exp = (
        "",
        Game::Moves(
            vec![
                Game::Value(Value::Number(1)),
                Game::Value(Value::Star(2)),
                Game::Value(Value::Star(3)),
            ],
            vec![],
        ),
    );
    assert_eq!(exp, Game::parse(inp).unwrap());
}

#[test]
fn parse_right_game() {
    let inp = "{|42*  ,  21}";
    let exp = (
        "",
        Game::Moves(
            vec![],
            vec![Game::Value(Value::Star(42)), Game::Value(Value::Number(21))],
        ),
    );
    assert_eq!(exp, Game::parse(inp).unwrap());
}

#[test]
fn parse_nested_game() {
    let inp = "{{1 , 2* | 3*, 4}| { 0 | 0 }}";
    let exp = (
        "",
        Game::Moves(
            vec![Game::Moves(
                vec![Game::Value(Value::Number(1)), Game::Value(Value::Star(2))],
                vec![Game::Value(Value::Star(3)), Game::Value(Value::Number(4))],
            )],
            vec![Game::Moves(
                vec![Game::Value(Value::Number(0))],
                vec![Game::Value(Value::Number(0))],
            )],
        ),
    );
    assert_eq!(exp, Game::parse(inp).unwrap());
}

impl Display for Game {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Game::Value(v) => write!(f, "{}", v),
            Game::Moves(l, r) => {
                write!(f, "{{")?;
                for (idx, m) in l.iter().enumerate() {
                    if idx != 0 {
                        write!(f, ",")?;
                    }
                    write!(f, "{}", *m)?;
                }
                write!(f, "|")?;
                for (idx, m) in r.iter().enumerate() {
                    if idx != 0 {
                        write!(f, ",")?;
                    }
                    write!(f, "{}", *m)?;
                }
                write!(f, "}}")?;
                Ok(())
            }
        }
    }
}

impl Game {
    fn canonical_form(self) -> Self {
        match self {
            Game::Value(n) => Game::Value(n),
            // Empty game
            Game::Moves(left, right) if left.is_empty() && right.is_empty() => {
                Game::Value(Value::Number(0))
            }
            _ => self,
        }
    }
}

fn main() {
    let g = Game::parse("{|}").unwrap().1;
    println!("{}", g);

    let g = g.canonical_form();
    println!("{}", g);
}
