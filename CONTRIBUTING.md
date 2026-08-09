# Contributing to Helix Trainer

Thank you for your interest in contributing! This document provides guidelines for contributing to the project.

## Development Workflow

We follow a strict PR-based workflow. All changes go through:

1. Feature branch from `main`
2. Full check pipeline (format, tests, clippy, build)
3. Pull request with CI checks
4. Code review
5. Merge only when green

## Quick Start

```bash
# Clone repository
git clone https://github.com/bug-ops/helix-trainer.git
cd helix-trainer

# Create feature branch
git checkout -b feature/amazing-feature
```

## Pre-Commit Checks

> [!IMPORTANT]
> All PRs must pass these checks. CI enforces them automatically with zero tolerance for warnings.

**Always run before pushing:**

```bash
# 1. Format (requires nightly)
cargo +nightly fmt

# 2. Tests (fast parallel runner)
cargo nextest run

# 3. Lints (zero warnings policy)
cargo clippy --all-targets --all-features -- -D warnings

# 4. Build verification
cargo build --release
```

> [!TIP]
> With sccache configured, rebuilds are 5x faster (~10s vs ~54s).

## Commit Guidelines

We use [Conventional Commits](https://www.conventionalcommits.org/):

| Prefix | Purpose | Example |
|--------|---------|---------|
| `feat:` | New feature | `feat: add streak multiplier` |
| `fix:` | Bug fix | `fix: correct XP calculation` |
| `docs:` | Documentation | `docs: update installation guide` |
| `refactor:` | Code refactoring | `refactor: simplify command handler` |
| `test:` | Add/update tests | `test: add fsrs scheduler tests` |
| `chore:` | Maintenance | `chore: update dependencies` |
| `ci:` | CI/CD changes | `ci: add ARM64 builds` |

## Pull Request Process

1. Ensure all checks pass locally
2. Update documentation if needed
3. Add tests for new functionality
4. Write a clear PR description
5. Link related issues
6. Wait for CI to pass
7. Request review

## Code Style

- Follow idiomatic Rust patterns
- Use `rustfmt` with nightly (enforced by CI)
- Keep functions focused and small
- Prefer explicit error handling over `.unwrap()`
- Add doc comments for public APIs

> [!CAUTION]
> No `unsafe` code without explicit justification and approval.

## Adding Scenarios

Scenarios are defined in `scenarios/en/*.toml`. Each scenario must have:

```toml
[[scenarios]]
id = "unique_id"
name = "Human readable name"
description = "Clear description of what to achieve"

[scenarios.setup]
file_content = "initial text"
cursor_position = [0, 0]
# language = "rs"           # Optional: file-extension-style token for syntax
                             # highlighting (e.g. "md", "py"); defaults to "rs"

[scenarios.target]
file_content = "expected result"
cursor_position = [0, 0]

[scenarios.solution]
commands = ["command", "sequence"]

[scenarios.metadata]
category = "Movement"  # Movement, Editing, Clipboard, Selection, Search, Advanced
difficulty = "Beginner"  # Beginner, Intermediate, Advanced
tags = ["relevant", "tags"]
commands_taught = ["w", "e"]
```

Run `cargo nextest run scenario` to validate all scenarios.

## Testing

- Unit tests: `cargo nextest run`
- Specific test: `cargo nextest run test_name`
- With coverage: `cargo llvm-cov`

## Getting Help

- Open an [issue](https://github.com/bug-ops/helix-trainer/issues) for bugs or feature requests
- Check existing issues before creating new ones
- Use discussion threads for questions

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
