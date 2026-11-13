//! Sound configuration

use serde::{Deserialize, Deserializer, Serialize};

/// Output gain, always finite and within `0.0..=1.0`.
///
/// This is the single place where the volume rule lives: [`Volume::new`]
/// clamps a finite input into range and maps a non-finite input to
/// [`SoundConfig::DEFAULT_VOLUME`]. Deserialization goes through the same
/// constructor, so any number stored in `profile.json` loads as a valid value.
///
/// # Examples
///
/// ```
/// use helix_trainer::sound::Volume;
///
/// assert_eq!(Volume::new(12.0).get(), 1.0);
/// assert_eq!(Volume::new(-3.0).get(), 0.0);
/// assert_eq!(Volume::new(0.4).get(), 0.4);
/// assert_eq!(Volume::new(f32::NAN), Volume::default());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(transparent)]
pub struct Volume(f32);

impl Volume {
    /// Builds a volume, clamping into `0.0..=1.0` and replacing non-finite input with the default.
    pub fn new(value: f32) -> Self {
        if value.is_finite() {
            Self(value.clamp(0.0, 1.0))
        } else {
            Self(SoundConfig::DEFAULT_VOLUME)
        }
    }

    /// Returns the gain as a plain number in `0.0..=1.0`.
    pub fn get(self) -> f32 {
        self.0
    }
}

impl Default for Volume {
    fn default() -> Self {
        Self(SoundConfig::DEFAULT_VOLUME)
    }
}

impl<'de> Deserialize<'de> for Volume {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        f32::deserialize(deserializer).map(Self::new)
    }
}

/// Sound configuration stored in user profile
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SoundConfig {
    volume: Volume,
    enabled: bool,
}

impl SoundConfig {
    /// Default volume level
    pub const DEFAULT_VOLUME: f32 = 0.7;

    /// Create new config with custom settings
    pub fn new(volume: Volume, enabled: bool) -> Self {
        Self { volume, enabled }
    }

    /// Master volume
    pub fn volume(&self) -> Volume {
        self.volume
    }

    /// Whether sound is enabled
    pub fn enabled(&self) -> bool {
        self.enabled
    }

    /// Sets the master volume
    pub fn set_volume(&mut self, volume: Volume) {
        self.volume = volume;
    }

    /// Sets the enabled flag
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }
}

impl Default for SoundConfig {
    fn default() -> Self {
        Self {
            volume: Volume::default(),
            enabled: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    #[test]
    fn test_volume_table() {
        for (input, expected) in [
            (0.4, 0.4),
            (-3.0, 0.0),
            (12.0, 1.0),
            (0.0, 0.0),
            (1.0, 1.0),
            (f32::NAN, SoundConfig::DEFAULT_VOLUME),
            (f32::INFINITY, SoundConfig::DEFAULT_VOLUME),
            (f32::NEG_INFINITY, SoundConfig::DEFAULT_VOLUME),
        ] {
            assert!((Volume::new(input).get() - expected).abs() < f32::EPSILON);
        }
    }

    proptest! {
        #[test]
        fn prop_volume_always_in_range(bits in any::<u32>()) {
            let v = Volume::new(f32::from_bits(bits)).get();
            prop_assert!((0.0..=1.0).contains(&v));
        }
    }

    #[test]
    fn test_volume_deserialize_clamps() {
        let config: SoundConfig =
            serde_json::from_str(r#"{"volume": 12.0, "enabled": true}"#).unwrap();
        assert!((config.volume().get() - 1.0).abs() < f32::EPSILON);

        let config: SoundConfig =
            serde_json::from_str(r#"{"volume": -2.5, "enabled": false}"#).unwrap();
        assert!(config.volume().get().abs() < f32::EPSILON);
        assert!(!config.enabled());
    }

    #[test]
    fn test_volume_serializes_as_plain_number() {
        let json = serde_json::to_string(&SoundConfig::new(Volume::new(0.5), true)).unwrap();
        assert_eq!(json, r#"{"volume":0.5,"enabled":true}"#);
    }

    #[test]
    fn test_sound_config_default() {
        let config = SoundConfig::default();
        assert!((config.volume().get() - 0.7).abs() < f32::EPSILON);
        assert!(config.enabled());
    }

    #[test]
    fn test_sound_config_new() {
        let config = SoundConfig::new(Volume::new(0.3), false);
        assert!((config.volume().get() - 0.3).abs() < f32::EPSILON);
        assert!(!config.enabled());
    }

    #[test]
    fn test_sound_config_serialization() {
        let config = SoundConfig::new(Volume::new(0.8), true);
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: SoundConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(config, deserialized);
    }

    #[test]
    fn test_sound_config_toml_serialization() {
        let config = SoundConfig::new(Volume::new(0.8), true);
        let toml = toml::to_string(&config).unwrap();
        let deserialized: SoundConfig = toml::from_str(&toml).unwrap();
        assert_eq!(config, deserialized);
    }
}
