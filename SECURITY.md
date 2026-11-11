# Security Policy

## Supported Versions

Only the latest minor release line receives security fixes.

| Version | Supported |
|---------|-----------|
| 0.6.x   | Yes |
| < 0.6   | No |

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

- **No first-party `unsafe` code** — `unsafe_code = "forbid"` in the `Cargo.toml` lints table; third-party dependencies are outside this guarantee
- **Path validation for scenario and quest files** — directory loaders canonicalize paths and check them against an allow-list
- **Bounded file reads** — every file read into memory (profile, profile lock, config, keymap, scenario and quest files) goes through a size-capped reader that rejects non-regular files such as FIFOs and device nodes
- **Local-only data** — No network, no telemetry, no cloud sync
- **Owner-only persisted data** — on Unix the data directory is created `0700` and files `0600`; other platforms use process defaults

### Implemented Protections

| Protection | Implementation |
|------------|----------------|
| Path traversal | Canonical path validation in the scenario and quest directory loaders |
| Content limits | Size-capped reads, scenario count and field length limits |
| Input validation | Scenario, quest and keymap fields validated at load time |
| Error handling | User-facing errors are sanitized and carry no file paths |
| Dependencies | `cargo deny check` (advisories and licenses) in CI |

## For Users

1. **Only load scenarios from trusted sources**
2. **Run with standard user privileges** (never as root)
3. **Verify downloads** via SHA256 checksums
4. **Keep updated** — use latest release

### Recommended Permissions

```bash
chmod 755 helix-trainer
chmod 700 ~/.config/helix-trainer/   # Linux and macOS
```

## For Contributors

> [!IMPORTANT]
> All code must pass `cargo clippy --all-targets --all-features -- -D warnings` and `cargo deny check`.

Requirements:

- Validate all user input
- Sanitize file paths
- Use `?` operator instead of `.unwrap()`
- Add security tests for new features
- No secrets in commits

## Known Limitations

1. **Bundled scenarios** — Scenarios and quests are compiled into the binary; the directory loaders exist for development and are not reachable from the shipped application
2. **Keymap file** — The optional Helix `config.toml` keymap is read from the user's own config directory without an allow-list check (size-capped, regular files only)
3. **Terminal I/O** — Raw mode requires terminal access
4. **User Data** — Stored unencrypted in `~/.config/helix-trainer/` on Linux and macOS, and in `helix-trainer/` under `%AppData%` on Windows
5. **Native dependency** — The `syntect` syntax highlighter is built with the `regex-onig` backend, which links the C Oniguruma library into the binary

## CI Security Checks

Every PR runs:

- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo deny check` (license and vulnerability audit)
- All tests must pass

## Resources

- [Rust Security Guidelines](https://anssi-fr.github.io/rust-guide/)
- [cargo-audit](https://crates.io/crates/cargo-audit)
- [cargo-deny](https://crates.io/crates/cargo-deny)
