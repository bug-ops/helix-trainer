---
aliases:
  - Profile XP Progress Label
  - Level XP progress definition
tags:
  - sdd
  - spec
  - ui
  - gamification
  - bug-fix
  - status/proposed
created: 2026-11-11
status: proposed
related:
  - "[[constitution]]"
  - "[[end-game-summary-screen/spec]]"
  - "[[MOC-specs]]"
---

# Feature: One Definition of Level XP Progress on Every Screen

> [!info] Metadata
> **Issues**: #572 (P2; labels `bug`, `ui`, `gamification`)
> **Status**: Proposed
> **Depth**: Lightweight bug-fix spec.

## 1. Overview

### Problem Statement

The Profile screen label reads `{xp_in_level}/{xp_for_next} XP` (`src/ui/render/profile.rs`), where `xp_for_next` is `UserProfile::xp_for_next_level()` (`src/gamification/profile.rs`). That method returns the XP still REMAINING to the next level (`next_level_total - total_xp`), not the size of the current level. The menu header (`src/ui/render/menu.rs`) and the results screen (`src/ui/render/results.rs`) both call `helpers::level_xp_progress` (`src/ui/render/helpers.rs`), which returns `(total_xp - xp_for_level(level), xp_for_level(level+1) - xp_for_level(level))`, the level span. The Profile gauge uses `UserProfile::xp_progress()` (a span-based ratio), so the Profile label contradicts its own bar and the other screens.

Verified drift points in the code:

| Site | Numerator | Denominator | Notes |
|------|-----------|-------------|-------|
| Profile label (`profile.rs`) | `UserProfile::xp_in_level()` (clamped to the span) | `xp_for_next_level()` = XP remaining | wrong denominator |
| Menu header (`menu.rs`) | `helpers::level_xp_progress` (unclamped) | level span | percentage comes from `xp_progress()` (clamped) |
| Results screen (`results.rs`) | `helpers::level_xp_progress` (unclamped) | level span | |
| End-game summary gauge (`end_game.rs`) | private re-computation of the same span math | level span | label shows level and total XP only; ratio clamped |

Evidence (live TUI, isolated profile): level 1 with 50 XP shows `XP: 50/100 (50%)` in the menu header and `50/50 XP` on the Profile screen; 9822 XP (level 22) shows `199/696` vs `199/497`. For a level-100 profile with 1,000,000 XP the menu header shows `901496/1496 (100%)` (unclamped numerator, clamped percentage) while the Profile screen shows `1496/0 XP`.

The numerator also differs: `xp_in_level()` clamps to the span, `level_xp_progress` does not, so a profile whose `total_xp` exceeds the stored level's range renders differently per screen. There are two parallel implementations of the same math (`UserProfile` methods and `helpers::level_xp_progress`) plus a third private copy in `end_game.rs`.

### Goal

Every screen that shows level progress derives numerator, denominator, percentage and bar ratio from one shared definition, so they can never disagree.

### Out of Scope

- The XP curve, `XPCalculator::xp_for_level`, level-up rules, or XP awards
- Redesigning the layout of the Profile, menu, results or end-game screens
- Introducing a maximum player level (no cap constant exists today; see section 6)

## 2. User Stories

### US-001: Consistent level progress

AS A player
I WANT the same `earned/needed` XP figure and percentage on the menu, Profile, results and end-game screens
SO THAT I can trust the number and know how far I am from the next level

**Acceptance criteria:**
```
GIVEN a profile at level 22 with total_xp 9822
WHEN  the menu header, Profile screen and results screen are rendered
THEN  all three show 199/696 and the Profile bar is filled to 28%
```
```
GIVEN a profile at level 1 with total_xp 50
WHEN  the Profile screen is rendered
THEN  the label reads 50/100 XP and the bar is half full
```

## 3. Functional Requirements

