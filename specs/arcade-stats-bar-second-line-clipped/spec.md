---
aliases:
  - Arcade Stats Bar Second Line Clipped
tags:
  - sdd
  - spec
  - bug-fix
  - ui
  - minigame
  - status/implemented
created: 2026-08-09
status: implemented
related:
  - "[[constitution]]"
  - "[[../arcade-gamification-session-fixes/spec]]"
  - "[[MOC-specs]]"
---

# Feature: Arcade/Survival/Challenge Stats Bar's Second HUD Line Is Always Clipped

> [!info] Metadata
> **Source**: CI cycle (live-testing finding), priority P2
> **Resolved by**: GitHub issue #393, merged via PR #397 (`f7bf0fe`)
> **Status**: Implemented
> **Depth**: Lightweight spec, per this project's SDD scaling guidance for a single scoped bug fix. Migrated from an earlier untracked draft into the tracked `specs/` tree and reconciled against the shipped code; the original problem analysis is preserved below, and "Resolution (as shipped)" records what actually landed.

> [!note] Reading guide
> Sections 1-8 preserve the original pre-fix analysis (including line numbers and "currently" wording that describe the code before the fix). Section 9 records the shipped resolution and is authoritative for current behavior.

## 1. Overview

### Problem Statement

`render_stats_bar` in `src/ui/render/minigame.rs` (lines 294-390) builds a
`ratatui::widgets::Paragraph` containing two `Line`s wrapped in a bordered `Block`:

- `stats_line` (lines 327-345): SCORE, LIVES, MULT, and the grace `[G]` indicator.
- `stats_line2` (lines 347-383): COMBO count, STREAK count, current
  `DifficultyController` level + tier name (Beginner/Intermediate/Advanced), and
  BEST (best streak).

A bordered `Block` with 2 content lines needs a minimum height of 4 rows (1 top
border + 2 content rows + 1 bottom border). Every caller of `render_stats_bar`
allocates the "Stats bar" layout chunk via `Constraint::Length(3)`, confirmed at
three call sites:

- `src/ui/render/minigame.rs:154` — `render_playing`, the main live
  Arcade/Survival/Challenge gameplay screen (constraints array at lines 149-155,
  "Stats bar" is `Constraint::Length(3)`).
- `src/ui/render/minigame.rs:101` — Countdown screen (constraints array at
  lines 98-102, "Stats" is `Constraint::Length(3)`).
- `src/ui/render/minigame.rs:429` — Paused screen (constraints array at
  lines 426-430, third constraint `Constraint::Length(3)`, consumed via
  `chunks[2]` at line 476).

With only 3 rows allocated, the inner content area after the block's top+bottom
border is exactly 1 row — enough for `stats_line` only. `stats_line2`
(COMBO/STREAK/Lv.N tier-name/BEST) is silently dropped by ratatui's `Paragraph`
rendering: it only draws as many lines as fit from the top of the available inner
area, without erroring or wrapping into extra rows.

This is a fixed-layout bug, not a terminal-size issue: `Constraint::Length(N)` is
an absolute row count, so the second line is clipped at any terminal size.
Confirmed empirically by launching the live TUI (`cargo run --no-default-features`)
in a tmux pty at two different heights (`-y 45` and `-y 55`) and capturing the
Arcade Playing-state screen at both — in both cases the stats-bar box shows only
`SCORE: N  LIVES: ... MULT: xN.N [G]` and its closing border immediately follows,
with no COMBO/STREAK/Lv./BEST row ever appearing.

During live Arcade/Survival/Challenge play, the player can never see their current
streak count, combo count, current `DifficultyController` level/tier name, or best
streak — all of which are core feedback for the minigame's core mechanics
(multiplier progression, grace-charge milestones at streak 10/25/50, adaptive
difficulty). The underlying state IS tracked correctly (confirmed via
`MULT:`/`[G]` on the visible line 1, and via score deltas) — this is purely a
rendering/layout defect, not a logic bug. It also explains why prior live-testing
cycles (ci-003 through ci-007) had to infer streak state indirectly from score
deltas rather than reading it directly off the HUD, since the STREAK number was
never actually visible on screen.

### Goal

The stats-bar layout chunk is at least `Constraint::Length(4)` (2 border rows + 2
content rows) at all three call sites, so both `stats_line` (SCORE/LIVES/MULT/grace)
and `stats_line2` (COMBO/STREAK/Lv./BEST) render simultaneously and are visible to
the player during Countdown, Playing, and Paused states of Arcade/Survival/Challenge
modes.

### Out of Scope

- Redesigning the stats-bar content or layout beyond fixing the clipped-row bug
  (e.g., reflowing into a single wider line, adding new stats, changing visual
  style). This fix only corrects the allocated height so existing content is fully
  visible.
