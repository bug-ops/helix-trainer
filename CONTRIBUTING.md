# Contributing to Helix Trainer

Thank you for your interest in contributing! This document provides guidelines for contributing to the project.

## Development Workflow

We follow a strict PR-based workflow. All changes go through:

1. Feature branch from `main`
2. Full check pipeline (format, tests, clippy, build)
3. Pull request with CI checks
4. Code review
5. Merge only when green

### Pre-Commit Checks

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

With sccache configured, rebuilds are 5x faster (~10s vs ~54s).

### Quick Guidelines

- Fork the repository
- Create feature branch (`git checkout -b feature/amazing-feature`)
- Make changes and add tests
- Run full check pipeline
- Commit with conventional commits (`feat:`, `fix:`, `docs:`)
- Push and create Pull Request
- Wait for CI checks to pass

See [CONTRIBUTING.md](CONTRIBUTING.md) for detailed guidelines.

</details>

---

## Releases

### Download Latest Release

**Latest version**: [v0.1.0](https://github.com/bug-ops/helix-trainer/releases/latest) (Phase 1 - Smart Learning & Gamification)

**Supported Platforms**:

- Linux x86_64 (GNU and musl)
- Linux ARM64 (aarch64 GNU and musl)
- macOS x86_64 (Intel)
- macOS ARM64 (Apple Silicon M1/M2/M3)
- Windows x86_64
- Windows ARM64

Each release includes:

- Pre-built binary
- README and documentation
- LICENSE file
- CHANGELOG with release notes
- SHA256 checksums for verification

**Release Schedule**: We follow semantic versioning (MAJOR.MINOR.PATCH)

- Major releases: Breaking changes or major new features
- Minor releases: New features, backward compatible
- Patch releases: Bug fixes and improvements

See [CHANGELOG.md](CHANGELOG.md) for detailed release history.

### Creating a Release (Maintainers)

Releases are automated via GitHub Actions:

1. Update version in `Cargo.toml`
2. Update `CHANGELOG.md` with release notes
3. Commit changes: `git commit -m "chore: prepare v0.2.0 release"`
4. Create and push tag: `git tag v0.2.0 && git push origin v0.2.0`
5. GitHub Actions will automatically:
   - Validate version consistency
   - Build binaries for all platforms
   - Generate SHA256 checksums
   - Create GitHub release
   - Upload all artifacts

**Workflow**: `.github/workflows/release.yml`

---

## Documentation

- [CHANGELOG.md](CHANGELOG.md) - Release history and version notes
- [CLAUDE.md](CLAUDE.md) - Project overview, tech stack, development workflow
