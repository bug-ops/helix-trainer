# Security Quick Reference Guide
## For Helix Keybindings Trainer Developers

This is a quick reference for security best practices when developing for this project.

---

## Critical Security Rules

### 🚨 NEVER Do These

```rust
// ❌ NEVER use unwrap() in production code
let value = risky_operation().unwrap();

// ❌ NEVER trust external input without validation
let path = Path::new(&user_input);
std::fs::read(path)?;

// ❌ NEVER use unchecked indexing
let item = list[user_index];

// ❌ NEVER ignore errors silently
let _ = important_operation();

// ❌ NEVER log sensitive information
tracing::debug!("User password: {}", password);

// ❌ NEVER use shell commands with user input
Command::new("sh").arg("-c").arg(format!("cat {}", user_file));

// ❌ NEVER hard-code secrets
const API_KEY: &str = "secret123";

// ❌ NEVER disable security checks
#[allow(unsafe_code)]
unsafe { /* dangerous code */ }
```

### ✅ ALWAYS Do These

```rust
// ✅ ALWAYS handle errors properly
let value = risky_operation()
    .map_err(|e| SecurityError::OperationFailed)?;

// ✅ ALWAYS validate external input
let path = validate_path(&user_input)?;
std::fs::read(path)?;

// ✅ ALWAYS check bounds
let item = list.get(user_index)
    .ok_or(SecurityError::IndexOutOfBounds)?;

// ✅ ALWAYS propagate errors
important_operation()?;

// ✅ ALWAYS sanitize logs
tracing::debug!("User authenticated: {}", sanitize_username(user));

// ✅ ALWAYS use direct commands
Command::new("cat").arg(validated_path);

// ✅ ALWAYS use environment variables
let api_key = std::env::var("API_KEY")?;

// ✅ ALWAYS use safe Rust
fn safe_function() -> Result<()> { /* safe code */ }
```

---

## Common Patterns

### Pattern 1: Loading Files Safely

```rust
use crate::security::{path_validator, limits::*};

pub fn load_file(user_path: &Path) -> Result<String, UserError> {
    // 1. Validate path
    let allowed_dirs = vec![PathBuf::from("./scenarios")];
    let validated_path = path_validator::validate_path(user_path, &allowed_dirs)
        .map_err(|e| UserError::from(e))?;

    // 2. Check file size
    path_validator::validate_file_size(&validated_path, MAX_SCENARIO_FILE_SIZE)
        .map_err(|e| UserError::from(e))?;

    // 3. Read file
    let content = fs::read_to_string(&validated_path)
        .map_err(|_| UserError::ScenarioLoadError)?;

    // 4. Sanitize content
    let sanitized = sanitizer::sanitize_content(&content)
        .map_err(|e| UserError::from(e))?;

    Ok(sanitized)
}
```

### Pattern 2: Parsing User Input Safely

```rust
pub fn parse_cursor_position(input: &str) -> Result<(usize, usize), SecurityError> {
    // 1. Validate input length
    if input.len() > 100 {
        return Err(SecurityError::InvalidInput);
    }

    // 2. Parse with error handling
    let parts: Vec<&str> = input.split(',').collect();
    if parts.len() != 2 {
        return Err(SecurityError::InvalidFormat);
    }

    // 3. Parse numbers with bounds checking
    let row = parts[0]
        .trim()
        .parse::<usize>()
        .map_err(|_| SecurityError::InvalidNumber)?;

    let col = parts[1]
        .trim()
        .parse::<usize>()
        .map_err(|_| SecurityError::InvalidNumber)?;

    // 4. Validate range
    if row > 10000 || col > 10000 {
        return Err(SecurityError::InvalidCursorPosition);
    }

    Ok((row, col))
}
```

### Pattern 3: Safe Arithmetic

```rust
pub fn calculate_score(optimal: usize, actual: usize, max: u32) -> Result<u32, SecurityError> {
    // 1. Validate inputs
    if optimal == 0 {
        return Err(SecurityError::InvalidScoringConfig);
    }

    if actual > 1_000_000 {
        return Err(SecurityError::TooManyActions);
    }

    // 2. Use checked arithmetic
    let score = if actual <= optimal {
        max
    } else {
        let numerator = (max as u64)
            .checked_mul(optimal as u64)
            .ok_or(SecurityError::ScoreOverflow)?;

        let result = numerator
            .checked_div(actual as u64)
            .ok_or(SecurityError::ScoreOverflow)?;

        result.min(u32::MAX as u64) as u32
    };

    Ok(score)
}
```

