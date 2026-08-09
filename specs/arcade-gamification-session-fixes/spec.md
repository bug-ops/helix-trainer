---
aliases:
  - Arcade Gamification Session Fixes
  - Double XP / Streak Freeze / Review Level-Up Fixes
tags:
  - sdd
  - spec
  - gamification
  - minigame
  - bug-fix
  - status/implemented
created: 2026-08-09
status: implemented
related:
  - "[[constitution]]"
  - "[[MOC-specs]]"
---

# Feature: Arcade Gamification Session Bookkeeping Fixes

> [!info] Metadata
> **Resolved by**: commit `623e0a8` — "fix(gamification): stop double XP on
> arcade replay, guard zero-streak freeze, notify review level-ups (#322)",
> closing issues #317, #319, #318
> **Status**: Implemented
> **Depth**: Lightweight spec only, per this project's SDD scaling
> guidance ("small bug fix → 3-5 sentences + acceptance criteria") — three
> independent single-guard-clause fixes, 6 files, 179/-9 lines, no new
> types or architecture. A full BRD/SRS/NFR/plan/tasks package would be
> disproportionate. This document exists to fill a genuine documentation
> gap: this work was previously undocumented in any spec, BRD, or decision
> record anywhere in the repository.

> [!note] Not the Same Feature as `002-arcade-game-mode-variety`
> This is unrelated to GitHub issue #264 ("Arcade Game Mode Variety" — a
> proposed reflex-drill minigame mechanic, rejected NO-GO; see
> `specs/arcade-game-mode-variety/`). This spec covers three narrow
> gamification bookkeeping bugs in the *existing* `Arcade`/review-session
> code paths, not a new game mode.

## 1. Overview

### Problem Statement

Three independent gamification bookkeeping defects existed in the arcade
minigame and FSRS review-session paths:

