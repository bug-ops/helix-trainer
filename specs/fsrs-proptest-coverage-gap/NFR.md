---
aliases:
  - FSRS Proptest Coverage NFR
  - Issue 263 NFR
tags:
  - nfr
  - requirements/non-functional
  - testing
  - learning-scheduler
  - status/implemented
created: 2026-08-09
project: "helix-trainer"
status: implemented
standard: "ISO/IEC 25010:2011"
related:
  - "[[BRD]]"
  - "[[SRS]]"
---

# FSRS Scheduler Property-Based Test Coverage Gap: Non-Functional Requirements Specification

> [!abstract]
> Verifies NFR-001..004 from the draft spec against the property tests
> actually shipped in commit `33bdaa1`.

## 1. Introduction

### 1.1 Purpose

Records the quality-attribute requirements attached to the FSRS proptest
work and verifies each against the shipped implementation.

### 1.2 Scope

Four NFRs were drafted; this document does not add new ones. Sections of the
ISO 25010 model not raised by the draft (Functional Suitability beyond what
[[SRS]] covers, Compatibility, Portability) are omitted.

> [!note] Not Applicable
> Compatibility and Portability are omitted — this is test-only, in-process
> code with no cross-system or cross-platform surface beyond what the rest
> of the project already targets.

### 1.3 Definitions

| Term | Definition |
|------|-----------|
| `ProptestConfig` | proptest's per-test configuration struct, controlling case count, shrinking iterations, etc. |
| Shrinking | proptest's automatic minimization of a failing generated input to the smallest reproducing counterexample |

### 1.4 References

- [[BRD]] — Business Requirements Document
- [[SRS]] — Software Requirements Specification
- ISO/IEC 25010:2011

### 1.5 Priority and Trade-offs

Maintainability (test coverage confidence) was prioritized over
minimizing CI time; no `ProptestConfig` override was added, so both
property blocks run at proptest's default 256 cases each. This was an
implicit trade-off, not an explicitly documented one in the commit.

## 2. Maintainability

### 2.1 Testability

| ID | Requirement | Target | Status |
|----|------------|--------|--------|
| NFR-001 | New property tests must run within CI time budgets; bound `ProptestConfig` case counts rather than leaving them unbounded | No `cargo nextest` slowdown attributable to the new tests | **Partially met.** No explicit `ProptestConfig` override was added — both blocks use the implicit default (256 cases). This satisfies "runs within CI budgets" in practice (no evidence of slowdown found), but does not satisfy the letter of "bound... rather than leaving them unbounded" as an explicit, intentional choice. |
| NFR-002 | Property test invariants must be traceable back to the specific FSRS/scheduler behavior they verify, via doc comments referencing this spec | Each property documented with rationale | **Met for the two properties that exist** — `prop_fsrs_state_transition_is_deterministic` and `prop_fsrs_state_stays_in_bounds` carry doc comments explaining the invariant. **Not applicable** to FR-002/004/005, which were never implemented, so there is nothing to trace for those. |

## 3. Reliability

### 3.1 Determinism (originally scoped under Maintainability as NFR-003)

| ID | Requirement | Target | Status |
|----|------------|--------|--------|
| NFR-003 | Property tests must not read live wall-clock time; any time baseline must be injected/fixed so shrunk failures reproduce deterministically | No `Utc::now()` calls inside property test bodies | **Met.** Both property tests use `FakeClock::at("2030-01-01T00:00:00Z")` plus `clock.advance_days(elapsed_days)` rather than live time. |

## 4. Maintainability (continued)

### 4.1 Dependency Hygiene

| ID | Requirement | Target | Status |
|----|------------|--------|--------|
| NFR-004 | `cargo deny check` must pass afterward with no new advisories or license issues, regardless of resolution path chosen | Clean `cargo deny check` | **Met** — no new dependency was introduced; `proptest = "1.11"` was already declared and audited. |

## 5. Verification Matrix

| ID | Method | Environment | Status |
|----|--------|-------------|--------|
| NFR-001 | Manual review of `ProptestConfig` usage; CI timing observation | CI | Implicit default in use; no measured regression found, but not an explicit bound |
| NFR-002 | Doc-comment review on the two shipped properties | Local | Pass for the 2 that exist |
| NFR-003 | Code inspection for `Utc::now()`/live-clock reads inside `proptest!` bodies | Local | Pass — `FakeClock` used throughout |
| NFR-004 | `cargo deny check` | CI | Pass |

## 6. Open Questions

> [!question] Unresolved Quality Requirements
> - [ ] Should an explicit `ProptestConfig { cases: N, .. }` be added to both
>       property tests now, to convert NFR-001 from "implicitly fine" to
>       "explicitly bounded," per the letter of the original requirement?
> - [ ] If `scheduler.rs` coverage is added as a follow-up (see
>       [[BRD#Residual Gap]]), should NFR-001..004 be re-verified against
>       those new tests specifically, or are they assumed to inherit the
>       same conventions established here?

## See Also

- [[BRD]] — business rationale (source)
- [[SRS]] — functional requirements and verdicts
- [[spec]] — original problem statement
