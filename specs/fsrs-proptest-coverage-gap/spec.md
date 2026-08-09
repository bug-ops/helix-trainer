---
aliases:
  - FSRS Proptest Coverage Gap
  - Proptest Dead Dependency Finding
tags:
  - sdd
  - spec
  - research
  - testing
  - learning-scheduler
  - status/implemented
created: 2026-08-08
status: implemented
related:
  - "[[constitution]]"
  - "[[BRD]]"
  - "[[SRS]]"
  - "[[NFR]]"
  - "[[plan]]"
  - "[[tasks]]"
  - "[[MOC-specs]]"
---

# Feature: FSRS Scheduler Property-Based Test Coverage Gap

> [!info] Metadata
> **Author**: rust-ci-analyst (continuous-improvement finding)
> **Resolved by**: commit `33bdaa1` — "test: harden root-CI failure injection and
> add FSRS proptest coverage (#330)", closing issue #263
> **Priority**: P4
> **Finding type**: research (process/testing gap)
> **Status**: Implemented, partial scope — see [[SRS]] for the FR-by-FR verdict.
> This document is retroactive: it was drafted before implementation and is
> preserved below as the original problem statement, with a Resolution
> section appended describing what actually shipped.

## 1. Overview

### Problem Statement

The project's own continuous-improvement guidance
(`.claude/rules/continuous-improvement.md`) states: "FSRS scheduling is
deterministic given fixed inputs — use proptest for property-based coverage."
The workspace `Cargo.toml` declares `proptest = "1.11"` under
`[dev-dependencies]` (line 45), signaling intent to use it.

A codebase-wide search (`rg -n "proptest" --type rust -g '!target' .`) at the
time this spec was drafted found only a single reference: a doc comment in
`src/minigame/challenge.rs:160` reading "Consider adding proptest for
comprehensive verification of deterministic..." There were zero `proptest!`
macro invocations, zero property test modules, and zero actual usage anywhere
in the codebase.

FSRS scheduling logic lives in `src/learning/scheduler.rs` (`Scheduler`,
`ReviewItem` priority/ordering) and `src/learning/performance.rs`
(`CardState`, `PerformanceTracker`, FSRS `MemoryState` transitions via the
`fsrs` crate). Both modules relied exclusively on example-based unit tests —
exactly the kind of deterministic-given-fixed-inputs logic proptest is meant
to cover.

This was a real gap between documented/stated testing practice and actual
test coverage: `proptest` was dead weight in `Cargo.toml`, and the FSRS
scheduler carried less correctness confidence than the project's own
documentation implied to future maintainers and coding agents relying on
`.claude/rules/continuous-improvement.md` as ground truth.

### Goal

Either (a) `src/learning/scheduler.rs` and the FSRS-related state transitions
in `src/learning/performance.rs` gain property-based test coverage using the
declared `proptest` dependency, with invariants matching the documented
practice, or (b) the `proptest` dependency is removed from `Cargo.toml` and
the claim in `.claude/rules/continuous-improvement.md` is corrected — so that
stated practice and actual coverage are consistent again.

### Out of Scope

- Rewriting or redesigning the FSRS algorithm itself (the `fsrs` crate
  internals are third-party and not under test here — only this project's
  usage/wrapping of it).
- General test-suite refactoring outside `src/learning/` and
  `src/minigame/challenge.rs`'s scenario-selection determinism.
- Adding proptest coverage to unrelated deterministic subsystems unless
  resolution path (a) is chosen and extended later.

## 2. User Stories

### US-001: Maintainer confidence in FSRS correctness
AS A helix-trainer maintainer
I WANT the FSRS scheduler's core invariants verified across a wide range of
generated inputs, not just hand-picked examples
SO THAT I can trust scheduling behavior holds under conditions not
anticipated by example-based tests.

**Acceptance criteria:**
```
GIVEN the FSRS scheduler module (src/learning/scheduler.rs and the FSRS
      transition logic in src/learning/performance.rs)
WHEN  a maintainer runs `cargo nextest run --workspace --all-features --lib --bins`
THEN  property-based tests execute alongside example-based unit tests and
      fail loudly (with a shrunk minimal counterexample) if a documented
      invariant is violated
```

**Status**: partially satisfied — see [[SRS#3.1 Functional Requirements — Verdict]]. `performance.rs` is covered; `scheduler.rs` is not.

### US-002: Documentation matches reality
AS A contributor or coding agent reading `.claude/rules/continuous-improvement.md`
I WANT the stated testing practice to accurately reflect what is actually
implemented in the codebase
SO THAT I do not make false assumptions about existing safety nets.

**Status**: still not fully satisfied after resolution — see [[BRD#Residual Gap]].

### US-003: Lean dependency surface
AS A maintainer concerned with build time and supply-chain surface
I WANT no declared dependency to sit unused in `Cargo.toml`
SO THAT every dependency has a demonstrable purpose.

**Status**: satisfied — path (a) was chosen; `proptest` is now genuinely exercised.

## 3. Functional Requirements

Use EARS notation. Prefix with FR-NNN. These requirements apply **only if
resolution path (a) — add coverage — is selected**; FR-006 covers path (b).
See [[SRS]] for the actual verdict of each.

| ID | Requirement | Priority |
|----|------------|----------|
| FR-001 | WHEN a property test runs the FSRS review-state transition across arbitrary valid `(stability, difficulty, elapsed_days, rating)` inputs THE SYSTEM SHALL assert that resulting `stability` and `difficulty` values remain non-negative and finite | must |
| FR-002 | WHEN a property test computes a due date from a review event with an arbitrary valid interval THE SYSTEM SHALL assert the computed due date is not earlier than the review timestamp it was derived from | must |
| FR-003 | WHEN a property test feeds an identical sequence of review ratings into two independently constructed `PerformanceTracker`/card-state instances THE SYSTEM SHALL assert both instances converge to bitwise-identical resulting state | must |
| FR-004 | WHEN a property test generates arbitrary `ReviewItem` values and compares them via `Ord`/`PartialOrd` THE SYSTEM SHALL assert the ordering is total, transitive, and consistent with `BinaryHeap`'s max-heap contract | must |
| FR-005 | WHEN a property test runs `Scheduler::get_due_reviews`/`get_review_queue` with an arbitrarily generated set of tracked commands THE SYSTEM SHALL assert only commands with `due <= now` are returned as due, and the queue never exceeds its requested size limit | must |
| FR-006 | IF resolution path (b) — removal — is selected THEN THE SYSTEM SHALL remove `proptest` from `Cargo.toml`, clean up the TODO, and update the continuous-improvement doc | must |
| FR-007 | WHERE new property tests are added THE SYSTEM SHALL colocate them in `#[cfg(test)]` modules within `src/learning/scheduler.rs` and `src/learning/performance.rs` | should |

## 4. Non-Functional Requirements

| ID | Category | Requirement |
|----|----------|-------------|
| NFR-001 | Maintainability | New property tests must run within CI time budgets; bound `proptest::ProptestConfig` case counts rather than leaving them unbounded |
| NFR-002 | Consistency | Property test invariants must be traceable back to the specific FSRS/scheduler behavior they verify |
| NFR-003 | Determinism | Property tests must not read live wall-clock time; any time baseline must be injected/fixed |
| NFR-004 | Dependency hygiene | `cargo deny check` must pass afterward with no new advisories |

## 5. Data Model

No new persistent entities. See [[SRS#3.3 Logical Data Requirements]] for
the existing types this finding concerns.

## 6. Edge Cases and Error Handling

See [[SRS#4.1 Verification Matrix]] for which of these are actually covered
by a shipped property test.

| Scenario | Expected Behavior |
|----------|-------------------|
| Property test generates `priority = NaN` for `ReviewItem` | Ordering must use `total_cmp` and must not panic |
| Property test generates zero elapsed days between reviews | FSRS transition must not produce negative stability/difficulty or a due date earlier than "now" |
| Property test generates an extremely large elapsed-days value | FSRS transition must remain finite, no overflow/NaN, no panic |
| Empty tracked-command set passed to `Scheduler::get_due_reviews`/`get_review_queue` | Must return an empty result, not panic |

## 7. Success Criteria

| ID | Metric | Target | Actual |
|----|--------|--------|--------|
| SC-001 | `rg -n "proptest!"` matches in `src/learning/` | >= 1 module | 1 module (`performance.rs`), 2 property blocks |
| SC-002 | FR-001..005 implemented as passing property tests | 5/5 | 2/5 (FR-001 partial/differently, FR-003 full; FR-002/004/005 not implemented) |
| SC-003 | `cargo nextest run --workspace --all-features --lib --bins` passes with new property tests | 100% pass, no flakes | Met |
| SC-004 | Consistency between continuous-improvement.md claim and codebase | Never contradicted | Not fully met — see [[BRD#Residual Gap]] |
| SC-005 | `cargo deny check` after resolution | Passes, no new advisories | Met (no new dependency added) |

## 8. Agent Boundaries

### Always (without asking)
- Run `cargo nextest run --workspace --all-features --lib --bins` after any change here
- Follow the existing colocated `#[cfg(test)]` module convention
- Use a fixed/fake clock, never live `Utc::now()`, inside any new property test

### Ask First
- Extending this coverage to `src/learning/scheduler.rs` (FR-004/FR-005 remain unimplemented — see [[plan#Residual Work]])
- Modifying `.claude/rules/continuous-improvement.md` wording

### Never
- Silently delete existing example-based unit tests to "replace" them with property tests
- Modify `fsrs` crate internals to work around behavior discovered during property testing

## 9. Open Questions

- [NEEDS CLARIFICATION: should `src/learning/scheduler.rs` (FR-004, FR-005 — `ReviewItem` ordering, `get_due_reviews`/`get_review_queue` bounds) receive property-test coverage as a follow-up, given it remains fully untested by this resolution? See [[BRD#Residual Gap]].]
- [NEEDS CLARIFICATION: should `.claude/rules/continuous-improvement.md` be reworded now to scope the proptest claim to "FSRS state-transition logic in `src/learning/performance.rs`" specifically, until/unless `scheduler.rs` coverage is added?]

## 10. Resolution (Retroactive)

Implemented by commit `33bdaa1` (2026-08-09), closing issue #263, bundled
with an unrelated CI-hardening fix (issue #295) in the same commit. Path (a)
— add coverage — was chosen. Full FR-by-FR verdict, property-test inventory,
and residual gap are in [[SRS]] and [[BRD]]. Summary: `performance.rs` (FSRS
state-transition determinism and bounds) is now covered by two property
tests; `scheduler.rs` (`ReviewItem` ordering, due-review queue bounds) was
**not** touched and remains uncovered — see [[plan#Residual Work]] for the
recommended follow-up scope.

A real correctness bug (retrievability decay sign error, causing NaN →
`null` → permanently unloadable `profile.json`) was found and fixed as a
direct result of writing this property-test coverage — see
[[BRD#Bug Found By This Work]].

## 11. See Also

- [[constitution]] — project principles
- [[BRD]] — business rationale, decision, and residual gap
- [[SRS]] — FR-by-FR verdict and property-test inventory
- [[NFR]] — non-functional requirement verdicts
- [[plan]] — as-built architecture and residual work
- [[tasks]] — retroactive task breakdown
- [[MOC-specs]] — all specifications
- `src/learning/scheduler.rs` — `Scheduler`, `ReviewItem` (uncovered)
- `src/learning/performance.rs` — `CardState`, `PerformanceTracker` (covered)
- `.claude/rules/continuous-improvement.md` — source of the proptest claim
