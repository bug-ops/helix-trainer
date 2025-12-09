# Security Policy

## Supported Versions

| Version | Supported |
|---------|-----------|
| 0.4.x   | Yes |
| < 0.4   | No |

## Reporting a Vulnerability

> [!CAUTION]
> **Do NOT open public GitHub issues for security vulnerabilities.**

To report a security vulnerability:

1. Open a private security advisory via GitHub (Settings → Security → Advisories)
2. Or email the maintainers directly

Include:

- Description of the vulnerability
- Steps to reproduce
- Potential impact
- Suggested fix (if available)

We will respond within 48 hours.

## Security Guarantees

### What We Protect

- **No `unsafe` code** — Entire codebase is safe Rust
- **Path traversal prevention** — All file operations are validated
- **Input validation** — All TOML parsing with size limits
- **Local-only data** — No network, no telemetry, no cloud sync
- **Secure defaults** — Restrictive file permissions

### Implemented Protections

| Protection | Implementation |
|------------|----------------|
| Path traversal | Canonical path validation |
| Content limits | Max file size, scenario count limits |
| Input sanitization | All user input validated |
| Error handling | No sensitive data in errors |
| Dependencies | Regular `cargo audit` checks |

## For Users

1. **Only load scenarios from trusted sources**
2. **Run with standard user privileges** (never as root)
3. **Verify downloads** via SHA256 checksums
4. **Keep updated** — use latest release

### Recommended Permissions

```bash
chmod 755 helix-trainer
chmod 700 ~/.config/helix-trainer/
```

## For Contributors

> [!IMPORTANT]
> All code must pass `cargo clippy -- -D warnings` and `cargo audit`.

Requirements:

- Validate all user input
- Sanitize file paths
- Use `?` operator instead of `.unwrap()`
- Add security tests for new features
- No secrets in commits

## Known Limitations

1. **Scenario Files** — TOML files are parsed from restricted directories only
2. **Terminal I/O** — Raw mode requires terminal access
3. **User Data** — Stored unencrypted in `~/.config/helix-trainer/`

## CI Security Checks

Every PR runs:

- `cargo clippy -- -D warnings`
- `cargo deny check` (license and vulnerability audit)
- All tests must pass

## Resources

- [Rust Security Guidelines](https://anssi-fr.github.io/rust-guide/)
- [cargo-audit](https://crates.io/crates/cargo-audit)
- [cargo-deny](https://crates.io/crates/cargo-deny)
