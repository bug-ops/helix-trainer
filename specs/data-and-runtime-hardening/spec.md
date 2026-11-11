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
> **Issues**: #440 (P1), #441, #444, #442, #443; extended 2026-11-11 with #467, #484, #515, #518; extended again 2026-11-11 with #460, #457, #456, #531, #454, #523, #463 (section 13)
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

- Encrypting profile data
- Pruning review cards already persisted by earlier versions (specified separately: [[fsrs-proptest-coverage-gap/spec#12. Bug-Fix Batch: Persisted Card Integrity, Panic-Free FSRS Updates, Single Urgency Authority (#511, #464, #469, #470)|FSRS spec section 12]])
- Windows ACL permissions (Unix modes only)
- Multi-generation history of the profile (only one `profile.json.prev` is kept)
- cargo-fuzz targets for the loaders (optional part of #454, tracked as a follow-up)

## 2. Functional Requirements

### 2.1 Profile load failure (#440)

| ID | Requirement | Priority |
|----|------------|----------|
| FR-001 | WHEN loading `profile.json` fails for any reason other than "file does not exist" THE SYSTEM SHALL keep the in-memory profile in a typed persistence state `ProfilePersistence` with variants `Loading`, `Writable` and `Quarantined(QuarantineOutcome)`; `ProgressState` starts in `Loading` (fail-safe: nothing is saved until a profile load result arrives); every save request (immediate, debounced, synchronous and final) SHALL be refused outside `Writable`, and `prepare_final_save_request` SHALL return `None` outside `Writable` | must |
| FR-002 | WHEN loading fails and the file exists THE SYSTEM SHALL COPY (not rename) the unreadable file to `profile.json.corrupt-<timestamp>` and SHALL leave the original byte-identical; the outcome of the copy is carried in `Quarantined(QuarantineOutcome)` and reported to the user | must |

| FR-003 | WHEN loading fails THE SYSTEM SHALL show a persistent banner at the top of the screen, derived from `ProfilePersistence::Quarantined` state rather than an entry in the (evictable, short-lived) notification queue. The banner wraps lines, its first line is `Profile load failed: saving is disabled this session.` and its second line is `Backup kept at: <path>` (or states that the backup failed), and it SHALL NOT cover the bottom footer line | must |
| FR-004 | WHEN `achievements_unlocked` contains an unknown `AchievementId` (downgrade) THE SYSTEM SHALL ignore the unknown entries and load the rest of the profile rather than fail the whole file | must |
| FR-005 | WHEN the file does not exist THE SYSTEM SHALL create a new default profile and save normally (first launch) | must |
| FR-006 | WHEN quarantining an unreadable profile and a byte-identical `profile.json.corrupt-*` file already exists THE SYSTEM SHALL NOT create another copy and SHALL report the existing backup path in `QuarantineOutcome` (#518) | must |
| FR-007 | WHILE the profile is `Quarantined` with a backup that exists on disk THE BANNER SHALL offer one action, **Start fresh** (bound to `Ctrl-N`, shown only for `Quarantined(BackedUp(_))`): the backup is kept, a new default profile is written, and `ProfilePersistence` becomes `Writable`; the action is a state-model message that yields an effect executed by the event loop (no I/O in `update()`); WHEN no backup exists (the copy failed) THE ACTION SHALL NOT be offered, so unreadable data is never overwritten without a copy (#518) | must |
| FR-008 | THE BACKUP DEDUPE of FR-006 SHALL consider only regular files as existing backups (never symlinks, FIFOs or other special files) and SHALL reject a candidate that is the source file's own inode; temporary files (backup and save) SHALL be created with `O_EXCL` semantics (create-new); the profile lock file SHALL be read through the capped reader of FR-010 | must |

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

### 2.7 Single save path for load-time achievements (#467)

| ID | Requirement | Priority |
|----|------------|----------|
| FR-060 | WHEN achievements unlock while a profile loads THE SYSTEM SHALL persist through `ProgressState::save_immediate` (the serialized off-thread writer path) and SHALL NOT call `ProfileStorage::save` from `handle_data_message` | must |
| FR-061 | WHEN that save fails THE SYSTEM SHALL report it through the `ProfileSaveError` notification and retry policy, log it, and continue; the failure SHALL NOT propagate as an `Err` out of `handle_data_message` | must |

### 2.8 One JSON store for profile and config (#484)

| ID | Requirement | Priority |
|----|------------|----------|
| FR-070 | `profile.json` and `config.json` SHALL be persisted by one shared generic JSON-document store (path resolution under the data directory, `read_capped` size guard per FR-010, envelope handling, one save implementation); the former separate implementations are removed | must |
| FR-071 | THE SHARED SAVE SHALL be atomic and durable for both documents: write a temporary file, `fsync`, rename over the target, `fsync` the parent directory, with the permissions of FR-020/FR-021 | must |
| FR-072 | THE PROFILE-ONLY concerns (lock file, quarantine, `ProfilePersistence`) SHALL stay outside the shared store; behavior of FR-001..FR-007 is unchanged | must |

> [!note]
> `config.json` currently has no production save caller (#477 removes the unused one; #456 stays open). FR-071 makes any future config save crash-safe; it does not add a save call site.

### 2.9 Data directory on macOS and one-time migration (#515)

| ID | Requirement | Priority |
|----|------------|----------|
| FR-080 | WHERE the platform is macOS THE DATA DIRECTORY SHALL be `~/.config/helix-trainer`, resolved by the same XDG resolver as the Helix keymap (a non-empty absolute `$XDG_CONFIG_HOME`, else `~/.config`), extracted into one shared function used by the keymap path and the data directory; on Linux and Windows the directory is unchanged (`dirs::config_dir()/helix-trainer`) | must |
| FR-081 | WHEN the trainer starts on macOS THE SYSTEM SHALL migrate each of `profile.json` and `config.json` independently: if the file is absent in the new directory and present in the old one (`~/Library/Application Support/helix-trainer`), COPY it (not move) into the new directory via temporary file, `fsync` and rename; there is no global marker, so a crash between the two files resumes on the next start | must |
| FR-082 | ONCE a destination file exists THE SYSTEM SHALL NOT read the old location for that file again (the new directory wins); the old files are left in place untouched | must |
| FR-083 | `*.corrupt-*` backups and `profile.json.lock` SHALL NOT be migrated (the lock is recreated in the new directory) | must |
| FR-084 | WHEN a copy fails (unreadable, non-regular or oversized source, I/O error) THE SYSTEM SHALL NOT create the destination, SHALL treat it as a profile load failure for `profile.json` (FR-001: saving disabled, banner) so a fresh default is never written over data that was not migrated, and SHALL fall back to defaults for `config.json` | must |
| FR-085 | `SECURITY.md` and the README data-directory lines SHALL state the per-platform directory and the migration rules; the CHANGELOG SHALL state that an older binary keeps using the old directory, so profile data diverges on downgrade | must |

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
GIVEN the same unreadable profile.json on two consecutive launches
WHEN  the app starts both times
THEN  exactly one profile.json.corrupt-* copy exists; choosing Start fresh in the banner writes a new profile, keeps the backup and re-enables saves
```
```
GIVEN macOS, profile.json only under ~/Library/Application Support/helix-trainer
WHEN  the app starts
THEN  profile.json is copied to ~/.config/helix-trainer, the old file remains, and later starts read only the new one
```
```
GIVEN achievements unlock during profile load and the save fails
WHEN  handle_data_message runs
THEN  it returns Ok, a save-error notification is raised, and the save is retried
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
| Same unreadable file on a later launch | No new copy; the existing byte-identical backup is reused (FR-006) |
| Start fresh chosen, then the new profile save fails | Remains `Quarantined`/non-writable, error shown; the original and the backup are untouched |
| Profile larger than the 1 MiB cap | Treated as a load failure (FR-001..003), not as a new profile |
| Config file bad (`config.json`) | Fallback to defaults is kept, the user sees a startup notification (FR-110), and the bad file is not overwritten by the load path. A `.corrupt-*` backup of `config.json` is out of scope: the app has no production config save call site (see the note under 2.8) |
| Panic hook invoked twice | Idempotent restore, no second panic |

## 5. Success Criteria

| ID | Metric | Target |
|----|--------|--------|
| SC-001 | Regression test: unparsable `profile.json` unchanged after `prepare_final_save_request` and after the debounced save path | Pass |
| SC-002 | Tests for `/dev/zero` symlink and FIFO (Unix-only, bounded by a timeout) | Pass |
| SC-003 | Permission test on Unix (`cfg(unix)`) | Pass |
| SC-004 | Test with `attempts = u32::MAX` | Pass |
| SC-005 | Hook installed and restore function unit-tested (restore called once, previous hook chained) | Pass |
| SC-006 | Tests for backup dedupe and Start fresh (FR-006, FR-007), including no-backup case and the FR-008 rules (regular files only, own inode rejected, O_EXCL temp files, capped lock read) | Pass |
| SC-007 | Test that load-time achievement saves go through `save_immediate` and a failing save does not return `Err` (FR-060/061) | Pass |
| SC-008 | Shared store tests: atomic config save, size cap, permissions for both documents (FR-070..072) | Pass |
| SC-009 | Migration tests with temporary directories: per-file copy, new-dir-wins, resume after a partial migration, backups and lock not migrated, failed copy leaves no destination (FR-081..084) | Pass |

## 6. Agent Boundaries

- Always: route capped reads through the single helper; keep the quarantine state a type, not a boolean checked in scattered places
- Ask first: deleting any user file; changing the data directory location again beyond FR-080
- Never: overwrite `profile.json` after a failed load without an existing backup; move (instead of copy) migrated files; add `unsafe`

## 13. Bug-Fix Batch: Typed Storage Errors, Config Directory, Config Notification, Profile Restore, Loader Robustness, Level Display, Typed Identifiers (#460, #457, #456, #531, #454, #523, #463)

> [!info] Added 2026-11-11 (spec-first update for the 25-issue batch)
> Implementation starts only from issues that link this section.

### 13.1 Typed storage and loader errors (#460)

| ID | Requirement | Priority |
|----|------------|----------|
| FR-090 | THE SYSTEM SHALL represent persistence failures as one typed error `StorageError` carrying the document (`Profile` or `Config`) and a closed set of kinds: read, too large, corrupt (parse), serialize, write, migrate, no backup, delete. Each variant keeps its cause as a `#[source]`; the cause text SHALL NOT be interpolated into the variant's own message, so a chain walker prints each cause once. `StorageError` replaces the former `JsonStoreError` and the string-payload `GamificationError::StorageError` | must |
| FR-091 | `path_validator::read_capped` SHALL return a typed `CappedReadError` (not a regular file, too large with limit and actual size, I/O, invalid UTF-8) convertible to `SecurityError`, so scenario, quest and keymap loader sites keep their behavior. "File does not exist" SHALL stay distinguishable from every other read failure (first launch, FR-005, relies on it) | must |
| FR-092 | THE library modules (`data_loader`, `config`, `gamification`, `async_state`) SHALL NOT expose `anyhow` or `String` error payloads; loader results, profile load, profile save and quarantine outcomes SHALL carry typed errors (a loader error, a storage error, a quarantine copy error, a save failure that is a storage error or a task join error). `anyhow` stays only in the binary entry point | must |
| FR-093 | `UserError` SHALL expose storage failures as a variant with `StorageError` as its `#[source]` and a sanitized Display (no filesystem paths or OS messages shown to the learner) | must |
| FR-094 | THE `From` conversions into `UserError` SHALL NOT log. EVERY place that consumes such an error (event-loop update and data-message error reporting, start-fresh and restore tasks, config load and exit-save in `main`, the profile, scenario, quest-registry and save-error arms of the data-message handler, and the review, scenario and gameplay handlers that convert errors) SHALL log the full cause chain through one shared helper, so no error becomes silent when logging leaves `From` | must |

### 13.2 Data directory without fallback (#457)

| ID | Requirement | Priority |
|----|------------|----------|
| FR-100 | WHEN no data directory can be resolved (no usable `HOME` or `XDG_CONFIG_HOME`, per FR-080 on macOS and `dirs::config_dir()` elsewhere) THE SYSTEM SHALL NOT fall back to the current working directory for `profile.json` or `config.json` | must |
| FR-101 | WHEN the data directory cannot be resolved THE SYSTEM SHALL exit with a non-zero status and a message that names `HOME` and `XDG_CONFIG_HOME`, before the terminal is set up (the message is readable in the normal terminal) | must |
| FR-102 | `--help` and `--version` SHALL keep working without a data directory (command-line parsing precedes resolution, see [[cli-version-and-help-flags/spec]]) | must |
| FR-103 | THE path-less constructors `ProfileStorage::new()` and `ConfigStorage::new()` (and their `Default` impls) SHALL be removed; storage is built only from a resolved directory or an explicit path. The loader entry point takes the storage instance, and `main` SHALL hand it a clone of the application's `ProfileStorage` so loader and application share the last-valid bytes of FR-121 | must |

### 13.3 Config parse failure notification (#456)

| ID | Requirement | Priority |
|----|------------|----------|
| FR-110 | WHEN `config.json` exists but cannot be read or parsed THE SYSTEM SHALL fall back to defaults AND show a visible startup notification stating that the configuration was not loaded (same mechanism as the Helix keymap startup message), and SHALL log the cause chain | must |
| FR-111 | THE atomic save required for `config.json` is delivered by FR-071 (temp file, fsync, rename); no additional save requirement is added here | must |

### 13.4 Restore from the previous profile copy (#531)

| ID | Requirement | Priority |
|----|------------|----------|
| FR-120 | WHEN the profile is saved THE SYSTEM SHALL first write the last known-valid profile bytes to `profile.json.prev` (best effort: a failure is logged and does not block the save), then write `profile.json` atomically per FR-071 | must |
| FR-121 | "Last known-valid" SHALL mean bytes held in memory that this process either loaded successfully or wrote successfully; the rotation SHALL NEVER copy bytes read back from disk, so external edits or corruption of `profile.json` while the app runs cannot overwrite the last good `.prev`. After a failed load nothing is held, so Start fresh does not rotate the corrupt file into `.prev`. The held bytes are shared by all clones of the storage handle (loader, writer, background tasks), and rotation, write and update happen under one lock. Rotation is a write, never a rename (a crash between renames must not leave no `profile.json`). The held bytes SHALL never appear in `Debug` output | must |
| FR-122 | `ProfilePersistence::Quarantined` SHALL carry both the backup outcome and whether a restorable previous copy exists; the previous copy is loaded through the same full load path as `profile.json` (size cap, lenient card loader, level recomputation) and counts as available only if it passes | must |
| FR-123 | WHILE the profile is `Quarantined` with a backup on disk (`BackedUp`) AND an available previous copy THE BANNER SHALL offer a second action, **Restore previous** (bound to `Ctrl-R`, shown only in that state), next to Start fresh. It is a state-model message yielding an effect executed by the event loop (no I/O in `update()`) | must |
| FR-124 | WHEN Restore previous is chosen THE SYSTEM SHALL re-validate `profile.json.prev` through the full load path at that moment (the file may have changed since the offer); on success it SHALL write the validated bytes to `profile.json` atomically, make `ProfilePersistence` `Writable`, and keep the `.corrupt-*` backup; on failure it SHALL refuse, leave every file untouched and report the error | must |
| FR-125 | `delete` of the profile SHALL also remove `profile.json.prev`. WHEN the user chooses Start fresh THE SYSTEM SHALL write the new default profile; the old `.prev` stays restorable only until the next save, which replaces it with the in-memory last-valid bytes (this discard is intended) | must |

### 13.5 Loader robustness gates (#454)

| ID | Requirement | Priority |
|----|------------|----------|
| FR-130 | THE modules `gamification`, `learning` and `config` SHALL enable `clippy::arithmetic_side_effects`, `clippy::indexing_slicing`, `clippy::unwrap_used` and `clippy::expect_used` as warnings for non-test code (module-level `cfg_attr(not(test), warn(..))`); every hit is fixed or carries a local `expect` with a reason | must |
| FR-131 | A property test SHALL mutate a valid profile JSON (integer leaves set to 0 and `MAX`, extreme chrono dates, unknown fields), load it from a temporary file, and on every successful load run `record_attempt`, streak update, achievement check, quest generation and the XP progress computation without panicking; case count is bounded for CI | must |

### 13.6 Level display overflow (#523)

| ID | Requirement | Priority |
|----|------------|----------|
| FR-140 | WHEN the menu or results screen shows the next level THE SYSTEM SHALL compute `level + 1` with saturating addition (`level = u32::MAX` does not panic or wrap); this extends FR-032 to display code | must |

### 13.7 Command, scenario and quest identifiers (#463)

| ID | Requirement | Priority |
|----|------------|----------|
| FR-150 | `ScenarioId` and `QuestId` SHALL be validated newtypes (existing rule: 1 to 64 characters of `[A-Za-z0-9_]`), constructed only through a fallible parse; deserialization goes through the same validation (an invalid persisted or TOML id is a load error of that document, not a silently accepted value). They have no `Default` and no infallible conversion from `&str` | must |
| FR-151 | `CommandId` SHALL be exclusively the FSRS review-card identifier (the former `ReviewableCommand`): its only constructors are the canonical derivation from a raw action and re-validation of a stored canonical id; there is no permissive parse. Card ids are limited to 64 characters | must |
| FR-152 | A command whose canonical card id exceeds 64 characters (for example an overlong `:name` command) SHALL NOT create a review card (`CommandId::from_action` returns none); this is an intended behavior change from the previous unbounded `:name` ids and SHALL be stated in the PR text and CHANGELOG | must |
| FR-153 | `NormalizedCommand` SHALL be the type of normalized command strings (counts kept, as `3h` or `"y`) used by `commands_taught`, quest `requires_commands`, the `CommandPractice` command and `commands_used_today`. It is constructed by normalizing a raw string (`from_raw` returns a `Result` with a typed `IdError` cause, which serde `try_from` needs) and rejects empty or over-long input; deserialization normalizes (never stricter than today's normalize-then-compare matching). Conversion to a card id is explicit and fallible | must |
| FR-154 | THE performance data loader SHALL keep its lenient per-entry behavior: entries whose key is not a valid canonical card id (including over-cap ids) are dropped and counted in the existing warning, never failing the whole profile. `CommandPerformance` SHALL NOT duplicate its map key as a field (`profile.json` layout is unchanged: the field was never written) | must |
| FR-155 | OTHER persisted ids (`completed_quests_today`, `daily_quests[].id`, `scenario_history` attempts) are strict: a profile written by any earlier version SHALL still load. This is proven by the legacy-profile fixture of [[fsrs-7-crate-migration/spec]] (SC-004), extended with these fields | must |
| FR-156 | RAW input (`UserAction.command`, key buffers) stays `String`; rendering, i18n and tracing use string views of the id types | must |
| FR-157 | `ScenarioHistory` ids follow the common `ScenarioId` rule (previously hyphens and up to 100 characters were accepted; no historic id violates the common rule) | must |
| FR-158 | `ScenarioPrioritizer::extract_commands` SHALL map solution commands through `CommandId::from_actions`, so novelty keys match real FSRS cards. Arcade prioritization changes: Insert-mode text and raw tokens such as `3h` or `Escape` no longer count as commands. The change SHALL be stated in the PR text and CHANGELOG | must |

### 13.8 Acceptance Criteria

```
GIVEN a profile loaded cleanly at startup and one later save
WHEN  profile.json is corrupted externally between saves and the app saves again
THEN  profile.json.prev still holds the bytes loaded at startup (FR-121)
```
```
GIVEN an unreadable profile.json, a backup, and a valid profile.json.prev
WHEN  the banner offers Restore previous and the user presses Ctrl-R
THEN  profile.json holds the validated previous bytes, persistence is Writable, the backup remains (FR-123, FR-124)
```
```
GIVEN the banner offered Restore previous and profile.json.prev is corrupted afterwards
WHEN  the user presses Ctrl-R
THEN  restore is refused, no file changes, an error is reported (FR-124)
```
```
GIVEN the profile is deleted
THEN  profile.json.prev is removed too (FR-125)
```
```
GIVEN no HOME and no XDG_CONFIG_HOME
WHEN  the app starts
THEN  it exits non-zero naming both variables and writes no file in the current directory; --help and --version still print (FR-100..102)
```
```
GIVEN config.json with invalid JSON
WHEN  the app starts
THEN  defaults are used and a startup notification is shown (FR-110)
```
```
GIVEN a profile with level = u32::MAX
WHEN  the menu and results screens render
THEN  no panic occurs (FR-140)
```
```
GIVEN a 6.x-era profile.json containing performance_data keys over 64 characters or with counts ("3h")
WHEN  the profile loads
THEN  those cards are dropped and counted in a warning, the rest of the profile loads (FR-154)
```

### 13.9 Edge Cases

| Scenario | Expected Behavior |
|----------|-------------------|
| First launch (no profile) | No rotation (nothing last-valid); no `.prev` |
| Save while `Quarantined` | Refused (FR-001); `.prev` untouched |
| `.prev` write fails (read-only dir) | Logged; the main save proceeds |
| `.prev` exists but fails validation | Restore is not offered; Start fresh is |
| Two storage clones save concurrently | Serialized by the shared lock; `.prev` always parses |
| Debug output of a storage handle | Prints the path and byte length, never profile contents |

### 13.10 Success Criteria

| ID | Metric | Target |
|----|--------|--------|
| SC-010 | Storage tests: each `StorageError` kind is produced by its failing scenario and a chain walker prints each cause once; sinks of FR-094 log the chain | Pass |
| SC-011 | Rotation tests: from-memory rotation, external corruption between saves, first save after a load through the loader entry point (`.prev` equals the loaded bytes), Start fresh does not rotate the corrupt file, restore re-validation, `delete` removes `.prev` | Pass |
| SC-012 | Startup tests for FR-100..103 (no data dir, help and version unaffected) and FR-110 | Pass |
| SC-013 | Profile proptest of FR-131 and lint gates of FR-130 pass in CI | Pass |
| SC-014 | Legacy-profile fixture loads with the strict id types; id newtypes reject invalid input at every deserialization site; every bundled `commands_taught` value equals its normalized form | Pass |
| SC-015 | A level of `u32::MAX` renders on menu and results without panic | Pass |

> [!question] Out of this batch
> Optional cargo-fuzz targets for the config and keymap loaders (part of #454) are tracked as a separate follow-up issue.

## 7. See Also

- [[constitution]] - Section V (security, recoverable errors)
- [[custom-helix-keymap-support/spec]] - keymap config path; shares the XDG resolver (FR-080)
- [[sound-setting-persistence/spec]] - sound settings must not be written into a quarantined profile
- [[cli-version-and-help-flags/spec]] - shares `main.rs`; `--help`/`--version` must work without a data directory (FR-102)
- [[fsrs-7-crate-migration/spec]] - legacy-profile fixture (SC-004) reused for FR-155
- [[MOC-specs]] - all specifications
