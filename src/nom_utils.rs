//! Nom parsing utilities

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

/// Implement [std::str::FromStr] using nom parser. Type must have `parse` method implemented.
macro_rules! impl_from_str_via_nom {
    ($t: ident) => {
        impl std::str::FromStr for $t {
            type Err = nom::Err<nom::error::Error<String>>;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                use nom::error::ParseError;

                $t::parse(s).map(|(_, nus)| nus).map_err(|err| {
                    err.map(|err| {
                        nom::error::Error::from_error_kind(err.input.to_string(), err.code)
                    })
                })
            }
        }
    };
}
pub(crate) use impl_from_str_via_nom;
