//! Parsing utilities

use std::fmt::{self, Display};

/// Implement [`std::str::FromStr`] using parser. Type must have `parse` method implemented.
macro_rules! impl_from_str_via_parser {
    ($t: ident) => {
        impl std::str::FromStr for $t {
            type Err = $crate::parsing::ParseError;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                let (p, result) = $t::parse($crate::parsing::Parser::new(s))?;
                p.expect_end_of_input()?;
                Ok(result)
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

/// Place in the parsed input
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InputLocation {
    /// Line (starting with 0)
    pub line: u32,

    /// Column (starting with 0)
    pub column: u32,
}

impl InputLocation {
    /// Beginning of the input
    pub const START: InputLocation = InputLocation { line: 0, column: 0 };

    /// Check whether the location is further in the input than the `other` one
    #[must_use]
    pub const fn is_further_than(&self, other: &InputLocation) -> bool {
        self.line > other.line || (self.line == other.line && self.column > other.column)
    }
}

impl Display for InputLocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}, column {}", self.line + 1, self.column + 1)
    }
}

/// Fragment of the input, from `start` (inclusive) to `end` (exclusive)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InputSpan {
    /// First character of the fragment
    pub start: InputLocation,

    /// First character after the fragment
    pub end: InputLocation,
}

impl InputSpan {
    /// Span over `length` characters of a single line, starting at `start`
    #[must_use]
    pub const fn new(start: InputLocation, length: u32) -> InputSpan {
        InputSpan {
            start,
            end: InputLocation {
                line: start.line,
                column: start.column + length,
            },
        }
    }
}

// HACK: std::error::Error::provide is gated behind error_generic_member_access feature
// Once it gets stabilized remove that trait
/// Error that knows the fragment of the input that caused it
pub trait SyntaxError {
    /// Fragment of the input that caused the error
    fn span(&self) -> Option<InputSpan>;
}

/// Input that parser expected to see
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Expected {
    /// A particular character
    Char(char),

    /// Any ascii character
    AsciiChar,

    /// A class of inputs, described in a human readable way, e.g. `"game value"`
    Description(&'static str),

    /// No more input
    EndOfInput,
}

impl Display for Expected {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Expected::Char(c) => write!(f, "`{c}`"),
            Expected::AsciiChar => write!(f, "an ascii character"),
            Expected::Description(description) => write!(f, "{description}"),
            Expected::EndOfInput => write!(f, "end of input"),
        }
    }
}

/// Parser failure
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ParseErrorReason {
    /// Input ended before the parser could finish
    UnexpectedEndOfInput {
        /// Input that the parser expected instead
        expected: Expected,
    },

    /// Parser got input that it cannot use at that position
    Unexpected {
        /// Input that the parser expected
        expected: Expected,

        /// Character that the parser got
        got: char,
    },

    /// Number literal does not fit in the type that it is parsed into
    NumberTooLarge {
        /// Name of the type that the number is parsed into
        type_name: &'static str,
    },

    /// Input is well formed but does not denote a valid value
    InvalidValue {
        /// Human readable explanation why the value is invalid
        reason: &'static str,
    },
}

impl std::error::Error for ParseErrorReason {}

impl Display for ParseErrorReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseErrorReason::UnexpectedEndOfInput { expected } => {
                write!(f, "unexpected end of input, expected {expected}")
            }
            ParseErrorReason::Unexpected { expected, got } => {
                write!(f, "expected {expected}, got `{got}`")
            }
            ParseErrorReason::NumberTooLarge { type_name } => {
                write!(f, "number does not fit in `{type_name}`")
            }
            ParseErrorReason::InvalidValue { reason } => write!(f, "invalid value: {reason}"),
        }
    }
}

/// Error that happened during parsing, together with the place in the input where it happened
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ParseError<Reason = ParseErrorReason> {
    /// Why parsing failed
    pub reason: Reason,

    /// Fragment of the input that could not be parsed
    pub span: InputSpan,
}

impl ParseError {
    /// Check whether the error only means that the parser should try a different alternative,
    /// rather than that the input is malformed no matter what is expected there
    #[must_use]
    pub const fn is_recoverable(&self) -> bool {
        matches!(
            self.reason,
            ParseErrorReason::UnexpectedEndOfInput { .. } | ParseErrorReason::Unexpected { .. }
        )
    }

