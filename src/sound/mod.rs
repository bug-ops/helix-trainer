//! Audio feedback system for mini-games
//!
//! Provides sound effects for arcade-style gameplay with:
//! - Embedded assets (local-first, no external files)
//! - Graceful degradation when audio unavailable
//! - Volume control and mute toggle
//! - Per-user configuration persistence
//!
//! When compiled without the `audio` feature (e.g., musl builds),
//! this module provides no-op stubs that maintain the same API.

#[cfg(feature = "audio")]
mod assets;
#[cfg(feature = "audio")]
mod effects;
#[cfg(feature = "audio")]
mod error;
#[cfg(feature = "audio")]
mod manager;

// Config is always needed (stored in profile)
mod config;

pub use config::SoundConfig;

#[cfg(feature = "audio")]
pub use effects::SoundEffect;
#[cfg(feature = "audio")]
pub use error::SoundError;
#[cfg(feature = "audio")]
pub use manager::SoundManager;

// No-op stubs when audio is disabled
#[cfg(not(feature = "audio"))]
mod stub {
    use super::SoundConfig;

    /// Sound effect types (no-op when audio disabled)
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum SoundEffect {
        /// Success chime on scenario completion
        ScenarioComplete,
        /// Failure buzz on timeout
        ScenarioFailed,
        /// Rising tone when multiplier increases
        MultiplierUp,
        /// Fanfare when difficulty level increases
        LevelUp,
        /// Alert sound when losing a life
        LifeLost,
        /// Jingle when game ends
        GameOver,
        /// Tick sound for 3-2-1 countdown
        Countdown,
        /// Warning when <25% time remaining
        TimerWarning,
    }

    /// Sound error (never occurs when audio disabled)
    #[derive(Debug, Clone)]
    pub struct SoundError;

    impl std::fmt::Display for SoundError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "Audio not available (compiled without audio feature)")
        }
    }

    impl std::error::Error for SoundError {}

    /// No-op sound manager when audio is disabled
    #[derive(Debug, Clone)]
    pub struct SoundManager {
        config: SoundConfig,
    }

    impl SoundManager {
        pub fn new(config: SoundConfig) -> Self {
            Self { config }
        }

        pub fn try_init(&mut self) -> Result<(), SoundError> {
            // No-op: audio not available
            Ok(())
        }

        pub fn play(&self, _effect: SoundEffect) {
            // No-op: audio not available
        }

        pub fn set_enabled(&mut self, enabled: bool) {
            self.config.enabled = enabled;
        }

        pub fn is_enabled(&self) -> bool {
            self.config.enabled
        }

        pub fn toggle(&mut self) -> bool {
            self.config.enabled = !self.config.enabled;
            self.config.enabled
        }

        pub fn set_volume(&mut self, volume: f32) {
            self.config.volume = volume.clamp(0.0, 1.0);
        }

        pub fn volume(&self) -> f32 {
            self.config.volume
        }

        pub fn config(&self) -> &SoundConfig {
            &self.config
        }

        pub fn is_available(&self) -> bool {
            false
        }
    }

    impl Default for SoundManager {
        fn default() -> Self {
            Self::new(SoundConfig::default())
        }
    }
}

#[cfg(not(feature = "audio"))]
pub use stub::{SoundEffect, SoundError, SoundManager};
