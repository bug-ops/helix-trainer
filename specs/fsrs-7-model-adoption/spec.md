---
aliases:
  - FSRS-7 Model Adoption
  - FSRS-7 Go/No-Go Evaluation
tags:
  - sdd
  - spec
  - research
  - learning-scheduler
  - fsrs
  - status/proposed
created: 2026-11-10
status: proposed
related:
  - "[[constitution]]"
  - "[[fsrs-7-crate-migration/spec]]"
  - "[[fsrs-proptest-coverage-gap/README]]"
  - "[[MOC-specs]]"
---

# Feature: Evaluate Switching the Scheduler from the FSRS-6 Model to the FSRS-7 Model

> [!info] Metadata
> **Source**: research CI cycle finding (priority P4, research)
> **Status**: Proposed (not built)
> **Depth**: Lightweight spec, per this project's SDD scaling guidance. This is a go/no-go evaluation, not an implementation commitment: it specifies what must be measured and how the decision is made, not how the model is wired in.
> **Prerequisite**: [[fsrs-7-crate-migration/spec|FSRS-7 crate migration]] (mechanical `fsrs` 6.x to 7.0.0 bump)

> [!abstract]
> `fsrs` 7.0.0 (released 2026-11-10) ships the FSRS-7 model: 34 parameters, fractional (sub-day) intervals, and a second "fast stability" trace (`MemoryState.stability_fast`). The trainer is on the FSRS-6 model (`fsrs = "6.6"`, 21 parameters, via `src/learning/performance.rs`). The upstream benchmark claims materially better same-day-review accuracy (RMSE about 0.381 to about 0.326, per earlier research notes, not independently verified here). Because a trainer session drills many commands several times in one sitting, same-day behavior is its dominant regime. This spec defines an offline, deterministic evaluation that decides whether adopting the FSRS-7 model is worth its migration cost.

## 1. Overview

### Problem Statement

The scheduler (`src/learning/performance.rs`, `src/learning/scheduler.rs`) models each command's memory with FSRS-6 and works at day granularity: it passes `elapsed_days: u32` to `FSRS::next_states`, stores `scheduled_days: u32` and a `due: DateTime<Utc>` derived from it, and clamps intervals to at least one day. A learner who repeats a command several times within one session therefore gets coarse, day-quantized updates for exactly the short-term behavior the trainer exercises most.

FSRS-7 is designed for this regime, but adopting it is not a drop-in change:

- Sub-day elapsed time and intervals must be represented somewhere (7.0.0 exposes `next_states_with_elapsed_days` for fractional elapsed time).
- `MemoryState` gains `stability_fast`; the persisted `~/.config/helix-trainer/profile.json` stores only `stability` and `difficulty`.
- The current retrievability code reads the decay exponent from `FSRS_PARAMS[20]` (`performance.rs`, `update_fsrs_state`). FSRS-7 retrievability does not depend on w[20] and the parameter table has 34 entries, so this read is invalid for FSRS-7 and must be replaced by the model's own `FSRS::current_retrievability`.
- No automatic FSRS-6 to FSRS-7 state conversion exists in the crate (only `memory_state_from_sm2` and full-history replay), so existing review history needs an explicit migration decision.
- `FSRS::new(&[])` now defaults to FSRS-7, so the model choice becomes implicit unless pinned.

Whether the accuracy gain justifies this cost for a single-user, offline trainer is unknown. The decision needs evidence, not a changelog claim.

### Goal

A documented go/no-go decision, backed by a reproducible offline simulation, on whether the scheduler should adopt the FSRS-7 model, with explicit decision criteria fixed before the numbers are seen.

### Out of Scope

- The mechanical crate bump itself (owned by [[fsrs-7-crate-migration/spec|fsrs-7-crate-migration]]).
- Implementing the model switch, the state migration, or sub-day scheduling. Those become a separate spec if and only if the decision is GO.
- Training or optimizing FSRS parameters on user data (see NFR-002).
- Changing the learning UX, review screens, or gamification rules.
- The residual proptest defects #433 and #434 (see [[fsrs-proptest-coverage-gap/README|FSRS proptest coverage gap]]).

## 2. User Stories

### US-001: Accurate short-term scheduling

AS A learner drilling the same Helix command several times in one session
I WANT the scheduler to model my forgetting within the session, not only across days
SO THAT repeated drills are spaced and surfaced when I actually need them again.

**Acceptance criteria:**
```
GIVEN a recorded review sequence containing several reviews of one command on the same day
WHEN the FSRS-6 and FSRS-7 models are each replayed over it
THEN the evaluation reports the prediction error of each model on those same-day reviews
```

### US-002: No lost progress

