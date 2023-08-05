use std::fmt::{self, Display, Write};

pub fn sep(w: &mut impl Write, separator: &str, xs: &[impl Display]) -> fmt::Result {
    for (idx, v) in xs.iter().enumerate() {
        if idx != 0 {
            write!(w, "{}", separator)?;
        }
        write!(w, "{}", v)?;
    }
    Ok(())
}
