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
> **Issues**: #459; extended 2026-11-13 with #659 (section 6)
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
| FR-001 | WHEN `DataLoadMessage::ProfileReady` is handled THE SYSTEM SHALL apply `profile.sound_config` to the sound manager through the `Volume` type (FR-006), so volume is clamped and non-finite values are replaced by the default | must |
| FR-002 | WHEN `Message::ToggleSound` is handled THE SYSTEM SHALL mirror the new enabled state into `profile.sound_config` and mark the profile dirty so the debounced save persists it | must |
| FR-003 | WHEN `profile.sound_config` holds an out-of-range volume (negative, above 1.0, NaN, infinite) THE SYSTEM SHALL normalize it through `Volume` (on deserialization, FR-006) and SHALL NOT pass it unclamped to the audio backend | must |
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

## 6. Volume Type, Single Normalization and Output State (#659)

> [!info] Added 2026-11-13 (spec-first update for the typed-position batch)
> FR numbers continue after FR-005. The persisted `sound_config` layout in `profile.json` is unchanged (`volume` stays a JSON number, `enabled` a boolean).

### 6.1 Requirements

| ID | Requirement | Priority |
|----|------------|----------|
| FR-006 | THE SYSTEM SHALL represent output gain as a `Volume` newtype whose invariant is "finite and within `0.0..=1.0`". Its only constructor clamps a finite input into the range and maps a non-finite input to `SoundConfig::DEFAULT_VOLUME`; its `Deserialize` goes through the same constructor, so any number in `profile.json` loads as a valid `Volume`; `Serialize` writes the plain number. There is no public way to build or mutate a `Volume` holding an out-of-range value | must |
| FR-007 | THE finite-and-clamp rule SHALL exist in exactly one place (`Volume`). `SoundConfig`, `SoundManager::set_volume` and the profile-apply path (`ProgressState` construction and `ProfileReady`) SHALL take or hold a `Volume` and SHALL NOT re-implement the rule. `SoundConfig` fields are private with accessors; no caller writes a raw `f32` volume | must |
| FR-008 | THE `SoundManager` output state SHALL be a single two-state value (silent, or ready with its stream and mixer); there is no separate `initialized` flag that can disagree with the mixer. WHEN the output is silent THE SYSTEM SHALL treat `play` as a no-op | must |
| FR-009 | `SoundManager::play` SHALL be the only playback entry point and SHALL check `enabled`; the duplicate `play_if_enabled` is removed (pre-1.0, no deprecation shim) | must |
| FR-010 | WHEN audio initialization fails THE SYSTEM SHALL log the failure once and keep the output silent; the call site SHALL NOT discard a `Result` with `let _ =` | must |

### 6.2 Acceptance Criteria

```
GIVEN any f32 including NaN, +inf, -inf, -3.0, 0.4 and 12.0
WHEN  Volume is built from it
THEN  the value is within 0.0..=1.0: 0.4 is kept, -3.0 gives 0.0, 12.0 gives 1.0, non-finite gives DEFAULT_VOLUME
```
```
GIVEN profile.json contains "sound_config": {"volume": 12.0, "enabled": true}
WHEN  the profile is deserialized and applied at ProgressState construction
THEN  the manager volume is 1.0 and no unclamped value reaches the backend
```
```
GIVEN a SoundManager whose output is silent (no audio device or the audio feature is off)
WHEN  play is called
THEN  nothing happens and no panic occurs
```

### 6.3 Edge Cases

| Scenario | Expected Behavior |
|----------|-------------------|
| Build without the `audio` feature | `Volume`, `SoundConfig` and the profile round trip behave identically; output is permanently silent |
| `volume` missing from `sound_config` | `DEFAULT_VOLUME` (serde default, unchanged) |
| Volume exactly `0.0` or `1.0` | Kept as is |

### 6.4 Success Criteria

| ID | Metric | Target |
|----|--------|--------|
| SC-003 | Unit tests of `Volume` (table above), a proptest that every `f32` yields a value in range, and a deserialization test for FR-006 | Pass |
| SC-004 | `rg "is_finite"` under `src/sound/` and `src/ui/state/` matches only the `Volume` constructor; `rg "play_if_enabled\|initialized"` in `src/sound/` | 1 match, 0 matches |

## 7. See Also

- [[constitution]] - project principles (audio is compile-checked, not executed, in CI)
- [[data-and-runtime-hardening/spec]] - profile save safety
- [[MOC-specs]] - all specifications
