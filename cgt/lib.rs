//! Combinatorial Game Theory framework.
//!
//! System supports short [partizan](crate::short::partizan::games) and
//! [impartial](crate::short::impartial::games) games, displaying games as SVG images,
//! computing canonical form of a game value and
//! [calculations on canonical forms](crate::short::partizan::canonical_form::CanonicalForm)

#![warn(missing_docs)]

pub mod drawing;
pub mod genetic_algorithm;
pub mod graph;
pub mod grid;
pub mod has;
pub mod loopy;
pub mod misere;
pub mod numeric;
pub mod parsing;
pub mod short;

mod display;
