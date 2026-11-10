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
(the project's continuous-improvement rules) states: "FSRS scheduling is
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
the project's continuous-improvement rules as ground truth.

### Goal

Either (a) `src/learning/scheduler.rs` and the FSRS-related state transitions
in `src/learning/performance.rs` gain property-based test coverage using the
declared `proptest` dependency, with invariants matching the documented
practice, or (b) the `proptest` dependency is removed from `Cargo.toml` and
the claim in the project's continuous-improvement rules is corrected — so that
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
AS A contributor or coding agent reading the project's continuous-improvement rules
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
- Extending this coverage to `src/learning/scheduler.rs` (FR-004/FR-005 were later implemented by #428 — see [[plan#Residual Work]])
- Modifying the project's continuous-improvement rules wording

### Never
- Silently delete existing example-based unit tests to "replace" them with property tests
- Modify `fsrs` crate internals to work around behavior discovered during property testing

## 9. Open Questions

- [RESOLVED by #428; three ignored properties tracked by #433/#434] Original question: should `src/learning/scheduler.rs` (FR-004, FR-005 — `ReviewItem` ordering, `get_due_reviews`/`get_review_queue` bounds) receive property-test coverage as a follow-up, given it remains fully untested by this resolution? See [[BRD#Residual Gap]].]
- [NEEDS CLARIFICATION: should the project's continuous-improvement rules be reworded now to scope the proptest claim to "FSRS state-transition logic in `src/learning/performance.rs`" specifically, until/unless `scheduler.rs` coverage is added?]

## 10. Resolution (Retroactive)

Implemented by commit `33bdaa1` (2026-08-09), closing issue #263, bundled
with an unrelated CI-hardening fix (issue #295) in the same commit. Path (a)
— add coverage — was chosen. Full FR-by-FR verdict, property-test inventory,
and residual gap are in [[SRS]] and [[BRD]]. Summary: `performance.rs` (FSRS
state-transition determinism and bounds) is now covered by two property
tests; `scheduler.rs` (`ReviewItem` ordering, due-review queue bounds) was
**not** touched by that commit; #428 later added FR-004/FR-005 coverage (3 properties `#[ignore]`d pending #433/#434) — see [[plan#Residual Work]] for the
recommended follow-up scope.

A real correctness bug (retrievability decay sign error, causing NaN →
`null` → permanently unloadable `profile.json`) was found and fixed as a
direct result of writing this property-test coverage — see
[[BRD#Bug Found By This Work]].

## 11. Follow-Up Requirements: Review Queue Ordering, Eq/Ord Agreement, Reviewable Commands (#433, #434, #505)

> [!info] Added 2026-11-10 (spec-first update for the P1-P3 bug batch)
> #433 and #434 are the defects behind the three `#[ignore]`d properties recorded in [[BRD#Residual Gap]]. This batch fixes them and removes the `#[ignore]` markers, closing the residual gap. #505 extends the scope from scheduler mechanics to what becomes a review card. Requirement IDs here continue after FR-007.

| ID | Requirement | Issue | Priority |
|----|------------|-------|----------|
| FR-008 | WHEN `Scheduler::get_review_queue(tracker, limit)` is called THE SYSTEM SHALL return at most `limit` items ordered by priority descending and containing the `limit` highest-priority due items (top-N, not an arbitrary subset) | #433 | must |
| FR-009 | WHEN two due items have equal priority THE SYSTEM SHALL order them deterministically (earlier `due` first, then command id ascending) so the queue does not depend on hash-map iteration order | #433 | must |
| FR-010 | WHEN two `ReviewItem` values are compared THE SYSTEM SHALL satisfy `a == b` if and only if `a.cmp(&b) == Ordering::Equal`, including `0.0` vs `-0.0` and `NaN` priorities (`PartialEq` is defined via `Ord`) | #434 | must |
| FR-011 | THE SYSTEM SHALL remove `#[ignore]` from `prop_get_review_queue_is_sorted_by_priority_descending`, `prop_get_review_queue_keeps_the_highest_priority_items` and `prop_review_item_eq_agrees_with_ord`, and they SHALL pass | #433, #434 | must |
| FR-012 | WHEN a scenario completion creates or updates review cards THE SYSTEM SHALL create cards only for reviewable Helix commands, represented by a typed `ReviewableCommand` (`src/learning/reviewable.rs`) produced by a smart constructor; `Escape`, text typed in Insert mode (the actions between an entering command such as `i`/`a`/`o`/`c` and `Escape`), single characters that are not commands, argument characters and empty names SHALL NOT produce a card | #505 | must |
| FR-013 | WHEN a recorded action carries a count prefix THE SYSTEM SHALL strip it (`3h` becomes `h`, `3fa` becomes `f`) | #505 | must |
| FR-014 | WHEN a recorded action is a find, till or replace command with its argument character (`f=`, `T,`, `r_`) THE SYSTEM SHALL keep only the command (`f`, `T`, `r`) | #505 | must |
| FR-015 | WHEN a recorded action is a search with a typed pattern (`/ sum`) THE SYSTEM SHALL record the command `/` (and `? sum` the command `?`, see [[../regex-selection-and-macro-commands/spec#11. Bug-Fix Batch: Backward Search, Keep/Remove Selections, Hint Key (#509, #510, #513)\|regex spec section 11]]); selection-regex forms keep the existing normalization (`select_regex`/`split_selection`, FR-009 of [[../regex-selection-and-macro-commands/spec]]) | #505 | must |
| FR-016 | WHEN a recorded action is a surround command with arguments (`ma(`) THE SYSTEM SHALL keep the form `ma(` (the delimiter is part of the skill) | #505 | should |
| FR-017 | THE SAME filtering SHALL apply to Training completions and arcade/minigame completions | #505 | must |
| FR-018 | `normalize_command_id`, used by quests and the minigame scorer, SHALL keep its current contract; review-card filtering is a separate constructor | #505 | must |
| FR-019 | `profile.commands_executed` SHALL keep counting every executed command (it is not filtered by `ReviewableCommand`); only review-card creation is filtered | #505 | must |

### Acceptance Criteria

```
GIVEN a tracker with more due items than `limit`, distinct priorities
WHEN  get_review_queue(tracker, limit) runs
THEN  the result is sorted by priority descending and equals the top `limit` items
```
```
GIVEN recorded actions ["!", "Escape", " ", "3h", "f=", "T,", "r_", "/ sum", "ma("]
WHEN  review cards are extracted
THEN  the cards are exactly [h, f, T, r, /, ma(]  (typed text and Escape produce none)
```

### Edge Cases

| Scenario | Expected Behavior |
|----------|-------------------|
| Persisted junk cards from earlier versions (`!`, `Escape`, `3h`, `f=`) | Dropped once on profile load by FR-020 (#511); valid history is kept |
| `NaN` priority | Orders deterministically via `total_cmp`; equality agrees with ordering |
| Insert-mode typed characters in a solution | Never create cards (everything from the entering command up to `Escape`, exclusive of the entering command itself, is dropped); the entering command (`i`, `a`, `o`, `c`) still does, and `Escape` never does |
| Single typed character outside Insert mode that is not a command | No card |

### Success Criteria

| ID | Metric | Target |
|----|--------|--------|
| SC-006 | No `#[ignore]` left on the scheduler property tests; `ReviewableCommand` unit tests cover count stripping (`3h` to `h`), argument stripping (`f=` to `f`), search (`/ sum` to `/`), surround (`ma(` kept), `Escape` and Insert-mode text dropped | 3 previously ignored properties pass; `ReviewableCommand` tests pass |
| SC-007 | Unit tests for the extraction table above on both the training and minigame paths, plus a test that `profile.commands_executed` still counts all commands and `normalize_command_id` outputs are unchanged | Pass |

## 12. Bug-Fix Batch: Persisted Card Integrity, Panic-Free FSRS Updates, Single Urgency Authority (#511, #464, #469, #470)

> [!info] Added 2026-11-11 (spec-first update for the P2-P3 bug batch)
> Requirement IDs continue after FR-019. FSRS-7 work (#436) is outside this batch and not specified here.

### 12.1 Requirements

| ID | Requirement | Issue | Priority |
|----|------------|-------|----------|
| FR-020 | WHEN a profile is loaded THE SYSTEM SHALL drop every persisted card whose command does not pass the `ReviewableCommand` constructor (FR-012), once and without touching valid cards or their history; the number of dropped cards is logged (`tracing`); this is NOT a profile load failure and does not trigger the quarantine flow of [[../data-and-runtime-hardening/spec]] | #511 | must |
| FR-021 | THE PERSISTED CARD SHALL be parsed at the deserialization boundary into a validated type: stability and difficulty finite and inside their documented ranges (difficulty 0-10, stability >= 0), `successes <= attempts`; the fields are private to the learning module and read through accessors, so the FSRS update path is the only writer | #464 | must |
| FR-022 | WHEN a persisted card fails FR-021 validation THE SYSTEM SHALL drop that card, log the dropped count and command ids, and load the rest of the profile (one policy shared with FR-020: invalid cards are dropped and logged, never a profile-level load failure and never a later panic) | #464 | must |
| FR-023 | THE SYSTEM SHALL NOT persist values derivable from other fields: `avg_time`, `mastery_level` and `retrievability` are computed on read, and the redundant `command` field is not stored (the map key is the command id); older files that still contain these keys SHALL load, ignoring them | #464 | must |
| FR-024 | WHEN `PerformanceTracker` records an attempt and the FSRS state update fails THE SYSTEM SHALL return `LearningError::FsrsError`, record the attempt without changing the schedule, and log the error; production code SHALL contain no `unwrap`/`expect` on the FSRS update or on tracker map lookups (use `entry()` or `let Some(..) else`) | #469 | must |
| FR-025 | `ReviewSession::record_and_next` SHALL return `Result<_, LearningError>` and SHALL NOT panic when called on a completed session | #469 | must |
| FR-026 | THE SYSTEM SHALL compute "how much does this command need practice" in one place, `learning::urgency::command_urgency(perf, now) -> Urgency`, with the clock injected (the tracker's clock; no direct `Utc::now()` in arcade selection). `Urgency` is `Novel` when the tracker has NO entry for the command (the key is the entry, not the presence of FSRS memory), otherwise carries `overdue_ratio` (days overdue divided by `scheduled_days.max(1)`), `difficulty` and `weakness` (`1 - success_rate`). A card with attempts but no FSRS memory stays on the practiced branch with today's behavior | #470 | must |
| FR-027 | EACH CONSUMER SHALL apply a documented projection of `Urgency`. Review priority keeps `overdue_ratio x difficulty / 5` exactly, so review order and the scheduler property tests are unchanged. The arcade weight is `W1 x min(overdue_ratio, 1) + W2 x weakness`, plus the novelty weight for `Novel`; the term stays bounded so no single card dominates the selection and weak but not-due commands keep their weakness weight | #470 | must |
| FR-028 | THE ARCADE SCORER module SHALL be renamed to `minigame::prioritizer` (`ScenarioPrioritizer`) to separate FSRS prioritization from the points scorers (`minigame::scoring`, `game::scorer`), and be constructed from the tracker's clock | #470 | must |

> [!warning] Documented behavior change (#470)
> The arcade overdue term changes from absolute days divided by `FSRS_MAX_OVERDUE_DAYS` to the scheduled-relative ratio capped at 1. Consequence: Learning-state cards (`scheduled_days` 1) reach the cap after one overdue day (formerly one seventh of the cap), so freshly learned commands rank higher in arcade selection than before. Accepted because the term is bounded; recorded in the CHANGELOG.

> [!warning] Breaking persisted format (#464)
> Profiles written by this version omit `retrievability` and other derived keys, so they cannot be read by versions up to 0.6.2. Pre-1.0 policy: accepted, recorded in the CHANGELOG as a breaking change.

### 12.2 Acceptance Criteria

```
GIVEN a profile containing cards Escape, 3h, f= and valid card h
WHEN  the profile loads
THEN  only h remains and the dropped count is logged; no quarantine, no notification of failure
```
```
GIVEN a card with difficulty NaN (or stability -1, or successes > attempts)
WHEN  the profile loads
THEN  that card is dropped and logged, other cards load, and no later review of any command panics
```
```
GIVEN a tracker whose FSRS update returns an error
WHEN  an attempt is recorded
THEN  the attempt is counted, the schedule is unchanged, LearningError::FsrsError is returned and logged
```
```
GIVEN the same tracker and FakeClock
WHEN  Scheduler::get_review_queue and the arcade prioritizer rank commands
THEN  both derive from command_urgency; review order equals the pre-change order; arcade selection is deterministic under FakeClock
```

### 12.3 Edge Cases

| Scenario | Expected Behavior |
|----------|-------------------|
| Tracker has no entry for the command | `Urgency::Novel` (arcade novelty weight; excluded from the review queue as today) |
| Card has attempts but no FSRS memory | Practiced branch, not Novel |
| Old file with `avg_time`/`mastery_level`/`retrievability`/`command` keys | Loads; keys ignored |
| Hand-edited card with `attempts = u32::MAX` | Saturating arithmetic of the data-hardening spec applies (see [[../data-and-runtime-hardening/spec]] FR-030) |

## 13. See Also

- [[constitution]] — project principles
- [[BRD]] — business rationale, decision, and residual gap
- [[SRS]] — FR-by-FR verdict and property-test inventory
- [[NFR]] — non-functional requirement verdicts
- [[plan]] — as-built architecture and residual work
- [[tasks]] — retroactive task breakdown
- [[MOC-specs]] — all specifications
- `src/learning/scheduler.rs` — `Scheduler`, `ReviewItem` (covered by #428)
- `src/learning/performance.rs` — `CardState`, `PerformanceTracker` (covered)
- the project's continuous-improvement rules — source of the proptest claim
