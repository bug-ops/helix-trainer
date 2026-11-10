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
> **Issue**: #488
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

## 7. See Also

- [[constitution]] - project principles
- [[training-progress-indicator/spec]] - companion status-bar fix
- [[regex-selection-and-macro-commands/spec]] - counted-command string forms
- [[MOC-specs]] - all specifications