| ID | Requirement | Priority |
|----|------------|----------|
| FR-001 | THE SYSTEM SHALL define level progress once, as a single shared helper that returns XP earned inside the current level and the XP span of the current level (`xp_for_level(level+1) - xp_for_level(level)`, saturating), with the numerator clamped to `[0, span]` | must |
| FR-002 | WHEN the Profile screen renders its XP label THE SYSTEM SHALL format it as `{earned}/{span} XP` from the shared helper | must |
| FR-003 | THE menu header, results screen, Profile screen and end-game summary gauge SHALL obtain numerator, denominator, percentage and gauge ratio exclusively from the shared helper (no per-screen recomputation) | must |
| FR-004 | THE percentage and the gauge ratio on every screen SHALL equal `earned / span` of the same helper result | must |
| FR-005 | WHEN `span` is 0 or `total_xp` is outside the stored level's range THE SYSTEM SHALL render the same clamped, non-panicking values on every screen (numerator never exceeds the denominator; percentage never exceeds 100) | must |
| FR-006 | THE `UserProfile::xp_for_next_level()` method SHALL NOT be used as a progress denominator; it MAY remain for "XP remaining" wording, or be removed if unused (pre-1.0, no deprecation shim) | should |
| FR-007 | THE end-game summary label SHALL keep its current content (level and total XP) unless the spec for that screen decides otherwise; only its gauge ratio is bound by FR-003/FR-004 | should |

## 4. Non-Functional Requirements

| ID | Category | Requirement |
|----|----------|-------------|
| NFR-001 | Architecture | Rendering stays pure (constitution section I): the helper takes the profile (or level and total XP) and returns values; no state mutation |
| NFR-002 | Type safety | The helper returns a named struct (earned, span) rather than a bare `(u64, u64)` tuple, so the two values cannot be swapped |
| NFR-003 | Documentation | A public helper carries a `///` doc comment with a doc-test (project documentation rules) |

## 5. Data Model

| Entity | Description | Key Attributes |
|--------|-------------|----------------|
| Level progress | Derived view of a profile's position inside its level | earned XP, level span, ratio, whole percent |

## 6. Edge Cases and Error Handling

| Scenario | Expected Behavior |
|----------|-------------------|
| `total_xp` below current level's start (inconsistent profile) | earned = 0 |
| `total_xp` beyond the level's end (stale level, or very high level) | earned = span, 100%, identical on every screen (today: menu shows `901496/1496`, Profile shows `1496/0`) |
| Level `u32::MAX` / next level saturates | span computed with saturating arithmetic, no panic or overflow (#523 behavior preserved) |
| span = 0 | ratio 1.0, label `earned/0` avoided: render `0/0 XP` or equivalent consistently [NEEDS CLARIFICATION: exact label for span 0] |
| Maximum level | No cap exists in the code today; behavior follows the over-range rows above [NEEDS CLARIFICATION: should a MAX marker be introduced if a level cap is ever defined?] |

## 7. Success Criteria

| ID | Metric | Target |
|----|--------|--------|
| SC-001 | Test over several `(level, total_xp)` pairs (level 1/50, level 22/9822, boundary exactly at a level start, over-range) asserts menu, Profile, results text and Profile gauge ratio all derive from the helper and agree | Pass |
| SC-002 | Property test: for any `(level, total_xp)`, `earned <= span` and percent `<= 100`; no panic | Pass |
| SC-003 | `rg "xp_for_next_level"` in `src/ui/` | 0 matches |
| SC-004 | Private span math in `end_game.rs` removed | 0 duplicate computations |

## 8. Agent Boundaries

### Always (without asking)
- Run `cargo nextest run --workspace --all-features --lib --bins`, clippy and fmt before finishing
- Reuse the existing `level_xp_progress` logic as the starting point for the shared helper (DRY)

### Ask First
- Changing the end-game label wording or adding a MAX marker
- Removing `UserProfile::xp_for_next_level()` if other consumers appear

### Never
- Change the XP curve, level thresholds, or persisted profile format
- Add a level cap as part of this fix

## 9. Open Questions

> [!question]
> - [NEEDS CLARIFICATION: exact label when span is 0 (only reachable with a corrupt level)]
> - [NEEDS CLARIFICATION: if a level cap is introduced later, show a MAX marker or `span/span`? The issue proposes either; neither is built today]
> - [NEEDS CLARIFICATION: should the end-game gauge label also show `earned/span`? Current label shows level and total XP]

## 10. See Also

- [[constitution]] - project principles
- [[end-game-summary-screen/spec]] - end-game gauge
- [[MOC-specs]] - all specifications
