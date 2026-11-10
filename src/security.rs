//! Security utilities and error types
//!
//! This module provides security-related functionality including
//! error definitions, validation utilities, and security helpers.

use std::path::{Path, PathBuf};
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

    #[error("Content too large (max {max} bytes, got {actual} bytes)")]
    ContentTooLarge { max: usize, actual: usize },

    #[error("Invalid cursor position")]
    InvalidCursorPosition,

    #[error("Too many hints (max {max})")]
    TooManyHints { max: usize },

    #[error("Too many alternatives (max {max})")]
    TooManyAlternatives { max: usize },

    #[error("Too many actions (max: 1000000)")]
    TooManyActions,

    #[error("Score calculation overflow")]
    ScoreOverflow,

    #[error("Command sequence too long (max {max})")]
    CommandSequenceTooLong { max: usize },

    #[error("Invalid content: contains null bytes or invalid encoding")]
    InvalidContent,

    #[error("Invalid UTF-8 encoding")]
    InvalidEncoding,

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Invalid state: {0}")]
    InvalidState(String),

    #[error("Too many keymap bindings (max {max}, got {actual})")]
    TooManyKeymapBindings { max: usize, actual: usize },

    #[error("Not a regular file")]
    NotRegularFile,

    #[error("Failed to read file: {0}")]
    ReadFailed(String),
}

/// User-facing error messages (sanitized)
#[derive(Error, Debug)]
pub enum UserError {
    #[error("Invalid application state. Please restart the application.")]
    InvalidState { message: String },

    /// A message arrived for a screen other than the current one (e.g. a stale
    /// timer message racing a key-driven transition). Not a user-visible fault.
    #[error("That action is not available on the current screen.")]
    WrongScreen,

    #[error("Failed to load scenario file. Please check the file path and format.")]
    ScenarioLoadError,

    #[error("The scenario file is too large or complex. Please use a smaller file.")]
    ScenarioTooComplex,

    #[error("Operation failed. Please try again.")]
    OperationFailed,

    #[error("Command execution failed: {context}")]
    CommandFailed { context: String },
}

impl UserError {
    /// Create an InvalidState error with a descriptive message
    pub fn invalid_state(message: &str) -> Self {
        Self::InvalidState {
            message: message.to_string(),
        }
    }

    /// Create a CommandFailed error with context for debugging
    pub fn command_failed(context: impl Into<String>) -> Self {
        Self::CommandFailed {
            context: context.into(),
        }
    }
}

impl From<SecurityError> for UserError {
    fn from(err: SecurityError) -> Self {
        tracing::error!("Security error occurred: {:?}", err);

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

            SecurityError::InvalidState(msg) => UserError::InvalidState { message: msg },

            SecurityError::InvalidCursorPosition
            | SecurityError::TooManyActions
            | SecurityError::ScoreOverflow
            | SecurityError::CommandSequenceTooLong { .. }
            | SecurityError::InvalidContent
            | SecurityError::InvalidEncoding
            | SecurityError::InvalidInput(_)
            | SecurityError::TooManyKeymapBindings { .. }
            | SecurityError::NotRegularFile
            | SecurityError::ReadFailed(_) => UserError::OperationFailed,
        }
    }
}

impl From<crate::gamification::GamificationError> for UserError {
    fn from(err: crate::gamification::GamificationError) -> Self {
        use crate::gamification::GamificationError;
        tracing::error!("Gamification error occurred: {:?}", err);

        match err {
            GamificationError::StorageError(_) => UserError::OperationFailed,
            GamificationError::QuestNotFound(_) => UserError::OperationFailed,
            GamificationError::InvalidLevel(_) => UserError::invalid_state("Invalid level"),
            GamificationError::StreakFreezeUnavailable => UserError::OperationFailed,
            GamificationError::StreakFreezeNothingToProtect => UserError::OperationFailed,
            GamificationError::StreakFreezeGapOutOfRange { .. } => UserError::OperationFailed,
            GamificationError::AchievementAlreadyUnlocked(_) => UserError::OperationFailed,
        }
    }
}

impl From<crate::learning::LearningError> for UserError {
    fn from(err: crate::learning::LearningError) -> Self {
        use crate::learning::LearningError;
        tracing::error!("Learning error occurred: {:?}", err);

        match err {
            LearningError::InvalidStability(_) | LearningError::InvalidDifficulty(_) => {
                UserError::invalid_state("Invalid learning parameters")
            }
            LearningError::CommandNotFound(_) => {
                UserError::command_failed("Command not found in registry")
            }
            LearningError::FsrsError(_) => UserError::OperationFailed,
        }
    }
}

