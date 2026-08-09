---
aliases:
  - FSRS Proptest Coverage BRD
  - Issue 263 BRD
tags:
  - brd
  - testing
  - learning-scheduler
  - status/implemented
created: 2026-08-09
project: "helix-trainer"
status: implemented
related:
  - "[[spec]]"
  - "[[SRS]]"
  - "[[NFR]]"
  - "[[plan]]"
---

# FSRS Scheduler Property-Based Test Coverage Gap: Business Requirements Document

> [!abstract]
> Records the business rationale for closing a documented testing-practice
> gap — `proptest` declared but unused against the FSRS scheduler — and the
> outcome after implementation. This is a retroactive record: written after
> commit `33bdaa1` shipped, to document what was decided and what actually
> landed against the original finding in [[spec]].

## Executive Summary

`.claude/rules/continuous-improvement.md` claimed FSRS scheduling used
`proptest` for property-based coverage. It did not — `proptest` was declared
in `Cargo.toml` but had zero real usages anywhere in the codebase. This
finding (issue #263) was resolved by adding two property tests to
`src/learning/performance.rs`, which in turn surfaced and fixed a live
correctness bug. The companion module, `src/learning/scheduler.rs`, was left
uncovered — the resolution is **partial**, not complete, relative to the
original finding's full scope.

## Problem Statement

- **What problem existed?** A stated testing practice
  ("use proptest for property-based coverage" of FSRS scheduling) had zero
  backing implementation. `proptest = "1.11"` sat as dead weight in
  `[dev-dependencies]`.
- **Who was affected?** Maintainers and coding agents relying on
  `.claude/rules/continuous-improvement.md` as ground truth for what safety
  nets exist before proposing changes to `src/learning/`.
- **Impact of not solving it**: false confidence in FSRS correctness beyond
  what example-based tests actually verified; continued unused dependency
  surface.

## Decision

> [!important] GO — Path (a): add coverage
> The team chose to implement property-based coverage rather than remove the
> dependency. Commit `33bdaa1` (2026-08-09) added two `proptest!` blocks to
> `src/learning/performance.rs`, closing issue #263. It was bundled in the
> same commit with an unrelated CI-hardening fix for issue #295 (root-CI
> false-positive test, see [[plan#Bundled Unrelated Change]]).

## What Was Implemented

Two property tests were added to `src/learning/performance.rs`
(`#[cfg(test)] mod tests`), both using a fixed clock so results are
deterministic and reproducible:

| Property | Invariant |
|---|---|
| `prop_fsrs_state_transition_is_deterministic` | Running an identical simulated review sequence twice against independently constructed `PerformanceTracker`s yields bit-identical `stability`, `difficulty`, `state`, `reps`, `lapses`, `scheduled_days`, `due`, and `retrievability` |
| `prop_fsrs_state_stays_in_bounds` | After every simulated attempt in an arbitrary rating/duration/elapsed-day sequence: `difficulty ∈ [1.0, 10.0]`, `stability > 0.0`, `retrievability ∈ [0.0, 1.0]` |

Both use `proptest`'s default configuration (256 cases; no explicit
`ProptestConfig` override was added). Generators are inline tuple/range
strategies over `(success: bool, duration_secs: 1..90, optimal_secs: 1..90,
elapsed_days: 0..365)`, composed via `prop::collection::vec(.., 1..12)` or
`1..20` attempts per case.

## Bug Found By This Work

Writing `prop_fsrs_state_stays_in_bounds` surfaced a real defect:
`update_fsrs_state` was passing a hardcoded `-0.5` as the decay parameter to
`fsrs::current_retrievability`, when the correct value is the raw positive
`w[20]` FSRS weight (the crate negates it internally). The wrong sign
produced NaN retrievability under some generated inputs, which `serde_json`
serializes as `null`; the derived `Deserialize` on `CommandPerformance`
therefore failed outright on load, meaning **an affected user's
`profile.json` would become permanently unloadable**. Fixed in the same
commit by reading `fsrs::DEFAULT_PARAMETERS[20]` instead of a hardcoded
literal, plus a custom deserializer (`Option<f32>` → `unwrap_or(1.0)`) so any
already-corrupted `null` value on disk self-heals on next load rather than
hard-failing. This is exactly the class of regression the original finding
argued property testing would catch that example-based tests would not.

## Residual Gap

> [!warning] Not Fully Resolved
> `src/learning/scheduler.rs` (`Scheduler`, `ReviewItem` `Ord`/`PartialOrd`,
> `get_due_reviews`, `get_review_queue`) received **zero** property-test
> coverage from this commit — the diff to that file is empty. FR-004 and
> FR-005 from [[spec]] remain unimplemented. The
> `.claude/rules/continuous-improvement.md` claim, taken literally, is
> therefore still not fully backed by implementation: it is now true for the
> FSRS *state-transition* half of the scheduler subsystem, but not for the
> *priority-queue/due-review-selection* half.

This is recorded here rather than silently treated as fully resolved, per
this project's convention that retroactive specs must document actual
shipped scope, not the original ambition (see [[constitution#VII. Simplicity]]).

## Target Users

### Primary Users
helix-trainer maintainers who need to trust FSRS scheduling correctness
before shipping changes to `src/learning/`.

### Secondary Users
Coding agents (including future SDD pipeline runs) that read
`.claude/rules/continuous-improvement.md` as ground truth.

### Stakeholders
Project maintainers balancing test-suite completeness against P4 priority
(this was a process/coverage gap, not a user-facing defect, prior to the
retrievability bug being found).

## Functional Requirements

See [[SRS]] for the full FR-001..007 list with per-requirement verdicts.
Summary: FR-003 (determinism) fully satisfied; FR-001 (bounds) satisfied via
a differently-scoped property than originally drafted; FR-002, FR-004, FR-005
not implemented; FR-006 not applicable (path (a) was chosen, not (b));
FR-007 (colocation convention) satisfied.

## Non-Functional Requirements

See [[NFR]] for NFR-001..004, all verified against the shipped tests.

## Scope & Boundaries

### In Scope (of what shipped)
- Property-test coverage for `PerformanceTracker`/FSRS state-transition
  determinism and bounds in `src/learning/performance.rs`
- Fixing the retrievability sign bug and its `null`-deserialization
  consequence, discovered via this work

### Out of Scope (of what shipped, contrary to the original finding's ask)

> [!danger] Explicit Gaps
> - `src/learning/scheduler.rs` — `ReviewItem` ordering and due-review
>   queue-bound property tests (FR-004, FR-005) were not implemented
> - Due-date-not-earlier-than-review-timestamp property (FR-002) was not
>   implemented as a standalone property test — the code comment added at
>   the call site argues it is "correct by construction," which is an
>   assertion, not a verified property
> - `.claude/rules/continuous-improvement.md` was not reworded to scope the
>   claim precisely to what is actually covered

## Constraints & Assumptions

### Technical Constraints
Property tests must not read live wall-clock time (satisfied — both use a
`FakeClock` fixed at `2030-01-01T00:00:00Z`).

### Business Constraints
P4 priority; this was addressed opportunistically alongside an unrelated
CI-hardening commit rather than as a scheduled, standalone PR.

### Assumptions

> [!warning] Assumptions
> - The residual `scheduler.rs` gap is assumed acceptable at current
>   priority unless a future finding elevates it (e.g. a real scheduling bug
>   that property testing would have caught)
> - `proptest`'s default case count (256) is assumed sufficient for CI time
>   budgets; no evidence of CI slowdown was found in the researched commit

## Success Criteria

See [[SRS#3.2 Success Criteria]] for the metric-by-metric actual outcome.

## Open Questions

> [!question] Unresolved Items
> - [ ] Should `src/learning/scheduler.rs` property coverage (FR-004,
>       FR-005) be filed as a follow-up P4 finding, given the residual gap
>       documented above?
> - [ ] Should `.claude/rules/continuous-improvement.md` be reworded now to
>       avoid overclaiming coverage it does not have for `scheduler.rs`?

## Glossary

| Term | Definition |
|------|-----------|
| Path (a) / Path (b) | The two resolution options from the original finding: (a) add proptest coverage, (b) remove the unused dependency. Path (a) was chosen. |
| `FakeClock` | Test helper providing a fixed, advanceable timestamp so time-dependent logic is deterministic under proptest |
| Retrievability | FSRS-computed probability `[0.0, 1.0]` that a memory is still recallable at a given point in time |

## See Also

- [[spec]] — original problem statement and retroactive resolution summary
- [[SRS]] — functional requirements, marked with implementation verdicts
- [[NFR]] — non-functional requirements, verified against shipped tests
- [[plan]] — as-built architecture, bundled unrelated change, residual work
- [[tasks]] — retroactive task breakdown
- [[constitution]] — project principles (testing standards, Section III)
