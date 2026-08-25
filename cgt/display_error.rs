//! Utilities for displaying errors with traces

use std::fmt::Display;

struct TraceError<E>(E);

impl<E> std::fmt::Display for TraceError<E>
where
    E: std::error::Error,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{}", self.0)?;

        let mut depth = 0;
        let mut current = self.0.source();
        while let Some(source) = current {
            if depth == 0 {
                writeln!(f, "Caused by:")?;
            }
            writeln!(f, "  {depth}: {source}")?;
            depth += 1;
            current = source.source();
        }

        Ok(())
    }
}

/// Extension trait to display errors with source chain
pub trait DisplayError {
    /// Display error with source chain
    fn display_error(&self) -> impl Display + '_;
}

impl<E> DisplayError for E
where
    E: std::error::Error,
{
    fn display_error(&self) -> impl Display + '_ {
        TraceError(self)
    }
}
