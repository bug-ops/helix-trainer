---
aliases:
  - End-Game Summary Screen
  - Curriculum Completion Screen
tags:
  - sdd
  - spec
  - ui
  - gamification
  - status/implemented
created: 2026-08-10
status: implemented
related:
  - "[[constitution]]"
  - "[[MOC-specs]]"
---

# Feature: End-Game Summary Screen on Curriculum Completion

> [!info] Metadata
> **Resolved by**: commit `0dbfcbe` — "feat(ui): add end-game summary
> screen on curriculum completion (#342)", closing issue #145.
> **Status**: Implemented
> **Depth**: Lightweight spec, per this project's SDD scaling guidance —
> single self-contained UI feature (one new screen, one new
> `ScenarioCollection` query surface), retroactively documented because it
> shipped with no spec anywhere in the repository.

## 1. Overview

### Problem Statement

Before this feature, completing the last available scenario left the
Results screen's `(n)` "next" key with nowhere to go — reaching the end of
the curriculum was a dead end rather than a milestone.

### Goal

Completing the curriculum transitions to a dedicated `EndGameSummary`
screen presenting a full progress recap and concrete next steps, computed
from the actual unfiltered scenario/history data rather than
replay-inclusive lifetime counters.

### Out of Scope

- Any change to scoring, XP, or FSRS mechanics
- A "New Game+" or reset-and-replay-all mode
- Persisting a distinct "curriculum completed" flag — completion is
  always recomputed live from `ScenarioCollection` + `ScenarioHistory`

## 2. User Stories

### US-001: See a milestone summary on finishing the curriculum
AS A learner who has completed every available scenario
I WANT a dedicated summary screen showing my overall progress
SO THAT reaching the end of the curriculum feels like a real milestone
with clear next steps, not a dead end

**Acceptance criteria:**
```
GIVEN the player has completed every scenario in the (unfiltered) library
  at least once, per ScenarioCollection::is_curriculum_complete
WHEN  they press (n) on the Results screen
THEN  the system transitions to TypedScreen::EndGameSummary, showing:
      total scenarios, perfected count, lifetime completions, level, XP,
      command success rate, commands mastered, a per-category mastery
      breakdown, and next-step suggestions (due FSRS reviews, open
      quests, imperfect scenarios, Arcade mode)
```
```
GIVEN the player has not yet completed the curriculum
WHEN  they view the Results screen
THEN  no discoverability hint for the EndGameSummary transition is shown —
      the hint is computed live, exactly when (n) would lead there
```

## 3. Functional Requirements

Use EARS notation. Prefix with FR-NNN.

| ID | Requirement | Priority |
|----|------------|----------|
| FR-001 | WHEN `ScenarioCollection::is_curriculum_complete(profile)` is `true` THE SYSTEM SHALL make the Results screen's `(n)` key transition to `TypedScreen::EndGameSummary` instead of the previous no-op | must |
| FR-002 | WHEN rendering the `EndGameSummary` screen THE SYSTEM SHALL display total scenarios, perfected count, lifetime completions, level, XP, command success rate, commands mastered, and a per-category mastery breakdown | must |
| FR-003 | WHEN rendering the `EndGameSummary` screen THE SYSTEM SHALL surface next-step suggestions: due FSRS reviews, open quests, imperfect (non-mastered) scenarios, and Arcade mode | must |
| FR-004 | WHEN the Results screen is rendered THE SYSTEM SHALL compute, live, whether `(n)` would lead to `EndGameSummary` and show a discoverability hint exactly then | should |
| FR-005 | WHEN computing curriculum-completion state THE SYSTEM SHALL use `ScenarioCollection::curriculum_stats`/`completed_count`/`is_curriculum_complete` — joining the unfiltered scenario set against `scenario_history` — instead of the replay-inclusive `profile.scenarios_completed`/`perfect_scenarios` counters, which double-count replays of an already-mastered scenario | must |

## 4. Non-Functional Requirements

| ID | Category | Requirement |
|----|----------|-------------|
| NFR-001 | Correctness | `is_curriculum_complete` on an empty scenario collection (`ScenarioCollection::new(vec![])`) MUST return `false`, never vacuously `true` |
| NFR-002 | Performance | `completed_count` is a fast, per-frame-safe query (used for the live Results-screen hint); `curriculum_stats` is a single-pass aggregate that allocates a `per_category` breakdown, called only when actually rendering the summary screen |
| NFR-003 | Consistency | Rendering (`src/ui/render/end_game.rs`) MUST remain pure — no side effects, computed entirely from `AppState`, per the constitution's rendering principle |

## 5. Data Model

| Entity | Description |
|--------|-------------|
| `ScenarioCollection::curriculum_stats()` | Single-pass aggregate over the unfiltered scenario set joined against `scenario_history`: totals, per-category completion breakdown |
| `ScenarioCollection::completed_count(&profile)` | Fast, allocation-free count derived from the same join, safe to call every frame for the Results-screen hint |
| `ScenarioCollection::is_curriculum_complete(&profile)` | `true` iff every scenario in the collection has at least one completion in history, and the collection is non-empty |
| `TypedScreen::EndGameSummary` | New screen variant carrying the computed summary snapshot |

No new persistent entities — the summary is always recomputed live from
existing `ScenarioCollection` and `UserProfile`/`ScenarioHistory` data.

## 6. Edge Cases and Error Handling

| Scenario | Expected Behavior |
|----------|-------------------|
| Scenario library is empty | `is_curriculum_complete` returns `false` — no summary is reachable (NFR-001) |
| Player replays an already-mastered scenario after reaching the summary once | Replay does not re-trigger the transition; only pressing `(n)` from the Results screen does, and only while `is_curriculum_complete` still holds |
| A new scenario is added to the library after the player had completed everything | `is_curriculum_complete` becomes `false` again on next evaluation (recomputed live, not cached) — the discoverability hint disappears until the new scenario is also completed |

## 7. Success Criteria

| ID | Metric | Target |
|----|--------|--------|
| SC-001 | `is_curriculum_complete` correctness | Verified by doc-tests and unit tests, including the empty-collection edge case |
| SC-002 | No regression in existing Results-screen `(n)` behavior for an incomplete curriculum | `cargo nextest run --workspace --all-features --lib --bins` green |

## 8. Agent Boundaries

### Always (without asking)
- Keep `curriculum_stats`/`completed_count`/`is_curriculum_complete` as the
  single source of truth for curriculum-completion state over the
  replay-inclusive profile counters

### Never
- Reintroduce a dead-end for `(n)` once the curriculum is complete
- Cache curriculum-completion state across frames in a way that could go
  stale relative to `ScenarioHistory`

## 9. See Also

- [[constitution]] — project principles, rendering purity
- [[../arcade-gamification-session-fixes/spec|Gamification Live-Trigger
  Wiring, Notifications, and Bookkeeping Fixes]] — sibling
  progress/notification work from the same release cycle
- [[MOC-specs]] — all specifications
- Issue #145 — original feature request