### Pattern 4: Spawning Processes Safely

```rust
pub fn spawn_helix(file_path: &Path) -> Result<Child, SecurityError> {
    // 1. Validate file path
    let validated = validate_temp_file_path(file_path)?;

    // 2. Spawn with minimal environment
    let child = Command::new("helix")
        .arg(&validated)
        // Security: Clear environment
        .env_clear()
        // Set minimal safe environment
        .env("PATH", "/usr/local/bin:/usr/bin:/bin")
        .env("TERM", "xterm-256color")
        // Redirect streams
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| SecurityError::ProcessSpawnFailed(e.to_string()))?;

    Ok(child)
}
```

### Pattern 5: Secure Logging

```rust
pub fn log_operation(path: &Path, user_id: &str) {
    // ❌ BAD: Leaks full path and user ID
    tracing::info!("User {} loaded file {}", user_id, path.display());

    // ✅ GOOD: Sanitized logging
    tracing::info!(
        user = %sanitize_user_id(user_id),
        file = %sanitize_path_for_logging(path),
        "Scenario loaded"
    );
}

fn sanitize_user_id(id: &str) -> String {
    // Hash or truncate user ID
    format!("user_{}", &id.chars().take(4).collect::<String>())
}

fn sanitize_path_for_logging(path: &Path) -> String {
    // Only show filename
    path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("[redacted]")
        .to_string()
}
```

### Pattern 6: Resource Management with RAII

```rust
pub struct SessionGuard {
    limiter: Arc<Mutex<usize>>,
}

impl Drop for SessionGuard {
    fn drop(&mut self) {
        // Automatically release resource
        let mut count = self.limiter.lock().unwrap();
        *count = count.saturating_sub(1);
    }
}

pub fn create_session() -> Result<SessionGuard, SecurityError> {
    let mut count = LIMITER.lock().unwrap();

    if *count >= MAX_SESSIONS {
        return Err(SecurityError::TooManySessions { max: MAX_SESSIONS });
    }

    *count += 1;
    Ok(SessionGuard { limiter: LIMITER.clone() })
}

// Usage - automatic cleanup on drop
fn run_session() -> Result<()> {
    let _guard = create_session()?;
    // Session automatically released when _guard drops
    do_work()?;
    Ok(())
}
```

---

## Security Checklist for PRs

Before submitting a pull request:

### Code Quality
- [ ] No unwrap() or expect() in production code paths
- [ ] All errors properly handled and propagated
- [ ] Bounds checking on all array/vector access
- [ ] Arithmetic operations use checked variants
- [ ] Input validation on all external data

### Security
- [ ] Path traversal prevention implemented
- [ ] Command injection prevention verified
- [ ] Resource limits enforced
- [ ] Sensitive data not logged
- [ ] Error messages don't leak internal details

### Testing
- [ ] Unit tests added for new functionality
- [ ] Security tests added for security-critical code
- [ ] Edge cases tested (0, MAX, overflow, etc.)
- [ ] Error paths tested

### Documentation
- [ ] Security considerations documented
- [ ] Unsafe blocks justified (if any)
- [ ] API documentation updated
- [ ] Examples use secure patterns

### Dependencies
- [ ] New dependencies justified
- [ ] Dependencies from trusted sources
- [ ] cargo audit passes
- [ ] Minimal dependency versions used

---

## Quick Security Tests

### Test Path Traversal

```rust
#[test]
fn test_path_traversal_prevention() {
    let malicious_paths = vec![
        "../../etc/passwd",
        "/etc/passwd",
        "../../../root/.ssh/id_rsa",
        "scenarios/../../../secret",
    ];

    for path in malicious_paths {
        assert!(validate_path(Path::new(path)).is_err());
    }
}
```

### Test Input Validation

```rust
#[test]
fn test_malicious_input_rejection() {
    let malicious_inputs = vec![
        "$(malicious)",
        "`backdoor`",
        "normal; rm -rf /",
        "../../etc/passwd",
        "\0null_byte",
    ];

    for input in malicious_inputs {
        assert!(validate_input(input).is_err());
    }
}
```

### Test Resource Limits

```rust
#[test]
fn test_resource_limits() {
    // Test large content
    let huge = "A".repeat(1_000_000);
    assert!(validate_content(&huge).is_err());

    // Test many items
    let many_scenarios = vec![Scenario::default(); 1000];
    assert!(validate_scenario_count(&many_scenarios).is_err());
}
```

