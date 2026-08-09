---
aliases:
  - Register and Command-Line Mode SRS
  - Issue 282 SRS
tags:
  - srs
  - requirements/functional
  - helix-simulator
  - input-system
  - status/implemented
created: 2026-08-09
project: "helix-trainer"
status: implemented
standard: "ISO/IEC/IEEE 29148:2018"
related:
  - "[[BRD]]"
  - "[[NFR]]"
---

# Named Register and Command-Line Mode Support: Software Requirements Specification

> [!abstract]
> Functional requirements as originally drafted, each marked against the
> actual implementation in commit `1ba668d`. Traceable to [[BRD]].

## 1. Introduction

### 1.1 Purpose

Records the functional requirements originally drafted for named-register
and command-line support and evaluates each against what commit `1ba668d`
actually shipped.

### 1.2 Scope

Covers `src/helix/simulator/` register/command-line additions and
`src/input/typestate/` state additions. Does not cover macro
recording/playback, scroll commands, or selection-regex (tracked separately
under issue #198).

### 1.3 Definitions, Acronyms, and Abbreviations

| Term | Definition |
|------|-----------|
| `RegisterFile` | `HashMap<char, String>` register storage, `src/helix/simulator/register_file.rs:35` |
| `RegisterPending` / `RegisterOpPending` | Typestate marker states for `"` and `"<reg>` respectively, `src/input/typestate/state_types.rs` |
| `CommandLinePending` | Typestate marker state for an open `:` prompt |
| `CommandLine::Goto(usize)` | The sole implemented command-line command |
| `normalize_command_id` | Canonicalizes raw command strings before FSRS/quest tracking, `src/helix/commands.rs:153-175` |

### 1.4 References

- [[BRD]] — Business Requirements Document
- [[NFR]] — Non-Functional Requirements Specification
- [Helix Keymap Reference](https://docs.helix-editor.com/keymap.html)
- [Helix Command-Line Reference](https://docs.helix-editor.com/command-line.html)

### 1.5 Document Overview

Section 3 lists FR-001 through FR-010 as drafted, each with a verdict and
evidence. Section 4 is the verification matrix against actual shipped
tests.

## 2. Overall Description

### 2.1 Product Perspective

Extends `HelixSimulator` (`src/helix/`) and the typestate input dispatcher
(`src/input/typestate/`), subsystems of the single-binary helix-trainer TUI.

### 2.2 Product Functions

Named-register yank/paste/replace; command-line goto-line navigation; two
new scenario categories; FSRS command-taught normalization.

### 2.3 User Classes and Characteristics

Unchanged from [[BRD#Target Users]].

### 2.4 Operating Environment

Unchanged — TUI via ratatui/crossterm.

### 2.5 Design and Implementation Constraints

`#![forbid(unsafe_code)]`; must follow the existing typestate pattern
(NFR-001).

### 2.6 Assumptions and Dependencies

See [[BRD#Assumptions]].

## 3. Specific Requirements

### 3.1 Functional Requirements — Verdict

> [!info] Traceability
> Traces to [[spec#3. Functional Requirements]].

**FR-001** — WHEN the user presses `"` followed by a register name
character in Normal mode THE SYSTEM SHALL enter a register-selection
pending state.

- *Priority*: must
- *Verdict*: **Implemented as-is.** `"` in `BaseState` transitions to
  `InputState::RegisterPending`
  (`src/input/typestate/handlers/base.rs:243-244`); any subsequent
  character transitions to `RegisterOpPending { register }`
  (`src/input/typestate/handlers/register.rs:538-551`).

**FR-002** — WHEN a register has been selected via `"<reg>` and the user
presses `y` THE SYSTEM SHALL yank the current selection into that named
register.

- *Priority*: must
- *Verdict*: **Implemented as-is.** `"<reg>y` dispatches to
  `clipboard::yank_to_register(sim, Some(register))`.

**FR-003** — WHEN a register has been selected via `"<reg>` and the user
presses `p` or `P` THE SYSTEM SHALL paste the content of that named
register after/before the cursor respectively.

- *Priority*: must
- *Verdict*: **Implemented as-is, plus `R`.** `p`/`P` behave as drafted;
  the implementation additionally scoped `R` (replace-with-yanked) to
  registers, which the draft did not request but is consistent with
  FR-002/FR-003's intent.

**FR-004** — WHEN no register is explicitly selected THE SYSTEM SHALL
preserve existing default-clipboard yank/paste behavior unchanged.

- *Priority*: must
- *Verdict*: **Implemented as-is.** `RegisterFile`'s unnamed key (`'"'`) is
  what plain `y`/`p`/`P`/`R` read/write
  (`src/helix/simulator/register_file.rs:14-20`); a `None` register
  parameter defaults to it. Existing clipboard tests remained passing
  unmodified aside from the field rename `clipboard` → `registers`.

**FR-005** — WHERE the user presses `:` in Normal mode THE SYSTEM SHALL
enter a new command-line/prompt mode distinct from `NormalMode` and
`InsertMode`.

- *Priority*: must
- *Verdict*: **Implemented differently.** `:` enters `CommandLinePending`
  in the **input typestate layer**, not a new `EditorMode::CommandMode`
  variant. `src/helix/simulator/mode.rs` is unchanged — still only
  `NormalMode`/`InsertMode` (verified: no new variant, no `Sealed` impl
  added). This directly contradicts NFR-002's letter, though it satisfies
  the user-observable behavior (a distinct prompt mode exists and is
  reachable via `:`).

**FR-006** — WHEN in command-line mode and the user types a valid
`:s/pattern/replacement/` expression and confirms THE SYSTEM SHALL apply
the substitution and return to Normal mode.

- *Priority*: must
- *Verdict*: **Not implemented.** No `:s` substitute exists — no regex, no
  pattern/replacement parsing anywhere in the diff. Only `:goto N`/`:g N`
  (alias) is implemented, per the commit title ("command-line **goto**
  mode"). This is the single largest scope gap between the draft and what
  shipped.

**FR-007** — WHEN in command-line mode and the user presses Escape THE
SYSTEM SHALL discard the pending input and return to Normal mode without
modifying the document.

- *Priority*: must
- *Verdict*: **Implemented as-is.** Esc while `CommandLinePending` →
  `HandlerResult::Cancel`, no document mutation
  (`src/input/typestate/handlers/command_line.rs:295`).

**FR-008** — WHEN in command-line mode and the user types an unrecognized
or malformed command THE SYSTEM SHALL surface a `UserError` rather than
panicking or silently no-op-ing.

- *Priority*: must
- *Verdict*: **Implemented as-is, with a safer failure path than drafted.**
  `CommandLine::parse` returns `UserError::CommandFailed` on malformed
  input; the handler intercepts this at Enter-time and converts it to a
  `Cancel` rather than propagating the `Err` further up the call stack —
  documented in the implementation as preventing an app-crashing failure
  mode found during review.

**FR-009** — WHERE new scenario categories are added THE SYSTEM SHALL
validate them via `cargo nextest run scenario` with no changes to the
validation harness itself.

- *Priority*: should
- *Verdict*: **Implemented as-is.** `tests/scenario_validation.rs` extended
  (+43 lines) but the harness mechanism itself is unchanged; new categories
  validate through it.

**FR-010** — WHEN a scenario's `metadata.commands_taught` references a
register-select or command-line command THE SYSTEM SHALL recognize it as a
valid taught command for FSRS tracking.

- *Priority*: should
- *Verdict*: **Implemented differently.** No changes to a `src/learning/`
  command-mapping table. Instead, a new `helix::commands::
  normalize_command_id()` (`src/helix/commands.rs:153-175`) canonicalizes
  raw command strings (e.g. `"ay` → `"y`, `:g 3` → `:goto`) at three call
  sites before they reach FSRS/quest tracking: `scenario_completion.rs:163`,
  `minigame/session.rs:848`, `ui/state/handlers/quests.rs:21`. This means
  every register letter collapses to one FSRS-tracked skill per operation
  (`"y`, `"p`, `"P`, `"R`), not one skill per letter.

**Net**: 6 of 8 "must" FRs fully satisfied (FR-001..004, FR-007, FR-008);
FR-005 satisfied behaviorally but architecturally deviant; FR-006 (the
user-facing goal of US-002) **not implemented at all**. Both "should" FRs
(FR-009, FR-010) satisfied, FR-010 via a different mechanism than drafted.

### 3.2 Success Criteria

| ID | Metric | Verdict |
|----|--------|---------|
| SC-001 | Named-register yank/paste round-trip correctness, 100% pass, no regression | **Met** |
| SC-002 | Command-line `:s` substitute correctness | **Not met — `:s` never implemented.** `:goto`/`:g` has its own passing test suite instead. |
| SC-003 | ≥1 scenario category per capability, ≥3 scenarios/category spanning difficulty | **Partially met** — `scenarios/en/registers/named-registers.toml` (1 scenario, intermediate) and a command-line goto scenario under `scenarios/en/movement/command-line-goto.toml` (1 scenario, intermediate); new `ScenarioCategory::Registers` variant added |
| SC-004 | Code review confirms new states follow the typestate pattern, zero ad-hoc tracking | **Met for registers; not met for command-line's relationship to `EditorMode`** (FR-005 deviation) |
| SC-005 | Code review confirms no vim-marks scope creep | **Met** |

### 3.3 Logical Data Requirements

| Entity | Description | Key Attributes |
|--------|-------------|----------------|
| `RegisterFile` | Register storage map | `HashMap<char, String>`, `UNNAMED_REGISTER = '"'` |
| `CommandLine` | Parsed command-line command | Single variant `Goto(usize)` — no `Substitute` variant exists |
| `InputState::RegisterPending`/`RegisterOpPending { register: char }` | Pending typestate for register selection | register name char, resolves after exactly one subsequent `y`/`p`/`P`/`R` |
| `InputState::CommandLinePending { buffer: String }` | Pending typestate for an open `:` prompt | accumulated raw input |

## 4. Verification and Validation

### 4.1 Verification Matrix

| Requirement | Method | Criteria | Status |
|------------|--------|----------|--------|
| FR-001..004 | Unit tests in `register_file.rs`, `handlers/register.rs` (11 tests total) | Register select/yank/paste/replace round-trip, unnamed-register regression | **Pass** |
| FR-005 | Code inspection | New prompt mode reachable via `:` | **Pass behaviorally, fails architecturally** (no `EditorMode::CommandMode`) |
| FR-006 | — | `:s` substitute apply-and-return-to-Normal | **Not run — feature does not exist** |
| FR-007, FR-008 | Unit tests in `command_line.rs`, `handlers/command_line.rs` (25 tests total) | Cancel-on-Escape, malformed-input handling | **Pass** |
| FR-009 | `cargo nextest run scenario` | New categories validate | **Pass** |
| FR-010 | Regression test in `minigame/session.rs` | `normalize_command_id` produces expected FSRS-tracked skill IDs | **Pass** |

### 4.2 Acceptance Test Outline

`cargo nextest run --workspace --all-features --lib --bins` runs ~30+
new/changed test functions across `register_file.rs` (5), `command_line.rs`
(13), `keytrie.rs` (6, incl. multi-byte-char panic regressions),
`handlers/register.rs` (6), `handlers/command_line.rs` (12),
`commands/mod.rs` dispatch tests (6), state-machine boundary-cancel tests
(2), `tests/ui_multi_key_commands.rs` integration flows (+159 lines), an
FSRS-normalization regression test, and arcade Esc-routing tests (3).
`cargo nextest run scenario` additionally validates the two new scenario
TOML files.

## 5. Appendices

### 5.1 Traceability Matrix — Original Open Questions Resolved

| Draft Open Question | Resolution |
|---|---|
| Does `"<reg>` selection apply only to `y`/`p`/`P`, or to any operator? | Resolved narrower-but-adjacent: exactly `y`/`p`/`P`/`R`; anything else cancels the pending state |
| What is Helix's default `:s` scope? | Moot — `:s` was never implemented |
| Should `:g` (global) be included alongside `:s`? | Resolved as an alias, not the real command: `:g N` is an alias for `:goto N`, not Helix's real global-command-with-subcommand |
| Should registers be one FSRS skill or per-letter? | Resolved as one skill per operation family via `normalize_command_id` |
| Does `RepeatBuffer` need to record register/command-line input for `.`? | Resolved as explicitly non-repeatable — `cmd_to_key_events` has no arm for these shapes; `.` after either is a documented no-op, covered by `test_cmd_to_key_events_register_op_and_command_line_are_not_repeatable` |

### 5.2 BRD/SRS/NFR Traceability Matrix

| BRD Section | SRS Requirement(s) | NFR Requirement(s) |
|----------------|--------------------|--------------------|
| [[BRD#Named Registers]] | FR-001..004 (met, FR-003 extended) | NFR-001 (met), NFR-004 (met) |
| [[BRD#Command-Line Mode]] | FR-005 (deviant), FR-006 (not met), FR-007, FR-008 (met), FR-009 (met) | NFR-002 (not met), NFR-006 (met) |
| [[BRD#What Was Implemented]] (FSRS tracking) | FR-010 (met, different mechanism) | NFR-005, NFR-007 (met) |

## See Also

- [[BRD]] — business rationale and what actually shipped (source)
- [[NFR]] — non-functional requirements
- [[plan]] — as-built architecture
- [[spec]] — original research spec
