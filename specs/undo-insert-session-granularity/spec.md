---
aliases:
  - Undo Insert Session Granularity
  - Insert session is one undo step
tags:
  - sdd
  - spec
  - helix
  - undo
  - bug-fix
  - status/proposed
created: 2026-11-11
status: proposed
related:
  - "[[constitution]]"
  - "[[helix-core-post-25-07-1-upgrade-readiness/spec]]"
  - "[[MOC-specs]]"
---

# Feature: An Insert Session Is One Undo Step

> [!info] Metadata
> **Issues**: #571 (P2; labels `bug`, `helix-core`; found by live testing)
> **Related**: #501 (selection restore on undo, already specified as FR-022 in [[helix-core-post-25-07-1-upgrade-readiness/spec]])
> **Status**: Proposed
> **Depth**: Lightweight bug-fix spec.

## 1. Overview

### Problem Statement

In Helix 25.07.1 (the vendored `helix-core` version) everything done between entering Insert mode and the Escape that leaves it forms one history revision: after `i a b Escape`, one `u` restores the original text and the selection that existed before the change. The simulator (`src/helix/simulator/`) records one undo entry per transaction. `apply_transaction` (`simulator/mod.rs`) pushes a `HistoryEntry` for every non-empty transaction, and Insert mode applies one transaction per typed character (`insert_text`, `insert_newline`, `backspace`, `apply_insert_command` in `simulator/insert_mode.rs`). `c` is two entries: the deletion runs in Normal mode before `enter_insert_mode` (`change_selection_impl`), then each typed character adds its own. `o`/`O` are likewise one entry for the opened line plus one per typed character. One `u` therefore undoes only the last keystroke, which also breaks `undo`/`redo` scenarios taught to players.

The only Insert-session step already folded correctly is the unused auto-indent removal on Escape (`strip_unused_auto_indent` applies its transaction without a history entry).

### Evidence

Live run in the TUI (tmux) on scenario 14 "Navigate multiline code" (first line `fn add(a: i32, b: i32) -> i32 {`, cursor 0,0): `i a b Escape` gives `abfn add(...`; `u` gives `afn add(...)` (expected `fn add(...)`).

Differential run against `hx` 25.07.1, same text, followed by `c @ Escape` to expose the selection (trainer vs hx, first line):

| Sequence | Trainer | hx 25.07.1 |
|----------|---------|-----------|
| `i a b Escape u` | `a@n add(...` | `@n add(...` |
| `i a b c Escape u` | `ab@n add(...` | `@n add(...` |
| `a x y Escape u` | `fx@ add(...` | `@ add(...` |
| `o x y Escape u` | opened line remains, containing `x` | opened line removed entirely |
| `c @ Escape u` | `@ add(...` (only the insert undone) | `@n add(...` (whole change undone, selection restored) |

`i a b Escape u U` already matches hx, because redo after a partial undo hides the defect.

### Goal

Everything from the command that enters Insert mode (`i`, `a`, `I`, `A`, `o`, `O`, `c`, `C`-style changes) to the Escape that leaves it is exactly one undo step: `u` reverts all of it and restores the pre-change selection, and `U` re-applies all of it.

### Out of Scope

