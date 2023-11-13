//! Loopy game graph vertex

use crate::{display, numeric::nimber::Nimber};
use std::fmt::Display;

/// Vertex set used during graph orbiting
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum UnresolvedVertex {
    /// Vertex that is equal to some finite nimber or a loop.
    Resolved(Vertex),

    /// Vertex that is yet to be resolved to a finite nimber or a loop.
    Unresolved,
}

/// Value of graph vertex - finite or infinite
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Vertex {
    /// Vertex that is equal to some finite nimber.
    Value(Nimber),

    /// Vertex that can move in a finite loop, or escape to one of the nimbers.
    Loop(Vec<Nimber>),
}

impl Display for Vertex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Value(n) => write!(f, "{}", n),
            Self::Loop(infs) => {
                write!(f, "∞")?;
                if !infs.is_empty() {
                    display::parens(f, |f| display::commas(f, infs))?;
                }
                Ok(())
            }
        }
    }
}

impl UnresolvedVertex {
    /// Check if vertex is a finite zero
    pub const fn is_zero(&self) -> bool {
        matches!(self, Self::Resolved(Vertex::Value(val)) if val.value() == 0)
    }
}
