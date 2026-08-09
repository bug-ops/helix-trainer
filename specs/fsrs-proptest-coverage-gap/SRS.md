---
aliases:
  - FSRS Proptest Coverage SRS
  - Issue 263 SRS
tags:
  - srs
  - requirements/functional
  - testing
  - learning-scheduler
  - status/implemented
created: 2026-08-09
project: "helix-trainer"
status: implemented
standard: "ISO/IEC/IEEE 29148:2018"
related:
  - "[[BRD]]"
  - "[[NFR]]"
---

# FSRS Scheduler Property-Based Test Coverage Gap: Software Requirements Specification

> [!abstract]
> Functional requirements as originally drafted for issue #263, each marked
> against the actual implementation in commit `33bdaa1`. Traceable to
> [[BRD]].

## 1. Introduction

### 1.1 Purpose

Records the functional requirements originally drafted for FSRS
property-test coverage and evaluates each against what commit `33bdaa1`
actually shipped, so a future reader can see precisely what is verified,
what is not, and why.

### 1.2 Scope

Covers property-based test coverage of `src/learning/scheduler.rs` and
`src/learning/performance.rs` only. Does not cover the `fsrs` crate's
internal algorithm, nor the unrelated CI-hardening fix bundled in the same
commit (documented in [[plan#Bundled Unrelated Change]] for completeness).

### 1.3 Definitions, Acronyms, and Abbreviations

| Term | Definition |
|------|-----------|
| `Scheduler` | Due-review priority queue and selection logic, `src/learning/scheduler.rs` |
| `ReviewItem` | Priority-queue entry for a due review, `src/learning/scheduler.rs` |
| `PerformanceTracker` | Per-command FSRS-derived performance record manager, `src/learning/performance.rs` |
| `CardState` | Application-managed lifecycle state of a practiced command |
| `FakeClock` | Test-only fixed/advanceable clock used to keep property tests deterministic |
| `proptest!` | The `proptest` crate's macro for declaring a property-based test |

### 1.4 References

- [[BRD]] — Business Requirements Document
- [[NFR]] — Non-Functional Requirements Specification
- `.claude/rules/continuous-improvement.md` — source of the original claim

### 1.5 Document Overview

Section 3 lists FR-001 through FR-007 as drafted, each with an
implementation verdict and evidence. Section 4 is the verification matrix
against the actual shipped tests.

## 2. Overall Description

### 2.1 Product Perspective

`src/learning/` is the FSRS spaced-repetition subsystem of the single-binary
helix-trainer TUI. This work added test coverage only — no new
user-observable functionality.

### 2.2 Product Functions

No new functional area. One correctness bug (retrievability sign error) was
found and fixed as a byproduct — see [[BRD#Bug Found By This Work]].

### 2.3 User Classes and Characteristics

Unchanged from [[BRD#Target Users]].

### 2.4 Operating Environment

Unchanged — CI via `cargo nextest run --workspace --all-features --lib --bins`.

### 2.5 Design and Implementation Constraints

Property tests must not read live wall-clock time (NFR-003).

### 2.6 Assumptions and Dependencies

See [[BRD#Assumptions]].

## 3. Specific Requirements

### 3.1 Functional Requirements — Verdict

> [!info] Traceability
> Traces to the draft spec's Functional Requirements section ([[spec#3. Functional Requirements]]).

**FR-001** — WHEN a property test runs the FSRS review-state transition
across arbitrary valid `(stability, difficulty, elapsed_days, rating)`
inputs THE SYSTEM SHALL assert that resulting `stability` and `difficulty`
values remain non-negative and finite.

- *Priority*: must
- *Verdict*: **Implemented differently.** No test isolates the four raw
  parameters directly. Instead, `prop_fsrs_state_stays_in_bounds`
  (`src/learning/performance.rs`) drives the invariant end-to-end through
  `PerformanceTracker::record_attempt` over arbitrary rating/duration/
  elapsed-day sequences, asserting `difficulty ∈ [1.0, 10.0]` and
  `stability > 0.0`. There is no explicit NaN/Inf assertion, but the sign
  bug that was the concrete source of NaN values in practice was fixed in
  the same commit.

**FR-002** — WHEN a property test computes a due date from a review event
with an arbitrary valid interval THE SYSTEM SHALL assert the computed due
date is not earlier than the review timestamp it was derived from.

- *Priority*: must
- *Verdict*: **Not implemented.** No proptest asserts `due >= review_time`.
  A code comment near the due-date computation argues it is "correct by
  construction" (`due = last_review + scheduled_days`, both non-negative by
  type), used as the rationale for omitting the property.

**FR-003** — WHEN a property test feeds an identical sequence of review
ratings into two independently constructed tracker instances THE SYSTEM
SHALL assert both converge to bitwise-identical resulting state.

- *Priority*: must
- *Verdict*: **Fully implemented.**
  `prop_fsrs_state_transition_is_deterministic` runs the same simulated
  sequence twice via a `run()` closure and asserts equality of `stability`,
  `difficulty`, `state`, `reps`, `lapses`, `scheduled_days`, `due`, and
  `retrievability`.

**FR-004** — WHEN a property test generates arbitrary `ReviewItem` values
(including edge-case `priority` floats) and compares them via `Ord`/
`PartialOrd` THE SYSTEM SHALL assert the ordering is total, transitive, and
consistent with `BinaryHeap`'s max-heap contract.

- *Priority*: must
- *Verdict*: **Not implemented.** `src/learning/scheduler.rs` was not
  touched by commit `33bdaa1` (confirmed: empty diff against that file).
  `ReviewItem::Ord` already uses `total_cmp` (pre-existing code) but has
  zero property-test coverage.

**FR-005** — WHEN a property test runs `Scheduler::get_due_reviews`/
`get_review_queue` with an arbitrarily generated set of tracked commands
THE SYSTEM SHALL assert only commands with `due <= now` are returned, and
the queue never exceeds its requested size limit.

- *Priority*: must
- *Verdict*: **Not implemented.** Same as FR-004 — `scheduler.rs` untouched.

**FR-006** — IF resolution path (b) — removal — is selected THEN THE SYSTEM
SHALL remove `proptest`, clean up the TODO, and update the documentation
claim.

- *Priority*: must
- *Verdict*: **Not applicable.** Path (a) was chosen.

**FR-007** — WHERE new property tests are added THE SYSTEM SHALL colocate
them in `#[cfg(test)]` modules within `scheduler.rs`/`performance.rs` (or a
dedicated submodule).

- *Priority*: should
- *Verdict*: **Implemented (scope-limited).** New tests live in the existing
  `#[cfg(test)] mod tests` in `src/learning/performance.rs` — colocated, no
  new submodule needed. Not applicable to `scheduler.rs` since no coverage
  was added there.

**Net**: 2 of 5 "must" FRs fully/adequately satisfied (FR-001 partially/
differently, FR-003 fully); FR-002, FR-004, FR-005 not implemented. FR-007
satisfied. The commit resolved the *state-transition* half of the original
finding's scope but left the *priority-queue/due-selection* half completely
uncovered.

### 3.2 Success Criteria

| ID | Metric | Verdict |
|----|--------|---------|
| SC-001 | `rg -n "proptest!"` matches in `src/learning/` | **Met** — 1 module (`performance.rs`), 2 `proptest!` blocks |
| SC-002 | FR-001..005 implemented as passing property tests, 5/5 | **Partially met** — 2/5 (see FR verdicts above) |
| SC-003 | `cargo nextest run --workspace --all-features --lib --bins` passes, no flakes | **Met** |
| SC-004 | continuous-improvement.md claim never contradicted by codebase state | **Not fully met** — see [[BRD#Residual Gap]] |
| SC-005 | `cargo deny check` passes, no new advisories | **Met** — no new dependency was added |

### 3.3 Logical Data Requirements

No new persistent entities. Existing types exercised by the new property
tests:

| Entity | Description | Key Attributes |
|--------|-------------|----------------|
| `CardState` | Application-managed lifecycle state of a practiced command | `New`, `Learning`, `Review`, `Relearning` |
| Tracked command performance record | Per-command FSRS-derived state | `stability`, `difficulty`, `state`, `retrievability`, `reps`, `lapses`, `scheduled_days`, `due` |
| `ReviewItem` (uncovered) | Priority-queue entry for due reviews (`src/learning/scheduler.rs`) | `id`, `due`, `priority` |

## 4. Verification and Validation

### 4.1 Verification Matrix

| Requirement | Method | Criteria | Status |
|------------|--------|----------|--------|
| FR-001 | `prop_fsrs_state_stays_in_bounds` | `difficulty ∈ [1,10]`, `stability > 0`, `retrievability ∈ [0,1]` after every attempt | **Pass, scoped narrower than drafted (no explicit NaN/Inf assertion)** |
| FR-002 | — | Due date `>= review_time` | **Not run — no test exists** |
| FR-003 | `prop_fsrs_state_transition_is_deterministic` | Two independent trackers converge to identical state given identical input | **Pass** |
| FR-004 | — | `ReviewItem` total/transitive `Ord`, no panics | **Not run — no test exists** |
| FR-005 | — | `get_due_reviews`/`get_review_queue` bounds correctness | **Not run — no test exists** |
| FR-006 | N/A | N/A | **Not applicable** |
| FR-007 | Code review | Colocated in `#[cfg(test)]` | **Pass** |

### 4.2 Acceptance Test Outline

`cargo nextest run --workspace --all-features --lib --bins` runs both new
`proptest!` blocks (default 256 cases each) as part of the standard suite —
no separate invocation required. A regression test,
`test_deserialize_null_retrievability_recovers_as_one`, was added alongside
to lock in the self-healing behavior for already-corrupted `profile.json`
data from the sign bug found during this work.

## 5. Appendices

### 5.1 Traceability Matrix

| BRD Section | SRS Requirement(s) | NFR Requirement(s) |
|----------------|--------------------|--------------------|
| [[BRD#What Was Implemented]] | FR-001 (differently-scoped), FR-003 (full), FR-007 | NFR-001, NFR-003 (met) |
| [[BRD#Residual Gap]] | FR-002, FR-004, FR-005 (not implemented) | NFR-002 (partially met — invariants that exist are traceable; the missing ones are not documented as deferred in-code) |
| [[BRD#Bug Found By This Work]] | N/A — discovered via FR-001's property, not a drafted requirement | N/A |

## See Also

- [[BRD]] — business rationale, decision, and residual gap (source)
- [[NFR]] — non-functional requirements
- [[plan]] — as-built architecture and residual-work recommendation
- [[spec]] — original problem statement
