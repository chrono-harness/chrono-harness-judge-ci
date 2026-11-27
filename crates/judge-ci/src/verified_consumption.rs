//! Registered preparation verification root.
#[cfg(verus_keep_ghost)]
#[path = "consumption.rs"]
pub mod rules;
#[cfg(verus_keep_ghost)]
#[path = "verified_text.rs"]
mod verified_text;
#[cfg(not(verus_keep_ghost))]
pub mod rules {
    pub use crate::consumption::matches_preparation;
}
pub use rules::*;