AS A returning learner with existing review history
I WANT my progress to survive any scheduler change
SO THAT I do not have to relearn commands I already know.

**Acceptance criteria:**
```
GIVEN an existing profile.json written by the FSRS-6 scheduler
WHEN the evaluation assesses migration options
THEN it reports, per option, how many cards change due date or state and by how much
```

### US-003: Informed maintainer decision

AS A maintainer
I WANT a go/no-go recommendation with pre-declared criteria
SO THAT the model switch is adopted only if it measurably helps the trainer's regime.

**Acceptance criteria:**
```
GIVEN the evaluation results
WHEN the decision criteria in FR-006 are applied
THEN the outcome is exactly one of GO, NO-GO, or DEFER, with the numbers that justify it
```

## 3. Functional Requirements

| ID | Requirement | Priority |
|----|------------|----------|
| FR-001 | WHEN the evaluation runs THE SYSTEM SHALL replay recorded review sequences through both the FSRS-6 and FSRS-7 models using the crate's own simulation/evaluation facilities, entirely offline | must |
| FR-002 | THE evaluation SHALL define its input corpus: (a) anonymized or synthetic review sequences modeled on trainer sessions (multiple same-session repeats per command), and (b) the maintainer's own profile history if available. [NEEDS CLARIFICATION: is a real recorded corpus available, or must sequences be synthesized? The profile stores current state, not a per-review log.] | must |
| FR-003 | THE evaluation SHALL define a primary success metric and report it separately for same-day reviews (elapsed under 1 day) and multi-day reviews. Candidate: log-loss and RMSE (binned) of predicted retrievability against recalled/forgotten outcome. [NEEDS CLARIFICATION: how is a trainer attempt mapped to the 1-4 FSRS rating and to a binary recall outcome? The mapping lives in `PerformanceTracker` and must be taken from there, not invented.] | must |
| FR-004 | WHEN comparing models THE evaluation SHALL use identical inputs, ratings, and timestamps for both, with FSRS-7 run both with default parameters and, if applicable, with the elapsed time passed as fractional days | must |
| FR-005 | THE evaluation SHALL assess each candidate representation of sub-day elapsed time and interval (keep day-quantized, store fractional days, store seconds) for impact on `Scheduler` due-item selection and on daily quest generation and reset, which currently assume day granularity. [NEEDS CLARIFICATION: should "due" within a session be able to resurface a card minutes later, and does that conflict with the existing session/quest design?] | must |
| FR-006 | THE evaluation SHALL fix decision criteria before results are inspected. Proposed: GO if FSRS-7 improves same-day RMSE by at least a declared margin and does not degrade multi-day RMSE beyond a declared tolerance; NO-GO if the same-day gain is below the margin; DEFER if the corpus is too small to separate the models. [NEEDS CLARIFICATION: numeric margin and tolerance.] | must |
| FR-007 | THE evaluation SHALL enumerate state-migration options for existing profiles (reset, derive `stability_fast` via serde default, replay of reconstructable history, approximate mapping) with the user-visible consequences of each | must |
| FR-008 | THE evaluation SHALL confirm that retrievability for FSRS-7 is obtained through `FSRS::current_retrievability` rather than from `FSRS_PARAMS[20]` | must |
| FR-009 | THE evaluation SHALL record the model/parameter-set choice that `FSRS::new` would make implicitly, and require it be pinned explicitly in any GO follow-up | should |
| FR-010 | THE evaluation SHALL produce a written decision record in this package stating the outcome, evidence, and, for GO, the follow-up spec scope | must |

## 4. Non-Functional Requirements

| ID | Category | Requirement |
|----|----------|-------------|
| NFR-001 | Determinism | WHEN run twice on the same inputs and seed THE evaluation SHALL produce identical metrics; any randomness is seeded. |
| NFR-002 | Privacy / offline | The evaluation and any resulting scheduler SHALL run with no network access and SHALL NOT send review data off-device; no parameter optimization on user data may require a network. |
| NFR-003 | Startup cost | A GO outcome SHALL not measurably increase application startup time; construction of the model and loading of profile data stays within the existing startup path. [NEEDS CLARIFICATION: current baseline startup measurement.] |
| NFR-004 | Dependency hygiene | The evaluation SHALL NOT add dependencies to the shipped binary; any tooling is dev-only and passes `cargo deny check`. |
| NFR-005 | Reproducibility | The evaluation procedure and inputs SHALL be documented so a maintainer can rerun it against a future crate release. |

## 5. Data Model

