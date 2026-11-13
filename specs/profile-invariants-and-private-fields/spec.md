---
aliases:
  - Profile Invariants
  - Private UserProfile fields
  - Load repair
tags:
  - sdd
  - spec
  - gamification
  - persistence
  - status/proposed
created: 2026-11-13
status: proposed
related:
  - "[[constitution]]"
  - "[[profile-screen-xp-progress-label/spec]]"
  - "[[challenge-attempt-limit-not-enforced/spec]]"
  - "[[data-and-runtime-hardening/spec]]"
  - "[[MOC-specs]]"
---

# Feature: `UserProfile` and `ChallengeProgress` Keep Their Invariants by Construction

> [!info] Metadata
> **Issue**: #647 (labels `architecture`, `gamification`)
> **Status**: Proposed
> **Depth**: Lightweight spec.
> **Decision (2026-11-13, user)**: this change stays in the same PR as the other typed-model issues; it is the last step of the series.

## 1. Overview

### Problem Statement

`UserProfile` exposes all 30 fields as `pub` and `ChallengeProgress` exposes its counters, including values derived from or bounded by other fields: `level` is a function of `total_xp`; `longest_streak >= current_streak`; `perfect_scenarios <= scenarios_completed`; `attempts_used_today <= CHALLENGE_MAX_ATTEMPTS`; `best_score_today <= all_time_best_score`; `total_challenges_completed <= total_challenges_attempted`. Production code already goes through methods; only tests write the fields directly, so privacy changes no production behavior. Today only `level` is repaired on load.

### Goal

An illegal profile cannot be built by a caller, and a legacy profile file that violates an invariant is repaired on load, never rejected.

### Out of Scope

- Changing XP curves, streak rules or challenge rules
- Changing the `profile.json` layout beyond dropping the derived `level` key (FR-003)
- Rejecting or quarantining a profile for an invariant violation (see FR-004)

## 2. Functional Requirements

| ID | Requirement | Priority |
|----|------------|----------|
| FR-001 | THE fields of `UserProfile` and `ChallengeProgress` SHALL be private. Outside their modules a value is read through accessors and changed through the existing methods (`add_xp`, `complete_quest`, `unlock_achievement`, `record_*`, ...). Tests that need a specific state use a builder available under `cfg(test)` or a `test-util` feature, not field writes | must |
| FR-002 | THE level SHALL be derived from `total_xp` (`UserProfile::level()` calls `XPCalculator::level_from_xp`); there is no independent stored level that can disagree with the XP. `recompute_level()` is removed | must |
| FR-003 | THE legacy `level` key SHALL be accepted and ignored on load and SHALL NOT be written. A saved profile contains `total_xp` only | must |
| FR-004 | LOADING a profile that violates an invariant SHALL repair it and continue, never reject it, and log each repair at `warn` level: `longest_streak = max(longest_streak, current_streak)`; `attempts_used_today` clamped to `CHALLENGE_MAX_ATTEMPTS` (fail-closed: the attempts stay spent); `all_time_best_score = max(all_time_best_score, best_score_today)`; `total_challenges_attempted = max(attempted, completed)`; `scenarios_completed = max(scenarios_completed, perfect_scenarios)` | must |
| FR-005 | EACH repair of FR-004 SHALL have a legacy fixture, and the legacy-profile fixture loads from every earlier version unchanged in all other fields (see [[data-and-runtime-hardening/spec]] FR-155) | must |
| FR-006 | WHEN a method changes a bounded or derived value THE method SHALL keep every invariant of this package true on return (for example the streak methods keep `longest >= current`; `record_*` keep the best and completed bounds) | must |

## 3. Acceptance Criteria

```
GIVEN a legacy profile.json with total_xp = 1500 and a stored level that does not match
WHEN  the profile loads
THEN  level() equals level_from_xp(1500), a warning is logged, and a later save contains no level key
```
```
GIVEN a legacy profile with current_streak = 9 and longest_streak = 4
WHEN  it loads
THEN  longest_streak is 9 and the profile is accepted
```
```
GIVEN a legacy challenge progress with attempts_used_today = 7
WHEN  it loads
THEN  attempts_used_today equals CHALLENGE_MAX_ATTEMPTS and no attempt is available today
```
```
GIVEN best_score_today = 900, all_time_best_score = 400, completed = 5, attempted = 3, perfect = 8, scenarios_completed = 6
WHEN  the profile loads
THEN  all_time_best_score = 900, attempted = 5, scenarios_completed = 8, each repair logged once
```
```
GIVEN any code outside the gamification module
WHEN  it tries to assign profile.total_xp or profile.level
THEN  it does not compile
```

## 4. Edge Cases

| Scenario | Expected Behavior |
|----------|-------------------|
| Missing optional keys (older file) | Defaults as today (`#[serde(default)]`) |
| Profile already consistent | No repair, no log |
| `total_xp` at `u64` extremes | Derived level saturates as in [[profile-screen-xp-progress-label/spec]] edge cases; no panic (#523 behavior kept) |
| Repair changes a persisted value | Persisted on the next normal save, not by an extra write at load |

## 5. Success Criteria

| ID | Metric | Target |
|----|--------|--------|
| SC-001 | Legacy fixtures in the legacy-profile test, one per repair of FR-004, plus the `level` key case | Pass |
| SC-002 | Compile-time: `rg "pub (level\|total_xp\|current_streak\|longest_streak)" src/gamification/profile.rs` and the `ChallengeProgress` counters | 0 matches |
| SC-003 | Profile load fuzz target and proptest still pass (no panic on arbitrary input) | Pass |

## 6. Agent Boundaries

### Always (without asking)

- Add a repair rule together with its fixture and `warn` log
- Keep new bounded values private with a method that preserves the bound

### Never

- Reject or quarantine a profile for an invariant violation that FR-004 can repair
- Add a public field to `UserProfile` or `ChallengeProgress`

## 7. See Also

- [[constitution]] - project principles (illegal states unrepresentable)
- [[profile-screen-xp-progress-label/spec]] - level progress derived from `total_xp` (SC-002 amended)
- [[challenge-attempt-limit-not-enforced/spec]] - attempt limit this package keeps fail-closed
- [[data-and-runtime-hardening/spec]] - FR-155 legacy-profile fixture
- [[arcade-gamification-session-fixes/spec]] - FR-037 legacy `completed_quests_today`
- [[MOC-specs]] - all specifications
