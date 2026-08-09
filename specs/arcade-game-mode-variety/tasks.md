---
aliases:
  - Arcade Game Mode Variety Tasks
  - Issue 264 — Experiment Protocol
tags:
  - sdd
  - tasks
  - minigame
  - decision-record
  - status/rejected
created: 2026-08-09
status: rejected
related:
  - "[[spec]]"
  - "[[plan]]"
---

# Implementation Tasks: Arcade Game Mode Variety — None Approved

> [!danger] References
> **Spec**: [[spec]] — NO-GO
> **Plan**: [[plan]]
> **Total tasks**: 0 approved for execution; 1 optional experiment protocol
> recorded below for whoever revisits this decision

## Status

No implementation tasks exist for this feature, because it was rejected
before reaching a build phase — see [[spec#Decision]]. The only work item
this record describes is the kill-criterion experiment from
[[plan#Recommended Next Action — Kill-Criterion Experiment]], and it is
**not approved for execution by this document**. It requires a separate,
explicit go-ahead from whoever is revisiting the decision, because it writes
to a local environment and must not touch the real user profile.

> [!warning] This is a protocol, not a work order
> The steps below describe *how* to run the experiment safely, if and when
> someone decides to run it. Writing this protocol down is not the same as
> authorizing the experiment. See [[plan#Revisit Triggers]] for when running
> it would actually be warranted.

---

## Optional: T-EXP-001 — Kill-Criterion Self-Play Experiment

**Context**: The only way to falsify (or fail to falsify) the "is fast
cursor-acquisition fun" hypothesis underlying even the expensive continuous
variant, without committing any code. See [[plan#Kill-Criterion Experiment
Protocol]] for the full rationale.

**Spec reference**: [[plan#Recommended Next Action — Kill-Criterion
Experiment]], [[BRD#Finding 2 — The Real Differentiator Is Continuity, Not
Targeting]]

**Preconditions** (must all be true before starting):
- [ ] An explicit decision to revisit #264 has been made by whoever owns that
      call — this task is not self-authorizing
- [ ] At least one [[plan#Revisit Triggers|revisit trigger]] has actually
      fired — do not run this purely out of curiosity, since it still costs
      setup time and carries the isolation risk below

**Acceptance criteria**:
- [ ] Runs as a local, uncommitted patch only — no new `MiniGameMode`
      variant, no `MODE_COUNT` change, no menu entry, nothing staged for
      commit
- [ ] Scenario pool restricted to `category == movement` AND
      `optimal_count == 1` (42 scenarios) — not the full 61 `movement`
      scenarios (see [[plan#Kill-Criterion Experiment Protocol]] for why the
      broader pool manufactures a false negative)
- [ ] Timer set per the `optimal_count == 1` pool's actual solve time, not an
      arbitrary flat value copied from a different pool
- [ ] Config/profile isolation verified **before** the first session: either
      a separate config directory is in use, or `profile.json` has been
      copied aside — confirm the file the session will write to is not
      `~/.config/helix-trainer/profile.json` unless it was explicitly backed
      up first
- [ ] Session results are read as a falsifier only: a "not fun" result is
      recorded as sufficient to keep NO-GO; a "fun" result is recorded as
      necessary-but-not-sufficient and must be paired with a fired revisit
      trigger before being used to argue for a build
- [ ] Real profile (`profile.json`, FSRS state, XP, streak, high score, game
      count) verified unchanged after the experiment, or restored from
      backup
- [ ] Patch discarded (not committed, not left applied) after the session,
      regardless of outcome

**Dependencies**: none (this is the first and only task)

**Files**: none committed — any local patch touches
`src/minigame/modes.rs`/`session.rs` transiently only, per
[[plan#What This Experiment Must Never Become]]

**Complexity**: low (protocol), but isolation failure has real data-loss
consequences — see [[plan#Risks and Mitigations]]

---

## Implementation Notes

### Order of execution
N/A — single optional task, gated on preconditions above.

### Common patterns
N/A — no code pattern is being established, since nothing ships.

### Gotchas
- Do not reuse the full 61-scenario `movement` pool for the timer test — see
  the acceptance criteria above; this is the single most likely mistake if
  this protocol is followed loosely.
- Do not assume disabling FSRS recording alone is sufficient isolation — it
  is not; see [[plan#Experiment Isolation Requirements]].

## See Also

- [[spec]] — the rejected feature spec
- [[plan]] — cost basis, decision rationale, and revisit triggers
- [[README]] — decision record index
