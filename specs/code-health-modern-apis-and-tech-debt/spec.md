---
aliases:
  - Code Health Batch
  - Modern APIs and Tech Debt
tags:
  - sdd
  - spec
  - refactoring
  - tech-debt
  - status/proposed
created: 2026-11-11
status: proposed
related:
  - "[[constitution]]"
  - "[[module-layering-and-typed-commands/spec]]"
  - "[[custom-helix-keymap-support/spec]]"
  - "[[MOC-specs]]"
---

# Feature: Modern std APIs Within the MSRV and a Tech-Debt Batch

> [!info] Metadata
> **Issues**: #566 (stable std APIs within MSRV 1.98), #568 (tech-debt batch from the ci-012 audit)
> **Status**: Proposed
> **Depth**: Lightweight spec. No gameplay change. The only behavior change is overflow handling of two profile counters (FR-005), and the keymap startup status becomes a typed value (FR-007).

## 1. Overview

### Problem Statement

Several stable APIs within the declared MSRV (`rust-version = "1.98"`) are not used where manual patterns remain (`assert!(matches!(..))` prints no value on failure; `if cond { Ok(()) } else { Err(..) }`; `.windows(2)` with indexed access). Separately, a batch of small tech-debt items found by the ci-012 architecture audit lets intent depend on comments or positional order: untracked `TODO` markers, id newtypes that repeat impls the macro already generates, unchecked profile counters and an anonymous tuple from the keymap loader.

### Goal

Use the stable APIs where they replace the manual pattern, and close each audit item with a typed, tracked or checked form. Both are mechanical and covered by the existing test suite.

### Out of Scope

- Raising the MSRV. `String::from_utf8_lossy_owned` (stable in 1.99, above the MSRV) is NOT adopted; `src/testing/log_capture.rs` keeps `String::from_utf8_lossy(..).into_owned()` until the MSRV is raised by its own decision (a breaking change).
- Changing any gameplay or scoring rule.
- Implementing the features behind the tracked `TODO` markers (FR-004).

## 2. Functional Requirements

### 2.1 Modern API batch (#566)

| ID | Requirement | Priority |
|----|------------|----------|
| FR-001 | THE codebase SHALL use `assert_matches!` (stable since 1.96) instead of `assert!(matches!(x, Pat))` in tests, so a failure prints the value. The sites are RE-COUNTED on the final code before the work (53 at the time of writing; the issue listed 46) and every test-code site is converted. The source-text fixture in `src/layering_tests.rs` that contains `assert!(matches!(..))` as DATA for the layering self-checks is NOT converted. The doctest in `src/learning/session.rs` that needs the macro gets `use std::assert_matches;` | should |
| FR-002 | THE codebase SHALL use `bool::ok_or` (stable since 1.98) in place of `if cond { Ok(()) } else { Err(..) }` at `gamification::streak` and at `ScenarioLoader::validate_content_size` and `validate_command_sequence`. `ok_or_else` is NOT used because the errors there are constants and clippy (`unnecessary_lazy_evaluations`) rejects the lazy form; `ok_or_else` stays the choice only where the error is computed | should |
| FR-003 | THE codebase SHALL use `<[T]>::array_windows::<2>()` (stable since 1.94) in place of `.windows(2)` with `w[0]`/`w[1]` indexing at the sites in `learning::scheduler`, `learning::analytics` and `input::keymap::keys` | should |

### 2.2 Tech-debt batch (#568)

| ID | Requirement | Priority |
|----|------------|----------|
| FR-004 | EVERY `TODO` marker in the source SHALL name an open GitHub issue (`TODO(#N)`); a marker without an issue is removed or gets one. The three formerly untracked markers are: named macro registers (`"<reg>q`, #619) and unifying the `.` repeat recorder with the macro recorder (#620) in `src/helix/macro_recorder.rs`, and the `Arc<Scenario>` optimization (#621) in `src/minigame/challenge.rs`; each marker cites its issue number | must |
| FR-005 | THE id newtypes SHALL share ONE macro for the impls they all need (`as_str`, `Display`, `Borrow<str>`, `Serialize`): `ScenarioId`, `QuestId`, `CommandId` and `NormalizedCommand` use it, each keeping its own validating constructor. `CommandId` wraps `Box<str>` like the others (not `String`). No behavior change: the strict-identifier rules of [[data-and-runtime-hardening/spec]] section 7.7 hold | must |
| FR-006 | WHEN `ScenarioCompletionService::update_profile_counters` increments `scenarios_completed` or `perfect_scenarios` THE SYSTEM SHALL use `saturating_add`, as the rest of the profile code does (release builds have overflow checks off) | must |
| FR-007 | THE keymap startup resolver SHALL return a typed `StartupKeymap { overlay: KeymapOverlay, report: KeymapReport, status: KeymapStatus }` with `KeymapStatus { Disabled, Applied, NotFound, Invalid }` instead of the anonymous triple `(KeymapOverlay, KeymapReport, Option<String>)`. The user-facing message is rendered in the `ui` layer from `KeymapStatus` (through locale keys), not built as a string in `config`; the rendered English is unchanged ([[custom-helix-keymap-support/spec]] FR-005) | must |

## 3. Acceptance Criteria

```
GIVEN the final code
WHEN  `grep -rn "TODO" src` is run
THEN  every match is of the form TODO(#N) with N an open issue (FR-004)
```
```
GIVEN a profile with scenarios_completed = u32::MAX
WHEN  one more scenario is completed
THEN  the counter stays u32::MAX and nothing panics (FR-006)
```
```
GIVEN use_helix_keymap is true and the Helix config file is missing
WHEN  the app starts
THEN  StartupKeymap.status is NotFound and the ui shows the same notification text as before (FR-007)
```

## 4. Success Criteria

| ID | Metric | Target |
|----|--------|--------|
| SC-001 | No `assert!(matches!(..))` left in test code except the layering fixture; both doc-tests and unit tests pass | Pass |
| SC-002 | The three id newtype impls exist once (the macro); the id tests and the legacy-profile fixture still pass | Pass |
| SC-003 | `cargo clippy --all-targets --all-features --workspace -- -D warnings` and the full gate (including `cargo +1.98 check --all-features`) pass | Pass |

## 5. Agent Boundaries

### Always
- Re-grep the sites on the final code before converting (the counts drift)
- Keep the change mechanical: one commit per issue

### Never
- Raise the MSRV or use an API stabilized above 1.98 (FR-001 out of scope)
- Add a `TODO` without an issue number

## 6. See Also

- [[constitution]] - lint policy (Section IV) and the layer order (Section I)
- [[module-layering-and-typed-commands/spec]] - the layering test source fixture (FR-008, FR-009)
- [[custom-helix-keymap-support/spec]] - keymap startup warnings (FR-005)
- [[data-and-runtime-hardening/spec]] - strict identifier types (section 7.7)
- [[MOC-specs]] - all specifications
