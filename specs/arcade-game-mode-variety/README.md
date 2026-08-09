---
aliases:
  - Arcade Game Mode Variety — Decision Record
  - Issue 264 Decision Record
tags:
  - sdd
  - decision-record
  - minigame
  - no-go
created: 2026-08-09
status: rejected
related:
  - "[[BRD]]"
  - "[[SRS]]"
  - "[[NFR]]"
  - "[[spec]]"
  - "[[plan]]"
  - "[[tasks]]"
  - "[[appendix-spinoffs]]"
---

# Decision Record: Arcade Game Mode Variety (GitHub Issue #264)

> [!danger] Status: REJECTED (NO-GO)
> This package does **not** describe an approved feature. It documents an
> evaluation that concluded neither variant of the proposed feature should be
> built now. Every section below is written in the past/evaluative tense —
> "what was proposed", "what was found", "why it was rejected" — not as an
> implementation plan.

## What This Package Is

A full BRD → SRS → NFR → spec → plan → tasks pipeline, run against issue #264
("helix-trainer needs a reflex/targeting-drill arcade mechanic distinct from
scenario-completion, inspired by vim-be-good's `whackamole`"), and adapted to
record why the pipeline terminated in NO-GO rather than an approved build.

## Headline Finding

The premise of issue #264 does not hold as scoped. **90 of 134 shipped
scenarios already have `target` content identical to `setup` content** (61 in
the `movement` category, 51 with `optimal_count == 1`), and
`tests/scenario_validation.rs` proves these complete via
`EditorSnapshot::matches` (`src/helix/simulator/snapshot.rs:159-170`) content
+ cursor-offset comparison. These are, mechanically, whack-a-mole rounds —
acquire a target on an unmodified buffer under time/lives pressure — and they
already ship today, playable through the existing `Arcade` mode.

A **cheap** version of the proposed feature (generated targets + a shorter
timer, reusing the existing `Scenario`/`ActiveMiniScenario` machinery)
therefore fails the spec's own distinctiveness bar (FR-001 / SC-001): its only
delta from what already ships is pacing, which the original spec explicitly
places out of scope.

A **genuinely distinct** version exists in principle — the real differentiator
is *continuity* (one persistent buffer, instant target respawn, no per-round
transition screen, a real session clock), not targeting. But continuity
requires real subsystem work that does not exist today: no session-level
clock anywhere in `MiniGameSession`, the `Transition { success }` step would
need removing for this mode, and a target generator/respawn path bypassing
per-round `Scenario` re-instantiation.

**Decision: NO-GO on building either version now.** The durable justification
is P3 priority + single-reference-project evidence (vim-be-good only) + no
user demand — see [[BRD#Problem Statement]] and [[plan#Decision Rationale]].
The cost estimate for the continuous version is a secondary, perishable data
point (see [[plan#Cost Basis (Perishable)]]) — it is explicitly not the
load-bearing argument, because it will change if the independent session-clock
bug ([[appendix-spinoffs#Spin-off A]]) is fixed on its own merits.

## Traceability

| BRD Finding | SRS Impact | NFR Status | Recommended Next Action |
|---|---|---|---|
| [[BRD#Finding 1 — Targeting Parity Already Shipped]]: 90/134 scenarios are already content-identical reflex rounds, proven by `tests/scenario_validation.rs` | [[SRS#FR-001]] and [[SRS#FR-002 (Success Criteria)|SC-001]] FAIL — distinctiveness bar not met by the cheap version | [[NFR#NFR-002 — Consistency]] moot (nothing ships) | None — do not build the cheap version. Close #264 as researched-not-planned. |
| [[BRD#Finding 2 — The Real Differentiator Is Continuity, Not Targeting]]: a genuinely distinct mode exists in principle but requires a session clock, a state-machine change, and a respawn path | [[SRS#FR-002]], [[SRS#FR-003]], [[SRS#FR-004]] remain hypothetically satisfiable; [[SRS#FR-005]], [[SRS#FR-006]] moot (no variant added) | [[NFR#NFR-001 — Performance]], [[NFR#NFR-003 — Testability]] would apply if revisited; [[NFR#NFR-004 — i18n]] currently unachievable in the existing pattern (M3) | [[plan#Recommended Next Action — Kill-Criterion Experiment]] — a throwaway, uncommitted, falsifier-only local experiment, only if someone revisits this later |
| [[BRD#Finding 3 — Priority and Evidence Basis]]: P3, single reference project, no user demand | N/A — this is the priority gate, independent of the above two | N/A | Do not promote #264 ahead of issue #198 (content coverage), which has stronger evidence |
| Two independent spin-off bugs surfaced during investigation | N/A — out of scope for #264's requirements | N/A | File separately; see [[appendix-spinoffs]]. Not part of this decision, actionable regardless of it. |

## Revisit Triggers

Documented in [[plan#Revisit Triggers]]. In short: the session-clock bug
([[appendix-spinoffs#Spin-off A]]) is fixed independently; a real user demand
signal appears; a second reference project ships an equivalent mechanic.

> [!info] First trigger fired 2026-08-09 — see [[plan#4. Revisit Triggers]]
> The session-clock bug was fixed as #327. This NO-GO decision still stands;
> the priority/evidence re-check that trigger calls for has not been done.

## Package Contents

- [[BRD]] — business goal (competitive parity vs. vim-be-good) and the finding that undermines it
- [[SRS]] — FR-001..006 as originally proposed, marked pass/fail against current-state evidence
- [[NFR]] — NFR-001..004 carried forward from the draft spec, marked moot under NO-GO
- [[spec]] — decision-record version of the original feature spec (what was evaluated, not what will be built)
- [[plan]] — the kill-criterion experiment as the only recommended next action, plus revisit triggers
- [[tasks]] — the experiment protocol broken into steps, explicitly not approved for execution
- [[appendix-spinoffs]] — two independently actionable bugs found during the investigation, for team-lead to file verbatim

## Investigation Chain

This record synthesizes three rounds of design/critique, all under
`.local/handoff/` in this worktree (gitignored, not part of this committed
package):

1. Architect's first plan (2026-08-09T16:44) — GO at P3, "no new subsystem" claim
2. Critic's first critique (2026-08-09T16:57) — verdict: significant; 5 gaps, headlined by the targeting-parity finding
3. Architect's revised plan (2026-08-09T17:00) — verdict flipped to NO-GO, all 5 gaps conceded
4. Critic's second critique (2026-08-09T17:03) — verdict: minor; NO-GO endorsed with 4 corrections to the reasoning (not the conclusion)

## See Also

- Original draft spec (pre-critique, uncommitted): `.local/specs/002-arcade-game-mode-variety/spec.md` in the main checkout — superseded by [[spec]] in this package
- GitHub issue #264
- GitHub issue #198 — Helix command content-coverage gap (higher-evidence, separate concern)
- [vim-be-good](https://github.com/ThePrimeagen/vim-be-good) — sole reference project cited for this gap