    /// Replace what the error says was expected, keeping the span and everything else intact
    #[must_use]
    pub const fn expecting(self, expected: Expected) -> ParseError {
        let reason = match self.reason {
            ParseErrorReason::UnexpectedEndOfInput { .. } => {
                ParseErrorReason::UnexpectedEndOfInput { expected }
            }
            ParseErrorReason::Unexpected { got, .. } => {
                ParseErrorReason::Unexpected { expected, got }
            }
            reason => reason,
        };

        ParseError {
            reason,
            span: self.span,
        }
    }

    const fn at(bs: &[u8], location: InputLocation, expected: Expected) -> ParseError {
        let reason = match peek_char(bs) {
            None => ParseErrorReason::UnexpectedEndOfInput { expected },
            Some(got) => ParseErrorReason::Unexpected { expected, got },
        };

        ParseError {
            reason,
            span: InputSpan::new(location, 1),
        }
    }
}

impl<Reason> SyntaxError for ParseError<Reason> {
    fn span(&self) -> Option<InputSpan> {
        Some(self.span)
    }
}

impl<Reason> Display for ParseError<Reason> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "parse error at {}", self.span.start)
    }
}

impl<Reason> std::error::Error for ParseError<Reason>
where
    Reason: std::error::Error + 'static,
{
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.reason)
    }
}

#[must_use]
#[derive(Debug, Clone, Copy)]
/// `const`-capable string parser
pub struct Parser<'s> {
    /// Remaining unparsed input
    pub input: &'s str,

    /// Place in the input that the parser is at
    pub location: InputLocation,
}

macro_rules! try_parse {
    ($e:expr) => {
        match $e {
            Ok(v) => v,
            Err(err) => return Err(err),
        }
    };
}
pub(crate) use try_parse;

macro_rules! lexeme {
    ($p:expr, $f:expr) => {{
        let p = $p.trim_whitespace();
        match $f(p) {
            Err(err) => Err(err),
            Ok((p, val)) => {
                let p = p.trim_whitespace();
                Ok((p, val))
            }
        }
    }};
}
pub(crate) use lexeme;

/// Decode the first character of the input
const fn peek_char(bs: &[u8]) -> Option<char> {
    let code = match bs {
        [b, ..] if *b < 0x80 => *b as u32,
        [b, c1, ..] if *b & 0xE0 == 0xC0 => ((*b as u32 & 0x1F) << 6) | (*c1 as u32 & 0x3F),
        [b, c1, c2, ..] if *b & 0xF0 == 0xE0 => {
            ((*b as u32 & 0x0F) << 12) | ((*c1 as u32 & 0x3F) << 6) | (*c2 as u32 & 0x3F)
        }
        [b, c1, c2, c3, ..] if *b & 0xF8 == 0xF0 => {
            ((*b as u32 & 0x07) << 18)
                | ((*c1 as u32 & 0x3F) << 12)
                | ((*c2 as u32 & 0x3F) << 6)
                | (*c3 as u32 & 0x3F)
        }
        // Empty input, or a truncated character which cannot happen as parser input is always a
        // valid utf-8 string
        _ => return None,
    };

    char::from_u32(code)
}

macro_rules! mk_number_parser {
    ($name:ident, $ty:ty $(, $minus:ident)?) => {
        /// Parse number
        ///
        /// # Errors
        /// - Input does not start with a number
        /// - Number does not fit in the type
        pub const fn $name(mut self) -> Result<(Parser<'s>, $ty), ParseError> {
            let start = self.location;
            let mut bs = self.input.as_bytes();

            $(let $minus = match bs {
                [b'-', rest @ ..] => {
                    self.location.column += 1;
                    bs = rest;
                    true
                }
                _ => false,
            };)?

            let mut parsed_anything = false;
            let mut acc: $ty = 0;

            loop {
                match bs {
                    [
                        b @ (b'0' | b'1' | b'2' | b'3' | b'4' | b'5' | b'6' | b'7' | b'8' | b'9'),
                        rest @ ..,
                    ] => {
                        self.location.column += 1;
                        parsed_anything = true;
                        match acc.checked_mul(10) {
                            Some(a) => acc = a,
                            None => {
                                return Err(ParseError {
                                    reason: ParseErrorReason::NumberTooLarge {
                                        type_name: stringify!($ty),
                                    },
                                    span: InputSpan { start, end: self.location },
                                });
                            }
                        }
                        match acc.checked_add((*b - b'0') as $ty) {
                            Some(a) => acc = a,
                            None => {
                                return Err(ParseError {
                                    reason: ParseErrorReason::NumberTooLarge {
                                        type_name: stringify!($ty),
                                    },
                                    span: InputSpan { start, end: self.location },
                                });
                            }
                        }

                        bs = rest;
                    }
                    _ => {
                        if !parsed_anything {
                            return Err(ParseError::at(
                                bs,
                                self.location,
                                Expected::Description("a number"),
                            ));
                        }

                        $(if $minus {
                            acc = -acc;
                        })?

                        return Ok((
                            Parser {
                                // const-hack
                                input: match core::str::from_utf8(bs) {
                                    Ok(input) => input,
                                    Err(_) => unreachable!(),
                                },
                                location: self.location,
                            },
                            acc,
                        ));
                    }
                }
            }
        }
    };
}

