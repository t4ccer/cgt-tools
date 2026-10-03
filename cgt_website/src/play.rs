//! Games against AIs that run in the browser. Each game implements [`game::Game`] and draws its
//! board, and everything else about playing is shared.

pub mod fjords;
pub mod game;
#[cfg(feature = "ssr")]
pub mod pages;
pub mod quelhas;
mod view;
#[cfg(feature = "hydrate")]
mod worker;
