//! Sound manager implementation

use std::collections::HashMap;
use std::io::Cursor;
use std::sync::Arc;

use rodio::mixer::Mixer;
use rodio::{Decoder, OutputStream, OutputStreamBuilder, Sink};

use super::assets;
use super::config::SoundConfig;
use super::effects::SoundEffect;
use super::error::SoundError;

/// Sound manager for playing audio effects
///
/// Handles audio device initialization and playback with graceful degradation.
/// If audio is unavailable, all play operations become no-ops.
pub struct SoundManager {
    /// Audio output stream (kept alive for playback)
    _stream: Option<OutputStream>,
    /// Mixer for creating sinks
    mixer: Option<Mixer>,
    /// Current configuration
    config: SoundConfig,
    /// Pre-loaded sound data
    sounds: HashMap<SoundEffect, Arc<[u8]>>,
    /// Whether audio system initialized successfully
    initialized: bool,
}

impl SoundManager {
    /// Create a new sound manager with the given configuration
    ///
    /// Audio device initialization is deferred to first use.
    pub fn new(config: SoundConfig) -> Self {
        let sounds = Self::load_embedded_sounds();
        Self {
            _stream: None,
            mixer: None,
            config,
            sounds,
            initialized: false,
        }
    }

    /// Try to initialize the audio device
    ///
    /// Call this early (e.g., at app startup) to detect audio availability.
    /// If this fails, the manager will operate in silent mode.
    pub fn try_init(&mut self) -> Result<(), SoundError> {
        if self.initialized {
            return Ok(());
        }

        match OutputStreamBuilder::open_default_stream() {
            Ok(stream) => {
                let mixer = stream.mixer().clone();
                self._stream = Some(stream);
                self.mixer = Some(mixer);
                self.initialized = true;
                tracing::info!("Audio system initialized successfully");
                Ok(())
            }
            Err(e) => {
                tracing::warn!(
                    "Audio initialization failed: {}. Running in silent mode.",
                    e
                );
                self.initialized = false;
                Err(SoundError::StreamError(e.to_string()))
            }
        }
    }

    /// Play a sound effect
    ///
    /// Does nothing if sound is disabled, audio unavailable, or effect not found.
    pub fn play(&self, effect: SoundEffect) {
        if !self.config.enabled || !self.initialized {
            return;
        }

        let Some(mixer) = &self.mixer else {
            return;
        };

        let Some(data) = self.sounds.get(&effect) else {
            tracing::warn!("Sound effect {:?} not found", effect);
            return;
        };

        // Create cursor from data
        let cursor = Cursor::new(data.clone());

        // Decode and play
        match Decoder::new(cursor) {
            Ok(source) => {
                let sink = Sink::connect_new(mixer);
                sink.set_volume(self.config.volume);
                sink.append(source);
                sink.detach(); // Play to completion without blocking
            }
            Err(e) => {
                tracing::debug!("Failed to decode sound {:?}: {}", effect, e);
            }
        }
    }

    /// Play a sound effect only if enabled
    ///
    /// Convenience method that checks enabled state before playing.
    #[inline]
    pub fn play_if_enabled(&self, effect: SoundEffect) {
        self.play(effect);
    }

    /// Set master volume (clamped to 0.0-1.0)
    pub fn set_volume(&mut self, volume: f32) {
        self.config.volume = if volume.is_finite() {
            volume.clamp(0.0, 1.0)
        } else {
            SoundConfig::DEFAULT_VOLUME
        };
    }

    /// Set enabled state
    pub fn set_enabled(&mut self, enabled: bool) {
        self.config.enabled = enabled;
    }

    /// Toggle sound on/off, returns new state
    pub fn toggle(&mut self) -> bool {
        self.config.enabled = !self.config.enabled;
        self.config.enabled
    }

    /// Get current configuration
    pub fn config(&self) -> &SoundConfig {
        &self.config
    }

    /// Check if audio is available
    pub fn is_available(&self) -> bool {
        self.initialized
    }

    /// Load embedded sounds into memory
    fn load_embedded_sounds() -> HashMap<SoundEffect, Arc<[u8]>> {
        let mut sounds = HashMap::new();
        sounds.insert(
            SoundEffect::ScenarioComplete,
            Arc::from(assets::SOUND_COMPLETE),
        );
        sounds.insert(SoundEffect::ScenarioFailed, Arc::from(assets::SOUND_FAILED));
        sounds.insert(
            SoundEffect::MultiplierUp,
            Arc::from(assets::SOUND_MULTIPLIER),
        );
        sounds.insert(SoundEffect::LevelUp, Arc::from(assets::SOUND_LEVELUP));
        sounds.insert(SoundEffect::LifeLost, Arc::from(assets::SOUND_LIFELOST));
        sounds.insert(SoundEffect::GameOver, Arc::from(assets::SOUND_GAMEOVER));
        sounds.insert(SoundEffect::Countdown, Arc::from(assets::SOUND_COUNTDOWN));
        sounds.insert(SoundEffect::TimerWarning, Arc::from(assets::SOUND_WARNING));
        sounds
    }
}