impl<'s> Parser<'s> {
    /// Create new parser marking the beginning of the input
    pub const fn new(input: &'s str) -> Parser<'s> {
        Parser {
            input,
            location: InputLocation::START,
        }
    }

    /// Create an error at the current position, saying that `expected` was expected there
    #[must_use]
    pub const fn expected(self, expected: Expected) -> ParseError {
        ParseError::at(self.input.as_bytes(), self.location, expected)
    }

    /// Create an error at the current position, saying that the input, although well formed, does
    /// not denote a valid value
    #[must_use]
    pub const fn invalid_value(self, reason: &'static str) -> ParseError {
        ParseError {
            reason: ParseErrorReason::InvalidValue { reason },
            span: InputSpan::new(self.location, 1),
        }
    }

    /// Remove whitespace from the beginning of the input
    pub const fn trim_whitespace(mut self) -> Parser<'s> {
        let mut bs = self.input.as_bytes();
        loop {
            match bs {
                [b'\t' | b'\r' | b' ', rest @ ..] => {
                    self.location.column += 1;
                    bs = rest
                }
                [b'\n', rest @ ..] => {
                    self.location.column = 0;
                    self.location.line += 1;
                    bs = rest
                }
                _ => {
                    return Parser {
                        input: {
                            // const-hack
                            match core::str::from_utf8(bs) {
                                Ok(input) => input,
                                Err(_) => unreachable!(),
                            }
                        },
                        location: self.location,
                    };
                }
            }
        }
    }

    /// Parse one ascii char if input is non-empty
    ///
    /// # Errors
    /// - Input is empty
    /// - Next character is not an ascii one
    pub const fn parse_any_ascii_char(self) -> Result<(Parser<'s>, char), ParseError> {
        match self.input.as_bytes() {
            [b'\n', rest @ ..] => Ok((
                Parser {
                    // const-hack
                    input: match core::str::from_utf8(rest) {
                        Ok(input) => input,
                        Err(_) => unreachable!(),
                    },

                    location: InputLocation {
                        line: self.location.line + 1,
                        column: 0,
                    },
                },
                '\n',
            )),
            [b, rest @ ..] if b.is_ascii() => Ok((
                Parser {
                    // const-hack
                    input: match core::str::from_utf8(rest) {
                        Ok(input) => input,
                        Err(_) => unreachable!(),
                    },

                    location: InputLocation {
                        line: self.location.line,
                        column: self.location.column + 1,
                    },
                },
                *b as char,
            )),
            _ => Err(self.expected(Expected::AsciiChar)),
        }
    }

    /// Parse one ascii char if input is non-empty and it matches the `expected`
    ///
    /// # Errors
    /// - Input is empty
    /// - Next character is not the expected one
    pub const fn parse_ascii_char(self, expected: char) -> Result<Parser<'s>, ParseError> {
        match self.parse_any_ascii_char() {
            Ok((p, c)) if c == expected => Ok(p),
            Ok(_) => Err(self.expected(Expected::Char(expected))),
            Err(err) => Err(err.expecting(Expected::Char(expected))),
        }
    }

    /// Ensure that there is nothing left to parse
    ///
    /// # Errors
    /// - Input is not fully consumed
    pub fn expect_end_of_input(self) -> Result<(), ParseError> {
        let Some(got) = peek_char(self.input.as_bytes()) else {
            return Ok(());
        };

        let leftover = self
            .input
            .lines()
            .next()
            .map_or(1, |line| line.chars().count() as u32);

        Err(ParseError {
            reason: ParseErrorReason::Unexpected {
                expected: Expected::EndOfInput,
                got,
            },
            span: InputSpan::new(self.location, leftover),
        })
    }

    mk_number_parser!(parse_u8, u8);
    mk_number_parser!(parse_u16, u16);
    mk_number_parser!(parse_u32, u32);
    mk_number_parser!(parse_u64, u64);

    mk_number_parser!(parse_i8, i8, minus);
    mk_number_parser!(parse_i16, i16, minus);
    mk_number_parser!(parse_i32, i32, minus);
    mk_number_parser!(parse_i64, i64, minus);
}
