---
aliases:
  - Review Screen Margin
  - Review Header and Status Bar Alignment
tags:
  - sdd
  - spec
  - ui
  - bug-fix
  - status/proposed
created: 2026-11-11
status: proposed
related:
  - "[[constitution]]"
  - "[[fsrs-review-rating-and-session/spec]]"
  - "[[MOC-specs]]"
---

# Feature: The Review Screen Uses the Same Margin as Every Other Screen

> [!info] Metadata
> **Issue**: #586 (found by live testing at 130x45)
> **Status**: Proposed
> **Depth**: Lightweight bug-fix spec, per this project's SDD scaling guidance.

## 1. Overview

### Problem Statement

The Review screen draws its header (`Review Session - N/M`) and its bottom status bar edge to edge from column 0, while its body panels (`Command Info`, `Practice`, the controls bar) and every other screen keep a 2-column margin. The header, the body and the bottom bar therefore do not line up.

### Goal

Every box of the Review screen starts at the same column as the Profile, Statistics and Training screens.

### Out of Scope

- Any change to Review content, scoring or rating ([[fsrs-review-rating-and-session/spec]])
- A shared layout abstraction for all screens

## 2. Functional Requirements

| ID | Requirement | Priority |
|----|------------|----------|
| FR-001 | THE Review screen header and bottom status bar SHALL be drawn inside the same 2-column horizontal margin as the Review body panels and as the Profile, Statistics and Training screens, so the first non-blank column of every row is 2 at 130x45 | must |
| FR-002 | THE change SHALL be confined to `src/ui/render/review.rs` and its render tests (pure rendering, no state change, [[constitution]] Section I) | must |

## 3. Acceptance Criteria

```
GIVEN a profile with due review cards and a 130x45 terminal
WHEN  the Review screen is rendered
THEN  the header box, the body panels and the bottom bar all start at column 2
```

## 4. Edge Cases

| Scenario | Expected Behavior |
|----------|-------------------|
| A terminal narrower than the margin plus the minimum box width | Renders as the other screens do at that width; no panic |

## 5. Success Criteria

| ID | Metric | Target |
|----|--------|--------|
| SC-001 | A render test asserts the first non-blank column of the header, body and bottom bar is equal | Pass |

## 6. See Also

- [[fsrs-review-rating-and-session/spec]] - Review session ownership
- [[key-history-popup-training-overlap/spec]] - another screen-layout fix
- [[MOC-specs]] - all specifications
