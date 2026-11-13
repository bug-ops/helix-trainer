//! Security utilities and error types
//!
//! This module provides security-related functionality including
//! error definitions, validation utilities, and security helpers.

use thiserror::Error;

/// Security-specific errors raised while validating untrusted input.
///
/// These carry internal detail for logs; they are converted to the sanitized
/// [`UserError`](crate::error::UserError) before reaching the UI.
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

    /// TOML parsing failed; the typed parser error (with line/column span) is the source.
    #[error("Invalid TOML format")]
    InvalidToml(#[source] Box<toml::de::Error>),

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

    /// Bytes are not valid UTF-8; the source carries the byte offset.
    #[error("Invalid UTF-8 encoding")]
    InvalidEncoding(#[source] std::string::FromUtf8Error),

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

    /// Reading a file failed; the source keeps the [`std::io::ErrorKind`].
    #[error("Failed to read file")]
    ReadFailed(#[source] std::io::Error),

    /// The file could not be inspected or opened; the source tells "not found" from
    /// "permission denied".
    #[error("File could not be opened")]
    Inaccessible(#[source] std::io::Error),
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
            CappedReadError::Inaccessible(e) => Self::Inaccessible(e),
            CappedReadError::Io(e) => Self::ReadFailed(e),
            CappedReadError::InvalidUtf8(e) => Self::InvalidEncoding(e),
        }
    }
}

/// A real `toml::de::Error` for tests that need a typed parse failure.
#[cfg(test)]
pub(crate) fn sample_toml_error() -> toml::de::Error {
    toml::from_str::<toml::Value>("key = ").unwrap_err()
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
    use std::assert_matches;
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
            SecurityError::InvalidToml(Box::new(sample_toml_error()))
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
        assert!(
            SecurityError::InvalidEncoding(String::from_utf8(vec![0xff]).unwrap_err())
                .to_string()
                .contains("UTF-8")
        );
        assert!(
            SecurityError::InvalidState("err".to_string())
                .to_string()
                .contains("Invalid state")
        );
    }

    mod typed_causes {
        use super::*;
        use std::error::Error;
        use std::io::ErrorKind;

        #[test]
        fn capped_read_conversion_keeps_io_kind() {
            let inaccessible = SecurityError::from(CappedReadError::Inaccessible(
                ErrorKind::PermissionDenied.into(),
            ));
            let source = inaccessible.source().unwrap();
            let io = source.downcast_ref::<std::io::Error>().unwrap();
            assert_eq!(io.kind(), ErrorKind::PermissionDenied);

            let read = SecurityError::from(CappedReadError::Io(ErrorKind::NotFound.into()));
            let io = read
                .source()
                .unwrap()
                .downcast_ref::<std::io::Error>()
                .unwrap();
            assert_eq!(io.kind(), ErrorKind::NotFound);
        }

        #[test]
        fn capped_read_conversion_keeps_utf8_offset() {
            let utf8 = String::from_utf8(vec![b'a', 0xff]).unwrap_err();
            let err = SecurityError::from(CappedReadError::InvalidUtf8(utf8));
            let cause = err
                .source()
                .unwrap()
                .downcast_ref::<std::string::FromUtf8Error>()
                .unwrap();
            assert_eq!(cause.utf8_error().valid_up_to(), 1);
        }

        #[test]
        fn toml_error_keeps_span() {
            let err = SecurityError::InvalidToml(Box::new(sample_toml_error()));
            let cause = err
                .source()
                .unwrap()
                .downcast_ref::<Box<toml::de::Error>>()
                .unwrap();
            assert!(cause.span().is_some());
        }
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
            assert_matches!(
                path_validator::read_capped(&path, 5),
                Err(CappedReadError::TooLarge { max: 5, .. })
            );
        }

        #[test]
        fn rejects_directory() {
            let dir = tempfile::tempdir().unwrap();
            assert_matches!(
                path_validator::read_capped(dir.path(), 5),
                Err(CappedReadError::NotRegularFile)
            );
        }

        #[test]
        fn rejects_non_utf8() {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("f.bin");
            std::fs::write(&path, [0xff, 0xfe]).unwrap();
            assert_matches!(
                path_validator::read_capped(&path, 5),
                Err(CappedReadError::InvalidUtf8(_))
            );
        }

        #[test]
        fn missing_path_is_not_found() {
            let dir = tempfile::tempdir().unwrap();
            let err = path_validator::read_capped(&dir.path().join("nope"), 5).unwrap_err();
            assert!(err.is_not_found(), "{err}");
            assert_matches!(SecurityError::from(err), SecurityError::Inaccessible(_));
        }

        #[cfg(unix)]
        #[test]
        fn stat_failures_other_than_not_found_map_to_inaccessible() {
            let dir = tempfile::tempdir().unwrap();
            let file = dir.path().join("f.txt");
            std::fs::write(&file, "x").unwrap();
            let err = path_validator::read_capped(&file.join("child"), 5).unwrap_err();
            assert_matches!(err, CappedReadError::Inaccessible(_), "{err}");
            assert!(!err.is_not_found(), "ENOTDIR is not NotFound");
            assert_matches!(SecurityError::from(err), SecurityError::Inaccessible(_));
        }

        #[test]
        fn read_phase_io_errors_map_to_read_failed() {
            let err = CappedReadError::Io(std::io::Error::other("boom"));
            assert_matches!(SecurityError::from(err), SecurityError::ReadFailed(_));
        }

        #[cfg(unix)]
        #[test]
        fn rejects_symlink_to_dev_zero_without_unbounded_read() {
            let dir = tempfile::tempdir().unwrap();
            let link = dir.path().join("zero.json");
            std::os::unix::fs::symlink("/dev/zero", &link).unwrap();
            assert_matches!(
                path_validator::read_capped(&link, 1024),
                Err(CappedReadError::NotRegularFile)
            );
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
            assert_matches!(
                path_validator::read_capped(&fifo, 1024),
                Err(CappedReadError::NotRegularFile)
            );
        }
    }
}
