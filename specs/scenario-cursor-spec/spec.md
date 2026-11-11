---
aliases:
  - Scenario Cursor Spec
  - Closed CursorSpec enum
tags:
  - sdd
  - spec
  - scenarios
  - config
  - type-safety
  - status/proposed
created: 2026-11-11
status: proposed
related:
  - "[[constitution]]"
  - "[[module-layering-and-typed-commands/spec]]"
  - "[[duplicate-scenario-ids/spec]]"
  - "[[MOC-specs]]"
---

# Feature: Scenario Cursor Spec Is a Closed Enum

> [!info] Metadata
> **Issue**: #553 (found by the architecture audit of CI cycle ci-012)
> **Status**: Proposed
> **Depth**: Lightweight spec. A type-safety refactor with a data migration; no gameplay change.

## 1. Overview

### Problem Statement

A scenario `setup` and `target` describe the cursor with four independent optional fields: `cursor_position`, `selection`, `cursors`, `selections`. The rule "at least one is set" is checked only after deserialization, contradictory combinations load without error, and three readers resolve them with three different precedence orders (the snapshot conversion used for gameplay and completion: `selections` > `cursors` > `selection` > `cursor_position` > `(0, 0)`; `effective_cursor_position`: `cursor_position` > `cursors` > `selections` > `(0, 0)`; `all_selections`: `selections` > `selection` > `cursors`). Positions are anonymous `(usize, usize)`, `[usize; 2]` and `[usize; 4]`, so row/column and anchor/head order exist only in comments, and the `(0, 0)` fallbacks encode a state the validator claims cannot exist. Six `CursorSpec` helpers have no production caller.

### Goal

The cursor of a scenario is ONE closed value that cannot be ambiguous: parse, do not validate. An ambiguous or empty cursor description is a load-time error with a clear message, and every consumer reads the same value through one exhaustive `match`.

### Out of Scope

- Changing what any bundled scenario demands from the player (every migrated entry keeps its exact setup and target snapshot)
- Changing the TOML key names or the value shapes of the four forms
- Validating the rest of `Scenario` (only the cursor description becomes unrepresentable-when-wrong)

## 2. Functional Requirements

| ID | Requirement | Priority |
|----|------------|----------|
| FR-001 | THE SYSTEM SHALL represent the setup and target cursor as one enum `CursorSpec { Point(Pos), Range(CursorRange), Points(Bounded<Pos, MAX_CURSORS_PER_SCENARIO>), Ranges(Bounded<CursorRange, MAX_SELECTIONS_PER_SCENARIO>) }`, deserialized through a raw intermediate type (`try_from`). `Pos` is `{ row, col }`, `CursorRange` is `{ anchor, head }`, both with private fields and getters, built by `TryFrom<[usize; 2]>` and `TryFrom<[usize; 4]>` with the existing coordinate limits; `Bounded<T, MAX>` is a non-empty vector of at most `MAX` items | must |
| FR-002 | THE TOML surface keeps the four keys: `cursor_position = [row, col]` (`Point`), `selection = [r0, c0, r1, c1]` (`Range`), `cursors = [[row, col], ...]` (`Points`), `selections = [[r0, c0, r1, c1], ...]` (`Ranges`). EXACTLY ONE of them SHALL be present; zero present keeps the existing "must specify" error, and two or more present is a load error `ScenarioValidationError::AmbiguousCursorSpec { fields }` naming the fields. There is no compatibility shim for the old "several fields" format | must |
| FR-003 | THE SYSTEM SHALL build the editor snapshot from a `CursorSpec` with ONE exhaustive `match` (in `game/scenario_state.rs`); `EditorSnapshot::from_scenario_config`, the cursor branch of `validate_setup_or_target_config`, and the helper methods with no production caller (`effective_cursor_position`, `all_cursors`, `all_selections`, `is_multi_cursor`, `cursors_ref`, `selections_ref`) SHALL be deleted, with no `(0, 0)` fallback | must |
| FR-004 | THE bundled scenarios that name two cursor fields SHALL be migrated: the 66 entries (64 `selection` and 2 `selections` entries that also carry a redundant `cursor_position`) drop the redundant `cursor_position`. A one-off check before the old reader is deleted SHALL confirm that every setup and target snapshot of every bundled scenario is identical before and after the migration; the migration script is re-runnable and is not committed. The affected files are `basic/replace`, `editing/advanced-editing`, `movement/{combined,find-till,paragraph,precision,word-basics,word-python,word}`, `registers/named-registers`, `search/basic-search`, `selection/{advanced-selection,line-selection,select-all-replace,text-objects}` and `writing/emphasis` | must |
| FR-005 | Constructors used by tests and benches (`ScenarioBuilder`, direct struct literals) SHALL take a `CursorSpec`; there is no way to build a scenario with an ambiguous cursor | must |

### 2.1 Acceptance Criteria

```
GIVEN a scenario TOML whose setup has both cursor_position and selection
WHEN  it is loaded
THEN  loading fails with AmbiguousCursorSpec naming both fields

GIVEN a scenario TOML with only selections = [[0,0,0,3],[1,0,1,3]]
WHEN  it is loaded
THEN  the setup is CursorSpec::Ranges with two ranges and the snapshot has two selections

GIVEN every bundled scenario
WHEN  the setup and target snapshots are built before and after the migration
THEN  they are identical (one-off check) and `cargo nextest run scenario` passes
```

## 3. Edge Cases

| Scenario | Expected Behavior |
|----------|-------------------|
| `cursors = []` or `selections = []` | Load error (`Bounded` is non-empty) |
| More than `MAX_CURSORS_PER_SCENARIO` cursors or `MAX_SELECTIONS_PER_SCENARIO` selections | Load error, same limits as before |
| A user scenario file in the old several-fields format | Load error naming the fields; the data-directory loader reports it as it reports any invalid scenario |
| `selection` where anchor equals head | Valid `Range` (a one-grapheme or point range as before) |

## 4. Success Criteria

| ID | Metric | Target |
|----|--------|--------|
| SC-001 | `TryFrom` table tests cover every limit and every ambiguity combination | Pass |
| SC-002 | Snapshot-equality check over the bundled scenarios before deleting the old reader | All equal |
| SC-003 | `cargo nextest run scenario` | Pass |

## 5. Agent Boundaries

### Always
- Reuse `limits` constants (now `limits`, see [[module-layering-and-typed-commands/spec]]) for the bounds
- Make illegal states unrepresentable; no stringly or untyped cursor data

### Ask First
- Changing a scenario's target or `optimal_count` while migrating

### Never
- Keep a fallback precedence order or a `(0, 0)` default for a missing cursor
- Edit specs by hand outside the `sdd` agent workflow

## 6. See Also

- [[constitution]] - type-safety principle
- [[duplicate-scenario-ids/spec]] - other scenario load-time guards
- [[MOC-specs]] - all specifications
