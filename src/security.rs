//! Security utilities and error types
//!
//! This module provides security-related functionality including
//! error definitions, validation utilities, and security helpers.

use std::path::{Path, PathBuf};
use std::time::Duration;
use thiserror::Error;

/// Security-specific errors
#[derive(Error, Debug)]
pub enum SecurityError {
    #[error("Access denied: path outside allowed directory")]
    PathTraversal,

    #[error("Invalid file path")]
    InvalidPath,

    #[error("Suspicious path pattern detected")]
    SuspiciousPath,

    #[error("Scenario file too large (max {max} bytes, got {actual} bytes)")]
    FileTooLarge { max: u64, actual: u64 },

    #[error("Invalid TOML format: {0}")]
    InvalidToml(String),

    #[error("Too many scenarios in file (max {max}, got {actual})")]
    TooManyScenarios { max: usize, actual: usize },

    #[error("Invalid scenario ID: must be alphanumeric with underscores")]
    InvalidScenarioId,

    #[error("Content too large (max {max} bytes, got {actual} bytes)")]
    ContentTooLarge { max: usize, actual: usize },

    #[error("Invalid cursor position")]
    InvalidCursorPosition,

    #[error("Too many hints (max {max})")]
    TooManyHints { max: usize },

    #[error("Too many alternatives (max {max})")]
    TooManyAlternatives { max: usize },

    #[error("Process spawn failed: {0}")]
    ProcessSpawnFailed(String),

    #[error("Session timeout (max duration: {0:?})")]
    SessionTimeout(Duration),

    #[error("Invalid scenario configuration")]
    InvalidScoringConfig,

    #[error("Too many actions (max: 1000000)")]
    TooManyActions,

    #[error("Score calculation overflow")]
    ScoreOverflow,

    #[error("Invalid duration")]
    InvalidDuration,

    #[error("Command sequence too long (max {max})")]
    CommandSequenceTooLong { max: usize },

    #[error("Invalid command")]
    InvalidCommand,

    #[error("Too many active sessions (max {max})")]
    TooManySessions { max: usize },

    #[error("Too many temporary files (max {max})")]
    TooManyTempFiles { max: usize },

    #[error("Rate limit exceeded, retry after {0:?}")]
    RateLimitExceeded(Duration),

    #[error("Invalid content: contains null bytes or invalid encoding")]
    InvalidContent,

    #[error("Invalid UTF-8 encoding")]
    InvalidEncoding,

    #[error("Invalid input: {0}")]
    InvalidInput(String),
}

/// User-facing error messages (sanitized)
#[derive(Error, Debug)]
pub enum UserError {
    #[error("Failed to load scenario file. Please check the file path and format.")]
    ScenarioLoadError,

    #[error("The scenario file is too large or complex. Please use a smaller file.")]
    ScenarioTooComplex,

    #[error("Failed to start editor. Please ensure Helix is installed.")]
    EditorStartFailed,

    #[error("Operation failed. Please try again.")]
    OperationFailed,

    #[error("Session has expired. Please start a new session.")]
    SessionExpired,
}

impl From<SecurityError> for UserError {
    fn from(err: SecurityError) -> Self {
        // Log the detailed error internally
        tracing::error!("Security error occurred: {:?}", err);

        // Return sanitized error to user
        match err {
            SecurityError::PathTraversal
            | SecurityError::InvalidPath
            | SecurityError::SuspiciousPath
            | SecurityError::InvalidToml(_) => UserError::ScenarioLoadError,

            SecurityError::FileTooLarge { .. }
            | SecurityError::TooManyScenarios { .. }
            | SecurityError::ContentTooLarge { .. }
            | SecurityError::TooManyHints { .. }
            | SecurityError::TooManyAlternatives { .. } => UserError::ScenarioTooComplex,

            SecurityError::ProcessSpawnFailed(_) => UserError::EditorStartFailed,

            SecurityError::SessionTimeout(_) => UserError::SessionExpired,

            _ => UserError::OperationFailed,
        }
    }
}

