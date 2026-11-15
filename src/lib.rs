//! The match owns rules; consumers receive only a player's observation.
pub mod assets;
pub mod game;
pub mod play;
mod saved_state;
#[cfg(feature = "tabletop")]
pub mod tabletop;
