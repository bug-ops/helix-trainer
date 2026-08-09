---
aliases:
  - Arcade Game Mode Variety NFR
  - Issue 264 NFR
tags:
  - nfr
  - requirements/non-functional
  - minigame
  - decision-record
  - status/rejected
created: 2026-08-09
project: "helix-trainer"
status: rejected
standard: "ISO/IEC 25010:2011"
related:
  - "[[BRD]]"
  - "[[SRS]]"
---

# Arcade Game Mode Variety: Non-Functional Requirements Specification

> [!abstract]
> Carries forward NFR-001..004 from the draft spec. All are moot under the
> NO-GO decision recorded in [[README]] — none were verified against a build,
> because no build occurred. Retained here so a future revisit does not have
> to re-derive them from scratch.

## 1. Introduction

### 1.1 Purpose

Records the quality-attribute requirements originally attached to the
proposed reflex-drill mode, annotated with what is known about their
achievability from static code inspection performed during the investigation
— not from a live implementation.

### 1.2 Scope

Four NFRs were drafted originally; this document does not add new ones, since
no feature exists to derive further quality requirements from. Sections of
the ISO 25010 model not covered by the original draft (Reliability,
Compatibility, Portability) are omitted as not applicable.

> [!note] Not Applicable
> Reliability, Compatibility, and Portability sections of the ISO 25010 model
> are omitted. The draft spec did not raise requirements in these categories,
> and no implementation exists to evaluate them against.

### 1.3 Definitions

| Term | Definition |
|------|-----------|
| Animation tick interval | The existing periodic tick used by `event_loop::run_async_event_loop` for UI animation and timeout polling |
| `t!()` | `rust-i18n` macro used project-wide for translated strings |

### 1.4 References

- [[BRD]] — Business Requirements Document
- [[SRS]] — Software Requirements Specification
- ISO/IEC 25010:2011

### 1.5 Priority and Trade-offs

Not applicable — no feature was built, so no quality-attribute trade-offs
were made in practice. The original draft did not specify a priority
ordering among its four NFRs.

## 2. Performance Efficiency

### 2.1 Time Behaviour

| ID | Requirement | Target | Status |
|----|------------|--------|--------|
| NFR-001 | A reflex-drill tick/render loop must not introduce perceptible input lag versus the existing animation tick interval | No perceptible lag vs. baseline | **Moot** — no tick/render loop was built. Would apply only to the continuous variant, which needs a genuine session-level tick path that does not exist today (see [[plan#Cost Basis (Perishable)]] item 1). |

## 3. Usability

### 3.1 Consistency (originally scoped under Usability)

| ID | Requirement | Target | Status |
|----|------------|--------|--------|
| NFR-002 | Terminology, scoring vocabulary, and UI chrome for the new mode should match existing minigame screens | No divergent visual language | **Moot** — no UI was built. Prior analysis found the target-marker rendering path would need real new work (not reuse) to be consistent: `preview_positions` in `render_line_with_multi_cursor` (`src/ui/render/editor.rs:112-137`) is only consulted inside the per-char loop and lacks the end-of-line fallback that cursors get, so a target at EOL or on a blank line would render nothing without new code; separately, `render_editor_with_diff` colors a line green when it equals the target line, which is permanently true when `target.content == setup.content`, degenerating the progress affordance. |

### 3.2 Internationalization

| ID | Requirement | Details | Status |
|----|------------|---------|--------|
| NFR-004 | Any new user-facing strings must go through `rust-i18n`/`locales/`, not be hardcoded | All new strings translated | **Moot, and currently unachievable in the existing pattern if it were built as originally scoped.** `MiniGameMode::{name, description}` return hardcoded `&'static str`, duplicated index-wise in `MiniGameModeSelection::{mode_name, mode_description}` (`src/ui/state/screen.rs:435-464`); no `t!()` call exists anywhere in `src/ui/render/mode_selection.rs` or `src/ui/render/minigame.rs` today. A future revisit must either amend this requirement to "consistent with existing (currently non-i18n) mode strings", or explicitly scope a `&'static str` → `String`/`t!()` refactor as its own prerequisite work — it is not free reuse. |

## 4. Maintainability

### 4.1 Testability

| ID | Requirement | Details | Status |
|----|------------|---------|--------|
| NFR-003 | Core targeting/timing logic for the new mode must be unit-testable without a real terminal, matching the existing `src/minigame/tests.rs` pattern | Deterministic, seed-reproducible tests | **Moot; the seam needed for this exists but is not wired end-to-end.** A generator taking `&mut R: RngExt` (the seam added in commit `8100ebc`) is necessary but not sufficient: `refill_queue` (`src/minigame/session.rs:940`) calls `rand::rng()` inline, so `MiniGameSession` would need a stored RNG or seed — a change to `with_mode`'s shape — before generated rounds could be made reproducible for tests. Only relevant if the continuous variant is ever built. |

## 5. Verification Matrix

| ID | Method | Environment | Status |
|----|--------|-------------|--------|
| NFR-001 | Manual perceived-lag check against baseline tick interval | Local | Not run — no build |
| NFR-002 | Visual review against existing minigame screens | Local | Not run — no build |
| NFR-003 | `cargo nextest run` with seeded RNG | CI | Not run — no build; seam exists but is not wired (see above) |
| NFR-004 | `rust-i18n` lint / manual review of `locales/` | Local | Not run — no build; current pattern would fail this check even for existing modes' hardcoded strings, independent of this feature |

## 6. Open Questions

> [!question] Unresolved Quality Requirements
> - [ ] If revisited, should NFR-004 be relaxed to match the existing
>       non-i18n pattern of `MiniGameMode::name`/`description`, or should the
>       i18n refactor of all three modes be taken as a prerequisite? This
>       decision record does not resolve it — flagged for whoever revisits.

## See Also

- [[BRD]] — business requirements (source)
- [[SRS]] — functional requirements
- [[README]] — decision record index