/// Security configuration constants
pub mod limits {
    use std::time::Duration;

    /// Maximum size of a scenario TOML file (10 MB)
    pub const MAX_SCENARIO_FILE_SIZE: u64 = 10 * 1024 * 1024;

    /// Maximum number of scenarios per file
    pub const MAX_SCENARIOS_PER_FILE: usize = 100;

    /// Maximum length of scenario file content
    pub const MAX_FILE_CONTENT_LENGTH: usize = 100_000;

    /// Maximum number of hints per scenario
    pub const MAX_HINTS: usize = 10;

    /// Maximum number of alternative solutions
    pub const MAX_ALTERNATIVES: usize = 20;

    /// Maximum command sequence length
    pub const MAX_COMMAND_SEQUENCE_LENGTH: usize = 100;

    /// Command timeout
    pub const COMMAND_TIMEOUT: Duration = Duration::from_secs(30);

    /// Maximum active sessions
    pub const MAX_ACTIVE_SESSIONS: usize = 10;

    /// Maximum temporary files
    pub const MAX_TEMP_FILES: usize = 100;

    /// Session timeout (1 hour)
    pub const SESSION_TIMEOUT: Duration = Duration::from_secs(3600);

    /// Minimum interval between scenario loads
    pub const MIN_LOAD_INTERVAL: Duration = Duration::from_millis(100);
}

/// Path validation utilities
pub mod path_validator {
    use super::*;
    use std::fs;

    /// Validates that a path is safe to access
    pub fn validate_path(path: &Path, allowed_bases: &[PathBuf]) -> Result<PathBuf, SecurityError> {
        // Canonicalize path to resolve symlinks and .. components
        let canonical = path
            .canonicalize()
            .map_err(|_| SecurityError::InvalidPath)?;

        // Check if path is within allowed directories
        let is_allowed = allowed_bases.iter().any(|base| {
            // Canonicalize base path
            if let Ok(canonical_base) = base.canonicalize() {
                canonical.starts_with(&canonical_base)
            } else {
                false
            }
        });

        if !is_allowed {
            return Err(SecurityError::PathTraversal);
        }

        // Check for suspicious patterns
        if is_suspicious_path(&canonical) {
            return Err(SecurityError::SuspiciousPath);
        }

        Ok(canonical)
    }

    /// Checks if path contains suspicious patterns
    pub fn is_suspicious_path(path: &Path) -> bool {
        let path_str = path.to_string_lossy();

        // Check for suspicious patterns
        path_str.contains("..")
            || path_str.contains("//")
            || path_str.contains("/etc/")
            || path_str.contains("/root/")
            || path_str.contains("/.ssh/")
            || path_str.contains('$')
            || path_str.contains('`')
    }

    /// Validates file size
    pub fn validate_file_size(path: &Path, max_size: u64) -> Result<(), SecurityError> {
        let metadata = fs::metadata(path).map_err(|_| SecurityError::InvalidPath)?;

        if metadata.len() > max_size {
            return Err(SecurityError::FileTooLarge {
                max: max_size,
                actual: metadata.len(),
            });
        }

        Ok(())
    }
}

/// Content sanitization utilities
pub mod sanitizer {
    use super::*;

    /// Sanitizes string for terminal output
    pub fn sanitize_terminal_output(input: &str) -> String {
        input
            .chars()
            .filter(|c| c.is_ascii_graphic() || matches!(c, ' ' | '\n' | '\t'))
            .collect()
    }

