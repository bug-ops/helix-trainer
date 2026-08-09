---
aliases:
  - FSRS Proptest Coverage — Retroactive Record
  - Issue 263 Retroactive Record
tags:
  - sdd
  - decision-record
  - testing
  - learning-scheduler
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

# Retroactive Record: FSRS Scheduler Property-Based Test Coverage Gap (GitHub Issue #263)

> [!important] Status: IMPLEMENTED (partial scope)
> This package documents work that already shipped in commit `33bdaa1`
> before this SDD package was written. It is retroactive: the pipeline was
> run backward from the merged code to reconstruct requirements,
> verification, and residual gaps — not forward from spec to implementation.

## What This Package Is

A full BRD → SRS → NFR → spec → plan → tasks pipeline, run against a
continuous-improvement finding (issue #263: `proptest` declared but unused
against FSRS scheduling logic), documented after the fact against what
commit `33bdaa1` actually implemented.

## Headline Finding

`.claude/rules/continuous-improvement.md` claimed proptest coverage for FSRS
scheduling. It had none. Commit `33bdaa1` added two property tests to
`src/learning/performance.rs` (state-transition determinism and bounds
invariants), which in the process **found and fixed a real bug**: a
hardcoded decay sign error that produced NaN retrievability under some
inputs, which serialized as JSON `null` and made the affected user's
`profile.json` permanently unloadable. A self-healing deserializer was added
alongside the fix.

**The resolution is partial.** `src/learning/scheduler.rs` — the other half
of the original finding's scope (`ReviewItem` ordering, due-review queue
selection) — received zero property-test coverage. See
[[BRD#Residual Gap]] and [[plan#Residual Work]].

## Traceability

| Finding | SRS Impact | NFR Status | Outcome |
|---|---|---|---|
| Proptest declared, unused, in `Cargo.toml` | FR-003 (determinism) fully met; FR-001 (bounds) met via a differently-scoped property | NFR-001 (case-count bounding) partially met — implicit default, not explicit; NFR-003 (determinism of the tests themselves) fully met | 2 property tests shipped in `performance.rs` |
| Real bug surfaced by the bounds property | N/A — not a drafted requirement | N/A | Sign-error fix + self-healing deserializer + regression test |
| `scheduler.rs` half of the original scope | FR-002, FR-004, FR-005 not implemented | N/A | Documented as residual work, not scheduled |

## Package Contents

- [[BRD]] — business rationale, decision (path (a): add coverage), and the
  residual gap
- [[SRS]] — FR-001..007 marked with actual implementation verdicts
- [[NFR]] — NFR-001..004 verified against the shipped tests
- [[spec]] — original problem statement plus a retroactive resolution
  summary
- [[plan]] — as-built architecture, bundled unrelated CI-hardening change,
  and recommended follow-up scope
- [[tasks]] — retroactive task breakdown (T001-T003 completed, T004
  recommended but not scheduled)

## See Also

- `specs/MOC-specs.md` — specifications index
- [[constitution]] — project principles, Section III (Testing)
- `src/learning/performance.rs` — covered
- `src/learning/scheduler.rs` — uncovered, see Residual Work
- `.claude/rules/continuous-improvement.md` — source of the original claim, still not fully accurate
