---
aliases:
  - Key-History Popup Training Overlap
tags:
  - sdd
  - spec
  - bug-fix
  - ui
  - status/implemented
created: 2026-08-09
status: implemented
related:
  - "[[constitution]]"
  - "[[../regex-selection-and-macro-commands/spec]]"
  - "[[MOC-specs]]"
---

# Feature: Key-History Popup Corrupts Task Screen Panel Borders in Training Mode

> [!info] Metadata
> **Source**: ci-004 (rust-live-tester, 2026-08-09, commit `463d363`); follow-up to #272 (closed, fixed for Arcade mode only)
> **Resolved by**: GitHub issue #364, merged via PR #372 (`7f34268`)
> **Status**: Implemented
> **Depth**: Lightweight spec, per this project's SDD scaling guidance for a single scoped bug fix. Migrated from an earlier untracked draft into the tracked `specs/` tree and reconciled against the shipped code; the original problem analysis is preserved below, and "Resolution (as shipped)" records what actually landed.

> [!note] Reading guide
> Sections 1-8 preserve the original pre-fix analysis (including line numbers and "currently" wording that describe the code before the fix). Section 9 records the shipped resolution and is authoritative for current behavior.

## 1. Overview

### Problem Statement

`render_key_history_popup` (`src/ui/render/popups.rs`) renders a `tui-big-text`
glyph popup showing the last `KEY_HISTORY_DISPLAY_SIZE` (5) pressed keys. It is
shared between Training mode's Task screen (`src/ui/render/task.rs:240`, called
with `reserved_bottom = 7`) and Arcade mode's minigame HUD
(`src/ui/render/minigame.rs:55`, `reserved_bottom = 6`).

The popup's width is computed purely from the key-history text length —
`(chars_count * BIG_TEXT_CHAR_WIDTH_CELLS).max(KEY_HISTORY_MIN_WIDTH)`, capped
only by `area.width.saturating_sub(4)` (the full terminal width minus a
constant margin) — and is horizontally anchored to the bottom-right corner of
the entire terminal (`popup_x = area.width - (popup_width + 2)`). `reserved_bottom`
only reserves *vertical* space above the Stats/Instructions HUD bars; it has no
awareness of the Task screen's own panel layout (Current State / Target State
side-by-side panels) or their column boundaries.

As a result, whenever the key-history text is long enough — e.g. during a
multi-keystroke pending input (regex-selection prompt `s`/`S`, macro
record/replay `q`/`Q`, named-register operations `"a` + motion) — the popup's
rendered `Rect` extends leftward past the Target State panel's right border and
paints directly over it, corrupting the panel's box-drawing characters (visible
as doubled/misaligned corners, e.g.
`└───────────────────────────└────────────────────────────┘┘`).

This was previously reported as part of #272 ("arcade key-history popup is
never cleared, permanently overlapping Target/Timer/Score HUD"), which was
fixed for Arcade mode and closed. A follow-up comment on #272 (ci-003,
2026-08-08) noted the same popup also transiently touched the Training-mode
Task screen but characterized it as "gated/cleared there, so less severe" —
implying it self-resolved and needed no separate fix. Live testing in ci-004
(2026-08-09) found this is incorrect: the corruption is reproducible and
severe (not merely transient/cosmetic) for the full duration any multi-key
pending input is active, and persists briefly after completion on the SUCCESS
screen, across three independent scenario types. This is functionally distinct
from #272's original defect (permanent, never-cleared overlap) — here the
popup does clear once `key_history`/`show_key_history` resets, but while
visible, it structurally corrupts the Target panel's own border rather than
overlapping empty space around it.

### Goal

The key-history popup renders entirely within its own bounded region and never
overlaps or overwrites another panel's border or content, in both Training
mode's Task screen and Arcade mode's minigame HUD, regardless of key-history
text length.

### Out of Scope

- Redesigning the key-history popup's visual style, font, or position choice
  (bottom-right) — only its bounding/collision behavior is in scope.
- Changing `KEY_HISTORY_DISPLAY_SIZE` (number of keys shown) or which keys are
  recorded.
- Re-litigating the original Arcade-mode fix from #272 (already closed and
  verified working).
- Non-overlap bugs unrelated to the key-history popup (tracked separately if
  found).

## 2. User Stories

### US-001: Readable Target panel during multi-key input

AS A trainee typing a multi-keystroke command (regex prompt, macro, register
operation) in Training mode
I WANT the Target State panel to remain fully visible and undistorted
SO THAT I can keep comparing my current state against the goal while typing

