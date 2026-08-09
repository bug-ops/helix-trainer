---
aliases:
  - Arcade Game Mode Variety Spec
  - Reflex Drill Minigame — Decision Record
tags:
  - sdd
  - spec
  - minigame
  - decision-record
  - status/rejected
created: 2026-08-09
status: rejected
related:
  - "[[BRD]]"
  - "[[SRS]]"
  - "[[NFR]]"
  - "[[README]]"
---

# Feature: Arcade Game Mode Variety (Reflex/Speed-Drill Mechanic) — REJECTED

> [!danger] Metadata
> **Status**: rejected (NO-GO)
> **Original author**: research finding (competitive parity scan vs. vim-be-good)
> **Issue**: #264
> **Priority**: P3
> **Decision date**: 2026-08-09
> **This is not an implementation spec.** It records what was evaluated and
> why it was rejected. See [[README]] for the full decision record and
> [[plan]] for the only recommended next action.

## 1. What Was Proposed

### Original Problem Statement

`src/minigame/` offers three named modes — `Arcade`, `Survival`, `Challenge`
(`src/minigame/modes.rs`, `MiniGameMode` enum) — that share one mechanic:
complete a scenario from `scenarios/en/<category>/*.toml` under time/lives
pressure, scored by `ScenarioScorer`/`ScoreCalculator`. They differ only in
pacing (session timer, lives count, escalation curve), not in what the player
does moment to moment.

vim-be-good, the reference project named in this project's
`continuous-improvement` rules for game-mode variety, ships mechanically
distinct games: `relative` (relative-jump line deletion), `ci{` (text-object
editing), `whackamole` (navigate to a moving/highlighted target as fast as
possible — a pure reflex/speed drill, not scenario completion), and `snake`
(a full arcade game played with `hjkl`).

The original ask: decide whether helix-trainer should add at least one arcade
mechanic that is *not* scenario-completion-under-pressure, inspired by
`whackamole`.

### Original Goal

Decide whether to add a reflex/targeting drill mechanic and, if so, specify
it in enough detail to hand off to `/sdd plan`.

## 2. What Was Found

### The Cheap Version Is Not Distinct

`EditorSnapshot::matches` (`src/helix/simulator/snapshot.rs:159-170`)
completes any scenario whose target has ≤1 selection and no real (non-point)
selection via content equality + cursor-offset equality alone. A reflex-drill
round is therefore already expressible as an ordinary `Scenario` whose
`target.file_content == setup.file_content`. Measured against the shipped
library: **90 of 134 scenarios already satisfy this** (61 `movement`, 25
`selection`, 4 `search`; 51 with `optimal_count == 1`). These already run
today through `Arcade`, under a timer, with lives and adaptive difficulty.

A mode that generates similar targets procedurally and applies a shorter
timer would differ from what already ships only in pacing — which the
original spec's own Out-of-Scope section excludes as a distinct mechanic
(*"Changes to the existing `Arcade`/`Survival`/`Challenge` pacing variants...
those remain as-is"*). Full detail in [[BRD#Finding 1 — Targeting Parity
Already Shipped]].

### The Distinct Version Is Not Cheap

The genuine differentiator — what makes `whackamole` feel different — is
**continuity**: one persistent buffer, instant target respawn, no per-round
transition screen, a real session clock. That requires:

1. A session-level clock, which does not exist anywhere in `MiniGameSession`
   (`started_at` at `session.rs:50` belongs to `ActiveMiniScenario`;
   `transition_started_at` at `:266` is for transitions only)
2. Removing the `Transition { success }` step from the mode's flow — a
   state-machine change
3. A target-respawn path inside a live buffer that bypasses
   re-instantiating `ActiveMiniScenario` per round