    /// Sanitizes file content
    pub fn sanitize_content(content: &str) -> Result<String, SecurityError> {
        // Check for null bytes
        if content.contains('\0') {
            return Err(SecurityError::InvalidContent);
        }

        // Check for excessive size
        if content.len() > limits::MAX_FILE_CONTENT_LENGTH {
            return Err(SecurityError::ContentTooLarge {
                max: limits::MAX_FILE_CONTENT_LENGTH,
                actual: content.len(),
            });
        }

        // Validate UTF-8
        String::from_utf8(content.as_bytes().to_vec()).map_err(|_| SecurityError::InvalidEncoding)
    }

    /// Sanitizes path for logging (only shows filename)
    pub fn sanitize_path_for_logging(path: &Path) -> String {
        path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("[redacted]")
            .to_string()
    }

    /// Removes ANSI escape sequences
    pub fn remove_ansi_codes(input: &str) -> String {
        // Simple implementation - for production use regex crate
        input.chars().filter(|c| *c != '\x1b').collect()
    }
}

/// Safe arithmetic operations for scoring and calculations
pub mod arithmetic {
    use super::*;

    /// Safely calculates score with overflow prevention
    ///
    /// # Errors
    /// Returns `ScoreOverflow` if the calculation would overflow
    ///
    /// # Examples
    /// ```ignore
    /// let score = checked_score_calculation(5, 10, 100)?;
    /// assert_eq!(score, 50);
    /// ```
    pub fn checked_score_calculation(
        optimal_count: usize,
        actual_count: usize,
        max_points: u32,
    ) -> Result<u32, SecurityError> {
        // Validate inputs to prevent division by zero and negative scenarios
        if optimal_count == 0 || actual_count == 0 {
            return Err(SecurityError::ScoreOverflow);
        }

        // Use checked arithmetic to prevent overflow
        let numerator = (max_points as u64)
            .checked_mul(optimal_count as u64)
            .ok_or(SecurityError::ScoreOverflow)?;

        let result = numerator
            .checked_div(actual_count as u64)
            .ok_or(SecurityError::ScoreOverflow)?;

        // Ensure result fits in u32 and doesn't exceed max_points
        let final_score = result.min(max_points as u64) as u32;

        Ok(final_score)
    }

    /// Safely adds two scores with overflow prevention
    ///
    /// # Errors
    /// Returns `ScoreOverflow` if the sum would overflow
    pub fn checked_score_add(a: u32, b: u32) -> Result<u32, SecurityError> {
        a.checked_add(b).ok_or(SecurityError::ScoreOverflow)
    }

    /// Safely multiplies a score with a multiplier (for alternatives)
    ///
    /// # Errors
    /// Returns `ScoreOverflow` if the multiplication would overflow
    pub fn checked_score_multiply(score: u32, multiplier: f32) -> Result<u32, SecurityError> {
        // Ensure multiplier is reasonable (0.0 to 2.0)
        if !(0.0..=2.0).contains(&multiplier) {
            return Err(SecurityError::ScoreOverflow);
        }

        let result = (score as f64 * multiplier as f64) as u64;

        // Check if result fits in u32
        if result > u32::MAX as u64 {
            return Err(SecurityError::ScoreOverflow);
        }

        Ok(result as u32)
    }

    /// Validates that action counts are within reasonable bounds
    ///
    /// # Errors
    /// Returns `TooManyActions` if count exceeds maximum
    pub fn validate_action_count(count: usize) -> Result<(), SecurityError> {
        if count > 1_000_000 {
            return Err(SecurityError::TooManyActions);
        }
        Ok(())
    }

