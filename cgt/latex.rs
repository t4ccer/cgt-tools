//! LaTeX related utilities

use std::fmt::{self, Write};

struct LatexStreamWriter<'a, 'b> {
    formatter: &'a mut fmt::Formatter<'b>,
}

impl fmt::Write for LatexStreamWriter<'_, '_> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for ch in s.chars() {
            match ch {
                '{' => self.formatter.write_str(r"\{")?,
                '}' => self.formatter.write_str(r"\}")?,
                '|' => self.formatter.write_str(r" \mid ")?,
                other => self.formatter.write_char(other)?,
            }
        }
        Ok(())
    }
}

/// Wrapper to display a type inside LaTeX math mode
///
/// # Example
/// ```rust
/// use cgt::{latex::LatexMathEscape, short::partizan::canonical_form::CanonicalForm};
/// use std::str::FromStr;
///
/// let cf = CanonicalForm::from_str("{1|-1}").unwrap();
/// assert_eq!(cf.to_string(), "{1|-1}");
/// assert_eq!(LatexMathEscape(&cf).to_string(), r"\{1 \mid -1\}");
/// ```
#[derive(Debug, Clone, Copy)]
pub struct LatexMathEscape<'a, T>(pub &'a T);

impl<T> fmt::Display for LatexMathEscape<'_, T>
where
    T: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut writer = LatexStreamWriter { formatter: f };
        write!(writer, "{}", self.0)
    }
}
