//! Security utilities and error types
//!
//! This module provides security-related functionality including
//! error definitions, validation utilities, and security helpers.

use thiserror::Error;

/// Security-specific errors raised while validating untrusted input.
///
/// These carry internal detail for logs; they are converted to the sanitized
/// [`UserError`] before reaching the UI.
#[derive(Error, Debug)]
pub enum SecurityError {
    /// A path resolved outside the allowed base directory.
    #[error("Access denied: path outside allowed directory")]
    PathTraversal,

    /// A path could not be interpreted (empty, unresolvable, malformed).
    #[error("Invalid file path")]
    InvalidPath,

    /// A path contains a pattern that is rejected on sight (e.g. `..` segments).
    #[error("Suspicious path pattern detected")]
    SuspiciousPath,

    /// A scenario file exceeds the size limit.
    #[error("Scenario file too large (max {max} bytes, got {actual} bytes)")]
    FileTooLarge {
        /// Maximum allowed size in bytes.
        max: u64,
        /// Observed size in bytes.
        actual: u64,
    },

    /// TOML parsing failed; carries the parser message for logs only.
    #[error("Invalid TOML format: {0}")]
    InvalidToml(String),

    /// A scenario file holds more scenarios than allowed.
    #[error("Too many scenarios in file (max {max}, got {actual})")]
    TooManyScenarios {
        /// Maximum allowed scenario count.
        max: usize,
        /// Observed scenario count.
        actual: usize,
    },

    /// Editor content exceeds the size limit.
    #[error("Content too large (max {max} bytes, got {actual} bytes)")]
    ContentTooLarge {
        /// Maximum allowed size in bytes.
        max: usize,
        /// Observed size in bytes.
        actual: usize,
    },

    /// A cursor position lies outside the document.
    #[error("Invalid cursor position")]
    InvalidCursorPosition,

    /// A scenario declares more hints than allowed.
    #[error("Too many hints (max {max})")]
    TooManyHints {
        /// Maximum allowed hint count.
        max: usize,
    },

    /// A session recorded more actions than the cap.
    #[error("Too many actions (max: 1000000)")]
    TooManyActions,

    /// A score computation overflowed its integer type.
    #[error("Score calculation overflow")]
    ScoreOverflow,

    /// A command sequence exceeds the length cap.
    #[error("Command sequence too long (max {max})")]
    CommandSequenceTooLong {
        /// Maximum allowed command count.
        max: usize,
    },

    /// Content contains null bytes or otherwise unusable encoding.
    #[error("Invalid content: contains null bytes or invalid encoding")]
    InvalidContent,

    /// Bytes are not valid UTF-8.
    #[error("Invalid UTF-8 encoding")]
    InvalidEncoding,

