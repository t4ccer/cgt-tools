#[cfg(feature = "rand")]
pub type RandomState = ahash::RandomState;

#[cfg(not(feature = "rand"))]
pub type RandomState = std::collections::hash_map::RandomState;
