//! Combinatorial Game Theory framework.

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
        clippy::too_many_lines // Amazing heuristic
    )
)]

pub mod drawing;
pub mod graph;
pub mod grid;
pub mod loopy;
pub mod numeric;
pub mod rw_hash_map;
pub mod short;

mod display;
mod macros;
mod nom_utils;
