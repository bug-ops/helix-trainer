---
aliases:
  - Optimal Count vs Action Counter Unit
  - scoring.optimal_count unit
tags:
  - sdd
  - spec
  - scenarios
  - scoring
  - bug-fix
  - status/proposed
created: 2026-11-10
status: proposed
related:
  - "[[constitution]]"
  - "[[training-progress-indicator/spec]]"
  - "[[MOC-specs]]"
---

# Feature: `optimal_count` and the Action Counter Use the Same Unit

> [!info] Metadata
> **Issues**: #488; extended 2026-11-11 with #512 (section 7)
> **Status**: Proposed
> **Depth**: Lightweight bug-fix spec.

## 1. Overview

### Problem Statement

For 37 bundled scenarios `scoring.optimal_count` counts keystrokes while the in-game action counter counts commands. A multi-key command (`gg`, `miw`, `md(`, `mr([`) is one command but is counted as 2-4 in `optimal_count`. The status bar shows `Actions: 1 (optimal: 3)`, the stated optimum is unreachable, and the scorer's tolerance (`actual <= optimal + tolerance` is Perfect, `src/game/scorer.rs`) lets a player spend up to `optimal` commands for a Perfect rating.

### Decision

The unit is the COMMAND, as counted by the action counter (one executed command string equals one action; a counted or argument-carrying command such as `3fa` or `md(` is one action). The scenario data is corrected; the counter is not changed. The scorer and its tolerance are not changed.

### Out of Scope

- Changing scoring tolerances or rating thresholds
- Changing how the action counter counts commands
- Per-scenario optimality analysis beyond making `optimal_count` equal the length of the documented solution

## 2. Functional Requirements

| ID | Requirement | Priority |
|----|------------|----------|
| FR-001 | FOR every bundled scenario THE SYSTEM SHALL have `scoring.optimal_count == solution.commands.len()` where each entry of `solution.commands` is one command in the counter's unit | must |
| FR-002 | WHEN a scenario's documented solution is executed THE SYSTEM SHALL finish with `actions == optimal_count` | must |
| FR-003 | THE SCENARIO VALIDATION SUITE (`cargo nextest run scenario`) SHALL fail when FR-001 is violated for any scenario file, naming the scenario id and both values | must |
| FR-004 | WHERE a solution lists keys that form one command (`m`, `d`, `(`) THE SOLUTION FORMAT SHALL list the command as one entry in the form the input layer emits (`md(`), so FR-001 and FR-002 hold | must |
| FR-005 | THE CORRECTION SHALL NOT lower the number of `Perfect` ratings reachable: a player executing the documented solution is rated Perfect | must |

## 3. Acceptance Criteria

```
GIVEN scenario 112 "Delete surrounding parentheses" (md()
WHEN  the documented solution is executed
THEN  the status bar reads `Actions: 1 (optimal: 1)` and the rating is Perfect
```
```
GIVEN any bundled scenario TOML
WHEN  the validation test runs
THEN  optimal_count equals the number of solution commands
```

## 4. Edge Cases

| Scenario | Expected Behavior |
|----------|-------------------|
| Insert-mode typed text in a solution | Counted as the counter counts it today (per typed command string); FR-001 is checked against that same unit |
| Scenario whose `solution.commands` entries mix keystroke and command granularity | Normalized per FR-004 as part of the data fix |
| A count-prefixed command (`3fa`) | One entry, one action |

## 5. Success Criteria

| ID | Metric | Target |
|----|--------|--------|
| SC-001 | Script over `scenarios/en/**/*.toml` finds zero mismatches (was 37) | 0 |
| SC-002 | Live-style test solving all bundled scenarios with their solutions | `actions == optimal` for all, all Perfect |

## 6. Open Questions

> [!question]
> - [NEEDS CLARIFICATION: confirm the decision to fix scenario data rather than change the counter to count keystrokes; the counter is the established unit used by scoring and arcade modes, so data correction is the recommended default.]

## 7. Follow-Up: Opt-In History-Aware Completion (#512)

