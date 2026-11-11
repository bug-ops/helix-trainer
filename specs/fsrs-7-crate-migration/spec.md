---
aliases:
  - FSRS 7 Crate Migration
  - fsrs 6.6 to 7.0 Dependency Bump
tags:
  - sdd
  - spec
  - dependency
  - learning-scheduler
  - status/proposed
created: 2026-11-10
status: proposed
related:
  - "[[constitution]]"
  - "[[fsrs-7-model-adoption/spec]]"
  - "[[fsrs-proptest-coverage-gap/BRD#Residual Gap|Residual Gap]]"
  - "[[MOC-specs]]"
---

# Feature: Migrate `fsrs` Crate from 6.6 to 7.0 While Keeping FSRS-6 Model Behavior

> [!info] Metadata
> **Source**: continuous-improvement research finding (dependency, major bump)
> **Priority**: P3
> **Status**: Proposed
> **Depth**: Lightweight spec, per this project's SDD scaling guidance for a small, mechanical, single-module change (one source file). No plan/tasks packages.

## 1. Overview

### Problem Statement

`Cargo.toml` declares `fsrs = "6.6"`. `fsrs` 7.0.0 was published 2026-11-10; Dependabot (weekly, Mondays) will open a major-bump PR that does not compile. Verified against the published 6.6.2 and 7.0.0 sources:

| Current code (`src/learning/performance.rs`) | Behavior in 7.0.0 | Effect |
|---|---|---|
| `const FSRS_PARAMS: [f32; 21] = fsrs::DEFAULT_PARAMETERS;` (line 21) | `fsrs::DEFAULT_PARAMETERS` is the FSRS-7 table (`[f32; 34]`); the FSRS-6 table is `fsrs::FSRS6_DEFAULT_PARAMETERS` (21 elements) | Type mismatch, compile error |
| `FSRS::new(&FSRS_PARAMS)` (lines 270, 422) | Model version chosen by parameter count: 17/19/21 select FSRS-6, 34 selects FSRS-7, empty selects FSRS-7 | Passing the 21-element FSRS-6 table keeps FSRS-6 math; an empty slice would silently switch models |
| `MemoryState { stability, difficulty }` literals (lines 170, 384, and tests) | `MemoryState` gains a public `stability_fast` field ("for FSRS-6 it equals `stability`") | Compile error at every struct literal |
| `fsrs::current_retrievability(state, days, FSRS_PARAMS[20])` (line 383) | Free function still exists but is FSRS-6-only; new `FSRS::current_retrievability(&self, state, days)` is model-version aware | Works, but bypasses the model the tracker is configured with |

Persisted data in `~/.config/helix-trainer/profile.json` stores only `stability` and `difficulty` per command (`CommandPerformance`). Staying on FSRS-6 parameters means `stability_fast = stability`, so no stored field is added or reinterpreted.

7.0.0 also drops the `log`, `priority-queue`, `strum` and burn dependencies, adds `bytemuck` and `wide`, moves to edition 2024 and declares no `rust-version` (project MSRV is 1.98).

### Goal

The project builds and tests green on `fsrs = "7"` with the learning scheduler producing numerically identical FSRS-6 results (stability, difficulty, retrievability, intervals) for identical review sequences, and with existing `profile.json` files loading unchanged.

### Out of Scope

