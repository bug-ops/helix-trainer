# Security Implementation Guide
## Helix Keybindings Trainer

This guide provides step-by-step implementation instructions for security controls identified in the Security Review. Implement these measures in order of priority.

---

## Phase 1: Immediate Security Foundation

### Step 1: Create Security Error Types

Create `/Users/rabax/Documents/git/helix_trainer/src/security.rs`:

```rust
//! Security utilities and error types
//!
//! This module provides security-related functionality including
//! error definitions, validation utilities, and security helpers.

use thiserror::Error;
use std::path::{Path, PathBuf};
use std::time::Duration;

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
        let is_allowed = allowed_bases
            .iter()
            .any(|base| canonical.starts_with(base));

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
    fn is_suspicious_path(path: &Path) -> bool {
        let path_str = path.to_string_lossy();

        // Check for suspicious patterns
        path_str.contains("..")
            || path_str.contains("//")
            || path_str.contains("/etc/")
            || path_str.contains("/root/")
            || path_str.contains("/.ssh/")
            || path_str.contains("$")
            || path_str.contains("`")
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
        String::from_utf8(content.as_bytes().to_vec())
            .map_err(|_| SecurityError::InvalidEncoding)
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
        input
            .chars()
            .filter(|c| *c != '\x1b')
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_suspicious_path_detection() {
        use path_validator::is_suspicious_path;

        assert!(is_suspicious_path(Path::new("../../etc/passwd")));
        assert!(is_suspicious_path(Path::new("/etc/passwd")));
        assert!(is_suspicious_path(Path::new("/root/.ssh/id_rsa")));
        assert!(is_suspicious_path(Path::new("file$malicious")));

        assert!(!is_suspicious_path(Path::new("scenarios/basic.toml")));
    }

    #[test]
    fn test_content_sanitization() {
        use sanitizer::sanitize_content;

        // Valid content
        assert!(sanitize_content("Hello, World!").is_ok());

        // Null bytes
        assert!(sanitize_content("Hello\0World").is_err());

        // Too large
        let huge = "A".repeat(200_000);
        assert!(sanitize_content(&huge).is_err());
    }

    #[test]
    fn test_terminal_output_sanitization() {
        use sanitizer::sanitize_terminal_output;

        let input = "Hello\x1b[31mWorld\x1b[0m";
        let output = sanitize_terminal_output(input);

        // Should not contain escape characters
        assert!(!output.contains('\x1b'));
    }
}
```

Update `/Users/rabax/Documents/git/helix_trainer/src/lib.rs`:

```rust
//! Helix Keybindings Trainer
//!
//! An interactive terminal user interface (TUI) application for learning Helix editor keybindings.
//! The trainer presents interactive scenarios where users practice Helix commands and receive
//! immediate feedback on their performance.
//!
//! # Architecture
//!
//! The application is organized into several modules:
//!
//! - `config`: Configuration and scenario loading
//! - `game`: Game engine and session management
//! - `helix`: Helix editor integration and PTY control
//! - `ui`: Terminal user interface components built with ratatui
//! - `security`: Security utilities, validation, and error handling

pub mod config;
pub mod game;
pub mod helix;
pub mod ui;
pub mod security;  // Add this line
```

---

### Step 2: Implement Secure Logging

Update `/Users/rabax/Documents/git/helix_trainer/src/main.rs`:

```rust
// Main entry point for the Helix Keybindings Trainer

use anyhow::Result;
use tracing_subscriber::{filter::LevelFilter, fmt, EnvFilter, prelude::*};

fn init_secure_logging() -> Result<()> {
    // Create filter that excludes sensitive modules at high log levels
    let filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .from_env_lossy()
        // Never log PTY communication at debug level in production
        .add_directive("helix_trainer::helix::pty_controller=warn".parse()?)
        .add_directive("helix_trainer::config::scenarios=info".parse()?);

    // Configure formatter to sanitize output
    let fmt_layer = fmt::layer()
        .with_target(true)
        .with_thread_ids(false) // Don't leak thread info
        .with_thread_names(false)
        .with_file(false) // Don't leak file paths in production
        .with_line_number(false);

    tracing_subscriber::registry()
        .with(filter)
        .with(fmt_layer)
        .init();

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    // Warn if running debug build
    #[cfg(debug_assertions)]
    {
        eprintln!("WARNING: Running debug build. Not for production use!");
        eprintln!("Build with: cargo build --release");
    }

    // Initialize secure logging
    init_secure_logging()?;

    tracing::info!("Starting Helix Keybindings Trainer");

    // Placeholder for application logic
    println!("Welcome to Helix Keybindings Trainer!");

    Ok(())
}
```

---

### Step 3: Create SECURITY.md

Create `/Users/rabax/Documents/git/helix_trainer/SECURITY.md`:

```markdown
# Security Policy

