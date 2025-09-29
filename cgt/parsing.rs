//! Parsing utilities

// TODO: Fancy errors

/// Implement [`std::str::FromStr`] using parser. Type must have `parse` method implemented.
macro_rules! impl_from_str_via_parser {
    ($t: ident) => {
        impl std::str::FromStr for $t {
            type Err = &'static str;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                match $t::parse($crate::parsing::Parser::new(s)) {
                    Some((p, result)) if p.input.is_empty() => Ok(result),
                    Some(_) => Err("Parse error: leftover input"),
                    None => Err("Parse error: parser failed"),
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

                $t::from_str(&String::deserialize(deserializer)?).map_err(serde::de::Error::custom)
            }
        }
    };
}
pub(crate) use impl_from_str_via_parser;

#[must_use]
#[derive(Debug, Clone, Copy)]
/// `const`-capable string parser
pub struct Parser<'s> {
    // TODO: Track location since construction with new()
    /// Remaining unparsed input
    pub input: &'s str,
}

macro_rules! try_option {
    ($e:expr) => {
        match $e {
            Some(v) => v,
            None => return None,
        }
    };
}
pub(crate) use try_option;

macro_rules! lexeme {
    ($p:expr, $f:expr) => {{
        let p = $p.trim_whitespace();
        match $f(p) {
            None => None,
            Some((p, val)) => {
                let p = p.trim_whitespace();
                Some((p, val))
            }
        }
    }};
}
pub(crate) use lexeme;

impl<'s> Parser<'s> {
    /// Create new parser marking the beginning of the input
    pub const fn new(input: &'s str) -> Parser<'s> {
        Parser { input }
    }

    /// Remove whitespace from the beginning of the input
    pub const fn trim_whitespace(self) -> Parser<'s> {
        let mut bs = self.input.as_bytes();
        loop {
            match bs {
                [b'\t' | b'\n' | b'\r' | b' ', rest @ ..] => bs = rest,
                _ => {
                    return Parser {
                        input: {
                            // const-hack
                            match core::str::from_utf8(bs) {
                                Ok(input) => input,
                                Err(_) => unreachable!(),
                            }
                        },
                    };
                }
            }
        }
    }

    /// Parse one ascii char if input is non-empty
    pub const fn parse_any_ascii_char(self) -> Option<(Parser<'s>, char)> {
        match self.input.as_bytes() {
            [b, rest @ ..] if b.is_ascii() => Some((
                Parser {
                    // const-hack
                    input: match core::str::from_utf8(rest) {
                        Ok(input) => input,
                        Err(_) => unreachable!(),
                    },
                },
                *b as char,
            )),
            _ => None,
        }
    }

    /// Parse one ascii char if input is non-empty and it matches the `expected`
    pub const fn parse_ascii_char(self, expected: char) -> Option<Parser<'s>> {
        match self.parse_any_ascii_char() {
            Some((p, c)) if c == expected => Some(p),
            _ => None,
        }
    }

    /// Parse signed number
    pub const fn parse_i64(self) -> Option<(Parser<'s>, i64)> {
        let mut bs = self.input.as_bytes();

        let minus = match bs {
            [b'-', rest @ ..] => {
                bs = rest;
                true
            }
            _ => false,
        };

        let mut parsed_anything = false;
        let mut acc: i64 = 0;

        loop {
            match bs {
                [
                    b @ (b'0' | b'1' | b'2' | b'3' | b'4' | b'5' | b'6' | b'7' | b'8' | b'9'),
                    rest @ ..,
                ] => {
                    parsed_anything = true;
                    match acc.checked_mul(10) {
                        Some(a) => acc = a,
                        None => {
                            return None;
                        }
                    }
                    match acc.checked_add((*b - b'0') as i64) {
                        Some(a) => acc = a,
                        None => {
                            return None;
                        }
                    }

                    bs = rest;
                }
                _ => {
                    if !parsed_anything {
                        return None;
                    }

                    if minus {
                        acc = -acc;
                    }

                    return Some((
                        Parser {
                            // const-hack
                            input: match core::str::from_utf8(bs) {
                                Ok(input) => input,
                                Err(_) => unreachable!(),
                            },
                        },
                        acc,
                    ));
                }
            }
        }
    }

    /// Parse unsigned number
    pub const fn parse_u32(self) -> Option<(Parser<'s>, u32)> {
        let mut bs = self.input.as_bytes();

        let mut parsed_anything = false;
        let mut acc: u32 = 0;

        loop {
            match bs {
                [
                    b @ (b'0' | b'1' | b'2' | b'3' | b'4' | b'5' | b'6' | b'7' | b'8' | b'9'),
                    rest @ ..,
                ] => {
                    parsed_anything = true;
                    match acc.checked_mul(10) {
                        Some(a) => acc = a,
                        None => {
                            return None;
                        }
                    }
                    match acc.checked_add((*b - b'0') as u32) {
                        Some(a) => acc = a,
                        None => {
                            return None;
                        }
                    }

                    bs = rest;
                }
                _ => {
                    if !parsed_anything {
                        return None;
                    }

                    return Some((
                        Parser {
                            // const-hack
                            input: match core::str::from_utf8(bs) {
                                Ok(input) => input,
                                Err(_) => unreachable!(),
                            },
                        },
                        acc,
                    ));
                }
            }
        }
    }
}
