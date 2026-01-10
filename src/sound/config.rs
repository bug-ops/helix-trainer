//! Sound configuration

use serde::{Deserialize, Serialize};

/// Sound configuration stored in user profile
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SoundConfig {
    /// Master volume (0.0 to 1.0)
    pub volume: f32,
    /// Sound enabled/disabled
    pub enabled: bool,
}

impl SoundConfig {
    /// Default volume level
    pub const DEFAULT_VOLUME: f32 = 0.7;

    /// Create new config with custom settings
    pub fn new(volume: f32, enabled: bool) -> Self {
        Self {
            volume: if volume.is_finite() {
                volume.clamp(0.0, 1.0)
            } else {
                Self::DEFAULT_VOLUME
            },
            enabled,
        }
    }
}

impl Default for SoundConfig {
    fn default() -> Self {
        Self {
            volume: Self::DEFAULT_VOLUME,
            enabled: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sound_config_default() {
        let config = SoundConfig::default();
        assert!((config.volume - 0.7).abs() < f32::EPSILON);
        assert!(config.enabled);
    }

    #[test]
    fn test_sound_config_volume_clamping() {
        // Above max
        let config = SoundConfig::new(1.5, true);
        assert!((config.volume - 1.0).abs() < f32::EPSILON);

        // Below min
        let config = SoundConfig::new(-0.5, true);
        assert!((config.volume - 0.0).abs() < f32::EPSILON);

        // Within range
        let config = SoundConfig::new(0.5, true);
        assert!((config.volume - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn test_sound_config_new() {
        let config = SoundConfig::new(0.3, false);
        assert!((config.volume - 0.3).abs() < f32::EPSILON);
        assert!(!config.enabled);
    }

    #[test]
    fn test_sound_config_serialization() {
        let config = SoundConfig::new(0.8, true);
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: SoundConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(config, deserialized);
    }

    #[test]
    fn test_sound_config_toml_serialization() {
        let config = SoundConfig::new(0.8, true);
        let toml = toml::to_string(&config).unwrap();
        let deserialized: SoundConfig = toml::from_str(&toml).unwrap();
        assert_eq!(config, deserialized);
    }

    #[test]
    fn test_sound_config_nan_infinity_handling() {
        // NaN should fall back to default
        let config = SoundConfig::new(f32::NAN, true);
        assert!((config.volume - SoundConfig::DEFAULT_VOLUME).abs() < f32::EPSILON);

        // Positive infinity should fall back to default
        let config = SoundConfig::new(f32::INFINITY, true);
        assert!((config.volume - SoundConfig::DEFAULT_VOLUME).abs() < f32::EPSILON);

        // Negative infinity should fall back to default
        let config = SoundConfig::new(f32::NEG_INFINITY, true);
        assert!((config.volume - SoundConfig::DEFAULT_VOLUME).abs() < f32::EPSILON);
    }
}