/// Security configuration constants
pub mod limits {
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

    /// Maximum number of commands stored in a single `q`/`Q` macro
    ///
    /// Recording stops accepting further commands once this cap is hit,
    /// keeping what was already captured rather than failing outright.
    pub const MAX_MACRO_LENGTH: usize = 100;

    /// Maximum length of a single-line prompt buffer
    ///
    /// Shared by every prompt that accumulates text one keystroke at a time
    /// before executing atomically: the `:`-prefixed command-line buffer and
    /// the `s`/`S` regex-selection prompt. `InputState` is cloned on every
    /// keystroke transition, so this bounds the per-keystroke clone cost
    /// (O(n²) in the worst case) as well as the rendered prompt line length.
    pub const MAX_COMMAND_LINE_LEN: usize = 256;

    /// Maximum number of selection ranges kept from a single `s`/`S` regex
    /// match pass
    ///
    /// `helix_core::selection::{select_on_matches, split_on_matches}` loop
    /// `find_iter` unbounded and accept no cap parameter, so this bounds the
    /// *retained* selection post-hoc rather than peak allocation during the
    /// scan. Peak allocation is bounded by document size (not
    /// attacker-controlled: documents are scenario-authored and tiny), and
    /// `regex-cursor` is finite-automaton based, so catastrophic backtracking
    /// is not a concern here.
    pub const MAX_REGEX_SELECTION_MATCHES: usize = 10_000;

    /// Maximum number of quest templates per file
    pub const MAX_QUEST_TEMPLATES_PER_FILE: usize = 100;

    /// Maximum target value for quest objectives
    pub const MAX_QUEST_TARGET: u32 = 100;

    /// Maximum speed run time limit in seconds (1 hour)
    pub const MAX_SPEED_RUN_TIME_SECONDS: u64 = 3600;

    /// Maximum custom XP reward
    pub const MAX_CUSTOM_XP_REWARD: u32 = 1000;

    /// Maximum quest name length
    pub const MAX_QUEST_NAME_LENGTH: usize = 100;

    /// Maximum quest description length
    pub const MAX_QUEST_DESCRIPTION_LENGTH: usize = 500;

    /// Maximum version string length
    pub const MAX_VERSION_LENGTH: usize = 20;

    /// Maximum locale string length
    pub const MAX_LOCALE_LENGTH: usize = 10;

    /// Maximum length of a scenario `Setup.language` token (file-extension-style,
    /// e.g. "rs", "md", "py" — longest bundled syntect extensions are well under this)
    pub const MAX_LANGUAGE_LENGTH: usize = 20;

    /// Maximum required conditions count
    pub const MAX_REQUIRED_CONDITIONS: usize = 20;

    /// Maximum number of cursors per scenario
    pub const MAX_CURSORS_PER_SCENARIO: usize = 100;

    /// Maximum number of selections per scenario
    pub const MAX_SELECTIONS_PER_SCENARIO: usize = 100;

    /// Maximum length of a scenario or quest ID
    pub const MAX_ID_LENGTH: usize = 64;

    /// Maximum size of the user's Helix `config.toml` we'll read for
    /// keymap remapping (1 MB — real configs are a few KB at most).
    pub const MAX_KEYMAP_FILE_SIZE: u64 = 1024 * 1024;

    /// Maximum number of keymap bindings (top-level plus minor-mode
    /// entries combined) accepted from a single Helix config file.
    pub const MAX_KEYMAP_BINDINGS: usize = 1000;

    /// Maximum size of `profile.json` (1 MB). At the current curriculum
    /// size (148 scenarios, 76 distinct commands tracked by FSRS), a
    /// saturated profile — every scenario completed, every command with a
    /// full review history — serializes to roughly 70 KB, so this cap
    /// leaves about 14x headroom for curriculum growth before it needs
    /// revisiting.
    pub const MAX_PROFILE_FILE_SIZE: u64 = 1024 * 1024;

    /// Maximum size of `config.json` (1 MB). `AppConfig` today is a
    /// handful of booleans (well under 1 KB serialized), so this leaves
    /// generous headroom for new settings fields.
    pub const MAX_CONFIG_FILE_SIZE: u64 = 1024 * 1024;
}

/// Path validation utilities
pub mod path_validator {
    use super::*;
    use std::fs;
    use std::io::Read;

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