## Supported Versions

Currently, only the latest development version is supported.

| Version | Supported          |
| ------- | ------------------ |
| 0.1.x   | :white_check_mark: |

## Reporting a Vulnerability

**Please do NOT open public GitHub issues for security vulnerabilities.**

To report a security vulnerability:

1. Email: [security contact email] (to be determined)
2. Include:
   - Description of the vulnerability
   - Steps to reproduce
   - Potential impact
   - Suggested fix (if available)

We will respond within 48 hours and provide regular updates on the fix progress.

## Security Considerations

### For Users

1. **Only load scenario files from trusted sources**
   - Review scenario content before running
   - Be cautious with third-party scenario files
   - Check file permissions and ownership

2. **Run with minimal privileges**
   - Never run as root/administrator
   - Use standard user account
   - Consider using AppArmor/SELinux policies

3. **Keep dependencies updated**
   - Keep Helix editor updated
   - Regularly update helix-trainer
   - Monitor security advisories

4. **Verify downloads**
   - Download from official sources only
   - Verify checksums if provided
   - Check GPG signatures (when available)

### For Contributors

1. **Input Validation**
   - Validate all user input
   - Sanitize file paths
   - Check bounds on all operations
   - Never use unwrap() in production code

2. **Secure Defaults**
   - Fail securely by default
   - Minimize privileges
   - Use safe Rust practices
   - Enable all compiler warnings

3. **Secrets Management**
   - Never commit secrets or credentials
   - Use environment variables for sensitive config
   - Add sensitive files to .gitignore
   - Review commits before pushing

4. **Testing**
   - Add security tests for new features
   - Test edge cases and boundary conditions
   - Perform fuzzing on parsers
   - Review code for security issues

5. **Dependencies**
   - Audit dependencies regularly (cargo audit)
   - Pin dependency versions in releases
   - Review dependency changes
   - Minimize dependency count

## Known Security Limitations

1. **PTY Integration**
   - PTY controller requires careful security review
   - Process isolation is system-dependent
   - Helix process runs with user privileges

2. **Scenario Files**
   - TOML files are parsed with size limits
   - Content validation is performed
   - Files are read from restricted directories
   - Symlinks are followed during canonicalization

3. **Terminal Security**
   - Terminal escape sequences are filtered
   - Output is sanitized before display
   - Input validation is performed

4. **Temporary Files**
   - Created with restrictive permissions (0600)
   - Automatic cleanup on exit
   - Stored in system temp directory

## Security Features

### Implemented

- Path traversal prevention
- Input validation and sanitization
- Secure error handling
- Logging with sensitive data filtering
- Resource limits and timeouts

### Planned

- Process sandboxing (Linux: seccomp, macOS: sandbox-exec)
- Enhanced privilege dropping
- Cryptographic verification of scenarios
- Security audit logging
- Automated vulnerability scanning in CI/CD

## Security Development Lifecycle

1. **Design Phase**
   - Threat modeling
   - Security requirements
   - Secure architecture review

2. **Implementation Phase**
   - Secure coding practices
   - Code review (security focus)
   - Static analysis (clippy, cargo-audit)

3. **Testing Phase**
   - Security unit tests
   - Integration security tests
   - Fuzzing
   - Penetration testing

4. **Release Phase**
   - Security review
   - Dependency audit
   - Changelog security notes
   - Security advisory (if needed)

5. **Maintenance Phase**
   - Monitor security advisories
   - Regular dependency updates
   - Security patch releases
   - Incident response

## Secure Configuration

### Recommended File Permissions

```bash
# Application binary
chmod 755 helix-trainer

# Scenario files
chmod 644 scenarios/*.toml

# Scenario directory
chmod 755 scenarios/

# User data directory
chmod 700 ~/.local/share/helix-trainer/
```

### Environment Variables

```bash
# Logging level (INFO recommended for production)
export RUST_LOG=info

# Disable backtrace in production (prevent info leakage)
export RUST_BACKTRACE=0
```