- Adopting the FSRS-7 model, its 34-parameter table, or any change to scheduling behavior: see [[fsrs-7-model-adoption/spec]].
- Parameter optimization or training from review history.
- Any `profile.json` schema change or migration.
- Resolving the defects behind the `#[ignore]`d scheduler properties (#433, #434); this spec must only not regress them.

## 2. User Stories

### US-001: Maintainer merges the dependency bump safely
AS A maintainer
I WANT the `fsrs` 7 upgrade to compile and preserve current scheduling output
SO THAT the Dependabot major bump can be merged without changing learner-visible behavior

**Acceptance criteria:**
```
GIVEN fsrs = "7" in Cargo.toml and the migration applied
WHEN the full check suite runs (fmt, clippy -D warnings, nextest, doc)
THEN all commands pass with no new warnings and no test expectations altered
```

### US-002: Existing learner keeps their review history
AS A helix-trainer user with an existing profile
I WANT my saved FSRS state to load and schedule exactly as before
SO THAT upgrading does not reset or shift my review schedule

**Acceptance criteria:**
```
GIVEN a profile.json written by the fsrs 6.6 build
WHEN the migrated build loads it and computes due dates and retrievability
THEN results equal those of the 6.6 build for the same stored state and the same "now"
```

## 3. Functional Requirements

| ID | Requirement | Priority |
|----|------------|----------|
| FR-001 | THE SYSTEM SHALL declare `fsrs = "7"` (or an equivalent caret requirement on 7.x) in `Cargo.toml` and update `Cargo.lock` accordingly | must |
| FR-002 | THE SYSTEM SHALL initialize the FSRS model from `fsrs::FSRS6_DEFAULT_PARAMETERS` (21 elements) at every `FSRS::new` call site and never from `fsrs::DEFAULT_PARAMETERS` or an empty slice | must |
| FR-003 | WHEN a `MemoryState` is constructed from persisted data THE SYSTEM SHALL set `stability_fast` equal to `stability` | must |
| FR-004 | WHEN retrievability is computed THE SYSTEM SHALL use the model method `FSRS::current_retrievability` on the tracker's `FSRS` instance instead of the free function with `FSRS_PARAMS[20]` | must |
| FR-005 | WHEN a `MemoryState` is read back from the model after a review THE SYSTEM SHALL persist only `stability` and `difficulty`, leaving the `CommandPerformance` schema unchanged | must |
| FR-006 | THE SYSTEM SHALL pin the model choice at the type level where the crate allows it (for example, a single named constant for the FSRS-6 table whose 21-element array type makes a 34-element table a compile error), so that an accidental FSRS-7 selection cannot compile | should |
| FR-007 | WHEN the migration lands THE SYSTEM SHALL add a `[Unreleased]` entry to `CHANGELOG.md` stating the dependency bump with the PR link, per project rules | must |
| FR-008 | THE SYSTEM SHALL remove the doc comments and workarounds that exist only because of the 6.x free-function API (for example, the note tying `FSRS_PARAMS[20]` to `fsrs::current_retrievability`) | should |

## 4. Non-Functional Requirements

| ID | Category | Requirement |
|----|----------|-------------|
| NFR-001 | Compatibility | `profile.json` files written by any 6.x-based release load without error and produce identical scheduling; no migration step. If this proves impossible, the break MUST be documented in `CHANGELOG.md` before merge (target: none needed) |
| NFR-002 | Reproducibility | Numerical equivalence is exact where the same inputs yield the same `f32` operations; otherwise within a documented tolerance (see SC-002) |
| NFR-003 | Supply chain | `cargo deny check` passes with the new transitive set (`bytemuck`, `wide` added; `log`, `priority-queue`, `strum`, burn removed); no license or advisory regressions |
| NFR-004 | Build | Builds on the project MSRV (1.98) with and without `--features audio`; edition-2024 dependency compiles on that toolchain |
| NFR-005 | Safety | `#![forbid(unsafe_code)]` is unaffected; no new `unsafe` in project code |

## 5. Data Model

No project data-model change.

| Entity | Description | Key Attributes |
|--------|-------------|----------------|
| `CommandPerformance` | Persisted per-command FSRS state (`src/learning/performance.rs`) | `stability`, `difficulty` (unchanged) |
| `MemoryState` (`fsrs` crate) | In-memory model state | `stability`, `difficulty`, plus new `stability_fast` (derived, equals `stability` under FSRS-6, never persisted) |

## 6. Edge Cases and Error Handling

| Scenario | Expected Behavior |
|----------|-------------------|
| Profile has a command with no FSRS state (New card) | `memory_state()` still returns `None`; no `stability_fast` fabricated |
| Profile contains zero or non-finite `stability`/`difficulty` | Same handling as the 6.6 build; no new panic path. [NEEDS CLARIFICATION: does 7.0.0 `FSRS::new`/`next_states` validate inputs differently from 6.6.2 (new `Result` errors) for out-of-range stored values?] |
| `FSRS::new(&FSRS6_DEFAULT_PARAMETERS)` returns `Err` | Same `unwrap`/error policy as today; [NEEDS CLARIFICATION: confirm 7.0.0 still accepts the 21-element table without a deprecation or feature gate] |
| Dependabot PR opened before this migration lands | Expected to fail compilation; close or rebase it onto the migration PR |
| Learner-visible intervals differ after the bump | Treated as a defect; the migration PR is blocked until equivalence (SC-001/SC-002) holds or the difference is explained and documented |

## 7. Success Criteria

| ID | Metric | Target |
|----|--------|--------|
| SC-001 | For a fixed corpus of review sequences (each of the four ratings, first review and repeated reviews, spaced across 0 to 365 days), `stability`, `difficulty` and the scheduled interval from the migrated build vs. golden values captured from the 6.6.2 build | Within 1e-6 relative. Bitwise equality does not hold: a probe of 6.6.2 vs 7.0.0 over `next_states` and retrievability found 295 of 3652 values differing, max relative error 1.13e-7 (a 10x margin under the tolerance). The reason is recorded in the PR |
| SC-002 | `retrievability` from `FSRS::current_retrievability` vs. the 6.6.2 free function for the same state and elapsed days, over a grid of at least 20 elapsed-day values | Equal within 1e-6 absolute |
| SC-003 | Scheduler proptests merged in #435 and `src/learning/performance.rs` proptests | All pass; ignored properties tied to #433/#434 remain `#[ignore]`d with unchanged reasons (neither newly failing nor accidentally un-ignored) |
| SC-004 | Existing `profile.json` fixture written by the 6.6 build | Loads and yields the same due list and retrievability values |
| SC-005 | Full check suite (fmt, clippy `-D warnings`, nextest workspace, doc with `broken_intra_doc_links` denied, `cargo deny check`, `cargo check --features audio`) | All green |

> [!tip] Golden values
> Capture SC-001/SC-002 golden values from the current `main` (fsrs 6.6) before touching `Cargo.toml`, and commit them as test constants so the equivalence test does not depend on the old crate version.

## 8. Agent Boundaries

### Always (without asking)
- Capture golden values on the 6.6 build before changing the dependency
- Run the full check suite and `cargo nextest run scenario` after changes
- Keep changes confined to the `fsrs` call sites in `src/learning/` and their tests
- Update `CHANGELOG.md` `[Unreleased]` and, after the PR exists, add its link

### Ask First
- Any change to `profile.json` structure or serde attributes
- Any parameter table other than `FSRS6_DEFAULT_PARAMETERS`
- Relaxing an equivalence tolerance beyond SC-001/SC-002
- Un-ignoring or editing the `#[ignore]`d proptests (#433/#434)

### Never
- Switch the model to FSRS-7 (34 parameters or empty slice); that belongs to [[fsrs-7-model-adoption/spec]]
- Edit test expectations to make the upgrade pass
- Add `unsafe` code or new dependencies beyond the `fsrs` bump
- Reference gitignored paths from specs, docs or code comments

## 9. Open Questions

> [!success] Resolved
> - 7.0.0 compiles and runs on the project MSRV: a probe built and ran fsrs 7.0.0 with `cargo +1.98`. MSRV stays 1.98; no bump.
> - `f32` output is not bitwise stable across the bump: 295 of 3652 probed values differ from 6.6.2, max relative error 1.13e-7. SC-001 therefore uses the 1e-6 relative tolerance branch.

> [!question] Unresolved
> - [NEEDS CLARIFICATION: Do `next_states` / `memory_state_from_sm2` / other APIs used by `performance.rs` and `scheduler.rs` keep their 6.6 signatures in 7.0.0, beyond the four items verified above?]
> - [NEEDS CLARIFICATION: Should the project keep `fsrs` pinned to `6.6` and ignore the Dependabot major until [[fsrs-7-model-adoption/spec]] is decided? Default: migrate now, since 6.x is unlikely to receive updates.]

## 10. See Also

- [[constitution]] — project principles (testing, MSRV, dependency policy)
- [[MOC-specs]] — all specifications
- [[fsrs-7-model-adoption/spec]] — follow-on spec for adopting the FSRS-7 model
- [[fsrs-proptest-coverage-gap/BRD#Residual Gap|Residual Gap]] — ignored properties tied to #433/#434
- `src/learning/performance.rs` — all affected call sites
- `src/learning/scheduler.rs` — scheduler proptests that must keep passing
