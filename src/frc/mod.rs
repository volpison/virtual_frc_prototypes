//! The pure rules of the effect: which color a frame shows, how two colors average, and how
//! steadily frames reach the monitor.
//!
//! No window, no GPU and no other crate is used here, so every rule can be tested on its own.

pub mod color;
pub mod gradient;
pub mod pattern;
pub mod timing;