**Acceptance criteria:**
```
GIVEN the Task screen is showing a scenario with the Current/Target panels visible
WHEN the player opens a multi-keystroke pending input (e.g. presses "s" to start a regex-selection prompt) and types additional characters
THEN the Target State panel's border and content remain fully intact and legible
  AND the key-history popup (if shown) renders only within its own bounded area, without overwriting the Target panel's border
```

### US-002: Readable SUCCESS screen after multi-key completion

AS A trainee who just completed a scenario using a multi-key command
I WANT the SUCCESS popup and surrounding panels to render without corruption
SO THAT I can read the result and my final state clearly

**Acceptance criteria:**
```
GIVEN a scenario was completed via a multi-keystroke command (macro replay, register paste, regex-selection)
WHEN the SUCCESS popup is displayed
THEN the Current/Target panels' borders remain intact and are not overwritten by the key-history popup
```

## 3. Functional Requirements

| ID | Requirement | Priority |
|----|------------|----------|
| FR-001 | WHEN `render_key_history_popup` computes its `Rect` THE SYSTEM SHALL clamp the popup's width and horizontal position so that it never extends into the column region occupied by the Task screen's Current/Target panels (or, in Arcade mode, the minigame's editor/HUD panels) | must |
| FR-002 | WHEN the key-history text is longer than the space available without overlapping another panel THE SYSTEM SHALL either truncate the displayed key history, shrink the glyph size, or skip rendering the popup entirely for that frame — but SHALL NOT allow it to paint over another panel's border or content | must |
| FR-003 | WHEN the key-history popup is rendered on the Task screen THE SYSTEM SHALL position and size it using knowledge of the actual panel layout `Rect`s (not just `frame.area()` and a scalar `reserved_bottom`), so width growth is bounded independent of terminal width | must |
| FR-004 | WHEN the key-history popup would not fit within its allotted non-overlapping region THE SYSTEM SHALL apply the same "skip rendering" guard that already exists for insufficient vertical room (see existing `area.height < reserved_bottom + popup_height` check) to the horizontal/panel-overlap case | must |
| FR-005 | WHEN this fix is applied to Training mode THE SYSTEM SHALL be verified not to regress the existing Arcade-mode non-overlap behavior fixed under #272 | must |

## 4. Non-Functional Requirements

| ID | Category | Requirement |
|----|----------|-------------|
| NFR-001 | Reliability | The fix must be covered by a `ratatui::backend::TestBackend`-based unit test (following the existing pattern in `src/ui/render/popups.rs` `#[cfg(test)] mod tests`) that renders a long key-history sequence at a realistic terminal size and asserts the Target panel's border cells are not overwritten |
| NFR-002 | Compatibility | No change to the public `render_key_history_popup` behavior when key-history text is short enough to already fit without overlap (i.e., short-history rendering stays visually identical) |
| NFR-003 | Performance | Fix must not introduce per-frame allocations or layout recomputation beyond what is already done for the Task screen's existing panel-chunking logic |

## 5. Data Model

No new data entities. This is a rendering-layer fix; existing types involved:

| Entity | Description | Key Attributes |
|--------|-------------|----------------|
| `TaskData.key_history` | Ring buffer of recently pressed keys, rendered as the popup source | last `KEY_HISTORY_DISPLAY_SIZE` (5) key labels |
| Task screen panel layout (`chunks` in `src/ui/render/task.rs`) | Existing `ratatui::layout::Rect`s for Current/Target panels and HUD bars | column/row bounds currently not passed into `render_key_history_popup` |

## 6. Edge Cases and Error Handling

| Scenario | Expected Behavior |
|----------|-------------------|
| Key history reaches max length (5 keys) with the widest possible glyph rendering (e.g. all displayed as full words like "Enter", "Escape") | Popup either truncates/shrinks or is skipped for that frame — Target panel border stays intact |
| Terminal is very narrow (e.g. `area.width` close to `KEY_HISTORY_MIN_WIDTH`) | Popup does not render, or renders without corrupting any visible panel, consistent with the existing vertical-room skip guard |
| Popup is visible at the exact moment the SUCCESS popup also renders | Neither popup corrupts the Current/Target panel borders; layering order does not cause partial overwrites |
| Arcade mode minigame HUD with a long key sequence | Continues to behave per the #272 fix — no regression |
| `show_key_history` toggled off mid-sequence | Popup disappears cleanly; no stale glyph remnants left on Current/Target panel borders |

## 7. Success Criteria

| ID | Metric | Target |
|----|--------|--------|
| SC-001 | Reproduction case 1 (regex-selection prompt `s`/`f`/`o`/`o`) re-tested via tmux capture | Target panel right border renders as a single clean box-drawing line, no doubled `└`/`┘` characters |
| SC-002 | Reproduction case 2 (macro record/replay `q`,`x`,`d`,`q`,`Q`) re-tested | SUCCESS screen renders with intact Current/Target panel borders |
| SC-003 | Reproduction case 3 (named-register `"ay` / `"aP`) re-tested | SUCCESS screen renders with intact Current/Target panel borders |
| SC-004 | New/updated unit test in `src/ui/render/popups.rs` | Passes under `cargo nextest run --workspace --all-features --lib --bins` |
| SC-005 | Arcade-mode regression check (existing tests + a manual tmux pass with a long key sequence) | No new overlap with Arcade HUD (Target/Timer/Score) |

## 8. Agent Boundaries

### Always (without asking)
- Run `cargo nextest run --workspace --all-features --lib --bins` and `cargo clippy --all-targets --all-features --workspace -- -D warnings` after changes
- Follow the existing `TestBackend`-based test pattern already present in `src/ui/render/popups.rs`
- Keep the fix scoped to `render_key_history_popup` and its call sites (`task.rs`, `minigame.rs`); do not touch unrelated rendering code

### Ask First
- Changing the public signature of `render_key_history_popup` in a way that affects both call sites (Training and Arcade) — acceptable, but confirm the chosen approach (pass panel `Rect`, add a max-width parameter, etc.) if multiple viable designs exist
- Reducing `KEY_HISTORY_DISPLAY_SIZE` or altering what keys are recorded as a workaround, instead of fixing the layout math

### Never
- Remove the key-history popup feature entirely as a workaround
- Suppress the popup unconditionally in Training mode instead of fixing the bounding logic
- Modify unrelated scenario or gamification code while addressing this rendering bug

## 9. Resolution (as shipped)

Merged in PR #372 (issue #364, commit `7f34268`); recorded in `CHANGELOG.md`.

- **Strategy**: the popup is bounded to the Target panel. `render_editor_pair` / `render_task_screen` / `render_playing` now return the Target panel's outer `Rect`, and `render_key_history_popup(frame, key_history, bounds)` takes it instead of the former `reserved_bottom: u16` row count (FR-001, FR-003).
- **Placement**: anchored to the bottom-right corner of `bounds`'s inner area; width clipped to that inner area, so the popup never draws over the panel border at any key-text length (FR-002).
- **Skip guard**: if the inner area is smaller than `KEY_HISTORY_MIN_WIDTH` wide or the popup height (`BIG_TEXT_HEIGHT_LINES / 2 + 2`), nothing is rendered for that frame (FR-004).
- **Layering**: Training mode's Task screen renders the key-history popup before the hint popup, so the (potentially wider) hint popup draws on top.
- **Arcade**: `render_playing` uses the same `bounds` contract, so Arcade keeps the #272 behavior (FR-005).
- **Tests** (NFR-001): `TestBackend` tests in `src/ui/render/popups.rs` (`test_render_key_history_popup_never_overwrites_panel_border`, `..._stays_inside_bounds_inner_area`, `..._skips_*`), plus `test_render_key_history_popup_does_not_corrupt_target_panel_border` in `src/ui/render/tests.rs` and an Arcade counterpart in `src/ui/render/minigame.rs`.

## 10. Resolved Questions

- **Remediation strategy?** Pass the Target panel `Rect` and clip to its inner area, with a skip guard for too-small areas.
- **Is Target-only protection sufficient?** Yes. The popup is anchored inside the Target panel, so it cannot reach the Current panel.

## 11. See Also

- [[constitution]] — project principles
- [[MOC-specs]] — all specifications
- `src/ui/render/popups.rs` — `render_key_history_popup`, existing `TestBackend` unit tests
- `src/ui/render/task.rs` — Training-mode Task screen call site (`render_task_screen`)
- `src/ui/render/minigame.rs` — Arcade-mode call site (`render_playing`) (already fixed under #272)
- `src/constants/ui.rs` — `KEY_HISTORY_DISPLAY_SIZE`, `BIG_TEXT_CHAR_WIDTH_CELLS`, `KEY_HISTORY_MIN_WIDTH`, `BIG_TEXT_HEIGHT_LINES`
- GitHub issue #272 (closed) — original Arcade-mode overlap defect
