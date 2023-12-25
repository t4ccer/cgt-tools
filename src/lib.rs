//! Combinatorial Game Theory framework.
//!
//! System supports short [partizan](crate::short::partizan::games) and
//! [impartial](crate::short::impartial::games) games, displaying games as SVG images,
//! computing canonical form of a game value and
//! [calculations on canonical forms](crate::short::partizan::canonical_form::CanonicalForm)

#![warn(missing_docs)]
#![cfg_attr(
    feature = "cargo-clippy",
    warn(
        clippy::all,
        clippy::nursery,
        clippy::pedantic,
    ),
    allow(
        clippy::new_without_default,
        clippy::similar_names,
        clippy::must_use_candidate,
        clippy::cast_lossless,
        clippy::cast_possible_wrap,
        clippy::cast_sign_loss,
        clippy::cast_possible_truncation,
        clippy::module_name_repetitions,
        clippy::uninlined_format_args, // LSP rename cannot handle inlined variables :(
        clippy::too_many_lines, // Amazing heuristic
        clippy::needless_update,
        clippy::cast_precision_loss
    )
)]

pub mod drawing;
pub mod genetic_algorithm;
pub mod graph;
pub mod grid;
pub mod loopy;
pub mod numeric;
pub mod short;

mod display;
mod macros;
mod nom_utils;