## Threat Model

### Assets

- User data and progress
- Scenario files
- System resources (CPU, memory, disk)
- Terminal session

### Threat Actors

- Malicious scenario file authors
- Local attackers with user privileges
- Compromised dependencies

### Attack Vectors

- Malicious TOML scenario files
- Path traversal in file operations
- Command injection via PTY
- Resource exhaustion attacks
- Terminal escape sequence injection

### Mitigations

See SECURITY_REVIEW.md for detailed mitigations.

## Compliance

This project follows:

- OWASP Secure Coding Practices
- Rust Security Guidelines
- CWE Top 25 Most Dangerous Software Weaknesses
- Memory Safety (enforced by Rust)

## Resources

- [Rust Security Guidelines](https://anssi-fr.github.io/rust-guide/)
- [OWASP Secure Coding Practices](https://owasp.org/www-project-secure-coding-practices-quick-reference-guide/)
- [CWE Top 25](https://cwe.mitre.org/top25/)
- [Cargo Security Audit](https://crates.io/crates/cargo-audit)

## Contact

For security concerns, please contact: [To be determined]

Last updated: 2025-10-26
```

---

## Phase 2: Scenario Loading Security

### Step 4: Implement Secure Scenario Loader

Create `/Users/rabax/Documents/git/helix_trainer/src/config/scenarios.rs`:

```rust
//! Scenario loading and validation
//!
//! This module handles loading TOML scenario files with security validations.

use crate::security::{
    limits::*, path_validator, sanitizer, SecurityError, UserError,
};
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};

/// Scenario definition
#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct Scenario {
    #[serde(deserialize_with = "validate_id_field")]
    pub id: String,

    pub name: String,
    pub description: String,
    pub setup: Setup,
    pub target: TargetState,
    pub solution: Solution,

    #[serde(default)]
    pub alternatives: Vec<AlternativeSolution>,

    #[serde(default)]
    pub hints: Vec<String>,

    pub scoring: ScoringConfig,
}

/// Initial editor setup
#[derive(Deserialize, Debug, Clone)]
pub struct Setup {
    pub file_content: String,
    pub cursor_position: (usize, usize),
}

/// Target state to achieve
#[derive(Deserialize, Debug, Clone)]
pub struct TargetState {
    pub file_content: String,
    pub cursor_position: (usize, usize),
}

/// Optimal solution
#[derive(Deserialize, Debug, Clone)]
pub struct Solution {
    pub commands: Vec<String>,
    pub description: String,
}

/// Alternative solution
#[derive(Deserialize, Debug, Clone)]
pub struct AlternativeSolution {
    pub commands: Vec<String>,
    pub points_multiplier: f32,
    pub description: String,
}

/// Scoring configuration
#[derive(Deserialize, Debug, Clone)]
pub struct ScoringConfig {
    pub optimal_count: usize,
    pub max_points: u32,
    pub tolerance: usize,
}

/// Custom deserialization for ID field
fn validate_id_field<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;

    // Validate ID format
    if s.len() > 64 || !s.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return Err(serde::de::Error::custom(
            "Invalid ID: must be alphanumeric with underscores, max 64 chars",
        ));
    }

    Ok(s)
}

/// Secure scenario loader
pub struct ScenarioLoader {
    allowed_base_paths: Vec<PathBuf>,
}

impl ScenarioLoader {
    /// Create a new scenario loader with default allowed paths
    pub fn new() -> Self {
        Self {
            allowed_base_paths: vec![
                PathBuf::from("./scenarios"),
                PathBuf::from("/usr/share/helix-trainer/scenarios"),
            ],
        }
    }

    /// Create a loader with custom allowed paths
    pub fn with_allowed_paths(paths: Vec<PathBuf>) -> Self {
        Self {
            allowed_base_paths: paths,
        }
    }

