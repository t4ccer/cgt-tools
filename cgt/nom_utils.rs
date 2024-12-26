//! Nom parsing utilities

use nom::{character::complete::multispace0, IResult, Parser};

pub fn lexeme<'input, Output, Error, F>(
    mut inner: F,
) -> impl FnMut(&'input str) -> IResult<&'input str, Output, Error>
where
    F: Parser<&'input str, Output, Error>,
    Error: nom::error::ParseError<&'input str>,
{
    move |input: &str| {
        let (input, _ws) = multispace0(input)?;
        let (input, res) = inner.parse(input)?;
        let (input, _ws) = multispace0(input)?;
        Ok((input, res))
    }
}

// TODO: Fancy errors

/// Implement [`std::str::FromStr`] using nom parser. Type must have `parse` method implemented.
macro_rules! impl_from_str_via_nom {
    ($t: ident) => {
        impl std::str::FromStr for $t {
            type Err = &'static str;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                match $t::parse(s) {
                    Ok((input, result)) if input.is_empty() => Ok(result),
                    Ok(_) => Err("Parse error: leftover input"),
                    Err(_) => Err("Parse error: parser failed"),
                }
            }
        }

        #[cfg(feature = "serde")]
        impl serde::Serialize for $t {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                serializer.serialize_str(&self.to_string())
            }
        }

        #[cfg(feature = "serde")]
        impl<'de> serde::Deserialize<'de> for $t {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                use std::str::FromStr;

                Ok($t::from_str(&String::deserialize(deserializer)?).unwrap())
            }
        }
    };
}
pub(crate) use impl_from_str_via_nom;
