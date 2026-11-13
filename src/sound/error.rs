//! Sound error types

use thiserror::Error;

/// Sound system errors
#[derive(Debug, Error)]
pub enum SoundError {
    /// Failed to create audio stream
    #[error("Audio stream error")]
    StreamError(#[source] rodio::stream::DeviceSinkError),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sound_error_keeps_device_cause() {
        let err = SoundError::StreamError(rodio::stream::DeviceSinkError::NoDevice);
        assert_eq!(err.to_string(), "Audio stream error");
        assert_eq!(
            std::error::Error::source(&err).unwrap().to_string(),
            "Could not find any output device"
        );
    }
}
