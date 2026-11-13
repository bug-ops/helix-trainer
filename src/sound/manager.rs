//! Sound manager implementation

use std::collections::HashMap;
use std::io::Cursor;
use std::sync::Arc;

use rodio::mixer::Mixer;
use rodio::{Decoder, DeviceSinkBuilder, MixerDeviceSink, Player};

use super::assets;
use super::config::{SoundConfig, Volume};
use super::effects::SoundEffect;
use super::error::SoundError;

/// Sound manager for playing audio effects
///
/// Handles audio device initialization and playback with graceful degradation.
/// If audio is unavailable, all play operations become no-ops.
pub struct SoundManager {
    output: Output,
    /// Current configuration
    config: SoundConfig,
    /// Pre-loaded sound data
    sounds: HashMap<SoundEffect, Arc<[u8]>>,
}

/// Audio output state: either no device, or a live stream with its mixer.
enum Output {
    Silent,
    Ready {
        /// Keeps the device open for the lifetime of the mixer
        _stream: MixerDeviceSink,
        mixer: Mixer,
    },
}

impl SoundManager {
    /// Create a new sound manager with the given configuration
    ///
    /// Audio device initialization is deferred to first use.
    pub fn new(config: SoundConfig) -> Self {
        let sounds = Self::load_embedded_sounds();
        Self {
            output: Output::Silent,
            config,
            sounds,
        }
    }

    /// Try to initialize the audio device
    ///
    /// Call this early (e.g., at app startup) to detect audio availability.
    /// If this fails, the manager stays silent; the caller decides how to report it.
    ///
    /// # Errors
    ///
    /// Returns [`SoundError::StreamError`] if no default audio output can be opened.
    pub fn try_init(&mut self) -> Result<(), SoundError> {
        if matches!(self.output, Output::Ready { .. }) {
            return Ok(());
        }

        let stream = DeviceSinkBuilder::open_default_sink()
            .map_err(|e| SoundError::StreamError(e.to_string()))?;
        let mixer = stream.mixer().clone();
        self.output = Output::Ready {
            _stream: stream,
            mixer,
        };
        tracing::info!("Audio output opened");
        Ok(())
    }

    /// Play a sound effect
    ///
    /// Does nothing if sound is disabled, audio unavailable, or effect not found.
    pub fn play(&self, effect: SoundEffect) {
        let Output::Ready { mixer, .. } = &self.output else {
            return;
        };
        if !self.config.enabled() {
            return;
        }

        let Some(data) = self.sounds.get(&effect) else {
            tracing::warn!("Sound effect {:?} not found", effect);
            return;
        };

        // Create cursor from data
        let cursor = Cursor::new(data.clone());

        // Decode and play
        match Decoder::new(cursor) {
            Ok(source) => {
                let sink = Player::connect_new(mixer);
                sink.set_volume(self.config.volume().get());
                sink.append(source);
                sink.detach(); // Play to completion without blocking
            }
            Err(e) => {
                tracing::debug!("Failed to decode sound {:?}: {}", effect, e);
            }
        }
    }

    /// Set master volume
    pub fn set_volume(&mut self, volume: Volume) {
        self.config.set_volume(volume);
    }

    /// Set enabled state
    pub fn set_enabled(&mut self, enabled: bool) {
        self.config.set_enabled(enabled);
    }

    /// Toggle sound on/off, returns new state
    pub fn toggle(&mut self) -> bool {
        let enabled = !self.config.enabled();
        self.config.set_enabled(enabled);
        enabled
    }

    /// Get current configuration
    pub fn config(&self) -> &SoundConfig {
        &self.config
    }

    /// Check if audio is available
    pub fn is_available(&self) -> bool {
        matches!(self.output, Output::Ready { .. })
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
        assert!(manager.config().enabled());

        // Toggle off
        let new_state = manager.toggle();
        assert!(!new_state);
        assert!(!manager.config().enabled());

        // Toggle on
        let new_state = manager.toggle();
        assert!(new_state);
        assert!(manager.config().enabled());
    }

    #[test]
    fn test_sound_manager_no_init_silent() {
        let manager = SoundManager::new(SoundConfig::default());

        // Silent output: play is a no-op (no panic)
        manager.play(SoundEffect::ScenarioComplete);

        // is_available should return false
        assert!(!manager.is_available());
    }

    #[test]
    fn test_sound_manager_set_volume() {
        let mut manager = SoundManager::new(SoundConfig::default());

        // Normal volume
        manager.set_volume(Volume::new(0.5));
        assert!((manager.config().volume().get() - 0.5).abs() < f32::EPSILON);

        // Clamp above max
        manager.set_volume(Volume::new(1.5));
        assert!((manager.config().volume().get() - 1.0).abs() < f32::EPSILON);

        // Clamp below min
        manager.set_volume(Volume::new(-0.5));
        assert!((manager.config().volume().get() - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_sound_manager_set_enabled() {
        let mut manager = SoundManager::new(SoundConfig::default());

        manager.set_enabled(false);
        assert!(!manager.config().enabled());

        manager.set_enabled(true);
        assert!(manager.config().enabled());
    }

    #[test]
    fn test_sound_manager_default() {
        let manager = SoundManager::default();
        assert!(
            (manager.config().volume().get() - SoundConfig::DEFAULT_VOLUME).abs() < f32::EPSILON
        );
        assert!(manager.config().enabled());
        assert!(!manager.is_available()); // Silent output
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
    fn test_sound_manager_set_volume_nan_infinity() {
        let mut manager = SoundManager::new(SoundConfig::default());

        // NaN should fall back to default
        manager.set_volume(Volume::new(f32::NAN));
        assert!(
            (manager.config().volume().get() - SoundConfig::DEFAULT_VOLUME).abs() < f32::EPSILON
        );

        // Positive infinity should fall back to default
        manager.set_volume(Volume::new(0.5)); // Reset to known value
        manager.set_volume(Volume::new(f32::INFINITY));
        assert!(
            (manager.config().volume().get() - SoundConfig::DEFAULT_VOLUME).abs() < f32::EPSILON
        );

        // Negative infinity should fall back to default
        manager.set_volume(Volume::new(0.5)); // Reset to known value
        manager.set_volume(Volume::new(f32::NEG_INFINITY));
        assert!(
            (manager.config().volume().get() - SoundConfig::DEFAULT_VOLUME).abs() < f32::EPSILON
        );
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
