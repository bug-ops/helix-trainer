---
aliases:
  - Arcade Game Mode Variety BRD
  - Issue 264 BRD
tags:
  - brd
  - minigame
  - decision-record
  - status/rejected
created: 2026-08-09
project: "helix-trainer"
status: rejected
related:
  - "[[README]]"
  - "[[SRS]]"
---

# Arcade Game Mode Variety: Business Requirements Document

> [!abstract]
> Records the original business goal behind GitHub issue #264 and the
> investigation finding that undermines it. This is a rejected-feature
> record, not a build authorization — see [[README]] for the overall
> decision.

## Executive Summary

Issue #264 proposed closing a perceived competitive-parity gap against
[vim-be-good](https://github.com/ThePrimeagen/vim-be-good): helix-trainer's
three minigame modes (`Arcade`, `Survival`, `Challenge`) all share one
mechanic — complete a scenario under time/lives pressure — while vim-be-good
ships mechanically distinct games (`relative`, `ci{`, `whackamole`, `snake`).
The specific ask was a reflex/targeting-drill mechanic modeled on
`whackamole`. Investigation found that targeting parity is **already met** by
90 existing scenarios played through the current `Arcade` mode, and that a
genuinely distinct mode would require real subsystem work not justified at
current priority and evidence. The recommendation is NO-GO.

## Problem Statement

- **What problem existed (as originally framed)?** helix-trainer's arcade
  offering was assessed as "one mechanic reskinned across pacing variants and
  content categories" — no mechanic built around raw movement speed or
  reflexive targeting, unlike vim-be-good's `whackamole`/`snake`.
- **Who would experience this?** Learners who have exhausted the novelty of
  the existing `Arcade`/`Survival`/`Challenge` modes and want a different kind
  of engagement (per the original spec's US-001).
- **What is the impact of not solving it (as originally claimed)?** Reduced
  engagement variety; a competitive gap versus the one reference project
  named in this project's `continuous-improvement` rules for game-mode
  variety.

> [!warning] Finding That Undermines the Premise
> The premise "helix-trainer has no reflex/targeting mechanic" is **false**
> as stated. See Finding 1 below. The premise survives only in a narrower,
> continuity-focused form (Finding 2), which is not the form issue #264 was
> filed under.

## Finding 1 — Targeting Parity Already Shipped

`EditorSnapshot::matches` (`src/helix/simulator/snapshot.rs:159-170`)
short-circuits to content-equality + cursor-offset-equality whenever the
target has at most one selection and no real (non-point) selection. That
means: **any `Scenario` whose `target.file_content == setup.file_content`,
differing only in cursor position, is mechanically a whack-a-mole round** —
acquire a target on an unmodified buffer, nothing else.

Measured against the shipped scenario library (134 total scenarios):

| Category | Content-identical scenarios | Of which `optimal_count == 1` |
|---|---|---|
| `movement` | 61 | 42 |
| `selection` | 25 | — |
| `search` | 4 | — |
| **Total** | **90** | **51** |

Full `optimal_count` distribution among the 90: `{1: 51, 2: 17, 3: 20, 7: 2}`.
Within the 61 `movement` rounds specifically: `{1: 42, 2: 14, 3: 3, 7: 2}` —
19 of 61 need two or more commands.

Concrete example: `find_char_001` in `scenarios/en/movement/find-till.toml`
— cursor `[1,4]` → `[1,15]`, solution `["f="]`, `optimal_count = 1`. This is
a single-motion target-acquisition round, played today in `Arcade` under a
per-scenario timer, with lives and adaptive difficulty
(`tests/scenario_validation.rs` verifies all shipped scenarios complete via
their solution).

These 90 (and especially the 51 single-command ones) are already-playable
reflex/targeting rounds via the existing `Arcade` mode. Any new mode that
merely generates similar targets with a shorter timer is **pacing**, not a
new mechanic — and the original spec itself places pacing changes out of
scope (draft spec, "Out of Scope": *"Changes to the existing
`Arcade`/`Survival`/`Challenge` pacing variants... those remain as-is"*).

## Finding 2 — The Real Differentiator Is Continuity, Not Targeting

What actually makes `whackamole` (and `snake`) feel different from
scenario-completion is **continuity, not targeting**: one persistent buffer,
targets that respawn instantly, no per-round transition screen, and a single
session clock — flow, rather than a series of discrete puzzles. A blind
description test supports this: *"solve a series of short editing puzzles,
each with its own timer"* vs. *"one buffer, targets keep appearing, hit as
many as you can in 60 seconds without stopping"* — these are distinguishable.

Delivering continuity requires subsystem work that does not exist in the
codebase today (see [[plan#Cost Basis (Perishable)]] for the itemized,
corrected cost). This is buildable in principle, but its cost is not the
"~1 focused PR" that the first-pass evaluation assumed — see Finding 3.

## Finding 3 — Priority and Evidence Basis

Issue #264 was filed at **P3**, sourced from a single automated
competitive-parity scan against **one** reference project
(vim-be-good). There is no user-demand signal for this specific mechanic. By
contrast, issue #198 (Helix command content-coverage gap) has stronger,
independent evidence and remains unimplemented. This is the primary and
durable basis for NO-GO — independent of the cost estimate for the
continuous-play variant, which is a secondary and perishable data point (see
[[plan#Cost Basis (Perishable)]]).

## Target Users (As Originally Framed)

### Primary Users
Learners already using `Arcade`/`Survival`/`Challenge` who want a
mechanically distinct engagement loop, not merely a harder/faster version of
the same drill.

### Secondary Users
N/A — no admin or occasional-user distinction in this single-player TUI.

### Stakeholders
Project maintainers weighing engagement variety against implementation and
maintenance cost, and against the competing, better-evidenced backlog item
(#198).

## Functional Requirements (As Originally Proposed)

See [[SRS]] for the full FR-001..006 list with pass/fail status against the
findings above. Summary: FR-001 (distinct core loop) and its associated
success criterion SC-001 fail for the cheap version; FR-002/FR-003/FR-004
remain hypothetically satisfiable only if the continuous version is ever
built; FR-005/FR-006 are moot because nothing is being added.

## Non-Functional Requirements (As Originally Proposed)

See [[NFR]] for NFR-001..004 carried forward from the draft spec — all moot
under NO-GO, annotated with what would apply if revisited.

## Scope & Boundaries

### In Scope (of this decision record)
- Recording the business rationale for NO-GO
- Recording the falsifiable kill-criterion as a documented, non-committed
  option for a future revisit
- Recording two independent spin-off bugs found during the investigation

### Out of Scope

> [!danger] Explicit Exclusions
> - Building either the cheap (pacing) or continuous (subsystem) version of
>   the reflex-drill mechanic now
> - A `snake`-style movement game (excluded on principle in the original
>   spec's investigation — it has no document, no target snapshot, no
>   `Scenario`, and no `HelixSimulator` execution, so it conflicts with the
>   spec's own FR-002 as originally proposed, not merely on scheduling)
> - Filing GitHub issues for the spin-off bugs (recorded here for team-lead
>   to file verbatim; not filed by this record)
> - Content-coverage gaps tracked separately under issue #198

## Constraints & Assumptions

### Technical Constraints
Any future revisit must route input through `HelixSimulator`/
`AnyModeSimulator` (project-wide invariant; not a new constraint introduced
here) and must not introduce a real-time per-frame game loop, since none
exists today (`event_loop.rs` ticks are predicate-polling, not a session
hook).

### Business Constraints
P3 priority; no allocated timeline or budget; competes for attention with
issue #198, which has stronger evidence.

### Assumptions

> [!warning] Assumptions
> - If a real user-demand signal for this mechanic appears, this NO-GO should
>   be revisited (see [[plan#Revisit Triggers]])
> - If the session-clock bug ([[appendix-spinoffs#Spin-off A]]) is fixed
>   independently, the cost basis for the continuous version drops and should
>   be re-evaluated on that basis alone

## Success Criteria

Not applicable — no feature is being built. See [[plan]] for the criteria
that would apply to the kill-criterion experiment, if it is ever run.

## Open Questions

> [!question] Unresolved Items
> - [ ] Should issue #264 be closed outright, or kept open and re-labeled to
>       reflect this record (recommendation: close as researched-not-planned;
>       team-lead decision)
> - [ ] Should the two spin-off bugs be filed before or independently of
>       closing #264 (recommendation: independently — they stand on their own
>       merits per [[appendix-spinoffs]])

## Glossary

| Term | Definition |
|------|-----------|
| Content-identical scenario | A `Scenario` where `target.file_content == setup.file_content`; only cursor position differs |
| `optimal_count` | Minimum number of commands (`NonZeroUsize`) required to complete a scenario, feeds `efficiency` scoring |
| Cheap version | Reflex mode expressed as generated `Scenario` rounds reusing existing per-round machinery — pacing variant |
| Continuous version | Reflex mode with a persistent buffer, instant target respawn, session clock — genuinely distinct mechanic |
| Kill-criterion experiment | A throwaway, never-shipped local experiment designed to falsify (not validate) the "is this fun" hypothesis before any real build |

## See Also

- [[README]] — decision record index and traceability
- [[SRS]] — functional requirements, marked pass/fail
- [[NFR]] — non-functional requirements, moot under NO-GO
- [[spec]] — decision-record version of the original feature spec
- [[plan]] — kill-criterion experiment and revisit triggers
- [[appendix-spinoffs]] — independently actionable bugs found during this investigation