    /// Reads a UTF-8 text file of at most `max_size` bytes without trusting its reported length.
    ///
    /// The path is rejected unless it resolves to a regular file (so `/dev/zero`
    /// symlinks, FIFOs, sockets and directories never reach an unbounded read),
    /// and at most `max_size + 1` bytes are read from the opened handle: a file
    /// that is larger, or grows between lookup and read, is refused instead of
    /// being buffered. [`SecurityError::FileTooLarge::actual`] is the number of
    /// bytes read, i.e. a lower bound on the real size.
    ///
    /// # Errors
    ///
    /// - [`SecurityError::InvalidPath`] if the path cannot be opened
    /// - [`SecurityError::NotRegularFile`] if it is not a regular file
    /// - [`SecurityError::FileTooLarge`] if it exceeds `max_size`
    /// - [`SecurityError::InvalidEncoding`] if the content is not UTF-8
    ///
    /// # Examples
    ///
    /// ```
    /// use helix_trainer::security::path_validator::read_capped;
    ///
    /// let dir = tempfile::tempdir().unwrap();
    /// let path = dir.path().join("a.txt");
    /// std::fs::write(&path, "hello").unwrap();
    /// assert_eq!(read_capped(&path, 16).unwrap(), "hello");
    /// assert!(read_capped(&path, 4).is_err());
    /// ```
    pub fn read_capped(path: &Path, max_size: u64) -> Result<String, SecurityError> {
        // `stat` never blocks, so a FIFO is rejected here before `open` could hang on it.
        if !fs::metadata(path)
            .map_err(|_| SecurityError::InvalidPath)?
            .is_file()
        {
            return Err(SecurityError::NotRegularFile);
        }

        let file = fs::File::open(path).map_err(|_| SecurityError::InvalidPath)?;
        let is_file = file
            .metadata()
            .map_err(|e| SecurityError::ReadFailed(e.to_string()))?
            .is_file();
        if !is_file {
            return Err(SecurityError::NotRegularFile);
        }

        let mut buf = Vec::new();
        file.take(max_size.saturating_add(1))
            .read_to_end(&mut buf)
            .map_err(|e| SecurityError::ReadFailed(e.to_string()))?;

        let read = buf.len() as u64;
        if read > max_size {
            return Err(SecurityError::FileTooLarge {
                max: max_size,
                actual: read,
            });
        }

        String::from_utf8(buf).map_err(|_| SecurityError::InvalidEncoding)
    }
}

/// Creation of user-private files and directories for persisted data.
///
/// On Unix directories are created `0700` and files `0600` (an existing file
/// reopened through `create_file` is tightened to `0600`); other platforms
/// use the process defaults.
pub mod private_fs {
    use std::fs;
    use std::io;
    use std::path::Path;

    /// Creates `path` and any missing parents, owner-only (`0700`) on Unix.
    ///
    /// Directories that already exist are left untouched.
    ///
    /// # Errors
    ///
    /// Returns the underlying I/O error if a directory cannot be created.
    pub fn create_dir_all(path: &Path) -> io::Result<()> {
        let mut builder = fs::DirBuilder::new();
        builder.recursive(true);
        #[cfg(unix)]
        std::os::unix::fs::DirBuilderExt::mode(&mut builder, 0o700);
        builder.create(path)
    }

    /// Creates or truncates `path` for writing, owner-only (`0600`) on Unix.
    ///
    /// # Errors
    ///
    /// Returns the underlying I/O error if the file cannot be opened or its
    /// permissions cannot be tightened.
    pub fn create_file(path: &Path) -> io::Result<fs::File> {
        let mut options = fs::OpenOptions::new();
        options.write(true).create(true).truncate(true);
        let file = private_options(&mut options).open(path)?;
        #[cfg(unix)]
        file.set_permissions(std::os::unix::fs::PermissionsExt::from_mode(0o600))?;
        Ok(file)
    }

    /// Creates `path` for writing, failing if it already exists, owner-only
    /// (`0600`) on Unix.
    ///
    /// # Errors
    ///
    /// Returns [`io::ErrorKind::AlreadyExists`] if `path` exists, or the
    /// underlying I/O error otherwise.
    pub fn create_new_file(path: &Path) -> io::Result<fs::File> {
        let mut options = fs::OpenOptions::new();
        options.write(true).create_new(true);
        private_options(&mut options).open(path)
    }

    fn private_options(options: &mut fs::OpenOptions) -> &mut fs::OpenOptions {
        #[cfg(unix)]
        std::os::unix::fs::OpenOptionsExt::mode(options, 0o600);
        options
    }
}

