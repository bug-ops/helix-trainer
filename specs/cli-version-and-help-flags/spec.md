---
aliases:
  - CLI Version and Help Flags
  - helix-trainer --version
tags:
  - sdd
  - spec
  - cli
  - bug-fix
  - status/proposed
created: 2026-11-10
status: proposed
related:
  - "[[constitution]]"
  - "[[data-and-runtime-hardening/spec]]"
  - "[[MOC-specs]]"
---

# Feature: `--version` and `--help` Command-Line Flags

> [!info] Metadata
> **Issue**: #452
> **Status**: Proposed
> **Depth**: Lightweight bug-fix spec, per this project's SDD scaling guidance.

## 1. Overview

### Problem Statement

The binary ignores all command-line arguments. `helix-trainer --version`, `--help` or an unknown flag starts the full-screen TUI, and fails with `Error: Device not configured (os error 6)` when no terminal is attached. The issue template asks reporters for the output of `helix-trainer --version`, which cannot be produced.

### Goal

`--version` and `--help` print text and exit 0 without touching the terminal; unknown arguments fail fast with a clear error; no arguments start the TUI as today.

### Out of Scope

- Any other flag (config path override, log level, data directory)
- Adding a command-line parsing dependency (constitution VII: no new dependencies without justification; the grammar is two flags)
- Subcommands

## 2. Functional Requirements

| ID | Requirement | Priority |
|----|------------|----------|
| FR-001 | WHEN the first argument is `--version` or `-V` THE SYSTEM SHALL print `helix-trainer <version>` (version from `CARGO_PKG_VERSION`) to stdout, exit 0, and NOT set up the terminal | must |
| FR-002 | WHEN the first argument is `--help` or `-h` THE SYSTEM SHALL print usage (name, one-line description, the flags above) to stdout, exit 0, and NOT set up the terminal | must |
| FR-003 | WHEN any other argument is present THE SYSTEM SHALL print an error naming the argument and a hint to use `--help` to stderr, exit with a non-zero code (2), and NOT set up the terminal | must |
| FR-004 | WHEN no arguments are given THE SYSTEM SHALL start the TUI exactly as before | must |
| FR-005 | THE SYSTEM SHALL parse arguments into a closed typed result (`CliAction::{Run, Version, Help, Error(String)}`) before any terminal, config or data-loader work, and the parser SHALL be a pure function of the argument list | must |
| FR-006 | WHEN `--version` or `--help` runs without a TTY (stdin redirected from `/dev/null`) THE SYSTEM SHALL still succeed | must |

## 3. Acceptance Criteria

```
GIVEN no terminal attached
WHEN  helix-trainer --version < /dev/null is run
THEN  stdout is "helix-trainer <CARGO_PKG_VERSION>" and the exit code is 0
```
```
GIVEN any terminal state
WHEN  helix-trainer --bogus is run
THEN  stderr names `--bogus`, the exit code is 2, and the terminal is not switched to the alternate screen
```
```
GIVEN the argument list []
WHEN  the parser runs
THEN  it returns CliAction::Run
```

## 4. Edge Cases

| Scenario | Expected Behavior |
|----------|-------------------|
| `--version --help` | First recognized argument wins (`Version`) |
| `--version extra` | `[NEEDS CLARIFICATION: treat trailing arguments as an error or ignore]` default: error (strict) |
| Empty-string argument | Unknown argument error |

## 5. Success Criteria

| ID | Metric | Target |
|----|--------|--------|
| SC-001 | Unit tests on the pure parser for each row above | Pass |
| SC-002 | Integration test running the built binary with `--version` and `--bogus` without a TTY | Exit codes 0 and 2, no escape sequences on stdout |

## 6. Agent Boundaries

- Always: keep the parser free of I/O so it is testable
- Never: add a CLI parsing crate for two flags; touch terminal state before the parse result is `Run`

## 7. See Also

- [[constitution]] - project principles
- [[data-and-runtime-hardening/spec]] - terminal restore (#443) shares `main.rs`
- [[MOC-specs]] - all specifications
