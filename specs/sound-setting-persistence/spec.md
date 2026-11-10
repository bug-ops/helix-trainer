---
aliases:
  - Sound Setting Persistence
  - sound_config persistence
tags:
  - sdd
  - spec
  - audio
  - persistence
  - bug-fix
  - status/proposed
created: 2026-11-10
status: proposed
related:
  - "[[constitution]]"
  - "[[data-and-runtime-hardening/spec]]"
  - "[[MOC-specs]]"
---

# Feature: Sound Toggle Persists Across Restarts

> [!info] Metadata
> **Issue**: #459
> **Status**: Proposed
> **Depth**: Lightweight bug-fix spec.

## 1. Overview

### Problem Statement

`UserProfile` has a `sound_config` field intended to store the sound settings, but it is never applied or written. `SoundManager` is built once from a fresh `UserProfile::new()` in `main` because the real profile arrives later via `DataLoadMessage::ProfileReady`, which never touches the sound manager. `Message::ToggleSound` flips only the in-memory manager state and never writes the profile. Sound always resets to defaults on launch.

### Goal

The saved `sound_config` is applied when the profile loads, and the toggle is saved with the profile.

### Out of Scope

- New sound settings (volume UI)
- Behavior of builds without the `audio` feature beyond keeping the profile field consistent

## 2. Functional Requirements

| ID | Requirement | Priority |
|----|------------|----------|
| FR-001 | WHEN `DataLoadMessage::ProfileReady` is handled THE SYSTEM SHALL apply `profile.sound_config` to the sound manager through `SoundConfig::new` so volume is clamped and non-finite values are rejected | must |
| FR-002 | WHEN `Message::ToggleSound` is handled THE SYSTEM SHALL mirror the new enabled state into `profile.sound_config` and mark the profile dirty so the debounced save persists it | must |
| FR-003 | WHEN `profile.sound_config` holds an out-of-range volume (negative, above 1.0, NaN, infinite) THE SYSTEM SHALL normalize it through `SoundConfig::new` and SHALL NOT pass it unclamped to the audio backend | must |
| FR-004 | WHEN the profile load fails (see [[data-and-runtime-hardening/spec]] FR-001) THE SYSTEM SHALL keep default sound settings and SHALL NOT write sound state into the quarantined profile | must |
| FR-005 | WHEN built without the `audio` feature THE SYSTEM SHALL still round-trip `sound_config` in the profile unchanged by the toggle path being a no-op | should |

## 3. Acceptance Criteria

```
GIVEN profile.json contains "sound_config": {"volume": 0.7, "enabled": false}
WHEN  the app starts and ProfileReady is handled
THEN  the sound manager reports enabled == false and volume == 0.7
```
```
GIVEN sound is enabled
WHEN  the user toggles sound off and the app saves and restarts
THEN  sound is disabled after restart
```
```
GIVEN profile.json contains "volume": 12.0
WHEN  ProfileReady is handled
THEN  the applied volume is clamped to the SoundConfig maximum
```

## 4. Edge Cases

| Scenario | Expected Behavior |
|----------|-------------------|
| Toggle pressed before the profile has loaded | Applies to the manager only; the loaded profile value wins when `ProfileReady` arrives `[NEEDS CLARIFICATION: confirm "loaded value wins"]` |
| Profile has no `sound_config` (older file) | Defaults apply (`#[serde(default)]`), no error |

## 5. Success Criteria

| ID | Metric | Target |
|----|--------|--------|
| SC-001 | State-level tests (no sound device needed) over the `ProfileReady` and `ToggleSound` handlers | Pass |
| SC-002 | `cargo check --features audio` and `cargo check --no-default-features` | Both compile |

## 6. See Also

- [[constitution]] - project principles (audio is compile-checked, not executed, in CI)
- [[data-and-runtime-hardening/spec]] - profile save safety
- [[MOC-specs]] - all specifications
