---
aliases:
  - Arcade Game Mode Variety Plan
  - Issue 264 — Recommended Next Action
tags:
  - sdd
  - plan
  - minigame
  - decision-record
  - status/rejected
created: 2026-08-09
status: rejected
related:
  - "[[spec]]"
  - "[[BRD]]"
  - "[[README]]"
---

# Technical Plan: Arcade Game Mode Variety — No Build Plan (NO-GO)

> [!danger] References
> **Spec**: [[spec]] — records the NO-GO decision
> **This document does not describe an implementation plan.** Since the
> feature was rejected, there is no architecture, data model, or rollout to
> plan. This document instead records: (1) the corrected cost basis behind
> the decision, (2) the one falsifiable experiment worth keeping as a
> documented option, and (3) the conditions under which this decision should
> be revisited.

## 1. Decision Rationale

The NO-GO decision rests on two independent legs, in priority order:

1. **Primary, durable: P3 priority + single-reference-project evidence +
   no user demand.** This is sufficient on its own regardless of cost. See
   [[BRD#Finding 3 — Priority and Evidence Basis]].
2. **Secondary, perishable: the continuous variant's cost is real
   subsystem work, not the near-free "~1 focused PR" the first-pass
   evaluation assumed.** See Cost Basis below.

> [!warning] Do not treat the cost estimate as the reason
> Reason 2 will change if the session-clock bug ([[appendix-spinoffs#Spin-off
> A]]) is fixed independently — see Revisit Triggers below. If this record is
> read after that bug is fixed and someone concludes "the cost dropped, so
> build it," that is a misread: reason 1 alone was always sufficient, and
> still is, unless the evidence and priority basis also change.

## 2. Cost Basis (Perishable)

The continuous (genuinely distinct) variant requires four items, as first
identified:

1. A session-level clock — does not exist anywhere in `MiniGameSession`
   (verified: no field carries it; `started_at` at `session.rs:50` belongs to
   `ActiveMiniScenario`, `transition_started_at` at `:266` is transition-only)
2. Removal of the `Transition { success }` step from this mode's flow — a
   state-machine change
3. A target-respawn path inside a live buffer, bypassing per-round
   `Scenario` re-instantiation
4. Instant next-target generation

### Two Corrections to This Cost Basis (from the second critique round)

**Correction A — item 1 is double-counted against an independent bug.** The
session-level clock is simultaneously booked as a cost of this feature *and*
filed as an independent P2 bug ([[appendix-spinoffs#Spin-off A]]: the UI
already promises "60 seconds" for `Arcade` and does not deliver it). It
cannot be both. If that bug is fixed on its own merits — and it should be,
since it is a user-visible false promise independent of #264 — the marginal
cost of the continuous mode drops from four items to three, at zero
attributable cost to #264.

**Correction B — items 3 and 4 are one item, not two.** If item 3 (respawn
without re-instantiating `ActiveMiniScenario`) is taken, the mode bypasses
`Scenario` entirely for its live rounds. That means there is no
`scoring.optimal_count` to synthesize and no `solution` to construct — so
item 4 is not "build a `Scenario` generator" at all, it reduces to "pick a
random reachable character offset != the current cursor" over a fixed
buffer. FR-002 (route through the simulator) and FR-003 (reuse
`ScoreCalculator`/`MiniGameStats`) both still hold under this simplified
item.

**Net corrected cost: ~2.5 items, not 4.** This does not flip the
recommendation (see Section 1) — it corrects the record so a future reader
is not anchored to an inflated number.

## 3. Recommended Next Action — Kill-Criterion Experiment

The only recommended next action, and only if someone chooses to revisit this
decision, is a throwaway, falsifier-only local experiment — **not** a
committed build step, and **not** approved for execution by this record
alone (see [[tasks]] for why).

### Kill-Criterion Experiment Protocol

Run a `Reflex` *pacing* config (short timer, no new mechanic) over the
existing shipped scenario pool, restricted to `category == movement` AND
`optimal_count == 1` — **42 of the 61 `movement` scenarios**, not all 61.

> [!warning] Scope restriction is load-bearing
> A flat 2-3s limit across all 61 `movement` rounds would manufacture a false
> negative: 19 of the 61 need ≥2 commands (`{1: 42, 2: 14, 3: 3, 7: 2}`
> distribution), and the 2 rounds needing 7 commands are unwinnable in 2s
> regardless of player skill. That would measure frustration at an
> impossible timer, not fun-vs-not-fun for the true whack-a-mole analogue.
> Restricting to `optimal_count == 1` isolates the actual hypothesis.

Frame the result explicitly as a **falsifier, not a validator**: a negative
signal from single-subject self-play ("this isn't fun") is trustworthy and
sufficient to keep the NO-GO. A positive signal ("this is fun") is *not*
sufficient on its own to flip the decision — see Revisit Triggers, which
requires more than a fun self-play session.

### Experiment Isolation Requirements

`handle_minigame_game_over` (`src/ui/state/handlers/minigame.rs:362-415`)
is mode-agnostic and unconditionally writes to the real user profile:
`profile.add_xp(xp)`, `minigame_high_score`, `minigame_best_streak`,
`minigame_games_played`, then calls `save_immediate()` →
`~/.config/helix-trainer/profile.json`. **Disabling FSRS recording alone is
not sufficient isolation** — `record_to_fsrs` has two call sites
(`handlers/minigame.rs:157,370`), and suppressing both still leaves XP, high
score, streak, and game count corrupted in the real profile.

Required isolation, either of:
- Point the experiment at an isolated `~/.config/helix-trainer/`-equivalent
  config directory, or
- Copy `profile.json` aside before the experiment and restore it after

This is a hard requirement, not a suggestion — running the experiment
without it corrupts the operator's real XP, high score, streak, game count,
and FSRS review schedule.

### What This Experiment Must Never Become

- A shipped `MiniGameMode` variant, an enum change, a menu entry, or any
  change to `MODE_COUNT`
- A committed public API of any kind
- Run against the real `~/.config/helix-trainer/` directory

## 4. Revisit Triggers

This decision should be actively revisited — not automatically reversed —
under any of the following:

| Trigger | What Changes |
|---|---|
| The session-clock bug ([[appendix-spinoffs#Spin-off A]]) is fixed independently | Cost basis drops from ~2.5 to ~1.5 items; re-run the priority/evidence check (Section 1, reason 1) fresh — it may still be NO-GO on evidence grounds alone |
| A real user-demand signal appears (e.g., users request this specific mechanic, not just "more variety") | Reason 1 (no user demand) no longer holds; re-evaluate against the then-current cost basis |
| A second reference project ships an equivalent reflex/targeting mechanic | Strengthens the evidence basis beyond "single reference project"; re-evaluate |
| The kill-criterion experiment is run and yields a strong positive signal | Necessary but not sufficient alone — combine with at least one of the above before proposing a build |

## 5. Constitution Compliance

No project constitution exists yet for helix-trainer (`.local/specs/
constitution.md` was not found). Compliance section omitted as not
applicable.

## 6. Risks and Mitigations

| Risk | Impact | Probability | Mitigation |
|------|--------|-------------|------------|
| A future reader treats the corrected ~2.5-item cost as low enough to justify a build on cost alone | Feature gets built without addressing the real blocker (evidence/priority) | Medium | Section 1's explicit warning; Revisit Triggers require more than cost alone |
| The kill-criterion experiment is run without isolation and corrupts a real user's profile | Data loss: XP, streak, high score, FSRS review schedule | Low (protocol documented) | Section 3's isolation requirement is marked hard, not optional |
| Issue #264 is left open indefinitely without this record being linked | Future contributors re-investigate from scratch | Medium | [[BRD#Open Questions]] flags this for team-lead; recommend linking this record in the issue close/relabel |

## See Also

- [[spec]] — the rejected feature spec
- [[BRD]] — business rationale
- [[tasks]] — experiment protocol broken into steps (not approved for execution)
- [[appendix-spinoffs]] — the independent bug this cost basis depends on
- [[README]] — decision record index