1. **Double XP on arcade replay** (#317) — `handle_minigame_back_to_menu`
   (`src/ui/state/handlers/minigame.rs`) unconditionally re-ran
   `handle_minigame_game_over` whenever a `minigame_session` existed, with
   no check for whether game-over bookkeeping had already run. Since
   `handle_minigame_timeout` already runs it once when lives are depleted
   (without clearing `minigame_session`), pressing the back-to-menu key at
   the game-over screen re-awarded XP, re-incremented
   `minigame_games_played`, and re-recorded an FSRS failure a second time
   on the same session.
2. **Zero-streak freeze guard** (#319) — `StreakManager::update_streak`
   (`src/gamification/streak.rs`) consumed an available streak freeze and
   emitted a `StreakFreezeUsed` notification on any missed day, without
   checking whether `current_streak` was actually nonzero — silently
   burning a freeze "protecting" a streak that was never active.
3. **Missing review-session level-up notification** (#318) —
   `handle_next_review_command` (`src/ui/state/handlers/review.rs`) called
   `profile.add_xp(xp)` and discarded the returned `leveled_up` boolean, so
   a level crossed purely by FSRS review-session XP produced no
   `NotificationType::LevelUp`, inconsistent with the training-mode and
   arcade-mode XP paths, which do notify.

### Goal

Each of the three bugs is fixed with a single additional guard condition,
verified by a dedicated regression test, with no change to public API
surface or data model.

### Out of Scope

- Any change to XP formula, streak-freeze earning rules, or level
  thresholds — only the bookkeeping/guard logic around existing rules
- Arcade game-mode variety (`002-arcade-game-mode-variety`, unrelated,
  separately researched and rejected)

## 2. Acceptance Criteria (as shipped)

```
GIVEN an arcade minigame session has already reached the GameOver state
  (via handle_minigame_timeout, which ran game-over bookkeeping once)
WHEN  the player then triggers MiniGameBackToMenu (Esc/m/Ctrl-q)
THEN  handle_minigame_game_over does NOT run again for that session — no
      additional XP, games-played increment, or FSRS-failure record occurs
```
Test: `test_minigame_back_to_menu_after_game_over_does_not_double_award`
(`src/ui/state/handlers/minigame.rs`).

```
GIVEN a user profile whose current_streak is 0 (never started, or already
  broken) and a streak freeze is available
WHEN  a missed-day streak check runs
THEN  the freeze is NOT consumed and no StreakFreezeUsed notification is shown
```
Tests: `test_streak_freeze_not_consumed_when_streak_already_zero`
(`src/gamification/streak.rs`),
`test_handle_profile_ready_no_freeze_used_for_never_started_streak`
(`src/data_handling.rs`).

```
GIVEN a review session's completion XP award crosses a level threshold
WHEN  the review session completes
THEN  a NotificationType::LevelUp notification is shown reporting the new
      level, consistent with the training-mode and arcade-mode XP paths
```
Tests: `test_review_session_completion_notifies_on_level_up`,
`test_review_session_completion_no_level_up_notification_without_level_up`
(`src/ui/state/tests/review_tests.rs`).

## 3. Functional Requirements

| ID | Requirement | Priority |
|----|------------|----------|
| FR-001 | WHEN the player returns to the menu from the arcade game-over screen AND the minigame session has already reached the `GameOver` state THE SYSTEM SHALL NOT re-run game-over XP, games-played, or FSRS-failure bookkeeping for that session | must |
| FR-002 | WHEN a streak-freeze check occurs on a missed day AND the user's `current_streak` is already 0 THE SYSTEM SHALL NOT consume the available streak freeze and SHALL NOT emit a `StreakFreezeUsed` notification | must |
| FR-003 | WHEN a review session's completion XP award causes the profile to cross a level threshold THE SYSTEM SHALL emit a `LevelUp` notification reporting the new level, consistent with the training-mode and arcade-mode XP paths | must |

All three implemented as-is, no deviation, no residual gap — confirmed by
the six regression tests listed above.

## 4. Data Model

No new persistent entities. No changes to `Profile`, `CardState`, or
`MiniGameStats` schema — only guard conditions on existing fields
(`is_game_over()`, `current_streak`, `leveled_up` return value).

## 5. Edge Cases and Error Handling

| Scenario | Expected Behavior |
|----------|-------------------|
| Player quits mid-game (before `GameOver` state) via back-to-menu | `handle_minigame_game_over` still runs exactly once — the guard only suppresses the *second* run, not the first |
| User has never had an active streak but has a freeze from some other earned source | Freeze is preserved (not silently consumed), per FR-002 |
| Review session XP does not cross a level boundary | No `LevelUp` notification shown — the existing no-op path, verified by `test_review_session_completion_no_level_up_notification_without_level_up` |

## 6. Success Criteria

| ID | Metric | Target | Actual |
|----|--------|--------|--------|
| SC-001 | Regression tests pass for all 3 fixes | 6/6 new tests pass | Met |
| SC-002 | No regression in `cargo nextest run --workspace --all-features --lib --bins` | 100% pass | Met (per CHANGELOG entry) |
| SC-003 | CHANGELOG.md documents all three fixes under their issue numbers | 3/3 entries present | Met — #317, #318, #319 all present under `[Unreleased]` |

## 7. Agent Boundaries

### Always (without asking)
- Run the full check suite after any change touching
  `src/gamification/streak.rs`, `src/ui/state/handlers/minigame.rs`, or
  `src/ui/state/handlers/review.rs`
- Preserve the guard conditions added here when refactoring these handlers

### Never
- Remove the `is_game_over()` guard in `handle_minigame_back_to_menu`
  without an equivalent replacement — doing so reintroduces double-XP
- Remove the `was_streak > 0` guard in `StreakManager::update_streak`
  without an equivalent replacement — doing so reintroduces the zero-streak
  freeze bug

## 8. See Also

- [[constitution]] — project principles
- [[MOC-specs]] — all specifications
- CHANGELOG.md — `[Unreleased]` section, entries for #317, #318, #319
- `specs/arcade-game-mode-variety/` — unrelated feature, do not confuse (see note above)
- `.local/specs/002-arcade-game-mode-variety/spec.md` — the actually-related-by-name-only draft, superseded, also unrelated to this fix
