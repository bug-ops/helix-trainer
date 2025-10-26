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