See [[plan#Cost Basis (Perishable)]] for the corrected, itemized cost — the
first-pass evaluation initially assumed this was near-free and later
conceded it is not.

### Priority and Evidence

This was raised at P3 from a single automated parity scan against one
reference project, with no user-demand signal. Full detail in
[[BRD#Finding 3 — Priority and Evidence Basis]].

## 3. Decision

> [!danger] NO-GO
> Neither the cheap (pacing) nor the continuous (subsystem) version should be
> built now. The cheap version fails the feature's own distinctiveness bar.
> The continuous version is buildable but not justified at P3 with
> single-source evidence and no user demand — see [[BRD#Finding 3]], which is
> the load-bearing argument, not the cost estimate.

Concretely: issue #264 should be closed as researched-not-planned, recording
that the parity gap vs. `whackamole` is *continuity/flow*, not *targeting*,
and that targeting parity is already met by 90 shipped scenarios. It should
not be promoted ahead of issue #198 (content coverage), which has stronger
evidence.

## 4. User Stories (As Originally Proposed, Not Delivered)

### US-001: Reflex-drill variety for engagement
AS A learner who has been using the existing Arcade/Survival/Challenge modes
I WANT an arcade option that tests raw cursor-movement speed and target
acquisition rather than full scenario completion
SO THAT I get a distinct kind of engagement and muscle-memory practice, not
just a faster/harder version of the same drill

**Status**: not satisfied by the cheap version (fails distinctiveness); would
require the continuous version, which is NO-GO at current priority.

### US-002: Real Helix motions, not simulated shortcuts
AS A learner practicing under time pressure
I WANT the reflex drill to respond to actual Helix keybindings executed
through the real editor simulator
SO THAT muscle memory built in the minigame transfers directly to real Helix
usage

**Status**: moot — would have been satisfied by construction had either
variant been built (see [[SRS#FR-002]]), but nothing was built.

### US-003: Fits the existing session/scoring architecture
AS A maintainer
I WANT any new arcade mechanic to plug into the existing `GameSession`/
`MiniGameSession` typestate and gamification systems
SO THAT XP, streaks, achievements, and FSRS-adjacent tracking keep working
uniformly across all minigame modes

**Status**: moot — see [[SRS#FR-003]].

## 5. Functional Requirements

See [[SRS]] for the full FR-001..006 list with individual pass/fail/moot
verdicts and evidence.

## 6. Non-Functional Requirements

See [[NFR]] for NFR-001..004 carried forward from the draft spec, all moot
under NO-GO.

## 7. Data Model (Not Implemented)

No persistent entities were added. See [[SRS#3.3 Logical Data Requirements]]
for what a generator would have needed to synthesize, had this been built.

## 8. Edge Cases Identified During Investigation (Not Implemented)

These were identified as risks the design would have had to handle, had the
continuous version been approved. Recorded for a future revisit, not as
outstanding work:

| Scenario | Risk Identified |
|----------|-------------------|
| A buffer-modifying command during a reflex round | Since target content == setup content, any edit makes `matches` permanently false; the round would silently burn to timeout instead of failing fast |
| `profile.minigame_high_score` is a single scalar shared across modes | Reflex scores are on a different scale and would pollute it; a separate `reflex_high_score` field would be needed |
| Target placed at the player's current cursor | The round would already be complete, but completion is only checked after a command executes — the generator must reject distance == 0 |
| FSRS pollution | `record_to_fsrs` (`session.rs:825-856`) would write short-timer motion commands into `PerformanceTracker` at a different time scale than normal training, inflating stability and suppressing legitimate reviews — applies to both a real build and to the kill-criterion experiment in [[plan]] |

## 9. Success Criteria

See [[SRS#3.2 Success Criteria (from the draft spec)]] — SC-001 fails,
SC-002/SC-003 are not applicable since nothing was built.

## 10. Agent Boundaries (For Any Future Revisit)

### Always (without asking), if ever revisited
- Route all in-game input through `HelixSimulator`/`AnyModeSimulator`, never
  simulate keypress effects with ad-hoc string/position math
- Run the full check suite before any commit
- Re-derive the cost basis in [[plan#Cost Basis (Perishable)]] against the
  current state of the session-clock bug before committing to a scope

### Ask First, if ever revisited
- Adding any new dependency
- Introducing a new top-level `GameMode`-like enum instead of a
  `MiniGameMode` variant
- Changing the `MiniGameSession` state machine's public API

### Never
- Build the cheap (generated-target, pacing-only) version — it fails the
  feature's own distinctiveness bar regardless of implementation quality
- Run the kill-criterion experiment ([[plan]]) against the user's real
  config/profile directory
- Treat this record as permission to implement — a revisit requires a fresh
  decision, triggered per [[plan#Revisit Triggers]]

## 11. Open Questions

All five `[NEEDS CLARIFICATION]` items from the original draft spec were
resolved during this investigation:

- ~~Is a new reflex/targeting mechanic worth building, given P3 priority and
  single-reference-project evidence?~~ **Resolved: no, not now — see
  Decision above.**
- ~~Should this be one `MiniGameMode` variant or a fourth top-level type?~~
  **Resolved: one variant, if ever built — a fourth type would duplicate
  `ActiveMiniScenario`, timers, `MiniGameStats`, countdown state, render
  chrome, input routing, and the FSRS bridge for no benefit.**
- ~~What counts as "fail" — lives, timer, or endless chase?~~ **Resolved:
  per-round timeout costs a life, nothing else, unless a session clock is
  built (which it isn't).**
- ~~Does target highlighting need new rendering support?~~ **Resolved: yes,
  more than initially assumed — see [[NFR#3.1 Consistency]].**
- ~~Should a `snake`-style mode be a separate spec?~~ **Resolved: excluded on
  principle, not scheduling — it has no document, no target snapshot, no
  `Scenario`, and no `HelixSimulator` execution, conflicting with FR-002.**

Remaining open question, not resolved by this record:

- [NEEDS CLARIFICATION: should issue #264 be closed outright or re-labeled
  and left open pointing at this record? Team-lead decision.]

## 12. See Also

- [[README]] — decision record index and full traceability
- [[BRD]] — business rationale and findings
- [[SRS]] — functional requirements with pass/fail verdicts
- [[NFR]] — non-functional requirements, moot under NO-GO
- [[plan]] — kill-criterion experiment and revisit triggers
- [[tasks]] — experiment protocol steps (not approved for execution)
- [[appendix-spinoffs]] — independently actionable bugs found during this investigation
- Original draft spec (superseded by this record): `.local/specs/002-arcade-game-mode-variety/spec.md` in the main checkout
- [vim-be-good](https://github.com/ThePrimeagen/vim-be-good) — sole reference project
- GitHub issue #198 — Helix command content-coverage gap (higher-evidence, separate concern)
