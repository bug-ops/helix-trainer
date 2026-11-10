---
aliases:
  - Training Progress Indicator
  - Initial Progress 100 Percent
tags:
  - sdd
  - spec
  - ui
  - game
  - bug-fix
  - status/proposed
created: 2026-11-10
status: proposed
related:
  - "[[constitution]]"
  - "[[optimal-count-action-counter-unit/spec]]"
  - "[[MOC-specs]]"
---

# Feature: Training Progress Indicator Reflects Cursor and Selection

> [!info] Metadata
> **Issue**: #451
> **Status**: Proposed
> **Depth**: Lightweight bug-fix spec.

## 1. Overview

### Problem Statement

`GameSession::calculate_progress` compares only line text with the target and ignores cursor position and selection. For the 101 of 160 bundled scenarios whose target text equals the setup text (all movement and selection scenarios) the status bar shows `Progress: 100%` before any key is pressed, and a movement-only solution never changes it. Editing text then makes the number drop.

### Goal

Progress starts below 100% for any scenario that is not already complete, increases toward the target including cursor/selection distance, and reaches 100% only when the scenario is complete.

### Out of Scope

- Redesigning the status bar layout
- Changing the completion check itself

## 2. Functional Requirements

| ID | Requirement | Priority |
|----|------------|----------|
| FR-001 | THE SYSTEM SHALL report 100% progress if and only if the session state satisfies the scenario's completion check (text and, where the target defines it, cursor/selection) | must |
| FR-002 | WHEN the text equals the target text but the cursor/selection does not match the target THE SYSTEM SHALL report a value strictly below 100% derived from the distance between the current and target cursor/selection | must |
| FR-003 | WHEN the text differs from the target THE SYSTEM SHALL keep the existing text-based computation, combined so the result never exceeds the text-only value | must |
| FR-004 | WHILE the editor is in Insert mode THE SYSTEM SHALL keep the existing cap of 99% | must |
| FR-005 | THE COMPUTATION SHALL be deterministic, total (no panic for empty buffers, out-of-range positions or multi-range selections) and bounded to `[0, 100]` | must |
| FR-006 | THE VALIDATION TEST SUITE SHALL assert that no bundled scenario starts in a completed state (setup does not already satisfy the target) | must |

## 3. Acceptance Criteria

```
GIVEN scenario 14 "Navigate multiline code" with setup text equal to target text
WHEN  the session starts and no key was pressed
THEN  progress is below 100
```
```
GIVEN any bundled scenario
WHEN  its documented solution is executed to completion
THEN  progress is 100 at the end and below 100 before the final completing command
```

## 4. Edge Cases

| Scenario | Expected Behavior |
|----------|-------------------|
| Scenario target defines only a cursor (no selection) | Distance is cursor distance |
| Scenario target defines a selection | Distance accounts for both anchor and head |
| Scenario whose setup already satisfies its target | Rejected by FR-006 in validation (content bug, not a runtime case) |
| Text edited away from target in a movement-only scenario | Progress reflects the text deviation (lower), never above the text-only value |

## 5. Success Criteria

| ID | Metric | Target |
|----|--------|--------|
| SC-001 | Table-driven test over all bundled scenarios: initial progress < 100 and final progress == 100 | Pass |
| SC-002 | Unit tests for the edge-case rows | Pass |

## 6. Open Questions

> [!question]
> - [NEEDS CLARIFICATION: exact distance formula (e.g. normalized Manhattan distance in characters or lines) is left to the plan; it only has to satisfy FR-001..FR-005.]

## 7. See Also

- [[constitution]] - project principles
- [[optimal-count-action-counter-unit/spec]] - the other status-bar value (`Actions` vs `optimal`)
- [[MOC-specs]] - all specifications
