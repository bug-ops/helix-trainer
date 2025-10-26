# Contributing to Helix Keybindings Trainer

Thank you for your interest in contributing! This document outlines the development workflow and guidelines.

## Table of Contents

- [Development Workflow](#development-workflow)
- [Git Workflow](#git-workflow)
- [Code Standards](#code-standards)
- [Testing](#testing)
- [Pull Request Process](#pull-request-process)
- [Commit Message Guidelines](#commit-message-guidelines)

## Development Workflow

### Prerequisites

- Rust toolchain (stable and nightly)
- Git
- A terminal emulator with good Unicode support

### Setup

```bash
# Clone the repository
git clone https://github.com/yourusername/helix-trainer.git
cd helix-trainer

# Install dependencies
cargo build

# Run tests
cargo test --lib

# Run the application
cargo run --release
```

## Git Workflow

We follow a **feature branch workflow** with pull requests:

### 1. Create a Feature Branch

```bash
# Update main branch
git checkout main
git pull origin main

# Create a new feature branch
git checkout -b feature/your-feature-name

# Or for bug fixes
git checkout -b fix/bug-description
```

### Branch Naming Convention

- `feature/` - New features (e.g., `feature/add-word-selection-commands`)
- `fix/` - Bug fixes (e.g., `fix/cursor-position-off-by-one`)
- `docs/` - Documentation updates (e.g., `docs/update-readme`)
- `refactor/` - Code refactoring (e.g., `refactor/simplify-scorer`)
- `test/` - Test improvements (e.g., `test/add-integration-tests`)
- `chore/` - Maintenance tasks (e.g., `chore/update-dependencies`)

### 2. Make Changes

```bash
# Make your changes
# Run tests frequently
cargo nextest run --lib

# Check formatting
cargo +nightly fmt

# Check for lints
cargo clippy -- -D warnings

# Run security audit
cargo deny check
```

### 3. Commit Changes

Follow [conventional commits](https://www.conventionalcommits.org/):

```bash
git add .
git commit -m "feat: add word selection commands (w, b, e)"
git commit -m "fix: correct cursor position after deletion"
git commit -m "docs: update README with new commands"
```

### 4. Push and Create PR

```bash
# Push your branch
git push origin feature/your-feature-name

# Create a pull request on GitHub
# Fill out the PR template completely
```

### 5. Code Review Process

- All PRs require at least one approval
- CI checks must pass (tests, clippy, fmt, security audit)
- Address review comments
- Keep PR scope focused and small

### 6. Merge

Once approved and CI passes:
- Squash and merge (preferred for feature branches)
- Regular merge (for release branches)

## Code Standards

### Rust Style

- Use `cargo +nightly fmt` for formatting
- Follow [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- Zero warnings: `cargo clippy -- -D warnings` must pass
- Document public APIs with `///` doc comments

### Code Organization

```rust
// Module structure
src/
├── main.rs          // Entry point, minimal logic
├── lib.rs           // Public API
├── ui/              // TUI components (pure rendering)
├── game/            // Game logic (state, scoring)
├── helix/           // Helix integration
└── config/          // Configuration and scenarios

// Each module should have:
// - Clear single responsibility
// - Comprehensive unit tests
// - Documentation for public items
```

### Security

- All user input must be validated
- Use safe arithmetic (checked operations)
- No `unsafe` blocks without detailed justification
- Run `cargo deny check` regularly

### Comments

```rust
// Good: Explain WHY, not WHAT
// Using helix-core's Selection to handle multi-cursor operations
let selection = Selection::point(cursor_pos);

// Bad: Redundant
// Create a new selection
let selection = Selection::point(cursor_pos);
```

## Testing

### Running Tests

```bash
# Run all library tests
cargo nextest run --lib

# Run specific test
cargo nextest run --lib test_name

# Run tests with output
cargo nextest run --lib --nocapture

# Run tests in release mode (faster)
cargo nextest run --lib --release
```

### Test Guidelines

1. **Coverage**: Aim for high test coverage (currently 128 tests)
2. **Isolation**: Each test should be independent
3. **Clarity**: Test names should describe the scenario
4. **Assertions**: Use descriptive assertion messages

```rust
#[test]
fn test_delete_line_reduces_line_count() {
    let mut sim = HelixSimulator::new("line1\nline2\nline3".to_string());
    sim.execute_command("dd").unwrap();

    let state = sim.get_state().unwrap();
    assert_eq!(
        state.content().lines().count(),
        2,
        "After deleting one line, should have 2 lines remaining"
    );
}
```

### Test Organization

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_functionality() { }

    #[test]
    fn test_edge_case() { }

    #[test]
    #[should_panic(expected = "specific error")]
    fn test_error_handling() { }
}
```

## Pull Request Process

### Before Submitting

- [ ] All tests pass: `cargo nextest run --lib`
- [ ] Code formatted: `cargo +nightly fmt`
- [ ] No clippy warnings: `cargo clippy -- -D warnings`
- [ ] Security audit clean: `cargo deny check`
- [ ] Documentation updated (if applicable)
- [ ] CLAUDE.md updated (if architecture changed)

### PR Description

Use the PR template to provide:
- Clear description of changes
- Type of change (bug fix, feature, etc.)
- Related issues (if any)
- Testing performed
- Screenshots/demo (if UI changes)

### Review Checklist

Reviewers will check:
- Code quality and style
- Test coverage
- Documentation completeness
- Security considerations
- Performance implications

### CI Checks

All PRs must pass:

- **Tests**: All platforms (Linux, macOS, Windows) using `cargo-nextest`
- **Formatting**: `cargo +nightly fmt -- --check`
- **Lints**: `cargo clippy -- -D warnings`
- **Security**: `cargo deny check` (vulnerabilities + licenses)
- **Build**: Release build on all platforms

## Commit Message Guidelines

### Format

```
<type>(<scope>): <subject>

<body>

<footer>
```

### Types

- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation changes
- `style`: Formatting, missing semicolons, etc.
- `refactor`: Code restructuring
- `test`: Adding or updating tests
- `chore`: Maintenance tasks
- `perf`: Performance improvements

### Scope (Optional)

- `ui`: TUI components
- `game`: Game logic
- `helix`: Helix integration
- `config`: Configuration
- `ci`: CI/CD changes

### Examples

```bash
feat(helix): add word selection commands (w, b, e)

Implemented three word movement commands using helix-core's
Movement::NextWordStart and related primitives.

Closes #42

---

fix(ui): correct cursor position rendering

The cursor was off by one column due to incorrect grapheme
boundary handling. Fixed by using helix-core's grapheme utilities.

---

docs: update README with installation instructions

Added detailed setup steps for Linux, macOS, and Windows.
Included troubleshooting section for common issues.
```

## Adding New Scenarios

See [.github/ISSUE_TEMPLATE/scenario_request.md](.github/ISSUE_TEMPLATE/scenario_request.md) for the template.

### Scenario Guidelines

1. **Clear objective**: User should understand the goal immediately
2. **Realistic content**: Use code/text that looks natural
3. **Optimal solution**: Should be the idiomatic Helix way
4. **Difficulty appropriate**: Match the skill level
5. **Well-tested**: Verify the scenario works as expected

### Scenario File Location

- `scenarios/basic.toml` - Beginner level
- `scenarios/intermediate.toml` - Intermediate level
- `scenarios/advanced.toml` - Advanced level

## Questions?

- Open an issue for questions
- Check existing issues and PRs
- Review [CLAUDE.md](CLAUDE.md) for architecture details

## License

By contributing, you agree that your contributions will be licensed under the project's license.
