//! Embedded audio assets
//!
//! Audio files are embedded at compile time for local-first architecture.
//! All files are OGG Vorbis format for good compression and cross-platform support.

/// Complete/success sound (0.3s chime)
pub const SOUND_COMPLETE: &[u8] = include_bytes!("../../assets/sounds/complete.ogg");

/// Failed/timeout sound (0.3s buzz)
pub const SOUND_FAILED: &[u8] = include_bytes!("../../assets/sounds/failed.ogg");

/// Multiplier increase sound (0.2s rising tone)
pub const SOUND_MULTIPLIER: &[u8] = include_bytes!("../../assets/sounds/multiplier.ogg");

/// Level up sound (0.5s fanfare)
pub const SOUND_LEVELUP: &[u8] = include_bytes!("../../assets/sounds/levelup.ogg");

/// Life lost sound (0.3s alert)
pub const SOUND_LIFELOST: &[u8] = include_bytes!("../../assets/sounds/lifelost.ogg");

/// Game over sound (1.0s jingle)
pub const SOUND_GAMEOVER: &[u8] = include_bytes!("../../assets/sounds/gameover.ogg");

/// Countdown tick sound (0.1s tick)
pub const SOUND_COUNTDOWN: &[u8] = include_bytes!("../../assets/sounds/countdown.ogg");

/// Timer warning sound (0.2s alert)
pub const SOUND_WARNING: &[u8] = include_bytes!("../../assets/sounds/warning.ogg");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embedded_assets_exist() {
        // All embedded audio data should be non-empty
        assert!(
            !SOUND_COMPLETE.is_empty(),
            "complete.ogg should not be empty"
        );
        assert!(!SOUND_FAILED.is_empty(), "failed.ogg should not be empty");
        assert!(
            !SOUND_MULTIPLIER.is_empty(),
            "multiplier.ogg should not be empty"
        );
        assert!(!SOUND_LEVELUP.is_empty(), "levelup.ogg should not be empty");
        assert!(
            !SOUND_LIFELOST.is_empty(),
            "lifelost.ogg should not be empty"
        );
        assert!(
            !SOUND_GAMEOVER.is_empty(),
            "gameover.ogg should not be empty"
        );
        assert!(
            !SOUND_COUNTDOWN.is_empty(),
            "countdown.ogg should not be empty"
        );
        assert!(!SOUND_WARNING.is_empty(), "warning.ogg should not be empty");
    }

    #[test]
    fn test_ogg_magic_bytes() {
        // OGG files start with "OggS" magic bytes
        assert!(
            SOUND_COMPLETE.starts_with(b"OggS"),
            "complete.ogg should have OGG magic bytes"
        );
        assert!(
            SOUND_FAILED.starts_with(b"OggS"),
            "failed.ogg should have OGG magic bytes"
        );
        assert!(
            SOUND_MULTIPLIER.starts_with(b"OggS"),
            "multiplier.ogg should have OGG magic bytes"
        );
        assert!(
            SOUND_LEVELUP.starts_with(b"OggS"),
            "levelup.ogg should have OGG magic bytes"
        );
        assert!(
            SOUND_LIFELOST.starts_with(b"OggS"),
            "lifelost.ogg should have OGG magic bytes"
        );
        assert!(
            SOUND_GAMEOVER.starts_with(b"OggS"),
            "gameover.ogg should have OGG magic bytes"
        );
        assert!(
            SOUND_COUNTDOWN.starts_with(b"OggS"),
            "countdown.ogg should have OGG magic bytes"
        );
        assert!(
            SOUND_WARNING.starts_with(b"OggS"),
            "warning.ogg should have OGG magic bytes"
        );
    }
}
