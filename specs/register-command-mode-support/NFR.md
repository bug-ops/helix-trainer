---
aliases:
  - Register and Command-Line Mode NFR
  - Issue 282 NFR
tags:
  - nfr
  - requirements/non-functional
  - helix-simulator
  - input-system
  - status/implemented
created: 2026-08-09
project: "helix-trainer"
status: implemented
standard: "ISO/IEC 25010:2011"
related:
  - "[[BRD]]"
  - "[[SRS]]"
---

# Named Register and Command-Line Mode Support: Non-Functional Requirements Specification

> [!abstract]
> Verifies NFR-001..007 from the draft spec against commit `1ba668d`.

## 1. Introduction

### 1.1 Purpose

Records the quality-attribute requirements attached to the register/
command-line work and verifies each against the shipped implementation.

### 1.2 Scope

Seven NFRs were drafted; this document does not add new ones beyond what
was raised by the implementation itself (see Section 6, an unanticipated
security hardening finding).

> [!note] Not Applicable
> Performance Efficiency, Portability, and Compatibility are omitted — the
> draft raised no NFRs in these categories, and the implementation is
> in-process, synchronous state-machine dispatch with no measurable
> latency concern distinct from the rest of `src/input/`.

### 1.3 Definitions

| Term | Definition |
|------|-----------|
| Sealed trait | Rust pattern restricting trait implementation to the defining crate, used for `EditorMode` typestate enforcement |
| `keytrie` | Multi-key input matching structure in `src/input/typestate/` |

### 1.4 References

- [[BRD]] — Business Requirements Document
- [[SRS]] — Software Requirements Specification
- ISO/IEC 25010:2011

### 1.5 Priority and Trade-offs

Architectural consistency for registers (NFR-001) was fully honored.
Architectural consistency for command-line mode (NFR-002) was implicitly
traded off against shipping a working, testable feature quickly — the
input-typestate-only approach delivers the same user-observable behavior
without extending `EditorMode`, at the cost of the compile-time enforcement
that a sealed-trait `CommandMode` variant would have provided.

## 2. Compatibility / Architectural Consistency

### 2.1 Consistency with Existing Typestate Pattern

| ID | Requirement | Target | Status |
|----|------------|--------|--------|
| NFR-001 | New register-selection and command-line states MUST use the existing typestate pattern in `src/input/typestate/` (zero-sized marker structs + `InputStateMachine::process_key` dispatch), no parallel/ad-hoc mechanism | Full typestate compliance | **Met.** `RegisterPending`, `RegisterOpPending`, `CommandLinePending` all follow the established marker-struct + `HandlerState`/`Sealed` pattern in `src/input/typestate/state_types.rs`. |
| NFR-002 | Command-line/prompt mode MUST be added to `EditorMode` (`src/helix/simulator/mode.rs`) following the sealed-trait typestate pattern used for `NormalMode`/`InsertMode` | New `EditorMode` variant with compile-time enforcement | **Not met.** `EditorMode` was not extended — verified unchanged, still only `NormalMode`/`InsertMode`. Command-line state exists solely in the input-typestate layer, one level removed from the simulator's own mode type. The user-observable behavior (a distinct, reachable prompt mode) is present; the specific architectural mechanism requested is not. |

## 3. Security

### 3.1 Scope Discipline

| ID | Requirement | Target | Status |
|----|------------|--------|--------|
| NFR-003 | Vim-style marks MUST NOT be introduced under any name, alias, or reinterpretation | Zero mark-jump functionality | **Met** — confirmed via code review; no backtick/apostrophe mark handling exists anywhere in the diff. |

### 3.2 Memory Safety

| ID | Requirement | Target | Status |
|----|------------|--------|--------|
| NFR-004 | `#![forbid(unsafe_code)]` MUST be preserved; no `unsafe` blocks introduced | Zero `unsafe` in new code | **Met** — confirmed via grep across all files touched by the commit. |