    /// Validates cursor position is within bounds
    ///
    /// # Errors
    /// Returns `InvalidCursorPosition` if row or col exceeds reasonable bounds
    pub fn validate_cursor_position(
        row: usize,
        col: usize,
        max_content_size: usize,
    ) -> Result<(), SecurityError> {
        // Cursor position should be reasonable - not exceeding a line that's 10x the content size
        let max_reasonable = max_content_size.saturating_mul(10);

        if row > max_reasonable || col > max_reasonable {
            return Err(SecurityError::InvalidCursorPosition);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_suspicious_path_detection() {
        assert!(path_validator::is_suspicious_path(Path::new(
            "../../etc/passwd"
        )));
        assert!(path_validator::is_suspicious_path(Path::new("/etc/passwd")));
        assert!(path_validator::is_suspicious_path(Path::new(
            "/root/.ssh/id_rsa"
        )));
        assert!(path_validator::is_suspicious_path(Path::new(
            "file$malicious"
        )));

        assert!(!path_validator::is_suspicious_path(Path::new(
            "scenarios/basic.toml"
        )));
    }

    #[test]
    fn test_content_sanitization() {
        // Valid content
        assert!(sanitizer::sanitize_content("Hello, World!").is_ok());

        // Null bytes
        assert!(sanitizer::sanitize_content("Hello\0World").is_err());

        // Too large
        let huge = "A".repeat(200_000);
        assert!(sanitizer::sanitize_content(&huge).is_err());
    }

    #[test]
    fn test_terminal_output_sanitization() {
        let input = "Hello\x1b[31mWorld\x1b[0m";
        let output = sanitizer::sanitize_terminal_output(input);

        // Should not contain escape characters
        assert!(!output.contains('\x1b'));
    }

    #[test]
    fn test_error_conversion() {
        let err = SecurityError::PathTraversal;
        let user_err = UserError::from(err);
        assert_eq!(
            user_err.to_string(),
            "Failed to load scenario file. Please check the file path and format."
        );
    }

    // Arithmetic safety tests
    #[test]
    fn test_normal_score_calculation() {
        let result = arithmetic::checked_score_calculation(5, 10, 100);
        assert_eq!(result.unwrap(), 50);
    }

    #[test]
    fn test_score_calculation_zero_optimal() {
        let result = arithmetic::checked_score_calculation(0, 10, 100);
        assert!(result.is_err());
    }

    #[test]
    fn test_score_calculation_zero_actual() {
        let result = arithmetic::checked_score_calculation(10, 0, 100);
        assert!(result.is_err());
    }

    #[test]
    fn test_score_calculation_capped_at_max() {
        // If actual < optimal, score should be capped at max_points
        let result = arithmetic::checked_score_calculation(10, 5, 100);
        assert_eq!(result.unwrap(), 100);
    }

    #[test]
    fn test_score_calculation_overflow_prevention() {
        // Test with values that would overflow
        let result = arithmetic::checked_score_calculation(usize::MAX / 2, 1, u32::MAX);
        // This will overflow when computing the numerator
        assert!(result.is_err());
    }

    #[test]
    fn test_score_add_normal() {
        let result = arithmetic::checked_score_add(50, 30);
        assert_eq!(result.unwrap(), 80);
    }

    #[test]
    fn test_score_add_overflow() {
        let result = arithmetic::checked_score_add(u32::MAX, 1);
        assert!(result.is_err());
    }

    #[test]
    fn test_score_multiply_normal() {
        let result = arithmetic::checked_score_multiply(100, 0.5);
        assert_eq!(result.unwrap(), 50);
    }

    #[test]
    fn test_score_multiply_invalid_multiplier() {
        let result = arithmetic::checked_score_multiply(100, -0.5);
        assert!(result.is_err());

        let result = arithmetic::checked_score_multiply(100, 3.0);
        assert!(result.is_err());
    }

    #[test]
    fn test_action_count_validation_normal() {
        let result = arithmetic::validate_action_count(1000);
        assert!(result.is_ok());
    }

    #[test]
    fn test_action_count_validation_exceeds() {
        let result = arithmetic::validate_action_count(2_000_000);
        assert!(result.is_err());
    }

    #[test]
    fn test_cursor_position_validation_normal() {
        let result = arithmetic::validate_cursor_position(10, 5, 1000);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cursor_position_validation_excessive() {
        let result = arithmetic::validate_cursor_position(1_000_000_000, 0, 100);
        assert!(result.is_err());
    }
}
