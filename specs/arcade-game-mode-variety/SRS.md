---
aliases:
  - Arcade Game Mode Variety SRS
  - Issue 264 SRS
tags:
  - srs
  - requirements/functional
  - minigame
  - decision-record
  - status/rejected
created: 2026-08-09
project: "helix-trainer"
status: rejected
standard: "ISO/IEC/IEEE 29148:2018"
related:
  - "[[BRD]]"
  - "[[NFR]]"
---

# Arcade Game Mode Variety: Software Requirements Specification

> [!abstract]
> Functional requirements as originally proposed for issue #264, each marked
> against current-state evidence. Traceable to [[BRD]]. This document does
> not authorize implementation — see [[README]] for the NO-GO decision.

## 1. Introduction

### 1.1 Purpose

Records the functional requirements originally drafted for a reflex/targeting
minigame mode, and evaluates each against the codebase as it exists today, so
a future reader can see exactly which requirements failed, which were never
tested because the feature was rejected before build, and which remain
hypothetically valid if this is ever revisited.

### 1.2 Scope

Covers the reflex-drill mechanic proposed in issue #264 only. Does not cover
the existing `Arcade`/`Survival`/`Challenge` modes (unchanged) or content
coverage (issue #198).

### 1.3 Definitions, Acronyms, and Abbreviations

| Term | Definition |
|------|-----------|
| `MiniGameMode` | Enum of minigame modes (`Arcade`, `Survival`, `Challenge`), `src/minigame/modes.rs` |
| `MiniGameSession` | Typestate session state machine driving a minigame round, `src/minigame/session.rs` |
| `ActiveMiniScenario` | Per-round active scenario state within a session |
| `EditorSnapshot::matches` | Completion predicate comparing current editor state to a scenario's target, `src/helix/simulator/snapshot.rs:159` |
| `DifficultyController` | Adaptive difficulty selection component, `src/minigame/difficulty.rs` |

### 1.4 References

- [[BRD]] — Business Requirements Document
- [[NFR]] — Non-Functional Requirements Specification
- `.local/specs/002-arcade-game-mode-variety/spec.md` (main checkout) — original draft spec this record supersedes

### 1.5 Document Overview

Section 2 gives overall context (unchanged system). Section 3 lists FR-001
through FR-006 exactly as drafted, each with a verdict and evidence. Section
4 is the verification matrix, marked "not run" throughout since no build
occurred.

## 2. Overall Description

### 2.1 Product Perspective

No system-context change: this record evaluates a proposed addition to
`src/minigame/`, a subsystem of the single-binary helix-trainer TUI. No
addition was made.

### 2.2 Product Functions

Not applicable — no new functional area was implemented.

### 2.3 User Classes and Characteristics

Unchanged from [[BRD#Target Users (As Originally Framed)]].

### 2.4 Operating Environment

Unchanged — TUI via ratatui/crossterm, tokio async runtime, as documented in
the project's `CLAUDE.md`.

### 2.5 Design and Implementation Constraints

N/A — no implementation occurred.

### 2.6 Assumptions and Dependencies

See [[BRD#Assumptions]].

## 3. Specific Requirements

### 3.1 Functional Requirements — Original Text and Verdict

> [!info] Traceability
> Traces to the draft spec's Functional Requirements section (`.local/specs/002-arcade-game-mode-variety/spec.md:130-141`, main checkout).

**FR-001** — WHEN the player selects the new reflex-drill mode THE SYSTEM
SHALL present a core loop mechanically distinct from scenario-completion
(e.g., acquire-and-act-on-a-target rather than transform-a-buffer-into-a-
target-state).

- *Priority*: should
- *Verdict*: **FAILS** (for the cheap/generated-target variant)
- *Evidence*: 90 of 134 shipped scenarios already have
  `target.file_content == setup.file_content` (61 `movement`, 25 `selection`,
  4 `search`); `EditorSnapshot::matches` (`src/helix/simulator/snapshot.rs:
  159-170`) completes these on content + cursor-offset equality alone. A
  generated-target reflex mode built on the same `Scenario` machinery differs
  from these only by a shorter timer — pacing, which the draft spec's own
  Out-of-Scope section excludes from counting as a distinct mechanic. See
  [[BRD#Finding 1 — Targeting Parity Already Shipped]].
- *If revisited*: only the continuous-play variant (persistent buffer,
  instant respawn, session clock — [[BRD#Finding 2]]) could plausibly satisfy
  this requirement; the cheap variant cannot regardless of implementation
  quality.

**FR-002** — WHEN the player executes an input during the reflex-drill mode
THE SYSTEM SHALL route it through `HelixSimulator`/`AnyModeSimulator` command
execution, never a bespoke coordinate-comparison shortcut.

- *Priority*: must
- *Verdict*: **hypothetically satisfiable, satisfied by construction** —
  untested in production since nothing was built
- *Evidence*: `ActiveMiniScenario::execute_single` (`src/minigame/session.rs:
  220`) already routes every key through `AnyModeSimulator::execute_command`
  for every existing mode; `src/input/handlers.rs:533,541` already emits
  `Message::MiniGameCommand` for the arcade screen. Any mode expressed as a
  `Scenario` (cheap variant) inherits this automatically. The continuous
  variant would need to preserve this property explicitly when bypassing
  per-round `Scenario` re-instantiation (see [[plan#Cost Basis (Perishable)]]
  item 3/4).

**FR-003** — WHEN a reflex-drill session ends THE SYSTEM SHALL report results
through the existing scoring/gamification pipeline (XP, streaks,
`MiniGameStats`-equivalent) used by `Arcade`/`Survival`/`Challenge`.

- *Priority*: must
- *Verdict*: **hypothetically satisfiable, satisfied by construction** —
  untested in production
- *Evidence*: `handle_minigame_game_over` (`src/ui/state/handlers/minigame.rs:
  362-415`) is mode-agnostic; `ScoreCalculator::calculate`
  (`src/minigame/scoring.rs:147`) takes only scalars. Both variants would
  reuse this path without modification. Caution noted in
  [[appendix-spinoffs]]-adjacent findings: this same reuse is what makes an
  uncontrolled kill-criterion experiment dangerous to the real user profile
  (see [[plan#Experiment Isolation Requirements]]) — `save_immediate()` is
  called unconditionally on game over.

**FR-004** — WHEN difficulty scaling applies to the reflex-drill mode THE
SYSTEM SHALL reuse or extend `DifficultyController` rather than introducing
an unrelated adaptive-difficulty mechanism.

- *Priority*: should
- *Verdict*: **hypothetically valid, moot under NO-GO**
- *Evidence*: `DifficultyController::current_level()` /
  `update_after_scenario(PerformancePoint)` could map level → target
  distance, motion class, round time limit, the way
  `SurvivalConfig::time_limit_for_level` already does. Generation would not
  route through `next_scenario` (`src/minigame/difficulty.rs:402`), since
  that selects from an existing `&[Scenario]` pool and generated rounds are
  not in one. Only relevant if the continuous version is ever approved.

**FR-005** — WHERE the reflex-drill mode is added THE SYSTEM SHALL expose it
as a new `MiniGameMode` variant (or equivalent), consistent with how
`Arcade`/`Survival`/`Challenge` are already modeled in
`src/minigame/modes.rs`.

- *Priority*: should
- *Verdict*: **moot** — nothing is being added
- *Evidence*: N/A. Prior analysis (superseded) concluded a single new variant
  was preferable to a fourth top-level type, to avoid duplicating
  `ActiveMiniScenario`, timers/pause, `MiniGameStats`, the countdown state
  machine, render chrome, input routing, and the FSRS bridge. This reasoning
  would still hold if revisited, but is not being acted on now.

**FR-006** — WHEN the reflex-drill mode is unavailable or not yet built THE
SYSTEM SHALL NOT regress or remove any existing `Arcade`/`Survival`/
`Challenge` behavior.

- *Priority*: must
- *Verdict*: **trivially satisfied, moot** — no code changed, so no
  regression is possible
- *Evidence*: This decision record introduces zero source changes.

### 3.2 Success Criteria (from the draft spec)

| ID | Metric | Verdict |
|----|--------|---------|
| SC-001 | New mode is mechanically distinguishable from existing modes in a blind description test | **FAILS** for the cheap variant, same evidence as FR-001; unproven either way for the continuous variant since it was never built |
| SC-002 | All new logic covered by unit tests, no regression in `cargo nextest run --workspace --all-features --lib --bins` | **Not applicable** — no new logic exists |
| SC-003 | No increase in P0/P1 issues on existing modes after the change lands | **Not applicable** — nothing landed |

### 3.3 Logical Data Requirements

No new persistent entities were introduced. The draft spec's proposed data
model (`Reflex-drill session state`: target location, acquisition deadline,
hit/miss streak) was never implemented. Prior analysis noted, if ever
revisited: `Scenario` (`src/config/scenarios.rs:93-114`) has no `Default`
impl and requires `solution.commands` and `scoring.optimal_count:
NonZeroUsize` to be synthesized by any generator — not cosmetic fields, since
`optimal_count` feeds `efficiency` in both `ScoreCalculator::calculate` and
`PerformancePoint` → `DifficultyController` (`src/minigame/session.rs:
492-527`).

## 4. Verification and Validation

### 4.1 Verification Matrix

| Requirement | Method | Criteria | Status |
|------------|--------|----------|--------|
| FR-001 | Blind description test | Distinguishable core loop | **FAILED — evaluated pre-build against existing content, not run as a live test** |
| FR-002 | Code inspection | Routes through `AnyModeSimulator` | Not run — no build occurred |
| FR-003 | Code inspection | Reuses existing scoring/gamification path | Not run — no build occurred |
| FR-004 | Code inspection | Reuses `DifficultyController` | Not run — no build occurred |
| FR-005 | Code review | New `MiniGameMode` variant, exhaustive matches | Not run — no build occurred |
| FR-006 | Regression test suite | No P0/P1 regressions | Not applicable — no changes made |

### 4.2 Acceptance Test Outline

Not applicable. If the kill-criterion experiment in [[plan]] is ever run, its
own acceptance criteria (a self-play fun/no-fun signal) apply instead — see
[[plan#Kill-Criterion Experiment Protocol]].

## 5. Appendices

### 5.1 Traceability Matrix

| BRD Requirement | SRS Requirement(s) | NFR Requirement(s) |
|----------------|--------------------|--------------------|
| [[BRD#Finding 1 — Targeting Parity Already Shipped]] | FR-001 (fails), SC-001 (fails) | NFR-002 (moot) |
| [[BRD#Finding 2 — The Real Differentiator Is Continuity, Not Targeting]] | FR-002, FR-003, FR-004 (hypothetically valid) | NFR-001, NFR-003 (would apply if revisited) |
| [[BRD#Finding 3 — Priority and Evidence Basis]] | FR-005, FR-006 (moot) | NFR-004 (currently unachievable in existing pattern) |

## See Also

- [[BRD]] — business requirements (source)
- [[NFR]] — non-functional requirements
- [[README]] — decision record index
- [[plan]] — kill-criterion experiment and revisit triggers
