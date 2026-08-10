---
aliases:
  - Arcade Gamification Session Fixes
  - Gamification Live-Trigger Wiring and Notification Fixes
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
  - "[[../end-game-summary-screen/spec]]"
  - "[[MOC-specs]]"
---

# Feature: Gamification Live-Trigger Wiring, Notifications, and Bookkeeping Fixes

> [!info] Metadata
> **Resolved by**: an iterative bug-fix thread across seven commits —
> `573fab8`, `1b1a8bb`, `04f0e8b`, `623e0a8`, `71ed504`, `a2b0185`,
> `e637c30` — each triggered by adversarial/impl-critic review of the
> previous PR, closing issues #256, #257, #267, #292, #309, #310, #317,
> #318, #319, #325, #345, #346, #376.
> **Status**: Implemented
> **Depth**: Lightweight spec, per this project's SDD scaling guidance.
> Each individual fix is small (a guard clause, a missing wiring call, a
> notification), but they form one continuous thread on the same
> subsystem (streaks/freezes/achievements/quests/level-ups/Daily
> Challenge), so they are grouped in one evolving document rather than
> one micro-package per commit. Originally scoped to `623e0a8` alone;
> broadened during the v0.6.0 spec sync (2026-08-10) to cover the full
> thread, which otherwise had no spec coverage anywhere in the
> repository.

> [!note] Not the Same Feature as `arcade-game-mode-variety`
> This is unrelated to GitHub issue #264 ("Arcade Game Mode Variety" — a
> proposed reflex-drill minigame mechanic, rejected NO-GO; see
> `specs/arcade-game-mode-variety/`). This spec covers a family of
> gamification bookkeeping/notification bugs in the *existing*
> Training/Arcade/review-session code paths, not a new game mode.

## 1. Overview

### Problem Statement

Streaks, streak freezes, achievements, quests, and Daily Challenge were
each fully implemented and unit-tested in isolation, but the wiring that
invokes them from live gameplay — and the player-facing notifications
confirming they fired — shipped incrementally and buggily across seven
commits, each surfaced by review of the one before it:

1. **`573fab8`** — `completed_quests_today`, streak-freeze eligibility, and
   achievement unlocks were never invoked from the live scenario-completion
   path: streaks could never increment, freezes could never be earned,
   achievements never unlocked for real players. Fixed by wiring
   `UserProfile::complete_quest` into quest completion, correcting freeze
   eligibility to "every quest generated today" (the quest generator caps
   at 4/day, so the previous fixed threshold of 5 was unreachable), and
   running `AchievementEngine::check_achievements` on scenario completion
   and profile load.
2. **`1b1a8bb`** — adversarial review of (1) found three more gaps: quest
   completion could double-award XP (a stale `previously_completed_quests`
   re-detection tracker ran alongside `award_quest_completion_xp`; removed,
   `QuestXpAward`'s return is now the single source of truth for both the
   applied XP and the results-screen breakdown); `StreakManager::update_streak`
   silently consumed a freeze with no player-facing notification (added
   `NotificationType::StreakFreezeUsed`); achievements unlocked correctly
   but had no persistent UI beyond a transient toast (added a scrollable
   `Achievements` screen, `TypedScreen::Achievements`, reachable via the
   `a` key, following the existing Statistics-screen pattern).
3. **`04f0e8b`** — arcade mode discarded the `leveled_up` result from all
   three of its `add_xp` calls (quest XP, scenario-completion XP,
   end-of-game XP), so level-ups during arcade play never notified, unlike
   training mode; all three now push `NotificationType::LevelUp`.
   `StreakChange::Broken` went through the same match site as `Protected`
   but was only logged at debug level; it now pushes a `StreakBroken`
   notification, gated on `was_streak > 0` to avoid notifying on a streak
   that was already at zero. `Protected`'s vestigial `used_freeze` field
   (always `true` in practice) was dropped.
