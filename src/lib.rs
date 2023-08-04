#![cfg_attr(feature = "pedantic", deny(warnings))]

/// Simple graph implementation
pub mod graph;

/// Various numerical types
pub mod numeric;

/// Simple thread safe hashmap
pub mod rw_hash_map;

/// Short games - normal play
pub mod short;

/// Loopy games - normal play
pub mod loopy;

/// Nom parsing utilities
mod nom_utils;
