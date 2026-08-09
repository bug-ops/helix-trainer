---
aliases:
  - Arcade Game Mode Variety — Spin-off Findings
  - Issue 264 Spin-off Bugs
tags:
  - sdd
  - appendix
  - minigame
  - bug
  - architecture
created: 2026-08-09
status: findings-only
related:
  - "[[README]]"
  - "[[plan]]"
---

# Appendix: Spin-off Findings (Independent of the #264 Decision)

> [!info] Purpose
> These two bugs were discovered as a byproduct of investigating issue #264.
> They are **not part of the NO-GO decision** recorded in [[README]] — they
> are real, independently actionable defects in code that exists today,
> regardless of what happens with #264. This appendix documents them
> precisely enough for team-lead to file as GitHub issues verbatim. This
> record does **not** file them.

## Spin-off A — Arcade's Advertised Session Time Limit Is Not Implemented

### Summary

The UI promises `Arcade` mode is bounded by a 60-second session timer with 3
lives. No such session-level timer exists in the code. `Arcade` actually runs
until lives are exhausted, with no time bound at all.

### Evidence

- UI promise, rendered to the player in two places:
  - `MiniGameMode::description()` → `"60 seconds, 3 lives, chase the high
    score!"` (`src/minigame/modes.rs:67`, and the same literal again at
    `:457`)
  - `MiniGameModeSelection::mode_description(0)` → the identical string
    (`src/ui/state/screen.rs:459`), rendered via `render_mode_option`'s
    `description` parameter in `src/ui/render/mode_selection.rs`
- The mechanism that should enforce this bound has zero non-test call sites:
  - `MiniGameMode::has_session_timer()` (`src/minigame/modes.rs:43`)
  - `MiniGameMode::session_duration()` (`src/minigame/modes.rs:48`) — both
    are called only from unit tests in `src/minigame/session.rs:1370-1382`
    and `src/minigame/modes.rs:264-372`, never from production code
- `ArcadeConfig::session_duration` defaults to `Duration::from_secs(60)`
  (`src/minigame/modes.rs:114`) — the value exists and is correct, it is
  simply never read outside tests
- The event loop's tick path only checks per-scenario timeout and
  transition auto-advance — no session-level check exists:
  ```
  src/event_loop.rs:140-152
  _ = tick_interval.tick() => {
      if is_minigame_playing(state) && is_minigame_timed_out(state) {
          ui::update(state, Message::MiniGameTimeout)?;
      }
      if should_minigame_advance(state) {
          ui::update(state, Message::MiniGameNextScenario)?;
      }
      ...
  }
  ```
- `MiniGameSession` (`src/minigame/session.rs:240-282`) has no session-level
  clock field at all — `started_at` (`:50`) belongs to `ActiveMiniScenario`
  (per-round), `transition_started_at` (`:266`) is for transitions only
- No `MiniGameSessionTimeout`-equivalent message exists in the `Message` enum

### Impact

User-visible false promise: a player reading "60 seconds, 3 lives" and
choosing `Arcade` for a short session will instead play until they lose all
3 lives, which can take arbitrarily longer (or shorter) than 60 seconds.

### Suggested Filing

- **Title** (imperative, problem not fix): `Arcade mode has no session time limit despite advertised "60 seconds"`
- **Labels**: `bug`, `minigame`, priority — recommend `P2` (matches this
  project's P1/P2 tier definitions: "incorrect scoring... broken FSRS-
  adjacent" for P1 vs. "suboptimal UX, minor display issue" for P2; this is a
  UI-promise-vs-behavior mismatch rather than data loss or incorrect
  scoring, so P2 fits — use judgment if a stronger read is warranted)
- **Reproduction**: Start `Arcade` mode, note the "60 seconds, 3 lives"
  description, play past the 60-second mark without losing 3 lives — the
  session continues.
- **Expected**: Session ends (or at minimum surfaces a warning) at 60
  seconds, matching the advertised description.
- **Actual**: Session runs indefinitely until 3 lives are lost, regardless of
  elapsed time.

## Spin-off B — Silent Non-Exhaustive `_ =>` Fallthroughs in Mode-Selection Code

### Summary

Multiple `match` expressions across the minigame mode-selection code use a
wildcard `_ =>` arm instead of exhaustive matching. Adding a new
`MiniGameMode` variant compiles cleanly without updating every site, and the
unhandled index silently falls back to `Arcade`-equivalent behavior or an
empty/placeholder string rather than failing to compile.

### Evidence

- `src/ui/state/screen.rs:435-464`, three functions on
  `MiniGameModeSelection`, all keyed by `self.selected_index` /
  `index: usize` (an integer, not the enum — inherently non-exhaustive by
  construction):
  - `selected_mode(&self, today) -> MiniGameMode` — `_ =>
    MiniGameMode::default()` (silently resolves to `Arcade`)
  - `mode_name(index: usize) -> &'static str` — `_ => "Unknown"`
  - `mode_description(index: usize) -> &'static str` — `_ => ""`
  - All three are gated by `Self::MODE_COUNT` (currently 3, at
    `screen.rs:413` per the investigation) — bumping `MODE_COUNT` without
    adding a matching arm to all three functions compiles cleanly and
    silently mislabels or misbehaves at runtime.
- `src/minigame/modes.rs`, same hazard class on the enum itself:
  - `has_session_timer()` (`:43-45`) — uses `matches!(self,
    Self::Arcade(_))`, silently `false` for any variant not explicitly
    listed
  - `is_arcade()` / `is_survival()` / `is_challenge()` (`:74-85`) — same
    `matches!` pattern
  - `session_duration()` (`:48-53`) — ends in `_ => None`

### Impact

None of these are compile-time enforced. A future contributor adding a
fourth `MiniGameMode` variant (for this feature or any other) could pass
every `cargo build`/`cargo clippy` check while `MiniGameModeSelection::
selected_mode` silently launches `Arcade` under the new mode's menu label —
a correctness bug that would only surface through manual play-testing, not
through the type system that the rest of this project relies on
(`#![forbid(unsafe_code)]`, typestate patterns elsewhere in `src/game/` and
`src/minigame/session.rs`).

### Suggested Filing

- **Title**: `Non-exhaustive matches in minigame mode-selection code create silent-fallthrough hazard for new modes`
- **Labels**: `architecture`, `minigame`, priority — recommend `P3` (cosmetic
  today since only 3 modes exist and all are correctly indexed; the risk is
  latent, triggered only by a future mode addition)
- **Description**: List all eight call sites above (three in `screen.rs`,
  five in `modes.rs`) in one issue — same fix (replace wildcard/`matches!`
  patterns with exhaustive `match` over the enum, or restructure
  `MiniGameModeSelection` to index by `MiniGameMode` variant rather than raw
  `usize`) applies to all of them.
- **Expected**: Adding a `MiniGameMode` variant without updating every
  consuming `match` fails to compile.
- **Actual**: Compiles cleanly; wrong mode is silently selected or labeled at
  runtime.

## See Also

- [[README]] — decision record index (these findings are appended to, not
  part of, the #264 NO-GO decision)
- [[plan#Cost Basis (Perishable)]] — Spin-off A's fix directly reduces the
  cost basis for a future #264 revisit, since fixing it independently is
  what item 1 of the continuous-variant cost currently double-counts
