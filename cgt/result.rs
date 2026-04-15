//! Utilities for working with the `Result` type

use std::convert::Infallible;

/// Uninhabited types
pub trait Void {
    /// Since `Self` is uninhabited, it can be converted into anything. This must not panic.
    fn absurd<T>(self) -> T;
}

impl Void for Infallible {
    fn absurd<T>(self) -> T {
        match self {}
    }
}

/// Types that allow for failure to happen, but error type is uninhabited
pub trait UnwrapInfallible {
    /// Type of a successful operation
    type R;

    /// Extract the result of the operation. This must not panic.
    fn unwrap_infallible(self) -> Self::R;
}

/// # Example
/// ```
/// use cgt::result::UnwrapInfallible;
/// use std::str::FromStr;
///
/// let s: String = String::from_str("Hello, World!").unwrap_infallible();
/// ```
impl<T, E> UnwrapInfallible for Result<T, E>
where
    E: Void,
{
    type R = T;

    fn unwrap_infallible(self) -> Self::R {
        match self {
            Ok(res) => res,
            Err(err) => err.absurd(),
        }
    }
}
