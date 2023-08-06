//! Display utilities

use std::fmt::{self, Display, Write};

#[allow(dead_code)]
fn sep(w: &mut impl Write, separator: &str, xs: &[impl Display]) -> fmt::Result {
    for (idx, v) in xs.iter().enumerate() {
        if idx != 0 {
            write!(w, "{}", separator)?;
        }
        write!(w, "{}", v)?;
    }
    Ok(())
}

#[allow(dead_code)]
#[inline]
pub fn commas(w: &mut impl Write, xs: &[impl Display]) -> fmt::Result {
    sep(w, ", ", xs)
}

fn bracket<W>(
    w: &mut W,
    left: &impl Display,
    right: &impl Display,
    middle: impl FnOnce(&mut W) -> fmt::Result,
) -> fmt::Result
where
    W: Write,
{
    write!(w, "{}", left)?;
    middle(w)?;
    write!(w, "{}", right)?;
    Ok(())
}

#[allow(dead_code)]
pub fn parens<W>(w: &mut W, middle: impl FnOnce(&mut W) -> fmt::Result) -> fmt::Result
where
    W: Write,
{
    bracket(w, &"(", &")", middle)
}

#[allow(dead_code)]
pub fn braces<W>(w: &mut W, middle: impl FnOnce(&mut W) -> fmt::Result) -> fmt::Result
where
    W: Write,
{
    bracket(w, &"{", &"}", middle)
}

#[allow(dead_code)]
pub fn brackets<W>(w: &mut W, middle: impl FnOnce(&mut W) -> fmt::Result) -> fmt::Result
where
    W: Write,
{
    bracket(w, &"[", &"]", middle)
}