| Entity | Description | Key Attributes |
|--------|-------------|----------------|
| `CommandPerformance` (existing) | Per-command scheduling record | `stability`, `difficulty`, `scheduled_days: u32`, `due`, `last_review` |
| `MemoryState` (crate, FSRS-7) | Model memory state | `stability`, `difficulty`, `stability_fast` |
| Review sequence (evaluation input) | Ordered timestamped ratings for one command | timestamps, rating 1-4, recall outcome |
| Decision record | Output of FR-010 | outcome, metrics, criteria, migration option |

## 6. Edge Cases and Error Handling

| Scenario | Expected Behavior |
|----------|-------------------|
| Existing profile has `stability`/`difficulty` but no `stability_fast` | Evaluated as a migration option (FR-007); must never panic on load. |
| Corpus has too few same-day reviews to separate the models | Outcome is DEFER, not GO. |
| Sub-day due times interact with daily quest reset at midnight | Assessed under FR-005; any GO follow-up must resolve it explicitly. |
| Parameter table length differs between models (21 vs 34) | Any code indexing a parameter by position is flagged (FR-008). |
| FSRS-7 better same-day but worse multi-day | Applied per FR-006 tolerance; reported, not averaged away. |
| Clock skew or review timestamps out of order in history | Excluded from the corpus and counted in the report. |

## 7. Success Criteria

| ID | Metric | Target |
|----|--------|--------|
| SC-001 | Decision criteria fixed and recorded before results are inspected | yes |
| SC-002 | Both models evaluated on identical inputs, same-day and multi-day reported separately | yes |
| SC-003 | Evaluation reproducible (identical metrics on rerun) | 100% |
| SC-004 | Migration options enumerated with per-option user impact | all four options in FR-007 |
| SC-005 | Decision record produced with outcome GO, NO-GO, or DEFER | yes |

## 8. Agent Boundaries

### Always (without asking)
- Keep the evaluation offline and deterministic.
- Read [[constitution]] and the sibling [[fsrs-7-crate-migration/spec|crate migration spec]] first.
- Report same-day and multi-day metrics separately.

### Ask First
- Adding any dependency, even dev-only.
- Changing the persisted `profile.json` schema (belongs to a GO follow-up, not this evaluation).
- Using real user profile data as a corpus.

### Never
- Modify scheduler source, manifests, or lockfiles as part of the evaluation.
- Send review data over the network or run parameter training that requires it.
- Declare GO without pre-declared criteria (FR-006).

## 9. Risks

| Risk | Impact | Probability | Mitigation |
|------|--------|-------------|------------|
| Existing review history cannot be converted faithfully to FSRS-7 state | high | high | Enumerate migration options (FR-007); prefer a reset-or-replay choice the learner can understand; document in CHANGELOG as a breaking change (pre-1.0, no back-compat required). |
| Sub-day intervals conflict with `Scheduler` due selection and daily quests | medium | medium | Assess under FR-005 before any GO. |
| No real per-review log exists, so the corpus is synthetic and may not reflect real learners | medium | high | Treat synthetic-only results as weaker evidence; prefer DEFER over GO. |
| Upstream benchmark gain does not transfer to the trainer's rating mapping | medium | medium | Measure on the trainer's own mapping (FR-003). |
| Silent default change in `FSRS::new(&[])` selects FSRS-7 unintentionally | medium | medium | Pin the model explicitly (FR-009), checked in the prerequisite migration. |
| Invalid `FSRS_PARAMS[20]` read yields wrong retrievability under FSRS-7 | high | certain if unaddressed | Use `FSRS::current_retrievability` (FR-008). |

## 10. Open Questions

> [!question] Unresolved
> - [NEEDS CLARIFICATION: Is a per-review log available or capturable for a real corpus, given `profile.json` stores only aggregate state?]
> - [NEEDS CLARIFICATION: Trainer attempt to FSRS rating and recall-outcome mapping used for the metric (FR-003).]
> - [NEEDS CLARIFICATION: Numeric GO margin and multi-day tolerance (FR-006).]
> - [NEEDS CLARIFICATION: Should within-session resurfacing of a card be a product goal, or is day-level due sufficient with FSRS-7 used only for better stability estimates?]
> - [NEEDS CLARIFICATION: Preferred migration for existing profiles (reset vs. replay vs. approximate).]
> - [NEEDS CLARIFICATION: Startup-time baseline for NFR-003.]

## 11. See Also

- [[constitution]] — project principles
- [[MOC-specs]] — all specifications
- [[fsrs-7-crate-migration/spec|FSRS-7 crate migration]] — prerequisite mechanical bump
- [[fsrs-proptest-coverage-gap/README|FSRS scheduler proptest coverage gap]] — related scheduler test coverage
- [fsrs-rs v7.0.0 release notes](`https://github.com/open-spaced-repetition/fsrs-rs/releases/tag/v7.0.0`)