- Auditing other bordered multi-line `Paragraph` widgets elsewhere in the codebase
  for the same class of clipping bug. Not requested here; may be filed as a
  separate follow-up if this fix reveals a systemic pattern.
- Changing `DifficultyController` level/tier logic, combo/streak calculation, or
  grace-charge thresholds — this is purely a rendering/layout fix, the underlying
  state is already tracked correctly.

## 2. User Stories

### US-001: See streak and combo feedback during Arcade play
AS A trainee playing Arcade/Survival/Challenge mode
I WANT to see my current STREAK and COMBO counts on the HUD while playing
SO THAT I can track my progress toward multiplier increases and grace-charge
milestones (streak 10/25/50) without inferring them indirectly from score deltas

**Acceptance criteria:**
```
GIVEN the player is in the Playing state of Arcade/Survival/Challenge mode
WHEN the stats bar renders at the bottom of the screen
THEN the bordered stats-bar box shows two content lines: SCORE/LIVES/MULT/[G] on
  the first line and COMBO/STREAK/Lv.N tier-name/BEST on the second line
AND both lines are fully visible within the block's borders at any terminal height
```

### US-002: See difficulty level and best streak during Countdown and Pause
AS A trainee starting or pausing an Arcade/Survival/Challenge session
I WANT the same two-line stats HUD visible on the Countdown and Paused screens
SO THAT difficulty level and best-streak context is consistent across all screens
that show the stats bar

**Acceptance criteria:**
```
GIVEN the player is on the Countdown screen or has paused an active session
WHEN the stats bar renders
THEN both stats_line and stats_line2 are fully visible, matching the Playing screen
```

## 3. Functional Requirements

Use EARS notation. Prefix with FR-NNN.

| ID | Requirement | Priority |
|----|------------|----------|
| FR-001 | WHEN `render_stats_bar` is invoked from `render_playing` THE SYSTEM SHALL allocate at least 4 rows to the "Stats bar" layout chunk (`src/ui/render/minigame.rs:154`) | must |
| FR-002 | WHEN `render_stats_bar` is invoked from `render_countdown` THE SYSTEM SHALL allocate at least 4 rows to the "Stats" layout chunk (`src/ui/render/minigame.rs:101`) | must |
| FR-003 | WHEN `render_stats_bar` is invoked from `render_paused` THE SYSTEM SHALL allocate at least 4 rows to the third layout chunk consumed as `chunks[2]` (`src/ui/render/minigame.rs:429`) | must |
| FR-004 | WHEN the stats bar renders at any of the three call sites THE SYSTEM SHALL display both `stats_line` and `stats_line2` fully within the bordered block, with no line silently dropped | must |
| FR-005 | WHEN the layout-chunk height changes at any call site THE SYSTEM SHALL preserve the total available vertical space for sibling chunks by adjusting a flexible (`Constraint::Min`/`Constraint::Fill`) neighbor rather than shrinking another fixed-height chunk that has its own content requirement | must |

## 4. Non-Functional Requirements