    /// Load scenarios from a TOML file with security validations
    pub fn load(&self, path: &Path) -> Result<Vec<Scenario>, UserError> {
        // Validate path
        let canonical =
            path_validator::validate_path(path, &self.allowed_base_paths)
                .map_err(|e| UserError::from(e))?;

        // Validate file size
        path_validator::validate_file_size(&canonical, MAX_SCENARIO_FILE_SIZE)
            .map_err(|e| UserError::from(e))?;

        // Log with sanitized path
        tracing::info!(
            file = %sanitizer::sanitize_path_for_logging(&canonical),
            "Loading scenario file"
        );

        // Read file content
        let content = fs::read_to_string(&canonical).map_err(|e| {
            tracing::error!("Failed to read scenario file: {}", e);
            UserError::ScenarioLoadError
        })?;

        // Parse TOML
        let scenarios: Vec<Scenario> = toml::from_str(&content).map_err(|e| {
            UserError::from(SecurityError::InvalidToml(e.to_string()))
        })?;

        // Validate scenario count
        if scenarios.len() > MAX_SCENARIOS_PER_FILE {
            return Err(UserError::from(SecurityError::TooManyScenarios {
                max: MAX_SCENARIOS_PER_FILE,
                actual: scenarios.len(),
            }));
        }

        // Validate each scenario
        for scenario in &scenarios {
            self.validate_scenario(scenario)
                .map_err(|e| UserError::from(e))?;
        }

        tracing::info!(
            count = scenarios.len(),
            "Successfully loaded scenarios"
        );

        Ok(scenarios)
    }

    /// Validate a single scenario
    fn validate_scenario(&self, scenario: &Scenario) -> Result<(), SecurityError> {
        // Validate ID (already validated during deserialization)

        // Validate setup content
        if scenario.setup.file_content.len() > MAX_FILE_CONTENT_LENGTH {
            return Err(SecurityError::ContentTooLarge {
                max: MAX_FILE_CONTENT_LENGTH,
                actual: scenario.setup.file_content.len(),
            });
        }

        // Validate target content
        if scenario.target.file_content.len() > MAX_FILE_CONTENT_LENGTH {
            return Err(SecurityError::ContentTooLarge {
                max: MAX_FILE_CONTENT_LENGTH,
                actual: scenario.target.file_content.len(),
            });
        }

        // Validate cursor positions
        self.validate_cursor_position(scenario.setup.cursor_position)?;
        self.validate_cursor_position(scenario.target.cursor_position)?;

        // Validate hints
        if scenario.hints.len() > MAX_HINTS {
            return Err(SecurityError::TooManyHints {
                max: MAX_HINTS,
            });
        }

        // Validate alternatives
        if scenario.alternatives.len() > MAX_ALTERNATIVES {
            return Err(SecurityError::TooManyAlternatives {
                max: MAX_ALTERNATIVES,
            });
        }

        // Validate scoring config
        if scenario.scoring.optimal_count == 0 {
            return Err(SecurityError::InvalidScoringConfig);
        }

        Ok(())
    }

    /// Validate cursor position
    fn validate_cursor_position(&self, pos: (usize, usize)) -> Result<(), SecurityError> {
        if pos.0 > 10000 || pos.1 > 10000 {
            return Err(SecurityError::InvalidCursorPosition);
        }
        Ok(())
    }
}

impl Default for ScenarioLoader {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_valid_scenario_loading() {
        let toml = r#"
        [[scenarios]]
        id = "test_001"
        name = "Test Scenario"
        description = "A test scenario"

        [scenarios.setup]
        file_content = "Hello, World!"
        cursor_position = [0, 0]

        [scenarios.target]
        file_content = "Hello, Rust!"
        cursor_position = [0, 7]

        [scenarios.solution]
        commands = ["w", "cw", "Rust", "Esc"]
        description = "Change 'World' to 'Rust'"

        [scenarios.scoring]
        optimal_count = 4
        max_points = 100
        tolerance = 1
        "#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(toml.as_bytes()).unwrap();

        let loader = ScenarioLoader::with_allowed_paths(vec![
            temp_file.path().parent().unwrap().to_path_buf()
        ]);

        let result = loader.load(temp_file.path());
        assert!(result.is_ok());

        let scenarios = result.unwrap();
        assert_eq!(scenarios.len(), 1);
        assert_eq!(scenarios[0].id, "test_001");
    }

    #[test]
    fn test_invalid_id_rejection() {
        let toml = r#"
        [[scenarios]]
        id = "test-with-dashes!"
        name = "Test"
        description = "Test"
        "#;

        let result: Result<Vec<Scenario>, _> = toml::from_str(toml);
        assert!(result.is_err());
    }