4. **`623e0a8`** — three more bugs from review of (3): double XP on arcade
   replay (`handle_minigame_back_to_menu` re-ran game-over bookkeeping on a
   session already past `GameOver`); a zero-streak freeze guard gap
   symmetric to (3)'s `StreakBroken` fix but for `StreakFreezeUsed`
   (`StreakManager::update_streak` could consume a freeze and notify while
   `current_streak` was already 0); a missing `LevelUp` notification on the
   FSRS review-session completion path (`handle_next_review_command`
   discarded `add_xp`'s `leveled_up` result, unlike training/arcade).
5. **`71ed504`** — a single streak freeze protected a streak regardless of
   how long the absence was, treating a 2-day gap and a 60-day gap
   identically. A freeze now only covers a gap of up to
   `STREAK_FREEZE_MAX_GAP_DAYS` (3 days, a Friday-to-Monday weekend
   absence); beyond that the streak breaks normally and the freeze is left
   unconsumed. Lower-bounded at 2 days so a negative gap from a backwards
   clock or NTP correction can't silently consume the freeze.
6. **`a2b0185`** — `use_freeze` (the explicit/manual freeze-use path) had
   no gap-length check at all, diverging from (5)'s cap-aware policy on
   `update_streak`. Both now share a `try_consume_freeze` helper;
   `use_freeze` returns a distinct error instead of silently no-oping when
   the gap or streak state can't be covered. `StreakChange::Broken` and the
   `StreakBroken` notification gained a `freeze_could_not_cover_gap` flag
   so a break caused by an out-of-cap gap reads distinctly from a plain
   unprotected break.
7. **`e637c30`** — `ChallengeProgress::can_attempt`/`start_attempt`/
   `record_result` (Daily Challenge mode) were fully implemented and
   unit-tested but had no production call site: Daily Challenge could be
   launched unlimited times per day and best-score tracking never
   persisted. `handle_launch_minigame_mode` now checks `can_attempt` before
   creating a session and only consumes an attempt once the launch is
   actually going to succeed (after the empty-scenario-pool guard);
   `handle_minigame_game_over` records the result for Challenge-mode
   sessions across every exit path; the mode-selection submenu shows
   attempts remaining as `(N/3)`.

### Goal

Every gamification subsystem that ships implemented-and-tested code
(streaks, freezes, achievements, quests, level-ups, Daily Challenge)
actually fires during live gameplay, with a player-facing notification for
every state change, and no double-counting across the
Training/Arcade/review-session paths.

### Out of Scope

- Any change to XP formula, streak-freeze earning rules, or level
  thresholds — only the bookkeeping/guard/notification logic around
  existing rules
- Arcade game-mode variety (`arcade-game-mode-variety`, unrelated,
  separately researched and rejected)
- Daily Challenge's scenario-selection logic itself — only the
  attempt-cap/persistence wiring around it (item 7 above)
- The Achievements screen's layout/rendering beyond following the
  existing Statistics-screen pattern — not a distinct rendering spec

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

```
GIVEN a quest is completed
WHEN  award_quest_completion_xp records XP
THEN  the previously-completed-quests re-detection tracker is not
      consulted — QuestXpAward is the single source of both the applied
      XP and the results-screen breakdown, so no quest can award XP twice
```

```
GIVEN a streak freeze is consumed to protect a streak
WHEN  StreakManager::update_streak (or use_freeze) protects it
THEN  a NotificationType::StreakFreezeUsed notification is shown, and
      unlocked achievements are reachable afterward via the scrollable
      TypedScreen::Achievements screen (key 'a')
```

```
GIVEN an arcade quest/scenario/end-of-game XP award crosses a level
  threshold
WHEN  that award applies
THEN  a NotificationType::LevelUp notification is shown, at parity with
      the training-mode path
```

```
GIVEN current_streak was nonzero and a missed day breaks it
WHEN  the break is recorded
THEN  a NotificationType::StreakBroken notification is shown, gated on
      was_streak > 0
```

```
GIVEN a streak-freeze-eligible gap exceeds STREAK_FREEZE_MAX_GAP_DAYS
  (3 days)
WHEN  update_streak or use_freeze evaluates the gap
THEN  the freeze is left unconsumed, the streak breaks, and the resulting
      StreakBroken notification's freeze_could_not_cover_gap flag is true
```

```
GIVEN a Daily Challenge session has already been attempted 3 times today
WHEN  the player tries to launch another Daily Challenge session
THEN  handle_launch_minigame_mode refuses the launch before creating a
      session, and the mode-selection submenu shows "(3/3)" attempts
      remaining, read live from the profile
```

## 3. Functional Requirements

| ID | Requirement | Priority |
|----|------------|----------|
| FR-001 | WHEN the player returns to the menu from the arcade game-over screen AND the minigame session has already reached the `GameOver` state THE SYSTEM SHALL NOT re-run game-over XP, games-played, or FSRS-failure bookkeeping for that session | must |
| FR-002 | WHEN a streak-freeze check occurs on a missed day AND the user's `current_streak` is already 0 THE SYSTEM SHALL NOT consume the available streak freeze and SHALL NOT emit a `StreakFreezeUsed` notification | must |
| FR-003 | WHEN a review session's completion XP award causes the profile to cross a level threshold THE SYSTEM SHALL emit a `LevelUp` notification reporting the new level, consistent with the training-mode and arcade-mode XP paths | must |
| FR-004 | WHEN quest-completion XP is awarded THE SYSTEM SHALL treat `QuestXpAward`'s return as the single source of truth for both the profile XP applied and the results-screen breakdown — no separate re-detection tracker may award it a second time | must |
| FR-005 | WHEN a streak freeze protects a streak THE SYSTEM SHALL emit a `NotificationType::StreakFreezeUsed` notification (previously silent), and WHEN an achievement unlocks THE SYSTEM SHALL make it reachable afterward via a scrollable `TypedScreen::Achievements` screen bound to the `a` key | must |
| FR-006 | WHEN an arcade quest, scenario-completion, or end-of-game XP award crosses a level threshold THE SYSTEM SHALL emit a `LevelUp` notification, and WHEN a nonzero streak breaks on a missed day THE SYSTEM SHALL emit a `StreakBroken` notification | must |
| FR-007 | WHEN a streak-freeze-eligible gap exceeds `STREAK_FREEZE_MAX_GAP_DAYS` (3 days, lower-bounded at 2) THE SYSTEM SHALL leave the freeze unconsumed and break the streak normally, via a policy shared identically between `update_streak` and `use_freeze` (`try_consume_freeze`); the resulting `StreakBroken` notification SHALL set `freeze_could_not_cover_gap` to distinguish it from a plain unprotected break | must |
| FR-008 | WHEN a Daily Challenge launch is attempted THE SYSTEM SHALL check `ChallengeProgress::can_attempt` before creating a session, consume an attempt only once the launch is actually going to succeed, record the result across every exit path, and display attempts remaining as `(N/3)` in the mode-selection submenu | must |

All eight requirements implemented as-is, confirmed by the regression
tests in each resolving commit.

## 4. Data Model

No new persistent entities beyond what the underlying subsystems already
declared. Changes are to bookkeeping guards, notification emission, and UI
surface:

| Entity | Description |
|--------|-------------|
| `TypedScreen::Achievements` / `AchievementsData` | New scrollable screen, key `a`, mirrors the Statistics-screen pattern |
| `NotificationType::{StreakFreezeUsed, StreakBroken, LevelUp}` | Player-facing notifications wired into every XP/streak path that previously fired silently or not at all |
| `StreakChange::Broken.freeze_could_not_cover_gap: bool` | Distinguishes an out-of-cap break from a plain unprotected one |
| `STREAK_FREEZE_MAX_GAP_DAYS` | Constant (3 days), shared cap enforced by both `update_streak` and `use_freeze` via `try_consume_freeze` |
| `ChallengeProgress` (`can_attempt`/`start_attempt`/`record_result`) | Pre-existing type, now actually invoked from `handle_launch_minigame_mode`/`handle_minigame_game_over` |

No changes to `Profile`, `CardState`, or `MiniGameStats` schema.

## 5. Edge Cases and Error Handling

| Scenario | Expected Behavior |
|----------|-------------------|
| Player quits mid-game (before `GameOver` state) via back-to-menu | `handle_minigame_game_over` still runs exactly once — the guard only suppresses the *second* run, not the first |
| User has never had an active streak but has a freeze from some other earned source | Freeze is preserved (not silently consumed), per FR-002 |
| Review session XP does not cross a level boundary | No `LevelUp` notification shown — existing no-op path |
| A missed-day gap is exactly `STREAK_FREEZE_MAX_GAP_DAYS` | Covered by the freeze (inclusive upper bound) |
| A missed-day gap is negative (backwards clock / NTP correction) | Lower-bounded at 2 days — cannot silently consume the freeze |
| Daily Challenge launch fails after the attempt was already consumed (e.g. empty scenario pool discovered late) | Attempt is only consumed once the launch is confirmed to succeed — a failed launch never burns an attempt |

## 6. Success Criteria

| ID | Metric | Target | Actual |
|----|--------|--------|--------|
| SC-001 | Regression tests pass for all fixes in this thread | All new tests across the 7 resolving commits pass | Met |
| SC-002 | No regression in `cargo nextest run --workspace --all-features --lib --bins` | 100% pass | Met |
| SC-003 | CHANGELOG.md documents every fix under its issue number | 12/12 issue references present under their respective `[Unreleased]`/release entries | Met |
| SC-004 | No remaining silent gamification state change (streak/freeze/level-up) without a player-facing notification | Verified by code review across all XP/streak call sites | Met |

## 7. Agent Boundaries

### Always (without asking)
- Run the full check suite after any change touching
  `src/gamification/streak.rs`, `src/gamification/quests.rs`,
  `src/gamification/achievements.rs`, `src/minigame/challenge.rs`,
  `src/ui/state/handlers/minigame.rs`, or
  `src/ui/state/handlers/review.rs`
- Preserve every guard condition documented in section 3 (FR-001..008)
  when refactoring these handlers
- Route a new gamification state change through a `NotificationType`
  rather than leaving it silent

### Never
- Remove the `is_game_over()` guard in `handle_minigame_back_to_menu`
  without an equivalent replacement — reintroduces double-XP (FR-001)
- Remove the `was_streak > 0` guard on `StreakFreezeUsed`/`StreakBroken`
  without an equivalent replacement — reintroduces zero-streak
  notifications (FR-002, FR-006)
- Let `update_streak` and `use_freeze` diverge on freeze-cap policy again
  — both MUST route through `try_consume_freeze` (FR-007)
- Consume a Daily Challenge attempt before a launch is confirmed to
  succeed (FR-008)

## 8. See Also

- [[constitution]] — project principles
- [[../end-game-summary-screen/spec|End-Game Summary Screen]] — sibling
  progress/gamification UI work from the same release cycle
- [[MOC-specs]] — all specifications
- CHANGELOG.md — entries for #256, #257, #267, #292, #309, #310, #317,
  #318, #319, #325, #345, #346, #376
- `specs/arcade-game-mode-variety/` — unrelated feature, do not confuse
  (see note above)