| ID | Category | Requirement |
|----|----------|-------------|
| NFR-001 | Usability | Fix must not introduce visual regressions to sibling chunks (title bar, queue, editor content, timer bar, paused controls) at common terminal sizes (confirmed range: height 45-55 rows, per the finding's reproduction) |
| NFR-002 | Testability | Fix must be verifiable via the project's documented tmux-based scripted-TUI-testing method (a local, untracked testing playbook), capturing the Playing, Countdown, and Paused screens and confirming both HUD lines are present in the captured pane text |
| NFR-003 | Consistency | The three call sites must use the same minimum stats-bar height, so the HUD does not visually resize/jump between Countdown → Playing → Paused transitions |

## 5. Data Model

Not applicable — this is a rendering/layout defect with no persistent entities.
For reference, the relevant in-memory/render constructs are:

| Entity | Description | Key Attributes |
|--------|-------------|----------------|
| `MiniGameSession` | Session state driving stats display | `stats()`, `combo_count()`, `grace_remaining()`, `difficulty_level()` |
| `stats_line` | First HUD `Line` | SCORE, LIVES (hearts), MULT, grace `[G]` indicator |
| `stats_line2` | Second HUD `Line`, currently clipped | COMBO, STREAK, `Lv.N` + tier name, BEST |
| Layout chunk ("Stats bar" / "Stats") | `ratatui::layout::Constraint` controlling the bordered block's allocated height | Currently `Constraint::Length(3)` at 3 call sites; needs `Constraint::Length(4)` or greater |

## 6. Edge Cases and Error Handling

| Scenario | Expected Behavior |
|----------|-------------------|
| Terminal height at the minimum supported size | Stats bar still reserves 4 rows; if this causes sibling chunks (e.g., editor content `Constraint::Min(8)`) to be squeezed below their minimum, the layout should degrade gracefully (ratatui clips excess content, no panic) — same degradation behavior as today, just shifted to a different chunk under extreme size pressure |
| Countdown, Playing, and Paused screens rendered in sequence during one session | Stats-bar height stays consistent (4 rows) across all three, avoiding a visible HUD resize/jump on screen transitions |
| `stats.streak()` or `combo_count()` reaches a large value that widens `stats_line2` beyond the terminal width | Existing `Alignment::Center` + `Paragraph` wrapping behavior applies unchanged; this fix only addresses vertical clipping, not horizontal |
| Difficulty level outside the known Beginner/Intermediate/Advanced ranges | Falls back to `("Unknown", Color::Gray)` per existing logic (line 306) — unaffected by this fix |

## 7. Success Criteria

Measurable metrics that prove the feature works:

| ID | Metric | Target |
|----|--------|--------|
| SC-001 | tmux-captured pane text of the Arcade Playing screen contains both `SCORE:`/`LIVES:`/`MULT:` and `COMBO:`/`STREAK:`/`Lv.`/`BEST:` substrings inside the stats-bar block | Pass, at terminal heights 45 and 55 (per original reproduction) |
| SC-002 | Same check repeated for the Countdown screen | Pass |
| SC-003 | Same check repeated for the Paused screen | Pass |
| SC-004 | `cargo nextest run --workspace --all-features --lib --bins` | 100% pass, zero regressions |
| SC-005 | `cargo clippy --all-targets --all-features --workspace -- -D warnings` and `cargo +nightly fmt --check` | Pass |

## 8. Agent Boundaries

### Always (without asking)
- Reproduce the bug first (per repo triage protocol) via the tmux method documented
  in the project's local testing playbook before implementing the fix, and
  re-capture after the fix to confirm both HUD lines are visible
- Fix all three call sites (`render_playing` line 154, `render_countdown` line 101,
  `render_paused` line 429) together, not just one — the finding confirms the same
  defect at all three
- Run `cargo +nightly fmt --check`, `cargo clippy --all-targets --all-features
  --workspace -- -D warnings`, `cargo nextest run --workspace --all-features --lib
  --bins` before considering the fix complete
- Follow existing patterns in `src/ui/render/minigame.rs` for `Layout`/`Constraint`
  usage
- Update `CHANGELOG.md` under `[Unreleased]` once the fix lands

### Ask First
- Any layout change that shrinks another sibling chunk's fixed height to compensate
  (e.g., reducing the title bar or timer bar below 3 rows) — prefer adjusting a
  `Constraint::Min`/`Constraint::Fill` neighbor instead
- Any change to the visual content or styling of `stats_line`/`stats_line2` beyond
  what's needed to make both fully visible

### Never
- Remove or truncate `stats_line2`'s content (COMBO/STREAK/Lv./BEST) as a
  workaround instead of fixing the layout height
- Modify `DifficultyController`, `Scorer`, or combo/streak calculation logic —
  this is a rendering-only fix

## 9. Resolution (as shipped)

Merged in PR #397 (issue #393, commit `f7bf0fe`); recorded in `CHANGELOG.md`.

- **Fix** (FR-001 to FR-004, NFR-003): all three call sites in `src/ui/render/minigame.rs` (`render_countdown`, `render_playing`, `render_paused`) now allocate `Constraint::Length(4)` for the bordered stats bar (2 border rows + 2 content rows).
- **Sibling chunks** (FR-005): the paused layout's `Constraint::Min(8)` message chunk and the playing layout's `Constraint::Min(8)` editor chunk absorb the extra row; no other fixed-height chunk was shrunk. A comment in `render_playing` warns that, at the 80x24 minimum terminal size, the editor chunk has no spare rows above the key-history popup's visibility threshold, so other fixed chunks must not grow without re-checking.
- **Tests** (NFR-002 in unit-test form): `test_render_playing_stats_bar_second_line_not_clipped`, `test_render_countdown_stats_bar_second_line_not_clipped`, `test_render_paused_stats_bar_second_line_not_clipped` assert the second line's content is present in the rendered buffer.

## 10. Resolved Questions

- **Does `render_paused` have room for the extra row?** Yes; the `Min(8)` paused-message chunk flexes.
- **Does 4 rows squeeze the editor chunk at the smallest supported size?** At 80x24 the Playing editor chunk is at its minimum with no slack (see the `render_playing` comment); this migration did not re-verify that with a tmux capture.

## 11. See Also

- [[MOC-specs]] — all specifications
- `src/ui/render/minigame.rs` — `render_stats_bar` (builds the two-line `Paragraph`), and its call sites `render_countdown`, `render_playing`, `render_paused`
- The project's local tmux-based scripted-TUI-testing playbook (not tracked) — method used to reproduce this bug