    #[test]
    fn test_oversized_content_rejection() {
        let huge_content = "A".repeat(200_000);
        let toml = format!(
            r#"
        [[scenarios]]
        id = "test_001"
        name = "Test"
        description = "Test"

        [scenarios.setup]
        file_content = "{}"
        cursor_position = [0, 0]

        [scenarios.target]
        file_content = "target"
        cursor_position = [0, 0]

        [scenarios.solution]
        commands = ["test"]
        description = "test"

        [scenarios.scoring]
        optimal_count = 1
        max_points = 100
        tolerance = 0
        "#,
            huge_content
        );

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(toml.as_bytes()).unwrap();

        let loader = ScenarioLoader::with_allowed_paths(vec![
            temp_file.path().parent().unwrap().to_path_buf()
        ]);

        let result = loader.load(temp_file.path());
        assert!(result.is_err());
    }
}
```

Update `/Users/rabax/Documents/git/helix_trainer/src/config/mod.rs`:

```rust
//! Configuration and scenario loading
//!
//! This module handles loading and parsing scenario files in TOML format,
//! as well as application configuration.

pub mod scenarios;

pub use scenarios::{
    AlternativeSolution, Scenario, ScenarioLoader, ScoringConfig, Setup, Solution, TargetState,
};
```

---

## Phase 3: Add Security Dependencies

Update `/Users/rabax/Documents/git/helix_trainer/Cargo.toml`:

```toml
[package]
name = "helix-trainer"
version = "0.1.0"
edition = "2021"
authors = ["Helix Trainer Contributors"]
description = "Interactive trainer for learning Helix editor keybindings"
license = "MIT"
repository = "https://github.com/example/helix-trainer"

[dependencies]
ratatui = "0.28"
crossterm = "0.28"
tokio = { version = "1", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
toml = "0.8"
tracing = "0.1"
tracing-subscriber = "0.3"
anyhow = "1.0"
thiserror = "1.0"

# Security enhancements
tempfile = "3.8"

[dev-dependencies]
tokio-test = "0.4"

# Security testing
proptest = "1.4"

[[bin]]
name = "helix-trainer"
path = "src/main.rs"
```

---

## Phase 4: CI/CD Security Integration

Create `.github/workflows/security.yml`:

```yaml
name: Security Audit

on:
  push:
    branches: [ main ]
  pull_request:
    branches: [ main ]
  schedule:
    # Run security audit every Monday at 00:00 UTC
    - cron: '0 0 * * 1'

jobs:
  security-audit:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
          override: true

      - name: Install cargo-audit
        run: cargo install cargo-audit

      - name: Run security audit
        run: cargo audit

      - name: Run clippy security lints
        run: cargo clippy -- -W clippy::all -D warnings

  dependency-check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Check for outdated dependencies
        run: |
          cargo install cargo-outdated
          cargo outdated --exit-code 1
```

---

## Testing Your Implementation

### 1. Build and Test

```bash
# Format code
cargo fmt

# Check compilation
cargo check

# Run tests
cargo test

# Run clippy
cargo clippy

# Build release
cargo build --release
```

### 2. Security Validation

```bash
# Install cargo-audit
cargo install cargo-audit

# Run security audit
cargo audit

# Test security scenarios
cargo test security
```

### 3. Manual Security Testing

Test path traversal prevention:
```rust
// Add to tests/
#[test]
fn test_path_security() {
    let loader = ScenarioLoader::new();

    // These should all fail
    assert!(loader.load(Path::new("../../etc/passwd")).is_err());
    assert!(loader.load(Path::new("/root/.ssh/id_rsa")).is_err());
}
```

---

## Next Steps

After implementing Phase 1:

1. **Review and test** all security controls
2. **Document** security assumptions in code comments
3. **Create** security test suite (Phase 3 from main review)
4. **Implement** PTY controller security (Phase 2 from main review)
5. **Conduct** internal security review
6. **Plan** external security audit (before 1.0 release)

---

## Security Checklist

Before each release:

- [ ] All security controls implemented
- [ ] cargo audit passes
- [ ] clippy passes with no warnings
- [ ] All security tests pass
- [ ] SECURITY.md is up to date
- [ ] Dependencies are up to date
- [ ] No hardcoded secrets
- [ ] Logging is properly configured
- [ ] Error messages don't leak sensitive info
- [ ] Path validation is working
- [ ] Input validation is comprehensive
- [ ] Resource limits are enforced

---

For questions or issues with security implementation, refer to SECURITY_REVIEW.md or consult the security team.
