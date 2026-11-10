---
aliases:
  - Challenge Attempt Limit Not Enforced
  - Dead ChallengeProgress
tags:
  - sdd
  - spec
  - bug-fix
  - minigame
  - gamification
  - status/implemented
created: 2026-08-09
status: implemented
related:
  - "[[constitution]]"
  - "[[../arcade-gamification-session-fixes/spec]]"
  - "[[MOC-specs]]"
---

# Feature: Daily Challenge Attempt Limit and Best-Score Tracking Enforcement

> [!info] Metadata
> **Source**: rust-live-tester (live-test finding, escalated via CI cycle), priority P1
> **Resolved by**: GitHub issue #376, merged via PR #384 (`e637c30`)
> **Status**: Implemented
> **Depth**: Lightweight spec, per this project's SDD scaling guidance for a single scoped bug fix. Migrated from an earlier untracked draft into the tracked `specs/` tree and reconciled against the shipped code; the original problem analysis is preserved below, and "Resolution (as shipped)" records what actually landed.

> [!note] Reading guide
> Sections 1-8 preserve the original pre-fix analysis (including line numbers and "currently" wording that describe the code before the fix). Section 9 records the shipped resolution and is authoritative for current behavior.

## 1. Overview

### Problem Statement

Daily Challenge is advertised — in the UI itself ("Daily puzzle. 10 scenarios. 3
attempts...") and in the doc comments of `ChallengeProgress` in
`src/minigame/challenge.rs` — as a once(ish)-a-day puzzle mode: 3 attempts per
calendar day, with best-score and best-scenarios-completed tracked across
sessions. In the live app, none of that enforcement or tracking actually runs.

`ChallengeProgress::start_attempt`, `can_attempt`, `attempts_remaining`, and
`record_result` are fully implemented and unit-tested, but are called from
`#[cfg(test)]` code only — there is zero production call site under
`src/ui/state/handlers/`. The `challenge_progress` field exists on both
`UserProfile` (`src/gamification/profile.rs:82`) and `MiniGameData`
(`src/ui/state/screen.rs:549`, as `Option<ChallengeProgress>`), but nothing
ever reads or mutates either field during actual play — `MiniGameData`'s copy
stays `None` forever and `UserProfile`'s copy stays at
`ChallengeProgress::default()` forever.

The net effect: a player can launch Daily Challenge an unlimited number of
times in a single day with zero warning, no remaining-attempts indicator, and
no degraded behavior — the entire "attempt cap" half of the feature is a
no-op. The "best score today" / "all-time best" promise made in the struct's
own doc comment ("Persisted to UserProfile for cross-session tracking") also
never happens, because nothing ever calls `record_result`.

By contrast, `select_challenge_scenarios` (the deterministic daily-seeded
scenario selection) IS correctly wired into `MiniGameSession::with_mode`
(`src/minigame/session.rs:378`) and was confirmed working live — repeated
Daily Challenge launches on the same calendar day consistently returned the
same 3-scenario preview queue. Only the attempt-limiting and score-persistence
halves of the feature are broken.

> [!note] Persistence claim vs. wire format
> The live-test evidence states `challenge_progress` is "entirely absent"
> from `profile.json` after a clean save. Source inspection shows the field
> IS present on `UserProfile` with `#[serde(default)]` (no
> `skip_serializing_if`), and `ProfileStorage::save` wraps the whole profile
> under a top-level `"profile"` key (`ProfileData { profile: UserProfile }`
> in `src/gamification/storage.rs:82-84`) — so a top-level
> `'challenge_progress' in d` check against the raw JSON would report
> `False` regardless, because the real path is `d['profile']['challenge_progress']`.
> This does not change the core finding: because nothing in production ever
> calls `start_attempt`/`record_result`, the persisted value — wherever it
> lives in the JSON tree — is permanently stuck at
> `ChallengeProgress::default()` (all zeros, `last_challenge_date: None`).
> See Open Questions for the one item that needs confirmation before `/sdd plan`.

### Goal

Daily Challenge enforces a maximum of 3 attempts per calendar day, showing the
player how many attempts remain, and correctly persists best-score and
best-scenarios-completed data (both "today" and "all-time") to `UserProfile`
across app restarts — matching the behavior the `ChallengeProgress` struct and
the mode-selection UI copy already promise.

### Out of Scope

- Changing the attempt cap value (stays at `CHALLENGE_MAX_ATTEMPTS = 3`,
  `src/constants/gameplay.rs:66`).
- Changing the daily scenario-selection algorithm (`select_challenge_scenarios`)
  — already correctly wired and working.
- Adding new UI screens; this only wires an existing `ChallengeProgress`
  struct and its `Option<ChallengeProgress>` slot on `MiniGameData` into the
  existing Daily Challenge launch/completion flow.
- Retroactively reconstructing attempt history lost prior to this fix — a
  player's `challenge_progress` on disk today is `ChallengeProgress::default()`
  and stays that way until the fix ships; no backfill is planned.
- Cross-device/cloud sync of challenge progress (out of scope for the whole
  project, not just this feature).

## 2. User Stories

### US-001: Attempt cap is enforced

AS A player
I WANT the game to stop me after 3 Daily Challenge attempts in a calendar day
SO THAT Daily Challenge stays a genuine once(ish)-a-day puzzle mode rather than
an unlimited-retry arcade variant.

**Acceptance criteria:**
```
GIVEN a player has used all 3 Daily Challenge attempts for today
WHEN they try to launch Daily Challenge a 4th time on the same calendar day
THEN the launch is blocked and the player is shown a message indicating
     attempts are exhausted and when they reset (next calendar day)
```

### US-002: Remaining attempts are visible before committing

AS A player
I WANT to see how many Daily Challenge attempts I have left before I start
SO THAT I can decide whether to spend an attempt now or save it

**Acceptance criteria:**
```
GIVEN a player has used 1 of 3 Daily Challenge attempts today
WHEN they open the "Select Game Mode" submenu / are about to launch Daily
     Challenge
THEN the UI shows "2 attempts remaining" (or equivalent) before or at launch
```

### US-003: Best scores persist across sessions

AS A player
I WANT my best Daily Challenge score (today and all-time) to be remembered
after I close and reopen the app
SO THAT I can track improvement and see my personal record

**Acceptance criteria:**
```
GIVEN a player completes a Daily Challenge attempt with a given score and
     scenarios-completed count
WHEN the attempt ends (game over, all 10 scenarios completed, or the player
     quits mid-session) and the profile is saved
THEN UserProfile.challenge_progress reflects updated attempts_used_today,
     best_score_today, best_scenarios_today, and — if beaten —
     all_time_best_score / all_time_best_scenarios, and total_challenges_attempted
     / total_challenges_completed counters
AND WHEN the app is restarted and profile.json is reloaded
THEN the same values are present and correctly deserialized
```

### US-004: Attempt counter resets on a new calendar day

AS A player
I WANT my attempt count to reset at the start of a new calendar day
SO THAT I get a fresh 3 attempts for each day's puzzle

**Acceptance criteria:**
```
GIVEN a player used all 3 attempts on 2026-08-09
WHEN they launch Daily Challenge on 2026-08-10
THEN the launch succeeds, attempts_used_today resets to reflect the new
     attempt (1 of 3), and best_score_today / best_scenarios_today reset to 0
     for the new day (matching ChallengeProgress::start_attempt's existing
     reset-on-new-day logic)
```

## 3. Functional Requirements

Use EARS notation. Prefix with FR-NNN.

| ID | Requirement | Priority |
|----|------------|----------|
| FR-001 | WHEN a player selects Daily Challenge from the mode-selection submenu THE SYSTEM SHALL check `ChallengeProgress::can_attempt` against the current calendar date before creating a `MiniGameSession` | must |
| FR-002 | WHEN `can_attempt` returns `false` THE SYSTEM SHALL NOT create a Daily Challenge session, SHALL keep the player on the mode-selection screen, and SHALL surface a message stating attempts are exhausted for today | must |
| FR-003 | WHEN a Daily Challenge session is successfully launched THE SYSTEM SHALL call `ChallengeProgress::start_attempt` with the current date on `UserProfile.challenge_progress`, incrementing `attempts_used_today` and `total_challenges_attempted` | must |
| FR-004 | WHEN a Daily Challenge session ends for any reason (out-of-lives game over, all-scenarios-completed, or mid-session abandon back to menu / quit) THE SYSTEM SHALL call `ChallengeProgress::record_result` with the session's final score and scenarios-completed count exactly once per session | must |
| FR-005 | THE SYSTEM SHALL treat the existing `handle_minigame_game_over` idempotency guard (`try_begin_game_over`) as the single choke point for FR-004, so `record_result` is not called more than once for the same session | must |
| FR-006 | WHEN a Daily Challenge session is in progress or about to launch THE SYSTEM SHALL surface `attempts_remaining` to the player via the existing Daily Challenge UI copy area (mode-selection submenu and/or in-session header) | must |
| FR-007 | WHEN the current calendar date differs from `ChallengeProgress.last_challenge_date` THE SYSTEM SHALL treat this as a new day: `attempts_used_today`, `best_score_today`, and `best_scenarios_today` reset per the existing `start_attempt` reset logic (no new logic needed — this behavior already exists in `ChallengeProgress`, only the call site is missing) | must |
| FR-008 | WHEN `UserProfile.challenge_progress` is updated during a Daily Challenge session THE SYSTEM SHALL persist it via the existing `save_immediate` / profile-save path used by `handle_minigame_game_over`, with no new persistence mechanism | must |
| FR-009 | WHERE a Daily Challenge attempt is blocked by the cap THE SYSTEM SHALL still allow the player to freely play Arcade Mode and Survival Mode (the cap applies only to the Challenge `MiniGameMode` variant) | must |

## 4. Non-Functional Requirements

| ID | Category | Requirement |
|----|----------|-------------|
| NFR-001 | Reliability | Attempt-cap and best-score state must survive an abnormal exit as well as a clean quit — reuse the existing non-fatal-on-error `save_immediate` call already invoked in `handle_minigame_game_over`, no weaker guarantee than existing profile fields (e.g. `minigame_high_score`) |
| NFR-002 | Backward compatibility | `ChallengeProgress` already derives `Default` and is `#[serde(default)]` on `UserProfile` — existing `profile.json` files with a stale/default `challenge_progress` (or missing the field entirely, from wire-format drift) must continue to deserialize without error |
| NFR-003 | Consistency | The attempt-cap check (`can_attempt`) and the state mutation (`start_attempt`) must use the same date source (a single `chrono::Utc::now().date_naive()`-equivalent call per launch attempt) to avoid a TOCTOU day-rollover edge case where the check and the mutation disagree about "today" |
| NFR-004 | Testability | New/changed handler logic must be covered by `#[cfg(test)]` unit tests in `src/ui/state/handlers/mode_selection.rs` and `src/ui/state/handlers/minigame.rs`, following existing test patterns in those files (e.g. `test_launch_challenge_mode`) |

## 5. Data Model

No new entities. This feature wires up two structs that already exist but are
currently dead in production.

| Entity | Description | Key Attributes |
|--------|-------------|----------------|
| `ChallengeProgress` (existing, `src/minigame/challenge.rs`) | Per-player Daily Challenge attempt/score tracking, persisted inside `UserProfile` | `last_challenge_date: Option<NaiveDate>` (lenient deserializer; stored as `"YYYY-MM-DD"` or `null`), `attempts_used_today: u8`, `best_score_today: u64`, `best_scenarios_today: u8`, `all_time_best_score: u64`, `all_time_best_scenarios: u8`, `total_challenges_attempted: u32`, `total_challenges_completed: u32` |
| `UserProfile.challenge_progress` (existing field, `src/gamification/profile.rs:82`) | The canonical, persisted copy of `ChallengeProgress` — source of truth read/written across sessions | `crate::minigame::ChallengeProgress` |
| `MiniGameData.challenge_progress` (removed) | Former per-screen `Option<ChallengeProgress>` snapshot. Removed as redundant: the render layer reads `state.progress.profile.challenge_progress` directly, so there is no stale copy after `record_result` | n/a |

## 6. Edge Cases and Error Handling

| Scenario | Expected Behavior |
|----------|-------------------|
| Player has 0 attempts remaining and selects Daily Challenge | Launch is blocked (FR-002); mode-selection screen stays open with a clear "no attempts remaining today" message; Arcade/Survival remain launchable |
| Player launches Daily Challenge, then quits mid-session (Ctrl-Q / back-to-menu) without completing any scenario | Counted as one used attempt (`start_attempt` already ran at launch); `record_result` is still called with whatever score/scenarios-completed the session had accumulated (may be 0/0) so the attempt is not "free" |
| Day rolls over while a Daily Challenge session is already in progress (session started 23:59, ends 00:01 next day) | The in-progress session's `record_result` still applies to the day it was **started** on (`last_challenge_date` set at `start_attempt` time is not re-evaluated mid-session); the next NEW launch attempt is what triggers the day-rollover reset per FR-007 |
| Corrupted/malformed `last_challenge_date` string in `profile.json` (e.g. hand-edited or from a future incompatible version) | Resolved by #427: the field is `Option<NaiveDate>` with a lenient deserializer, so a malformed value or wrong type loads as `None` (treated as never played) instead of failing the whole profile load; the storage format is unchanged |
| Player has never played Daily Challenge (`ChallengeProgress::default()`, `last_challenge_date: None`) | `can_attempt` returns `true` (different-day branch triggers on `None != Some(today)`) — first launch succeeds and starts tracking, no special-case code needed |
| Scenario collection is empty when Daily Challenge is selected | Existing behavior in `handle_launch_minigame_mode` already returns `HandlerOutcome::Stay` with a warning log before any session is created — attempt-cap check (FR-001) must run before or alongside this existing empty-scenarios guard so an attempt is never consumed for a launch that can't actually start |

## 7. Success Criteria

Measurable metrics that prove the feature works:

| ID | Metric | Target |
|----|--------|--------|
| SC-001 | Daily Challenge launches allowed per calendar day per profile | Exactly 3, 4th+ attempt blocked with a visible message |
| SC-002 | `challenge_progress` field round-trips through a real save/reload cycle with non-default values after at least one Daily Challenge attempt | 100% (verified via live-test: start attempt, quit cleanly, inspect `profile.json`'s `profile.challenge_progress`, restart app, confirm values match) |
| SC-003 | Regression: Arcade Mode and Survival Mode attempt counts | Unaffected — no attempt cap introduced for non-Challenge `MiniGameMode` variants |
| SC-004 | Unit test coverage for `can_attempt`/`start_attempt`/`record_result` call sites | New handler-level tests added in addition to the existing struct-level tests in `challenge.rs`; `cargo nextest run --workspace --all-features --lib --bins` passes |

## 8. Agent Boundaries

### Always (without asking)
- Reuse the existing `ChallengeProgress::can_attempt` / `start_attempt` /
  `record_result` / `attempts_remaining` methods as-is — they are already
  correctly implemented and unit-tested; do not rewrite their internal logic
- Reuse the existing `handle_minigame_game_over` idempotency mechanism
  (`try_begin_game_over`) as the single call site for `record_result` rather
  than introducing a second completion path
- Run `cargo nextest run --workspace --all-features --lib --bins` and
  `cargo clippy --all-targets --all-features --workspace -- -D warnings`
  after changes
- Follow existing handler patterns in `src/ui/state/handlers/mode_selection.rs`
  and `src/ui/state/handlers/minigame.rs` (e.g. mirror `test_launch_challenge_mode`
  style for new tests)

### Ask First
- Any change to `CHALLENGE_MAX_ATTEMPTS` or `CHALLENGE_SCENARIO_COUNT`
  constants (`src/constants/gameplay.rs`)
- Any change to `ChallengeProgress`'s public method signatures or its
  serialized field layout (would affect `profile.json` wire format for
  existing users)
- Adding a new UI element/popup beyond reusing existing Daily Challenge copy
  areas mentioned in FR-006
- Lenient date deserialization (formerly the CR-013 TODO) — not part of this
  change; delivered separately by #427

### Never
- Silently drop or reset an existing player's `all_time_best_score` /
  `all_time_best_scenarios` when wiring this up — the fix must read
  whatever is currently in `UserProfile.challenge_progress` (even if it's
  been stuck at defaults) and only ever move it forward
- Commit secrets or credentials (N/A here, noted per project-wide policy)
- Modify `select_challenge_scenarios` or the daily-seed determinism logic —
  confirmed working, out of scope

## 9. Resolution (as shipped)

Merged in PR #384 (issue #376, commit `e637c30`); recorded in `CHANGELOG.md`. Also listed in the thread covered by [[../arcade-gamification-session-fixes/spec]].

- **Launch gate** (FR-001/FR-002/FR-009): `handle_launch_minigame_mode` (`src/ui/state/handlers/mode_selection.rs`) checks `can_attempt(today)` for Challenge mode only. When blocked it pushes an Info notification (`minigame.challenge_no_attempts_left`: "Daily Challenge: no attempts remaining today. Come back tomorrow!") and returns `HandlerOutcome::Stay`. Arcade and Survival are not gated.
- **Attempt consumption** (FR-003, empty-pool edge case): `start_attempt(today)` plus an immediate, non-debounced `save_immediate` run only after the empty-scenario-pool guard, so a launch that cannot start never burns an attempt. A single `today` value feeds both check and mutation (NFR-003).
- **Result recording** (FR-004/FR-005/FR-008): `handle_minigame_game_over` (`src/ui/state/handlers/minigame.rs`) calls `record_result(score, scenarios_completed)` for Challenge sessions inside the existing game-over path, which covers abandon, timeout, session-timeout and back-to-menu exits and is guarded by the existing idempotency check.
- **Visibility** (FR-006): the mode-selection submenu shows attempts remaining as `(N/3)` on the Daily Challenge name line (`src/ui/render/mode_selection.rs`).
- **Day rollover** (FR-007): unchanged `ChallengeProgress` reset logic, now reached through the new call sites.
- **Tests** (NFR-004): `test_launch_challenge_mode_enforces_daily_attempt_cap`, `test_launch_survival_mode_unaffected_by_exhausted_challenge_attempts`, `test_launch_challenge_mode_with_no_scenarios_does_not_consume_attempt`, `test_launch_challenge_mode_resets_on_new_day_via_handler` (handlers/mode_selection.rs); `test_minigame_updates_challenge_progress` (handlers/minigame.rs); `test_render_mode_selection_submenu_shows_challenge_attempts_remaining` (render/tests.rs); `test_challenge_progress_persists_through_profile_storage_round_trip` (gamification/profile.rs).

## 10. Resolved Questions

- **Where to show attempts remaining?** In the submenu, on the mode name line as `(N/3)`. A first implementation appended it to the description line, which overflowed the submenu's fixed 43-column inner width and never reached the screen; entries are now single-line.
- **`MiniGameData.challenge_progress` snapshot or direct read?** Direct read from the profile; the field was removed.
- **Call-site wiring?** In `handle_launch_minigame_mode` and `handle_minigame_game_over`, no new wrapper.
- **CR-013 (malformed `last_challenge_date` hard-errors profile load)?** Resolved by #427: `last_challenge_date` is now `Option<NaiveDate>` with a lenient deserializer (malformed value or wrong type becomes `None`); the `TODO: CR-013` marker and `today_string` were removed.
- **Blocked-attempt copy?** `minigame.challenge_no_attempts_left` in `locales/en.toml`.

## 11. See Also

- [[constitution]] — project principles
- [[MOC-specs]] — all specifications
- `src/minigame/challenge.rs` — `ChallengeProgress` struct and its (formerly
  test-only) call sites
- `src/gamification/profile.rs` — `UserProfile.challenge_progress` field
- `src/ui/state/screen.rs` — (`MiniGameData.challenge_progress` was removed by the fix)
- `src/ui/state/handlers/mode_selection.rs` — `handle_launch_minigame_mode`,
  launch-time call site for `can_attempt`/`start_attempt`
- `src/ui/state/handlers/minigame.rs` — `handle_minigame_game_over`, the
  natural completion-time call site for `record_result`