### Test Arithmetic Safety

```rust
#[test]
fn test_arithmetic_overflow() {
    // Should not panic
    let result = calculate_score(1, usize::MAX, u32::MAX);
    assert!(result.is_ok() || result.is_err());

    // Should handle correctly
    let result = calculate_score(0, 100, 100);
    assert!(result.is_err()); // Division by zero prevention
}
```

---

## Common Vulnerabilities to Avoid

### 1. Path Traversal
```rust
// ❌ VULNERABLE
fn load(path: &str) -> String {
    std::fs::read_to_string(path).unwrap()
}

// ✅ SECURE
fn load(path: &str) -> Result<String> {
    let validated = validate_path(Path::new(path), &ALLOWED_DIRS)?;
    std::fs::read_to_string(validated)
        .map_err(|_| UserError::LoadFailed)
}
```

### 2. Command Injection
```rust
// ❌ VULNERABLE
fn run_helix(file: &str) {
    Command::new("sh")
        .arg("-c")
        .arg(format!("helix {}", file))
        .spawn()
        .unwrap();
}

// ✅ SECURE
fn run_helix(file: &Path) -> Result<Child> {
    let validated = validate_path(file, &ALLOWED_DIRS)?;
    Command::new("helix")
        .arg(validated)
        .env_clear()
        .spawn()
        .map_err(|e| SecurityError::ProcessFailed)
}
```

### 3. Integer Overflow
```rust
// ❌ VULNERABLE
fn calculate(a: usize, b: usize) -> usize {
    a * b
}

// ✅ SECURE
fn calculate(a: usize, b: usize) -> Result<usize> {
    a.checked_mul(b)
        .ok_or(SecurityError::Overflow)
}
```

### 4. Resource Exhaustion
```rust
// ❌ VULNERABLE
fn load_all_scenarios(paths: Vec<PathBuf>) -> Vec<Scenario> {
    paths.iter()
        .flat_map(|p| load_scenarios(p))
        .collect()
}

// ✅ SECURE
fn load_all_scenarios(paths: Vec<PathBuf>) -> Result<Vec<Scenario>> {
    if paths.len() > MAX_SCENARIO_FILES {
        return Err(SecurityError::TooManyFiles);
    }

    let scenarios: Result<Vec<_>> = paths.iter()
        .take(MAX_SCENARIO_FILES)
        .map(|p| load_scenarios(p))
        .collect();

    let all_scenarios = scenarios?;

    if all_scenarios.len() > MAX_TOTAL_SCENARIOS {
        return Err(SecurityError::TooManyScenarios);
    }

    Ok(all_scenarios)
}
```

### 5. Information Disclosure
```rust
// ❌ VULNERABLE
fn handle_error(e: Error) {
    println!("Error: {:?}", e); // Leaks internal details
}

// ✅ SECURE
fn handle_error(e: SecurityError) -> UserError {
    tracing::error!("Internal error: {:?}", e);
    UserError::from(e) // Return sanitized error
}
```

---

## Security Commands

### During Development
```bash
# Check code
cargo check

# Run all tests
cargo test

# Security lints
cargo clippy -- -W clippy::all -D warnings

# Format
cargo fmt
```

### Before Commit
```bash
# Audit dependencies
cargo audit

# Check for outdated deps
cargo outdated

# Run security tests
cargo test security

# Build release
cargo build --release
```

### Before Release
```bash
# Full security check
cargo audit
cargo clippy -- -D warnings
cargo test
cargo build --release

# Review security docs
cat SECURITY_REVIEW.md
cat SECURITY.md
```

---

## Getting Help

If you're unsure about security:

1. Check this quick reference
2. Review `SECURITY_REVIEW.md` for detailed guidance
3. Review `SECURITY_IMPLEMENTATION_GUIDE.md` for examples
4. Ask in PR comments for security review
5. Consult security team for critical features

---

## Security Resources

- [Rust Security Guidelines](https://anssi-fr.github.io/rust-guide/)
- [OWASP Cheat Sheets](https://cheatsheetseries.owasp.org/)
- [Cargo Audit](https://github.com/rustsec/rustsec)
- [Clippy Lints](https://rust-lang.github.io/rust-clippy/)

---

**Remember**: Security is everyone's responsibility. When in doubt, ask for review!