impl Default for SoundManager {
    fn default() -> Self {
        Self::new(SoundConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sound_manager_toggle() {
        let mut manager = SoundManager::new(SoundConfig::default());

        // Default is enabled
        assert!(manager.config().enabled);

        // Toggle off
        let new_state = manager.toggle();
        assert!(!new_state);
        assert!(!manager.config().enabled);

        // Toggle on
        let new_state = manager.toggle();
        assert!(new_state);
        assert!(manager.config().enabled);
    }

    #[test]
    fn test_sound_manager_no_init_silent() {
        let manager = SoundManager::new(SoundConfig::default());

        // Not initialized, play should be no-op (no panic)
        manager.play(SoundEffect::ScenarioComplete);

        // is_available should return false
        assert!(!manager.is_available());
    }

    #[test]
    fn test_sound_manager_set_volume() {
        let mut manager = SoundManager::new(SoundConfig::default());

        // Normal volume
        manager.set_volume(0.5);
        assert!((manager.config().volume - 0.5).abs() < f32::EPSILON);

        // Clamp above max
        manager.set_volume(1.5);
        assert!((manager.config().volume - 1.0).abs() < f32::EPSILON);

        // Clamp below min
        manager.set_volume(-0.5);
        assert!((manager.config().volume - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_sound_manager_set_enabled() {
        let mut manager = SoundManager::new(SoundConfig::default());

        manager.set_enabled(false);
        assert!(!manager.config().enabled);

        manager.set_enabled(true);
        assert!(manager.config().enabled);
    }

    #[test]
    fn test_sound_manager_default() {
        let manager = SoundManager::default();
        assert!((manager.config().volume - SoundConfig::DEFAULT_VOLUME).abs() < f32::EPSILON);
        assert!(manager.config().enabled);
        assert!(!manager.is_available()); // Not initialized
    }

    #[test]
    fn test_sound_manager_load_embedded_sounds() {
        let manager = SoundManager::new(SoundConfig::default());

        // All 8 sound effects should be loaded
        assert_eq!(manager.sounds.len(), 8);

        // Each should be non-empty
        assert!(!manager.sounds[&SoundEffect::ScenarioComplete].is_empty());
        assert!(!manager.sounds[&SoundEffect::ScenarioFailed].is_empty());
        assert!(!manager.sounds[&SoundEffect::MultiplierUp].is_empty());
        assert!(!manager.sounds[&SoundEffect::LevelUp].is_empty());
        assert!(!manager.sounds[&SoundEffect::LifeLost].is_empty());
        assert!(!manager.sounds[&SoundEffect::GameOver].is_empty());
        assert!(!manager.sounds[&SoundEffect::Countdown].is_empty());
        assert!(!manager.sounds[&SoundEffect::TimerWarning].is_empty());
    }

    #[test]
    fn test_sound_manager_play_when_disabled() {
        let mut manager = SoundManager::new(SoundConfig::default());

        // Disable sound
        manager.set_enabled(false);

        // play() should be no-op (no panic)
        manager.play(SoundEffect::GameOver);
    }

    #[test]
    fn test_sound_manager_play_if_enabled() {
        let manager = SoundManager::new(SoundConfig::default());

        // Should be no-op when not initialized (no panic)
        manager.play_if_enabled(SoundEffect::LevelUp);
    }

    #[test]
    fn test_sound_manager_set_volume_nan_infinity() {
        let mut manager = SoundManager::new(SoundConfig::default());

        // NaN should fall back to default
        manager.set_volume(f32::NAN);
        assert!((manager.config().volume - SoundConfig::DEFAULT_VOLUME).abs() < f32::EPSILON);

        // Positive infinity should fall back to default
        manager.set_volume(0.5); // Reset to known value
        manager.set_volume(f32::INFINITY);
        assert!((manager.config().volume - SoundConfig::DEFAULT_VOLUME).abs() < f32::EPSILON);

        // Negative infinity should fall back to default
        manager.set_volume(0.5); // Reset to known value
        manager.set_volume(f32::NEG_INFINITY);
        assert!((manager.config().volume - SoundConfig::DEFAULT_VOLUME).abs() < f32::EPSILON);
    }

    #[test]
    fn test_sound_manager_double_init_is_safe() {
        let mut manager = SoundManager::new(SoundConfig::default());
        // First call may succeed or fail depending on audio device
        let _ = manager.try_init();
        let initial_available = manager.is_available();

        // Second call should be no-op
        let _ = manager.try_init();
        assert_eq!(manager.is_available(), initial_available);
    }
}
