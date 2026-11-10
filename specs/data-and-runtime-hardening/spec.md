---
aliases:
  - Data and Runtime Hardening
  - Profile Load-Failure Safety
tags:
  - sdd
  - spec
  - security
  - persistence
  - robustness
  - bug-fix
  - status/proposed
created: 2026-11-10
status: proposed
related:
  - "[[constitution]]"
  - "[[sound-setting-persistence/spec]]"
  - "[[cli-version-and-help-flags/spec]]"
  - "[[MOC-specs]]"
---

# Feature: User-Data Safety and Runtime Robustness Hardening

> [!info] Metadata
> **Issues**: #440 (P1), #441, #444, #442, #443
> **Status**: Proposed
> **Depth**: Lightweight spec covering five related findings of the CI-011 security audit. Constitution V (security) and the recoverable-error rule in [[constitution#V. Security]] apply.

## 1. Overview

### Problem Statement

- #440: an unreadable `profile.json` (parse error, size cap, unknown enum variant after a downgrade) is replaced by `UserProfile::new()` and then overwritten on exit, permanently destroying all progress with no backup and no visible warning.
- #441: the size caps on `profile.json`, `config.json` and the Helix keymap are bypassed by symlinks to devices (`/dev/zero`, OOM) and FIFOs (hang), because the check and the read are separate lookups and the type is not checked.
- #444: the data directory and files are created with the process umask although `SECURITY.md` promises restrictive permissions.
- #442: a persisted `attempts = u32::MAX` wraps to 0 in release and `Duration / 0` panics (abort).
- #443: no panic hook, release uses `panic = "abort"`; any panic leaves the terminal in raw mode on the alternate screen.

### Goal

A failed profile load never destroys the file on disk; reads are bounded and reject non-regular files; files are private on Unix; counters saturate; a panic restores the terminal.

### Out of Scope

- Relocating or migrating the data directory (see [[custom-helix-keymap-support/spec#10. Bug-Fix: Helix Config Path on macOS (#458, keymap path and docs only)|keymap spec section 10]])
- Encrypting profile data
- Pruning review cards already persisted by earlier versions
- Windows ACL permissions (Unix modes only)

## 2. Functional Requirements

### 2.1 Profile load failure (#440)

| ID | Requirement | Priority |
|----|------------|----------|
| FR-001 | WHEN loading `profile.json` fails for any reason other than "file does not exist" THE SYSTEM SHALL keep the in-memory profile in a typed persistence state `ProfilePersistence` with variants `Loading`, `Writable` and `Quarantined(QuarantineOutcome)`; `ProgressState` starts in `Loading` (fail-safe: nothing is saved until a profile load result arrives); every save request (immediate, debounced, synchronous and final) SHALL be refused outside `Writable`, and `prepare_final_save_request` SHALL return `None` outside `Writable` | must |
| FR-002 | WHEN loading fails and the file exists THE SYSTEM SHALL COPY (not rename) the unreadable file to `profile.json.corrupt-<timestamp>` and SHALL leave the original byte-identical; the outcome of the copy is carried in `Quarantined(QuarantineOutcome)` and reported to the user | must |

> [!warning] Known limitation (accepted)
> Because the backup is a copy and the original stays in place, every launch with the same unreadable file creates a new `profile.json.corrupt-<timestamp>` copy and saving stays disabled until the user repairs or removes the file. A "start fresh" recovery flow is deferred to a follow-up.
| FR-003 | WHEN loading fails THE SYSTEM SHALL show a persistent banner at the top of the screen, derived from `ProfilePersistence::Quarantined` state rather than an entry in the (evictable, short-lived) notification queue. The banner wraps lines, its first line is `Profile load failed: saving is disabled this session.` and its second line is `Backup kept at: <path>` (or states that the backup failed), and it SHALL NOT cover the bottom footer line | must |
| FR-004 | WHEN `achievements_unlocked` contains an unknown `AchievementId` (downgrade) THE SYSTEM SHALL ignore the unknown entries and load the rest of the profile rather than fail the whole file | must |
| FR-005 | WHEN the file does not exist THE SYSTEM SHALL create a new default profile and save normally (first launch) | must |

### 2.2 Bounded reads (#441)

| ID | Requirement | Priority |
|----|------------|----------|
| FR-010 | WHEN reading `profile.json`, `config.json` or the Helix `config.toml` THE SYSTEM SHALL use the single helper `path_validator::read_capped`: reject a non-regular path, open once, re-check on the open handle, and read at most `max + 1` bytes (a file larger than `max` is rejected); the former `validate_file_size` is removed | must |
| FR-011 | WHEN the path is a character device, FIFO, socket or directory THE SYSTEM SHALL reject it with a `UserError` without blocking or unbounded allocation | must |
| FR-012 | THE SAME helper SHALL be used for every capped read site (profile, config, keymap, and the scenario and quest loaders) | must |

### 2.3 Permissions (#444)

| ID | Requirement | Priority |
|----|------------|----------|
| FR-020 | WHERE the platform is Unix THE SYSTEM SHALL create the data directory with mode `0700` and `profile.json`, its temporary file, `profile.json.lock` and `config.json` with mode `0600` | must |
| FR-021 | THE SYSTEM SHALL NOT change the permissions of a directory that already exists (the data path may sit inside an arbitrary directory such as `/tmp`); mode `0700` applies only to directories the application creates, and `0600` to files the application creates or rewrites (an existing file with broader permissions is therefore tightened when it is rewritten) | must |

### 2.4 Counter arithmetic (#442)

| ID | Requirement | Priority |
|----|------------|----------|
| FR-030 | WHEN `PerformanceTracker::record_attempt` updates counters THE SYSTEM SHALL use saturating addition for `attempts`, `successes`, `reps` and `lapses`, and SHALL compute the average with a divisor of at least 1 | must |
| FR-031 | WHEN the streak counter increments THE SYSTEM SHALL use saturating addition | must |
| FR-032 | WHEN any persisted `u32` counter is `u32::MAX` THE SYSTEM SHALL NOT panic or wrap in debug or release builds | must |

### 2.5 Terminal restore on panic (#443)

| ID | Requirement | Priority |
|----|------------|----------|
| FR-040 | WHEN a panic occurs after terminal setup THE SYSTEM SHALL, from a panic hook installed right after `setup_terminal()`, restore the terminal (disable raw mode, leave the alternate screen, pop keyboard-enhancement flags, show the cursor) on a fresh stdout handle, then chain to the previously installed hook | must |
| FR-041 | WHEN `main` returns early with an error after terminal setup THE SYSTEM SHALL restore the terminal through a drop guard | must |
| FR-042 | THE RESTORE LOGIC SHALL be shared with the normal exit path (single implementation) | must |

### 2.6 Event-loop error policy (#466, see [[constitution#V. Security]])

| ID | Requirement | Priority |
|----|------------|----------|
| FR-050 | WHEN `update()` or `handle_data_message()` returns `UserError::WrongScreen` (a message for a screen other than the current one; the variant carries no screen name, the screen for the debug log is taken from state) THE EVENT LOOP SHALL ignore it silently (debug-level `tracing` only) | must |
| FR-051 | WHEN either returns any other `UserError` (including a failed startup save and a rejected simulator command) THE EVENT LOOP SHALL show it as a user notification and keep running | must |
| FR-052 | THE EVENT LOOP SHALL contain no `?` on `update()`/`handle_data_message()`; one `apply_update` boundary owns this policy | must |
| FR-053 | WHEN the crossterm `EventStream` yields `Some(Err(_))` THE EVENT LOOP SHALL log it and exit with an error after 5 consecutive errors (the counter resets on any successful event); WHEN it yields `None` THE EVENT LOOP SHALL end with an error | must |

## 3. Acceptance Criteria

```
GIVEN profile.json holds total_xp 123456 and an unknown achievement id
WHEN  the app starts, runs, and exits normally
THEN  the file is not replaced by an empty profile; unknown ids are ignored on load (FR-004)
```
```
GIVEN profile.json is not parseable JSON
WHEN  the app starts and exits via prepare_final_save_request
THEN  profile.json is byte-for-byte unchanged, a profile.json.corrupt-<timestamp> copy exists, and a notification was shown
```
```
GIVEN profile.json is a symlink to /dev/zero
WHEN  ProfileStorage::load runs
THEN  it returns a UserError quickly and process memory stays bounded
```
```
GIVEN a FIFO at the profile path
WHEN  load runs
THEN  it returns an error without blocking
```
```
GIVEN a fresh data directory on Unix
WHEN  the first save completes
THEN  the directory is 0700 and profile.json, profile.json.lock and config.json are 0600
```
```
GIVEN a tracker with attempts = u32::MAX
WHEN  record_attempt runs
THEN  no panic occurs and attempts stays u32::MAX
```
```
GIVEN a panic after terminal setup
WHEN  the hook runs
THEN  raw mode is off and the alternate screen is left before the previous hook is called
```

## 4. Edge Cases

| Scenario | Expected Behavior |
|----------|-------------------|
| Backup move fails (read-only directory) | Saving stays disabled, the original is untouched, notification says so |
| Two quarantines in the same second | Timestamp plus a numeric suffix keeps names unique |
| Same unreadable file on a later launch | A new backup copy is created each launch (known limitation, see FR-002) |
| Profile larger than the 1 MiB cap | Treated as a load failure (FR-001..003), not as a new profile |
| Config file bad (`config.json`) | Existing fallback to defaults is kept; no overwrite of the bad file without a backup `[NEEDS CLARIFICATION: apply FR-002 backup to config.json too]` |
| Panic hook invoked twice | Idempotent restore, no second panic |

## 5. Success Criteria

| ID | Metric | Target |
|----|--------|--------|
| SC-001 | Regression test: unparsable `profile.json` unchanged after `prepare_final_save_request` and after the debounced save path | Pass |
| SC-002 | Tests for `/dev/zero` symlink and FIFO (Unix-only, bounded by a timeout) | Pass |
| SC-003 | Permission test on Unix (`cfg(unix)`) | Pass |
| SC-004 | Test with `attempts = u32::MAX` | Pass |
| SC-005 | Hook installed and restore function unit-tested (restore called once, previous hook chained) | Pass |

## 6. Agent Boundaries

- Always: route capped reads through the single helper; keep the quarantine state a type, not a boolean checked in scattered places
- Ask first: deleting any user file; changing the data directory location
- Never: overwrite `profile.json` after a failed load; add `unsafe`

## 7. See Also

- [[constitution]] - Section V (security, recoverable errors)
- [[sound-setting-persistence/spec]] - sound settings must not be written into a quarantined profile
- [[cli-version-and-help-flags/spec]] - shares `main.rs`
- [[MOC-specs]] - all specifications
