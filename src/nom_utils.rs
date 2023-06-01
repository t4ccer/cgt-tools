use nom::{character::complete::multispace0, IResult, Parser};

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