/// Shared serde validators for scenario/quest configuration fields
pub mod validators {
    use super::limits::{MAX_ID_LENGTH, MAX_LANGUAGE_LENGTH};
    use serde::Deserialize;

    /// Custom deserialization for ID field to validate format
    pub fn validate_id_field<'de, D>(deserializer: D) -> Result<String, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;

        // Validate ID is not empty
        if s.is_empty() {
            return Err(serde::de::Error::custom("Invalid ID: cannot be empty"));
        }

        // Validate ID format: alphanumeric with underscores, max MAX_ID_LENGTH chars
        if s.len() > MAX_ID_LENGTH || !s.chars().all(|c| c.is_alphanumeric() || c == '_') {
            return Err(serde::de::Error::custom(format!(
                "Invalid ID: must be alphanumeric with underscores, max {} chars",
                MAX_ID_LENGTH
            )));
        }

        Ok(s)
    }

    /// Custom deserialization for `Setup.language` to validate format
    ///
    /// Unlike [`validate_id_field`], an explicit empty string is accepted rather than
    /// rejected: it is a valid "unsupported language" value that the syntax highlighter
    /// resolves to plain-text rendering rather than a load-time error (see
    /// `specs/language-aware-syntax-highlighting/spec.md` edge cases). The character set is
    /// deliberately permissive - real syntect extension tokens include symbols such as
    /// `"c++"` or `"h++"` - rejecting only control characters, whitespace, and oversized
    /// values; the value is only ever used as an exact-match lookup key into syntect's
    /// `SyntaxSet`, never as a path or shell argument, so it carries no injection risk.
    pub fn validate_language_field<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let opt: Option<String> = Option::deserialize(deserializer)?;

        if let Some(ref language) = opt {
            if language.len() > MAX_LANGUAGE_LENGTH {
                return Err(serde::de::Error::custom(format!(
                    "Invalid language: max {} characters",
                    MAX_LANGUAGE_LENGTH
                )));
            }
            if language
                .chars()
                .any(|c| c.is_control() || c.is_whitespace())
            {
                return Err(serde::de::Error::custom(
                    "Invalid language: must not contain control characters or whitespace",
                ));
            }
        }

        Ok(opt)
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
    fn test_suspicious_path_double_slash() {
        assert!(path_validator::is_suspicious_path(Path::new(
            "/var//malicious"
        )));
    }

    #[test]
    fn test_suspicious_path_backtick() {
        assert!(path_validator::is_suspicious_path(Path::new(
            "file`command`"
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
    fn test_content_sanitization_valid_unicode() {
        // Valid Unicode should pass
        assert!(sanitizer::sanitize_content("Hello, Welt!").is_ok());
        assert!(sanitizer::sanitize_content("fn main() {}").is_ok());
    }

    #[test]
    fn test_terminal_output_sanitization() {
        let input = "Hello\x1b[31mWorld\x1b[0m";
        let output = sanitizer::sanitize_terminal_output(input);

        // Should not contain escape characters
        assert!(!output.contains('\x1b'));
    }

    #[test]
    fn test_terminal_output_sanitization_preserves_valid() {
        let input = "Hello World\n\tIndented";
        let output = sanitizer::sanitize_terminal_output(input);
        assert!(output.contains("Hello World"));
        assert!(output.contains('\n'));
        assert!(output.contains('\t'));
    }

    #[test]
    fn test_sanitize_path_for_logging() {
        let path = Path::new("/home/user/secret/file.toml");
        let sanitized = sanitizer::sanitize_path_for_logging(path);
        assert_eq!(sanitized, "file.toml");
    }

    #[test]
    fn test_sanitize_path_for_logging_no_filename() {
        let path = Path::new("/");
        let sanitized = sanitizer::sanitize_path_for_logging(path);
        assert_eq!(sanitized, "[redacted]");
    }

    #[test]
    fn test_remove_ansi_codes() {
        let input = "Hello\x1bWorld";
        let output = sanitizer::remove_ansi_codes(input);
        assert!(!output.contains('\x1b'));
        assert!(output.contains("Hello"));
        assert!(output.contains("World"));
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

    #[test]
    fn test_error_conversion_invalid_path() {
        let err = SecurityError::InvalidPath;
        let user_err = UserError::from(err);
        assert_eq!(
            user_err.to_string(),
            "Failed to load scenario file. Please check the file path and format."
        );
    }

    #[test]
    fn test_error_conversion_suspicious_path() {
        let err = SecurityError::SuspiciousPath;
        let user_err = UserError::from(err);
        assert_eq!(
            user_err.to_string(),
            "Failed to load scenario file. Please check the file path and format."
        );
    }

    #[test]
    fn test_error_conversion_invalid_toml() {
        let err = SecurityError::InvalidToml("parse error".to_string());
        let user_err = UserError::from(err);
        assert_eq!(
            user_err.to_string(),
            "Failed to load scenario file. Please check the file path and format."
        );
    }

    #[test]
    fn test_error_conversion_file_too_large() {
        let err = SecurityError::FileTooLarge {
            max: 1000,
            actual: 2000,
        };
        let user_err = UserError::from(err);
        assert_eq!(
            user_err.to_string(),
            "The scenario file is too large or complex. Please use a smaller file."
        );
    }

    #[test]
    fn test_error_conversion_too_many_scenarios() {
        let err = SecurityError::TooManyScenarios {
            max: 100,
            actual: 200,
        };
        let user_err = UserError::from(err);
        assert_eq!(
            user_err.to_string(),
            "The scenario file is too large or complex. Please use a smaller file."
        );
    }

    #[test]
    fn test_error_conversion_content_too_large() {
        let err = SecurityError::ContentTooLarge {
            max: 1000,
            actual: 2000,
        };
        let user_err = UserError::from(err);
        assert_eq!(
            user_err.to_string(),
            "The scenario file is too large or complex. Please use a smaller file."
        );
    }

    #[test]
    fn test_error_conversion_too_many_hints() {
        let err = SecurityError::TooManyHints { max: 10 };
        let user_err = UserError::from(err);
        assert_eq!(
            user_err.to_string(),
            "The scenario file is too large or complex. Please use a smaller file."
        );
    }

    #[test]
    fn test_error_conversion_too_many_alternatives() {
        let err = SecurityError::TooManyAlternatives { max: 20 };
        let user_err = UserError::from(err);
        assert_eq!(
            user_err.to_string(),
            "The scenario file is too large or complex. Please use a smaller file."
        );
    }

    #[test]
    fn test_error_conversion_invalid_state() {
        let err = SecurityError::InvalidState("bad state".to_string());
        let user_err = UserError::from(err);
        assert!(user_err.to_string().contains("Invalid application state"));
    }

    #[test]
    fn test_error_conversion_others_map_to_operation_failed() {
        let err = SecurityError::InvalidCursorPosition;
        let user_err = UserError::from(err);
        assert_eq!(user_err.to_string(), "Operation failed. Please try again.");

        let err = SecurityError::ScoreOverflow;
        let user_err = UserError::from(err);
        assert_eq!(user_err.to_string(), "Operation failed. Please try again.");

        let err = SecurityError::TooManyActions;
        let user_err = UserError::from(err);
        assert_eq!(user_err.to_string(), "Operation failed. Please try again.");
    }

    #[test]
    fn test_user_error_invalid_state_constructor() {
        let err = UserError::invalid_state("test message");
        assert!(err.to_string().contains("Invalid application state"));
    }

    #[test]
    fn test_user_error_command_failed_constructor() {
        let err = UserError::command_failed("test context");
        assert!(err.to_string().contains("Command execution failed"));
        assert!(err.to_string().contains("test context"));
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
    fn test_score_multiply_boundary() {
        // Test edge of valid range
        let result = arithmetic::checked_score_multiply(100, 2.0);
        assert_eq!(result.unwrap(), 200);

        let result = arithmetic::checked_score_multiply(100, 0.0);
        assert_eq!(result.unwrap(), 0);
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
    fn test_action_count_validation_boundary() {
        let result = arithmetic::validate_action_count(1_000_000);
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
    fn test_cursor_position_validation_col_excessive() {
        let result = arithmetic::validate_cursor_position(0, 1_000_000_000, 100);
        assert!(result.is_err());
    }

    #[test]
    fn test_cursor_position_validation_excessive() {
        let result = arithmetic::validate_cursor_position(1_000_000_000, 0, 100);
        assert!(result.is_err());
    }

    // Security error display tests
    #[test]
    fn test_security_error_display() {
        assert!(
            SecurityError::PathTraversal
                .to_string()
                .contains("Access denied")
        );
        assert!(
            SecurityError::InvalidPath
                .to_string()
                .contains("Invalid file path")
        );
        assert!(
            SecurityError::SuspiciousPath
                .to_string()
                .contains("Suspicious")
        );
        assert!(
            SecurityError::FileTooLarge {
                max: 100,
                actual: 200
            }
            .to_string()
            .contains("too large")
        );
        assert!(
            SecurityError::InvalidToml("error".to_string())
                .to_string()
                .contains("TOML")
        );
        assert!(
            SecurityError::TooManyScenarios {
                max: 10,
                actual: 20
            }
            .to_string()
            .contains("scenarios")
        );
        assert!(
            SecurityError::ContentTooLarge {
                max: 100,
                actual: 200
            }
            .to_string()
            .contains("Content")
        );
        assert!(
            SecurityError::InvalidCursorPosition
                .to_string()
                .contains("cursor")
        );
        assert!(
            SecurityError::TooManyHints { max: 10 }
                .to_string()
                .contains("hints")
        );
        assert!(
            SecurityError::TooManyAlternatives { max: 20 }
                .to_string()
                .contains("alternatives")
        );
        assert!(
            SecurityError::TooManyActions
                .to_string()
                .contains("actions")
        );
        assert!(
            SecurityError::ScoreOverflow
                .to_string()
                .contains("overflow")
        );
        assert!(
            SecurityError::CommandSequenceTooLong { max: 100 }
                .to_string()
                .contains("sequence")
        );
        assert!(
            SecurityError::InvalidContent
                .to_string()
                .contains("content")
        );
        assert!(SecurityError::InvalidEncoding.to_string().contains("UTF-8"));
        assert!(
            SecurityError::InvalidInput("err".to_string())
                .to_string()
                .contains("Invalid input")
        );
        assert!(
            SecurityError::InvalidState("err".to_string())
                .to_string()
                .contains("Invalid state")
        );
    }

    // User error display tests
    #[test]
    fn test_user_error_display() {
        assert!(
            UserError::InvalidState {
                message: "test".to_string()
            }
            .to_string()
            .contains("Invalid application state")
        );
        assert!(
            UserError::ScenarioLoadError
                .to_string()
                .contains("load scenario")
        );
        assert!(
            UserError::ScenarioTooComplex
                .to_string()
                .contains("too large")
        );
        assert!(
            UserError::OperationFailed
                .to_string()
                .contains("Operation failed")
        );
        assert!(
            UserError::CommandFailed {
                context: "test".to_string()
            }
            .to_string()
            .contains("Command")
        );
    }

    mod read_capped {
        use super::*;

        #[test]
        fn reads_file_within_cap() {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("f.txt");
            std::fs::write(&path, "12345").unwrap();
            assert_eq!(path_validator::read_capped(&path, 5).unwrap(), "12345");
        }

        #[test]
        fn rejects_file_over_cap() {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("f.txt");
            std::fs::write(&path, "123456").unwrap();
            assert!(matches!(
                path_validator::read_capped(&path, 5),
                Err(SecurityError::FileTooLarge { max: 5, .. })
            ));
        }

        #[test]
        fn rejects_directory() {
            let dir = tempfile::tempdir().unwrap();
            assert!(matches!(
                path_validator::read_capped(dir.path(), 5),
                Err(SecurityError::NotRegularFile)
            ));
        }

        #[test]
        fn rejects_non_utf8() {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("f.bin");
            std::fs::write(&path, [0xff, 0xfe]).unwrap();
            assert!(matches!(
                path_validator::read_capped(&path, 5),
                Err(SecurityError::InvalidEncoding)
            ));
        }

        #[test]
        fn missing_path_is_invalid_path() {
            let dir = tempfile::tempdir().unwrap();
            assert!(matches!(
                path_validator::read_capped(&dir.path().join("nope"), 5),
                Err(SecurityError::InvalidPath)
            ));
        }

        #[cfg(unix)]
        #[test]
        fn rejects_symlink_to_dev_zero_without_unbounded_read() {
            let dir = tempfile::tempdir().unwrap();
            let link = dir.path().join("zero.json");
            std::os::unix::fs::symlink("/dev/zero", &link).unwrap();
            assert!(matches!(
                path_validator::read_capped(&link, 1024),
                Err(SecurityError::NotRegularFile)
            ));
        }

        #[cfg(unix)]
        #[test]
        fn rejects_fifo_without_blocking() {
            let dir = tempfile::tempdir().unwrap();
            let fifo = dir.path().join("pipe.json");
            let status = std::process::Command::new("mkfifo")
                .arg(&fifo)
                .status()
                .unwrap();
            assert!(status.success());
            assert!(matches!(
                path_validator::read_capped(&fifo, 1024),
                Err(SecurityError::NotRegularFile)
            ));
        }
    }
}
