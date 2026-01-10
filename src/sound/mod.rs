//! Audio feedback system for mini-games
//!
//! Provides sound effects for arcade-style gameplay with:
//! - Embedded assets (local-first, no external files)
//! - Graceful degradation when audio unavailable
//! - Volume control and mute toggle
//! - Per-user configuration persistence

mod assets;
mod config;
mod effects;
mod error;
mod manager;

pub use config::SoundConfig;
pub use effects::SoundEffect;
pub use error::SoundError;
pub use manager::SoundManager;
