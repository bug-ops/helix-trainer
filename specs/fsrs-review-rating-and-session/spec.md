---
aliases:
  - FSRS Review Rating and Session
  - Review session ownership and rating mapping
tags:
  - sdd
  - spec
  - learning
  - fsrs
  - bug-fix
  - status/proposed
created: 2026-11-11
status: proposed
related:
  - "[[constitution]]"
  - "[[fsrs-7-model-adoption/spec]]"
  - "[[fsrs-proptest-coverage-gap/spec]]"
  - "[[optimal-count-action-counter-unit/spec]]"
  - "[[MOC-specs]]"
---

# Feature: One Review Session Owner and a Deterministic Rating Mapping

> [!info] Metadata
> **Issues**: #550 (review rating mapping and per-command timing), #551 (review session state owned by the UI instead of the learning module)
> **Status**: Proposed
> **Depth**: Lightweight bug-fix spec.

## 1. Overview

### Problem Statement

Two defects share one root: the review flow is not owned by the `learning` module.

1. **Session ownership (#551).** The review session lives in the UI layer (`ReviewSessionState` in `src/ui/state/mod.rs`) as a copy of the due list, an index and result list, so an empty or already-finished session is representable and the `learning` module cannot enforce its own invariants.
2. **Rating (#550).** `Scheduler` derives `optimal_time` from the average duration (`avg_duration.mul_f32(0.8)`), so every attempt has the same ratio to the "optimal" time (1.25) and the FSRS rating no longer depends on how quickly the player answered. The review answer duration is also not measured per command from the moment that command was shown.

### Goal

The `learning` module owns one review session type that cannot be built empty or used after it has finished, and every rating is a pure, monotonic function of success and the ratio of the measured duration to a reference time that does not depend on the measurement itself.

### Out of Scope

- The FSRS model and parameters ([[fsrs-7-model-adoption/spec]])
- Review queue ordering ([[fsrs-proptest-coverage-gap/spec]])
- Redesign of the review screens

## 2. User Stories

### US-001: Review answers reflect speed

AS A learner
I WANT the rating of a reviewed command to reflect how fast I answered it
SO THAT quick answers lengthen the interval and slow ones shorten it

**Acceptance criteria:**
```
GIVEN a command shown at t0 and answered correctly at t0 + 2 s
WHEN  the review is rated
THEN  the rating is Easy, and answering after 20 s gives Hard; a failure always gives Again
```

## 3. Functional Requirements

| ID | Requirement | Priority |
|----|------------|----------|
| FR-001 | THE `learning` module SHALL own one review session type that is started from a NON-EMPTY due list (an empty list yields no session, expressed in the type, not an index check) | must |
| FR-002 | THE review session SHALL be advanced by answering the current item by value (consuming transition); a finished session SHALL be unrepresentable as an "active" session (the final answer returns a distinct finished value carrying the results); the UI copy of the due list, index and results is removed | must |
| FR-003 | THE duration of a review answer SHALL be measured from the moment that command was shown to the answer; per-item timing lives inside the review session, not in the UI | must |
| FR-004 | THE mapping from an answer to an FSRS rating SHALL be: failure gives `Again`; success gives `Easy` for `ratio <= 1.5`, `Good` for `ratio <= 3.0`, otherwise `Hard`; a success SHALL never give `Again`; `ratio` is `duration / ReferenceTime` | must |
| FR-009 | THE rating mapping of FR-004 SHALL be ONE shared function used by the review, training AND arcade (minigame) paths. User decision (2026-11-11): the arcade change is accepted, so a slow success is `Hard` (never `Again`) on every path; the former arcade mapping, where a slow success was `Again`, is removed | must |
| FR-005 | THE training path (completing a scenario) SHALL rate each command against `ReferenceTime = optimal_count x PER_ACTION_BUDGET`, where `optimal_count` is the scenario's optimal action count ([[optimal-count-action-counter-unit/spec]]) and the `0.8 x average duration` reference is removed | must |
| FR-006 | `PER_ACTION_BUDGET` SHALL be one named constant of 3 seconds per optimal action. Rationale: it covers recalling a key plus typing it, so a learner solving at roughly the expected pace (ratio <= 1.5, up to 4.5 s per action) is rated Easy, one needing up to 9 s per action is Good, slower is Hard; the value is easy to retune in one place and any change is a spec change | must |
| FR-007 | THE rating function SHALL be monotonic in duration: for the same success flag and reference time, a longer duration never yields a better rating | must |
| FR-008 | WHEN a review or training answer changes a command's mastery level THE SYSTEM SHALL return the changes as typed levels, in a deterministic order (first occurrence in the session), not as strings and not in hash-map order | must |

## 4. Non-Functional Requirements

| ID | Category | Requirement |
|----|----------|-------------|
| NFR-001 | Type safety | The reference time, duration and ratio are distinct types or one validated type; no bare floats cross the module boundary |
| NFR-002 | Determinism | The same inputs always produce the same rating and the same ordering of mastery changes |
| NFR-003 | Documentation | Public items carry `///` docs with doc-tests per the project rules |

## 5. Data Model

| Entity | Description | Key Attributes |
|--------|-------------|----------------|
| Review session | Owned by `learning`; an active session or a finished value | current item, shown-at instant, collected results |
| `ReferenceTime` | Expected time for a command | `optimal_count x PER_ACTION_BUDGET` |
| Mastery change | A command whose mastery level changed | command, previous level, new level (typed) |

## 6. Edge Cases and Error Handling

| Scenario | Expected Behavior |
|----------|-------------------|
| Empty due list | No session is created; the review entry point shows the "nothing due" state |
| `optimal_count == 0` | `ReferenceTime` is clamped to the budget of one action; no division by zero |
| Duration 0 on success | Easy |
| Very long duration (hours) | Hard on success; no overflow |
| Answering after the session finished | Not expressible (consuming transition) |

## 7. Success Criteria

| ID | Metric | Target |
|----|--------|--------|
| SC-001 | Proptest: rating is monotonic in duration for fixed success and reference | Pass |
| SC-002 | Proptest: a success never yields `Again` | Pass |
| SC-003 | Unit test: thresholds 1.5 and 3.0 are inclusive for Easy and Good | Pass |
| SC-004 | `ReviewSessionState` list/index/results fields removed from `src/ui/state/` | 0 remaining |
| SC-005 | Mastery changes of a fixed session come back in the same order on every run | Pass |

## 8. Agent Boundaries

### Always (without asking)
- Reuse the existing `PerformanceTracker`, `Scheduler` and `CardState` mapping; the arcade callers use the same shared rating function
- Run the full pre-commit suite and `cargo nextest run scenario`

### Ask First
- Changing the thresholds 1.5 and 3.0 or `PER_ACTION_BUDGET`
- Changing the persisted profile or FSRS card format

### Never
- Rate a success as `Again`
- Leave a UI-owned copy of the due list next to the `learning` session

## 9. Open Questions

> [!question]
> - [NEEDS CLARIFICATION: whether 3 s per action should later be derived per scenario difficulty; out of scope here.]

## 10. See Also

- [[constitution]] - project principles
- [[fsrs-7-model-adoption/spec]] - model evaluation; its rating-mapping question (FR-003) is answered by this package
- [[fsrs-proptest-coverage-gap/spec]] - property-test conventions
- [[optimal-count-action-counter-unit/spec]] - the unit of `optimal_count`
- [[MOC-specs]] - all specifications
