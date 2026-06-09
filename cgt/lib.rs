//! Combinatorial Game Theory framework.
//!
//! System supports short [partizan](crate::short::partizan::games) and
//! [impartial](crate::short::impartial::games) games, displaying games as SVG images,
//! computing canonical form of a game value and
//! [calculations on canonical forms](crate::short::partizan::canonical_form::CanonicalForm)

#![warn(missing_docs)]
#![warn(missing_debug_implementations)]
#![warn(missing_copy_implementations)]

pub mod bit_vec;
pub mod drawing;
pub mod genetic_algorithm;
pub mod graph;
pub mod grid;
pub mod has;
pub mod latex;
pub mod loopy;
pub mod misere;
pub mod numeric;
pub mod parsing;
pub mod poset;
pub mod result;
pub mod short;
pub mod total;

mod atomic_enum;
mod display;
mod ref_wrapper;