- Selection restore for single-step changes (#501, already specified)
- Changing how the action counter or scoring counts Insert-mode commands
- Multi-cursor Insert mode (the simulator's Insert mode is single-cursor by design, see `enter_insert_mode`)
- Undo tree / `Alt-u` / `Alt-U` (earlier/later) semantics; the simulator keeps linear undo and redo stacks
- Changing the `HistoryEntry` snapshot representation for reasons other than this fix

## 2. User Stories

### US-001: Undo a whole insertion with one key
AS A learner practicing Helix undo
I WANT `u` after typing text in Insert mode to remove everything I typed in that session
SO THAT the trainer teaches the same undo behavior I will get in real Helix

**Acceptance criteria:**
```
GIVEN the first line `fn add(a: i32, b: i32) -> i32 {` and the cursor at 0,0
WHEN  i a b Escape u are executed
THEN  the first line is `fn add(a: i32, b: i32) -> i32 {` and the selection equals the one before `i`
```

### US-002: Undo a change as one step
AS A learner
I WANT `u` after `c`, typed text and Escape to restore the deleted text and its selection
SO THAT a change counts as one operation, as in Helix

**Acceptance criteria:**
```
GIVEN the first line `fn add(...)` with `f` selected
WHEN  c @ Escape u are executed
THEN  the first line is `fn add(...)` and the selection is the one that existed before `c`
```

### US-003: Redo a whole session
AS A learner
I WANT `U` after undoing an insert session to re-apply the whole session
SO THAT undo and redo round-trip

**Acceptance criteria:**
```
GIVEN i a b Escape u were executed
WHEN  U is executed
THEN  the first line is `abfn add(...` and the selection equals the one after the original Escape
```

## 3. Functional Requirements

| ID | Requirement | Priority |
|----|------------|----------|
| FR-001 | WHEN Insert mode is left with Escape THE SIMULATOR SHALL have recorded exactly one undo entry for the whole session, covering every transaction applied since the command that entered Insert mode began | must |
| FR-002 | WHEN the Insert-entering command itself edits the document before Insert mode starts (`c`, `o`, `O`, linewise `xc`) THE SIMULATOR SHALL include that edit in the same single undo entry as the typed text | must |
| FR-003 | WHEN `u` is executed after an Insert session THE SIMULATOR SHALL restore the document and the selection that existed immediately before the Insert-entering command | must |
| FR-004 | WHEN `U` is executed after undoing an Insert session THE SIMULATOR SHALL restore the document and selection as they were when Insert mode was left | must |
| FR-005 | WHEN an Insert session leads to no net document change (`i Escape`, `a Escape`, `c` of an empty selection followed by Escape) THE SIMULATOR SHALL record no undo entry and SHALL NOT clear the redo stack | must |
| FR-006 | WHEN any in-session edit is applied (typed character, newline with auto-indent, `Backspace`, `Ctrl-w`, `Ctrl-u`, indent) THE SIMULATOR SHALL NOT record an undo entry of its own | must |
| FR-007 | WHEN the unused auto-indent is stripped on Escape THE SIMULATOR SHALL keep folding it into the session's single entry (current behavior preserved) | must |
| FR-008 | WHILE an Insert session is active THE SIMULATOR SHALL NOT expose a partially recorded session through `u`/`U`; a session that has ended is always fully recorded | must |
| FR-009 | THE SIMULATOR SHALL represent "inside an Insert session" so that the session boundary cannot be left unclosed when the mode transitions back to Normal (type-level or single-owner invariant per the constitution's typestate principle, not a flag checked at call sites) | must |
| FR-010 | WHEN a session is closed THE SIMULATOR SHALL clear the redo stack exactly once, as any new edit does today | must |
| FR-011 | THE FIX SHALL NOT change the undo behavior of Normal-mode commands that do not enter Insert mode (`d`, `p`, `r`, `>`, and so on): each remains one entry | must |
| FR-012 | WHERE Helix 25.07.1 ends a session before Escape (for example cursor-movement keys in Insert mode) THE SIMULATOR SHALL end the undo step at the same points | should |

> [!question]
> - [NEEDS CLARIFICATION: FR-012 / Insert-mode cursor movement: does Helix 25.07.1 commit an undo checkpoint when arrow keys or other movement are used in Insert mode, splitting one `i ... Escape` into several revisions? Verify against the vendored `helix-core`/`helix-term` source and `hx` before implementing; the simulator currently handles arrow keys in Insert mode.]
> - [NEEDS CLARIFICATION: with a count (`3o`, `3a`) or a dot-repeat (`.`) of an Insert command, does Helix treat the repeated execution as one undo step with the original session? Verify against `hx`.]
> - [NEEDS CLARIFICATION: macro replay (`Q`/`q`) and `.` run Insert commands with `is_repeating` set; confirm each replayed session is one step.]

## 4. Non-Functional Requirements

| ID | Category | Requirement |
|----|----------|-------------|
| NFR-001 | Fidelity | Behavior matches real Helix 25.07.1 for the sequences in section 1; the differential table is reproduced as regression tests |
| NFR-002 | Type safety | Session state is not a bare `bool` or `Option` consulted ad hoc; per the constitution, the invariant is carried by the existing typestate (`HelixSimulator<InsertMode>`) or one owning type |
| NFR-003 | Performance | Typing in Insert mode must not become slower per keystroke; recording one entry per session must not clone the document per keystroke |
| NFR-004 | Documentation | Public docs of `undo`, `redo`, `enter_insert_mode`, `exit_insert_mode` and the module docs describe the session rule |

## 5. Data Model

| Entity | Description | Key Attributes |
|--------|-------------|----------------|
| `HistoryEntry` (`simulator/undo.rs`) | Document and selection snapshot stored on the undo and redo stacks | `doc`, `selection` |
| Insert session | The span from the Insert-entering command to Escape that yields at most one `HistoryEntry` | pre-command snapshot (document and selection) |
| `history` / `redo_stack` (`simulator/mod.rs`) | Linear undo and redo stacks | `Vec<HistoryEntry>` |

## 6. Edge Cases and Error Handling

| Scenario | Expected Behavior |
|----------|-------------------|
| `i Escape` with nothing typed | No undo entry; a following `u` undoes the previous real change; pending redo history is kept (FR-005) |
| `c` on an empty selection (nothing deleted) then Escape | No undo entry (FR-005) |
| `c`, no typing, Escape | One entry: the deletion (FR-002) |
| `o`, no typing, Escape | Opened line is stripped by `strip_unused_auto_indent`; net change is the new blank line or none, per the current rule; at most one entry (FR-007) |
| `i a Backspace Escape` (typed then deleted) | No net change; no entry (FR-005) |
| `i a b Escape u U u` | Round-trips; document equal at each step to the corresponding hx state |
| `x d` then `i a Escape u` | `u` removes only `a`; the `x`/`d` entries remain separate steps |
| Escape pressed in Normal mode | Not part of a session; unchanged |
| Redo stack non-empty, then `i a Escape` | Redo stack cleared once when the session closes with a change (FR-010) |
| Session containing a newline with auto-indent | One entry; Escape strip does not add another (FR-007) |
| Mode forced back to Normal without Escape (scenario reset, screen change) | Pending session is closed or discarded consistently; no partially recorded entry survives (FR-008, FR-009) |

## 7. Success Criteria

| ID | Metric | Target |
|----|--------|--------|
| SC-001 | The five differential sequences in section 1 run through `AnyModeSimulator` produce the hx column | All match |
| SC-002 | `history.len()` after `i a b Escape` on a fresh simulator | 1 |
| SC-003 | `history.len()` after `c` + typed text + Escape | 1 |
| SC-004 | `cargo nextest run --workspace --all-features --lib --bins` and `cargo nextest run scenario` | Pass; no existing undo/redo scenario regresses |
| SC-005 | Existing selection-restore tests in `simulator/undo.rs` (#501) | Pass unchanged |

## 8. Agent Boundaries

### Always (without asking)
- Read the existing history/undo code in `src/helix/simulator/` (`mod.rs`, `undo.rs`, `insert_mode.rs`, `commands/mod.rs`) and reuse its patterns
- Add regression tests with real `HelixSimulator`/`AnyModeSimulator` execution, not mocks
- Run the full pre-commit suite from the constitution and `cargo nextest run scenario`
- Update `CHANGELOG.md` `[Unreleased]` in the implementing PR

### Ask First
- Changing the public signature of `enter_insert_mode`, `exit_insert_mode`, or the `HistoryEntry` representation
- Resolving any `[NEEDS CLARIFICATION]` item by choosing behavior that differs from observed `hx` output
- Editing bundled scenario TOML solutions that rely on the old per-keystroke undo behavior

### Never
- Introduce a parallel history mechanism or bypass `HelixSimulator` with string-diff shortcuts
- Change the Normal-mode undo granularity of non-Insert commands (FR-011)
- Add `unsafe`, or weaken typing with an untyped flag to track the session
- Edit specs by hand outside the `sdd` agent workflow

## 9. Open Questions

> [!question]
> - [NEEDS CLARIFICATION: see the three questions under FR-012 (Insert-mode movement, counted/dot-repeated Insert commands, macro replay).]
> - [NEEDS CLARIFICATION: do any bundled scenarios (for example `undo_redo_*`) depend on per-keystroke undo? `cargo nextest run scenario` after the change must confirm; list affected ids in the plan.]
> - [NEEDS CLARIFICATION: the finding was recorded for `i`, `a`, `o`, `c`; confirm `I`, `A`, `O`, `C`, `s`-style changes against `hx` (they share the same entry path and are expected to behave identically).]

## 10. See Also

- [[constitution]] - project principles (simulator-only execution, typestate, regression tests with real dependencies)
- [[helix-core-post-25-07-1-upgrade-readiness/spec]] - FR-022 covers selection restore on `u`/`U` (#501)
- [[MOC-specs]] - all specifications