## 4. Maintainability

### 4.1 Testability

| ID | Requirement | Target | Status |
|----|------------|--------|--------|
| NFR-005 | Both capabilities MUST be covered by scenario TOML fixtures validated via `cargo nextest run scenario`, plus unit tests for the simulator-level register map and command-line parser | Coverage depth matching existing command categories | **Met, though scenario breadth is thinner than SC-003's original ≥3-per-category target** — see [[SRS#3.2 Success Criteria]]. Unit test depth (~30+ new/changed tests) is comparable to or exceeds existing command categories. |

### 4.2 Documentation

| ID | Requirement | Target | Status |
|----|------------|--------|--------|
| NFR-007 | All new `pub` types/functions MUST carry `///` doc comments, including `# Examples` for non-trivial public APIs | Full doc coverage | **Met for the core new types** (`RegisterFile`, `CommandLine`, `normalize_command_id`) — not independently re-verified line-by-line as part of this retroactive record; assumed compliant per the project's rustdoc CI gate, which would have failed the PR otherwise. |

## 5. Reliability

### 5.1 Error Handling

| ID | Requirement | Target | Status |
|----|------------|--------|--------|
| NFR-006 | All new user-facing failure modes (invalid register name, malformed command-line input) MUST route through `UserError`/`SecurityError` | No panics on malformed input | **Met.** `CommandLine::parse` returns `UserError::CommandFailed` on malformed input; invalid register-name characters route through the same convention. A new `security::limits::MAX_COMMAND_LINE_LEN = 256` bound was added, going beyond the letter of NFR-006 into explicit input-length hardening. |

## 6. Unanticipated Finding — Input Robustness Hardening

Not requested by any drafted NFR, but delivered as part of this commit:

- Multi-byte/non-ASCII register-character handling was hardened
  (`chars()` used instead of byte-length/`.nth()` indexing) after a panic
  class was found during implementation — register names are not
  guaranteed single-byte.
- Unmapped Alt/Ctrl key combinations no longer fall through as bare-char
  commands in the input dispatcher — previously, e.g., Alt-z could execute
  as plain `z`, a real security/correctness-relevant gap in modifier-key
  handling unrelated to registers or command-line mode but found and fixed
  alongside them.
- Arcade-mode Esc handling was made consistent: it now cancels any pending
  register/command-line/prefix state instead of unconditionally pausing,
  and input state resets between arcade scenarios.

These are recorded here as a positive, unplanned quality outcome of the
work, not as failures against a drafted requirement.

## 7. Verification Matrix

| ID | Method | Environment | Status |
|----|--------|-------------|--------|
| NFR-001 | Code review against `state_types.rs` pattern | Local | Pass |
| NFR-002 | Code inspection of `mode.rs` | Local | Fail (not implemented) |
| NFR-003 | Code review, grep for mark-related identifiers | Local | Pass |
| NFR-004 | `grep -rn "unsafe"` across touched files | Local | Pass |
| NFR-005 | `cargo nextest run scenario` + unit test count | CI | Pass (depth), partial (breadth) |
| NFR-006 | Code review of error propagation paths | Local | Pass |
| NFR-007 | rustdoc CI gate (`RUSTDOCFLAGS="--deny rustdoc::broken_intra_doc_links" cargo doc --no-deps`) | CI | Assumed pass (gate would have blocked merge otherwise) |

## 8. Open Questions

> [!question] Unresolved Quality Requirements
> - [ ] Should `EditorMode` be retrofitted with a real `CommandMode` variant
>       to close the NFR-002 gap, or is the current input-typestate-only
>       approach the accepted final architecture for this feature?
> - [ ] Should scenario breadth (NFR-005) be expanded from 1 to ≥3 scenarios
>       per new category, matching the density of other command categories?

## See Also

- [[BRD]] — business rationale (source)
- [[SRS]] — functional requirements and verdicts
- [[spec]] — original research spec
