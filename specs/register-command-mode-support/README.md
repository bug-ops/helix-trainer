---
aliases:
  - Register and Command-Line Mode — Retroactive Record
  - Issue 282 Retroactive Record
tags:
  - sdd
  - decision-record
  - helix-simulator
  - input-system
  - status/implemented
created: 2026-08-09
status: implemented
related:
  - "[[BRD]]"
  - "[[SRS]]"
  - "[[NFR]]"
  - "[[spec]]"
  - "[[plan]]"
  - "[[tasks]]"
---

# Retroactive Record: Named Register and Command-Line Mode Support (GitHub Issue #282)

> [!important] Status: IMPLEMENTED (command-line mode narrower than drafted)
> This package documents work that already shipped in commit `1ba668d`
> before this SDD package was written. It is retroactive.

## What This Package Is

A full BRD → SRS → NFR → spec → plan → tasks pipeline, run against a
competitive-parity research finding (named registers and command-line mode
absent from the Helix simulator), documented after the fact against what
commit `1ba668d` actually implemented.

## Headline Finding

Named registers shipped essentially as originally drafted — `"<reg>`
selection scoped to `y`/`p`/`P`, plus `R` as an implementation extension —
with the default/unnamed register preserved unchanged. Command-line mode
shipped at a **deliberately narrower scope** than drafted: only `:goto N`
(alias `:g N`) line-number navigation exists. The original minimum bar —
`:s/pattern/replacement/` substitute — was evaluated and explicitly dropped
during implementation as "not verifiable against the snapshot-based
scenario completion model," a constraint the original research spec did not
anticipate. `docs/HELIX_KEYBINDINGS.md` documents this narrowing explicitly
to users.

A second architectural deviation: command-line state lives entirely in the
input-typestate layer (`CommandLinePending`); `EditorMode`
(`src/helix/simulator/mode.rs`) was not extended with a `CommandMode`
variant as the draft's FR-005/NFR-002 requested.

## Traceability

| Draft Requirement | SRS Verdict | NFR Status | Outcome |
|---|---|---|---|
| FR-001..004 — named registers | Met, FR-003 extended with `R` | NFR-001, NFR-004 met | `RegisterFile` replacing `clipboard`, full typestate compliance |
| FR-005 — command-line as `EditorMode` variant | Met behaviorally, architecturally deviant | NFR-002 not met | `CommandLinePending` in input layer only |
| FR-006 — `:s` substitute | **Not implemented** | N/A | `:goto`/`:g N` shipped instead |
| FR-007, FR-008 — cancel/error handling | Met | NFR-006 met | Safer-than-drafted malformed-input path (intercepted, not propagated) |
| FR-009, FR-010 — scenario validation, FSRS tracking | Met | NFR-005, NFR-007 met | `normalize_command_id`, 2 new scenarios (thinner than SC-003's ≥3/category target) |

## Package Contents

- [[BRD]] — business rationale, decision, and what actually shipped
- [[SRS]] — FR-001..010 marked with actual implementation verdicts
- [[NFR]] — NFR-001..007 verified against shipped code, plus an
  unanticipated input-robustness hardening finding
- [[spec]] — original research spec plus a retroactive resolution summary
- [[plan]] — as-built architecture, key design decisions, documented scope
  narrowing
- [[tasks]] — retroactive task breakdown (T001-T007 completed, T008/T009
  recommended but not scheduled)

## See Also

- `specs/MOC-specs.md` — specifications index
- [[constitution]] — project principles, Section I (Architecture)
- `docs/HELIX_KEYBINDINGS.md` — user-facing scope disclaimer
- Issue #104 — vim-style marks explicitly rejected (unaffected by this work)
- Issue #198 — separate, still-open content-coverage gap (macros, scroll, view mode, selection-regex)
