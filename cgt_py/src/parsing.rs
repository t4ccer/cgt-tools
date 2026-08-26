use cgt::{display_error::DisplayError, parsing::SyntaxError};
use pyo3::{
    PyErr,
    exceptions::{PySyntaxError, PyValueError},
};
use std::error::Error;

pub fn parse_error<E>(err: &E, source: &str) -> PyErr
where
    E: Error + SyntaxError + 'static,
{
    let Some(span) = err.span() else {
        return PyValueError::new_err(err.display_error().to_string());
    };

    let message = err.display_error().to_string();
    let line = source.lines().nth(span.start.line as usize).unwrap_or("");

    // Python counts lines and columns starting with one
    PyErr::new::<PySyntaxError, _>((
        message,
        (
            "<input>",
            span.start.line + 1,
            span.start.column + 1,
            line.to_owned(),
            span.end.line + 1,
            span.end.column + 1,
        ),
    ))
}