    /// A quest template failed validation.
    #[error("Invalid quest template: {0}")]
    QuestTemplate(#[from] crate::config::quests::QuestTemplateError),

    /// A scenario's cursor setup or target failed validation.
    #[error("Invalid scenario: {0}")]
    Scenario(#[from] crate::config::scenarios::ScenarioValidationError),

    /// The application reached a state that violates an invariant.
    #[error("Invalid state: {0}")]
    InvalidState(String),

    /// A keymap file defines more bindings than allowed.
    #[error("Too many keymap bindings (max {max}, got {actual})")]
    TooManyKeymapBindings {
        /// Maximum allowed binding count.
        max: usize,
        /// Observed binding count.
        actual: usize,
    },

    /// The path exists but is not a regular file (directory, device, ...).
    #[error("Not a regular file")]
    NotRegularFile,

    /// Reading a file failed; carries the I/O message for logs only.
    #[error("Failed to read file: {0}")]
    ReadFailed(String),
}

/// Why [`path_validator::read_capped`] refused or failed to read a file.
///
/// Keeps the I/O and UTF-8 causes as sources so callers can tell "the file does not exist"
/// (first launch) from every other failure; [`SecurityError`] is derived from it for the
/// scenario, quest and keymap loaders.
#[derive(Error, Debug)]
pub enum CappedReadError {
    /// The path exists but is not a regular file (directory, device, FIFO, ...).
    #[error("not a regular file")]
    NotRegularFile,

    /// The file is larger than the cap.
    #[error("file too large (max {max} bytes, got {actual} bytes)")]
    TooLarge {
        /// Maximum allowed size in bytes.
        max: u64,
        /// Number of bytes read, a lower bound on the real size.
        actual: u64,
    },

    /// Inspecting (`stat`) or opening the file failed: the path is unusable.
    #[error("the file could not be inspected or opened")]
    Inaccessible(#[source] std::io::Error),

    /// Inspecting the opened handle or reading the file failed.
    #[error("I/O error while reading the file")]
    Io(#[source] std::io::Error),

    /// The content is not valid UTF-8.
    #[error("file content is not valid UTF-8")]
    InvalidUtf8(#[source] std::string::FromUtf8Error),
}

impl CappedReadError {
    /// Whether the file simply does not exist.
    #[must_use]
    pub fn is_not_found(&self) -> bool {
        matches!(
            self,
            Self::Inaccessible(e) | Self::Io(e) if e.kind() == std::io::ErrorKind::NotFound
        )
    }
}

impl From<CappedReadError> for SecurityError {
    fn from(err: CappedReadError) -> Self {
        match err {
            CappedReadError::NotRegularFile => Self::NotRegularFile,
            CappedReadError::TooLarge { max, actual } => Self::FileTooLarge { max, actual },
            CappedReadError::Inaccessible(_) => Self::InvalidPath,
            CappedReadError::Io(e) => Self::ReadFailed(e.to_string()),
            CappedReadError::InvalidUtf8(_) => Self::InvalidEncoding,
        }
    }
}

/// Renders `error` and every cause below it as `top: cause: root cause`.
///
/// Error sinks log this instead of `Display` alone, because variants that keep their cause
/// as a `#[source]` do not repeat it in their own message.
///
/// # Examples
///
/// ```
/// use helix_trainer::security::error_chain;
///
/// let io = std::io::Error::other("disk full");
/// let wrapped = helix_trainer::security::CappedReadError::Io(io);
/// assert_eq!(
///     error_chain(&wrapped),
///     "I/O error while reading the file: disk full"
/// );
/// ```
#[must_use]
pub fn error_chain(error: &dyn std::error::Error) -> String {
    let mut rendered = error.to_string();
    let mut source = error.source();
    while let Some(cause) = source {
        rendered.push_str(": ");
        rendered.push_str(&cause.to_string());
        source = cause.source();
    }
    rendered
}

/// Typed internal cause kept below a sanitized [`UserError`].
///
/// Never rendered to the user; it exists so [`error_chain`] can show the original failure in
/// logs after a domain error was reduced to generic user-facing text.
#[derive(Error, Debug)]
pub enum ErrorCause {
    /// Input validation or limit failure.
    #[error(transparent)]
    Security(#[from] SecurityError),

    /// Gamification (quests, streaks, achievements) failure.
    #[error(transparent)]
    Gamification(#[from] crate::gamification::GamificationError),

    /// Learning (FSRS, scheduling) failure.
    #[error(transparent)]
    Learning(#[from] crate::learning::LearningError),

    /// Helix simulator command failure.
    #[error(transparent)]
    Simulator(#[from] crate::helix::error::SimulatorError),
}

/// User-facing error messages (sanitized)
///
/// The `Display` text is path-free and generic. Variants produced from a domain error keep it
/// as a typed [`ErrorCause`] source for logs.
#[derive(Error, Debug)]
pub enum UserError {
    /// An internal invariant was violated; the message is for logs, the
    /// displayed text is generic.
    #[error("Invalid application state. Please restart the application.")]
    InvalidState {
        /// Diagnostic detail that is never shown to the user.
        message: String,
        /// Original failure, if the state error was derived from one.
        #[source]
        cause: Option<ErrorCause>,
    },

    /// A message arrived for a screen other than the current one (e.g. a stale
    /// timer message racing a key-driven transition). Not a user-visible fault.
    #[error("That action is not available on the current screen.")]
    WrongScreen,

    /// A scenario file could not be loaded or parsed.
    #[error("Failed to load scenario file. Please check the file path and format.")]
    ScenarioLoadError {
        /// Original failure, if any.
        #[source]
        cause: Option<ErrorCause>,
    },

    /// A scenario file breaches a size or count limit.
    #[error("The scenario file is too large or complex. Please use a smaller file.")]
    ScenarioTooComplex {
        /// Original failure, if any.
        #[source]
        cause: Option<ErrorCause>,
    },

    /// A generic failure with no actionable detail for the user.
    #[error("Operation failed. Please try again.")]
    OperationFailed {
        /// Original failure, if any.
        #[source]
        cause: Option<ErrorCause>,
    },

    /// A Helix command failed to execute in the simulator.
    #[error("Command execution failed: {context}")]
    CommandFailed {
        /// What was being executed, for debugging.
        context: String,
        /// Original failure, if any.
        #[source]
        cause: Option<ErrorCause>,
    },

    /// Reading or writing the saved profile or configuration failed. The cause is kept as a
    /// source for logs; the displayed text carries no paths or OS messages.
    #[error("Could not read or save your data. Please try again.")]
    Storage(#[source] crate::config::json_store::StorageError),
}

impl UserError {
    /// Create an `InvalidState` error with a descriptive message
    pub fn invalid_state(message: &str) -> Self {
        Self::InvalidState {
            message: message.to_string(),
            cause: None,
        }
    }

    /// Create a `CommandFailed` error with context for debugging
    pub fn command_failed(context: impl Into<String>) -> Self {
        Self::CommandFailed {
            context: context.into(),
            cause: None,
        }
    }

    /// A [`UserError::ScenarioLoadError`] without an underlying cause.
    #[must_use]
    pub fn scenario_load_error() -> Self {
        Self::ScenarioLoadError { cause: None }
    }

    /// A [`UserError::ScenarioTooComplex`] without an underlying cause.
    #[must_use]
    pub fn scenario_too_complex() -> Self {
        Self::ScenarioTooComplex { cause: None }
    }

    /// A [`UserError::OperationFailed`] without an underlying cause.
    #[must_use]
    pub fn operation_failed() -> Self {
        Self::OperationFailed { cause: None }
    }
}

impl From<SecurityError> for UserError {
    fn from(err: SecurityError) -> Self {
        match err {
            e @ (SecurityError::PathTraversal
            | SecurityError::InvalidPath
            | SecurityError::SuspiciousPath
            | SecurityError::InvalidToml(_)) => UserError::ScenarioLoadError {
                cause: Some(e.into()),
            },

            e @ (SecurityError::FileTooLarge { .. }
            | SecurityError::TooManyScenarios { .. }
            | SecurityError::ContentTooLarge { .. }
            | SecurityError::TooManyHints { .. }) => UserError::ScenarioTooComplex {
                cause: Some(e.into()),
            },

            SecurityError::InvalidState(msg) => UserError::InvalidState {
                message: msg.clone(),
                cause: Some(SecurityError::InvalidState(msg).into()),
            },

            e @ (SecurityError::InvalidCursorPosition
            | SecurityError::TooManyActions
            | SecurityError::ScoreOverflow
            | SecurityError::CommandSequenceTooLong { .. }
            | SecurityError::InvalidContent
            | SecurityError::InvalidEncoding
            | SecurityError::QuestTemplate(_)
            | SecurityError::Scenario(_)
            | SecurityError::TooManyKeymapBindings { .. }
            | SecurityError::NotRegularFile
            | SecurityError::ReadFailed(_)) => UserError::OperationFailed {
                cause: Some(e.into()),
            },
        }
    }
}

impl From<crate::gamification::GamificationError> for UserError {
    fn from(err: crate::gamification::GamificationError) -> Self {
        use crate::gamification::GamificationError;

        match err {
            e @ GamificationError::InvalidLevel(_) => UserError::InvalidState {
                message: "Invalid level".to_string(),
                cause: Some(e.into()),
            },
            e @ (GamificationError::QuestNotFound(_)
            | GamificationError::StreakFreezeUnavailable
            | GamificationError::StreakFreezeNothingToProtect
            | GamificationError::StreakFreezeGapOutOfRange { .. }
            | GamificationError::AchievementAlreadyUnlocked(_)) => UserError::OperationFailed {
                cause: Some(e.into()),
            },
        }
    }
}

impl From<crate::learning::LearningError> for UserError {
    fn from(err: crate::learning::LearningError) -> Self {
        use crate::learning::LearningError;

        match err {
            e @ (LearningError::InvalidStability(_)
            | LearningError::InvalidDifficulty(_)
            | LearningError::InvalidCounters { .. }) => UserError::InvalidState {
                message: "Invalid learning parameters".to_string(),
                cause: Some(e.into()),
            },
            e @ LearningError::CommandNotFound(_) => UserError::CommandFailed {
                context: "Command not found in registry".to_string(),
                cause: Some(e.into()),
            },
            e @ LearningError::FsrsError(_) => UserError::OperationFailed {
                cause: Some(e.into()),
            },
        }
    }
}

impl From<crate::config::json_store::StorageError> for UserError {
    fn from(err: crate::config::json_store::StorageError) -> Self {
        Self::Storage(err)
    }
}

/// Path validation utilities
pub mod path_validator {
    use super::{CappedReadError, SecurityError};
    use std::fs;
    use std::io::Read;
    use std::path::{Path, PathBuf};

    /// Validates that a path is safe to access
    ///
    /// # Errors
    ///
    /// Returns [`SecurityError::InvalidPath`] if `path` cannot be canonicalized,
    /// [`SecurityError::PathTraversal`] if it lies outside every allowed base, or
    /// [`SecurityError::SuspiciousPath`] if it matches a suspicious pattern.
    pub fn validate_path(path: &Path, allowed_bases: &[PathBuf]) -> Result<PathBuf, SecurityError> {
        // Canonicalize path to resolve symlinks and .. components
        let canonical = path
            .canonicalize()
            .map_err(|_| SecurityError::InvalidPath)?;

        // Check if path is within allowed directories
        let is_allowed = allowed_bases.iter().any(|base| {
            // Canonicalize base path
            base.canonicalize()
                .is_ok_and(|canonical_base| canonical.starts_with(&canonical_base))
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
    /// - [`CappedReadError::Io`] if the path cannot be inspected, opened or read (a missing
    ///   file is `Io` with [`std::io::ErrorKind::NotFound`])
    /// - [`CappedReadError::NotRegularFile`] if it is not a regular file
    /// - [`CappedReadError::TooLarge`] if it exceeds `max_size`
    /// - [`CappedReadError::InvalidUtf8`] if the content is not UTF-8
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
    pub fn read_capped(path: &Path, max_size: u64) -> Result<String, CappedReadError> {
        // `stat` never blocks, so a FIFO is rejected here before `open` could hang on it.
        if !fs::metadata(path)
            .map_err(CappedReadError::Inaccessible)?
            .is_file()
        {
            return Err(CappedReadError::NotRegularFile);
        }

        let file = fs::File::open(path).map_err(CappedReadError::Inaccessible)?;
        if !file.metadata().map_err(CappedReadError::Io)?.is_file() {
            return Err(CappedReadError::NotRegularFile);
        }

        let mut buf = Vec::new();
        file.take(max_size.saturating_add(1))
            .read_to_end(&mut buf)
            .map_err(CappedReadError::Io)?;

        let read = buf.len() as u64;
        if read > max_size {
            return Err(CappedReadError::TooLarge {
                max: max_size,
                actual: read,
            });
        }

        String::from_utf8(buf).map_err(CappedReadError::InvalidUtf8)
    }
}

/// Creation of user-private files and directories for persisted data.
///
/// On Unix directories are created `0700` and files `0600`; other platforms
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

/// Content sanitization utilities
pub mod sanitizer {
    use std::path::Path;

    /// Sanitizes string for terminal output
    pub fn sanitize_terminal_output(input: &str) -> String {
        input
            .chars()
            .filter(|c| c.is_ascii_graphic() || matches!(c, ' ' | '\n' | '\t'))
            .collect()
    }

    /// Sanitizes path for logging (only shows filename)
    pub fn sanitize_path_for_logging(path: &Path) -> String {
        path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("[redacted]")
            .to_string()
    }
}

/// Safe arithmetic operations for scoring and calculations
pub mod arithmetic {
    use super::SecurityError;

    /// Safely calculates score with overflow prevention
    ///
    /// # Errors
    /// Returns `ScoreOverflow` if the calculation would overflow
    ///
    /// # Examples
    /// ```
    /// use helix_trainer::security::arithmetic::checked_score_calculation;
    ///
    /// let score = checked_score_calculation(5, 10, 100)?;
    /// assert_eq!(score, 50);
    /// # Ok::<(), helix_trainer::security::SecurityError>(())
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
        let numerator = u64::from(max_points)
            .checked_mul(optimal_count as u64)
            .ok_or(SecurityError::ScoreOverflow)?;

        let result = numerator
            .checked_div(actual_count as u64)
            .ok_or(SecurityError::ScoreOverflow)?;

        // Ensure result fits in u32 and doesn't exceed max_points
        u32::try_from(result.min(u64::from(max_points))).map_err(|_| SecurityError::ScoreOverflow)
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

        let result = f64::from(score) * f64::from(multiplier);

        if result > f64::from(u32::MAX) {
            return Err(SecurityError::ScoreOverflow);
        }

        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "0.0 <= result <= u32::MAX is checked above"
        )]
        let scaled = result as u32;
        Ok(scaled)
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
    use std::path::Path;

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
            SecurityError::from(crate::config::quests::QuestTemplateError::XpRewardTooHigh {
                max: 1,
                actual: 2
            })
            .to_string()
            .contains("Invalid quest template")
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
            UserError::invalid_state("test")
                .to_string()
                .contains("Invalid application state")
        );
        assert!(
            UserError::scenario_load_error()
                .to_string()
                .contains("load scenario")
        );
        assert!(
            UserError::scenario_too_complex()
                .to_string()
                .contains("too large")
        );
        assert!(
            UserError::operation_failed()
                .to_string()
                .contains("Operation failed")
        );
        assert!(
            UserError::command_failed("test")
                .to_string()
                .contains("Command")
        );
    }

    #[test]
    fn user_error_keeps_typed_cause_for_logs() {
        let user = UserError::from(SecurityError::TooManyActions);
        assert_eq!(user.to_string(), "Operation failed. Please try again.");
        assert_eq!(
            error_chain(&user),
            "Operation failed. Please try again.: Too many actions (max: 1000000)"
        );

        let user = UserError::from(SecurityError::InvalidToml("bad key".to_string()));
        assert!(error_chain(&user).ends_with("Invalid TOML format: bad key"));
        assert!(!user.to_string().contains("bad key"));

        let user = UserError::from(SecurityError::InvalidState("broken".to_string()));
        assert!(error_chain(&user).contains("Invalid state: broken"));

        let user = UserError::from(crate::gamification::GamificationError::StreakFreezeUnavailable);
        assert!(matches!(
            std::error::Error::source(&user).and_then(|e| e.downcast_ref::<ErrorCause>()),
            Some(ErrorCause::Gamification(_))
        ));
    }

    #[test]
    fn error_chain_joins_every_cause() {
        let root = std::io::Error::other("disk full");
        let storage = crate::config::json_store::StorageError::Write {
            doc: crate::config::json_store::Document::Profile,
            path: "/secret/profile.json".into(),
            source: root,
        };
        let chain = error_chain(&storage);
        assert_eq!(
            chain,
            "failed to write the profile file /secret/profile.json: disk full"
        );

        let user = UserError::from(storage);
        assert!(!user.to_string().contains("/secret"), "{user}");
        assert_eq!(error_chain(&user).matches("disk full").count(), 1);
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
                Err(CappedReadError::TooLarge { max: 5, .. })
            ));
        }

        #[test]
        fn rejects_directory() {
            let dir = tempfile::tempdir().unwrap();
            assert!(matches!(
                path_validator::read_capped(dir.path(), 5),
                Err(CappedReadError::NotRegularFile)
            ));
        }

        #[test]
        fn rejects_non_utf8() {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("f.bin");
            std::fs::write(&path, [0xff, 0xfe]).unwrap();
            assert!(matches!(
                path_validator::read_capped(&path, 5),
                Err(CappedReadError::InvalidUtf8(_))
            ));
        }

        #[test]
        fn missing_path_is_not_found() {
            let dir = tempfile::tempdir().unwrap();
            let err = path_validator::read_capped(&dir.path().join("nope"), 5).unwrap_err();
            assert!(err.is_not_found(), "{err}");
            assert!(matches!(
                SecurityError::from(err),
                SecurityError::InvalidPath
            ));
        }

        #[cfg(unix)]
        #[test]
        fn stat_failures_other_than_not_found_map_to_invalid_path() {
            let dir = tempfile::tempdir().unwrap();
            let file = dir.path().join("f.txt");
            std::fs::write(&file, "x").unwrap();
            let err = path_validator::read_capped(&file.join("child"), 5).unwrap_err();
            assert!(matches!(err, CappedReadError::Inaccessible(_)), "{err}");
            assert!(!err.is_not_found(), "ENOTDIR is not NotFound");
            assert!(matches!(
                SecurityError::from(err),
                SecurityError::InvalidPath
            ));
        }

        #[test]
        fn read_phase_io_errors_map_to_read_failed() {
            let err = CappedReadError::Io(std::io::Error::other("boom"));
            assert!(matches!(
                SecurityError::from(err),
                SecurityError::ReadFailed(_)
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
                Err(CappedReadError::NotRegularFile)
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
                Err(CappedReadError::NotRegularFile)
            ));
        }
    }
}