> [!info] Added 2026-11-11 (spec-first update for the P2-P3 bug batch)
> Completion is state-based: it is checked after every action by comparing the editor state with the target. Undo and redo revisit states already visited, so `undo_redo_001` (`x d u U`) completes at action 2 and violates FR-002; the scenario is allowlisted (`ACTION_COUNT_EXEMPT_SCENARIO_IDS`) as a known exception. FR numbers continue after FR-005.

### 7.1 Requirements

| ID | Requirement | Priority |
|----|------------|----------|
| FR-006 | A scenario MAY declare a completion rule, a closed enum `CompletionRule` with the variants `TargetState` (the default when the key is absent: today's behavior, unchanged for every existing scenario) and `AfterTaughtCommands`, set in the scenario TOML as `completion = "after_taught_commands"` under `[scenarios.metadata]`; an unknown value is a scenario load error | must |
| FR-007 | WHERE a scenario uses `AfterTaughtCommands` THE SYSTEM SHALL complete it only when the editor state equals the target AND the action that produced that state is itself one of the scenario's `commands_taught`, compared after normalization (a count prefix is stripped, so `2u` matches `u`; argument characters follow the command normalization of the action counter) | must |
| FR-008 | THE RULE SHALL be one predicate, taking the target comparison result, the rule and the just-executed command, used by BOTH training completion (`GameSession::finish_action`/`check_completion`) and arcade completion (`ActiveMiniScenario::is_completed`, which calls the shared predicate `Scenario::is_completed_by`; `ScenarioState` is unchanged); the two paths SHALL NOT diverge | must |
| FR-009 | `undo_redo_001` SHALL use `completion = "after_taught_commands"` so its documented solution finishes with `actions == optimal_count` (FR-002), and its entry in `ACTION_COUNT_EXEMPT_SCENARIO_IDS` SHALL be removed | must |
| FR-010 | THE SCENARIO VALIDATION SUITE SHALL fail for a scenario with `AfterTaughtCommands` and an empty `commands_taught`, or whose documented solution does not complete exactly at its last command under the rule | must |

### 7.2 Acceptance Criteria

```
GIVEN undo_redo_001 (completion = "after_taught_commands", commands_taught includes u and U)
WHEN  the solution x d u U is executed in Training mode and again in arcade mode
THEN  the scenario does not complete after x d, completes after U at action 4, actions == optimal_count, rating Perfect
```
```
GIVEN the same scenario
WHEN  the commands u U x d are executed
THEN  it does not complete: the final state is reached by d, which is not a taught command (u and U on a fresh history are no-ops and do not satisfy the rule)
```
```
GIVEN the same scenario
WHEN  the commands x d 2u U are executed
THEN  the count prefix is normalized and the rule is evaluated on u/U as for the uncounted forms
```

### 7.3 Edge Cases

| Scenario | Expected Behavior |
|----------|-------------------|
| Existing scenario without the key | `TargetState`; no behavior change |
| Target reached by a taught command inside a multi-key command string (`3fa`) | The normalized base command is compared against `commands_taught` |
| Rule set on a scenario whose solution ends with a non-taught command | Rejected by the FR-010 validation |

### 7.4 Success Criteria

| ID | Metric | Target |
|----|--------|--------|
| SC-003 | `undo_redo_001` test in both training and arcade paths, with the `u U x d` negative case and a counted-command case | Pass |
| SC-004 | `ACTION_COUNT_EXEMPT_SCENARIO_IDS` no longer contains `undo_redo_001` and the suite passes | Pass |

> [!note] Alternatives rejected
> Comparing the undo-tree state cannot separate `x d` from `x d u U` (same tree head content), and "every taught command executed" alone accepts `u U x d` at action 4, which equals `optimal_count`.

## 8. See Also

- [[constitution]] - project principles
- [[training-progress-indicator/spec]] - companion status-bar fix
- [[regex-selection-and-macro-commands/spec]] - counted-command string forms
- [[MOC-specs]] - all specifications
