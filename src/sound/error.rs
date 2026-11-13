//! Sound error types

use thiserror::Error;

/// Sound system errors
#[derive(Debug, Error)]
pub enum SoundError {
    /// No audio output device available
    #[error("No audio output device available")]
    NoAudioDevice,

    /// Failed to create audio stream
    #[error("Audio stream error")]
    StreamError(#[source] rodio::stream::DeviceSinkError),

    /// Failed to decode audio data
    #[error("Audio decode error: {0}")]
    DecodeError(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sound_error_display() {
        let err = SoundError::NoAudioDevice;
        assert_eq!(err.to_string(), "No audio output device available");

        let err = SoundError::StreamError(rodio::stream::DeviceSinkError::NoDevice);
        assert_eq!(err.to_string(), "Audio stream error");
        assert_eq!(
            std::error::Error::source(&err).unwrap().to_string(),
            "Could not find any output device"
        );

        let err = SoundError::DecodeError("invalid format".to_string());
        assert_eq!(err.to_string(), "Audio decode error: invalid format");
    }

    #[test]
    fn test_sound_error_debug() {
        let err = SoundError::NoAudioDevice;
        let debug = format!("{err:?}");
        assert!(debug.contains("NoAudioDevice"));
    }
}
