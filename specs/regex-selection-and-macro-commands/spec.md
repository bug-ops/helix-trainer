---
aliases:
  - Regex Selection and Macro Commands
  - select_regex / split_selection / Macro Record-Replay
tags:
  - sdd
  - spec
  - helix-simulator
  - input-system
  - status/implemented
created: 2026-08-10
status: implemented
related:
  - "[[constitution]]"
  - "[[../register-command-mode-support/spec]]"
  - "[[MOC-specs]]"
---

# Feature: Regex Selection (`s`/`S`) and Macro Record/Replay (`q`/`Q`)

> [!info] Metadata
> **Resolved by**: commit `a4efc2e` — "feat: implement regex selection and
> macro record/replay commands (#351)", closing issues #337, #338.
> **Status**: Implemented
> **Depth**: Lightweight spec, per this project's SDD scaling guidance —
> retroactively documented because this shipped with no spec anywhere in
> the repository. Both capabilities were previously tracked only as a
> deferred bullet inside [[../register-command-mode-support/spec|Named
> Register and Command-Line Mode Support]]'s "Out of Scope" (referencing
> issue #198), which this feature resolves.

## 1. Overview

### Problem Statement

Issue #198's Normal Mode command-coverage audit found two documented Helix
commands registered in the command metadata but with no working handler:

1. `s` / `S` (`select_regex` / `split_selection`) — narrow or split the
   current selection by regex match — were stubbed as commented-out
   no-op placeholders.
2. `q` / `Q` (macro record/replay) had no key binding and no
   record/replay machinery at all in `HelixSimulator`.

`d66278d` (the commit that closed #198's scenario-coverage gap for
scroll/select-all/replace commands) explicitly deferred both of these as
"from-scratch feature work, not scenario content." This feature is that
follow-up work.

### Goal

`s`/`S` narrow or split the current selection by a user-supplied regex
pattern; `q`/`Q` record and replay a sequence of successfully-executed
commands, both reachable from Normal mode and usable inside scenario
solutions.

### Out of Scope

- Named macro registers (`"<reg>q`) — `RegisterFile` stores a single
  `String` per register, not a `Vec<String>`; deferred, tracked as a
  follow-up in `src/helix/macro_recorder.rs`'s module doc
- Unifying the macro recorder with the existing `.`-repeat recorder — the
  two capture fundamentally different shapes (raw `KeyEvent`s vs. resolved
  command strings) for real reasons; deferred, not attempted here
- `Alt-K` / `K` (keep/remove selections matching regex) — not part of this
  change (tracked separately, see `docs/HELIX_KEYBINDINGS.md`)
- Any change to the command-line `:goto`/`:g` scope already resolved by
  [[../register-command-mode-support/spec|register-command-mode-support]]

## 2. User Stories

### US-001: Narrow or split a selection by regex
AS A helix-trainer user practicing advanced selection workflows
I WANT to press `s` or `S` and type a regex pattern to narrow or split the
current selection
SO THAT I can learn Helix's real regex-based selection workflow instead of
manual character-by-character selection

**Acceptance criteria:**
```
GIVEN a selection spanning multiple regex-matchable substrings
WHEN  the user presses `s`, types a pattern, and confirms
THEN  the selection narrows to all non-overlapping matches inside the prior
      selection (select_on_matches)
```
```
GIVEN a selection spanning multiple regex-matchable substrings
WHEN  the user presses `S`, types a pattern, and confirms
THEN  the selection splits into the segments between matches (split_on_matches)
```

### US-002: Record and replay a macro
AS A helix-trainer user
I WANT to press `q` to start/stop recording a sequence of commands and `Q`
to replay it
SO THAT I can learn Helix's macro workflow for repetitive multi-step edits

**Acceptance criteria:**
```
GIVEN the simulator is idle (not recording, not replaying)
WHEN  the user presses `q`, executes N commands, then presses `q` again
THEN  those N commands are stored as the current macro, replacing any
      previously stored macro
```
```
GIVEN a macro has been recorded
WHEN  the user presses `Q`
THEN  every stored command replays through the same dispatch path
      (execute_command_any_mode) used for live input
```

## 3. Functional Requirements

Use EARS notation. Prefix with FR-NNN.

| ID | Requirement | Priority |
|----|------------|----------|
| FR-001 | WHEN the user presses `s` in Normal mode with an active selection THE SYSTEM SHALL enter the existing command-line prompt-input pattern to collect a regex pattern | must |
| FR-002 | WHEN a valid regex pattern is confirmed after `s` THE SYSTEM SHALL narrow the current selection to all non-overlapping matches via `helix-core`'s `select_on_matches` | must |
| FR-003 | WHEN a valid regex pattern is confirmed after `S` THE SYSTEM SHALL split the current selection at match boundaries via `helix-core`'s `split_on_matches` | must |
| FR-004 | WHEN a regex pattern exceeds a maximum length or a match count exceeds `MAX_REGEX_SELECTION_MATCHES` (10,000) THE SYSTEM SHALL truncate/reject rather than panic or hang | must |
| FR-005 | WHEN the user presses `q` while idle THE SYSTEM SHALL begin recording successfully-executed command strings into a single unnamed macro slot | must |
| FR-006 | WHEN the user presses `q` while recording THE SYSTEM SHALL stop recording and overwrite the stored macro with what was just captured (unconditionally, even if empty) | must |
| FR-007 | WHEN the user presses `Q` and a macro is stored THE SYSTEM SHALL replay every stored command through `execute_command_any_mode`, the same dispatch path used for live input, bounded by `MAX_MACRO_LENGTH` (100 commands recorded) and `MAX_MACRO_DEPTH` (10, replay recursion) | must |
| FR-008 | WHEN a command executes while a macro replay (`Q`) is in progress THE SYSTEM SHALL NOT feed it back into the recording tap, even if recording is simultaneously active | must |
| FR-009 | WHEN `s`/`S` scenario completions are recorded THE SYSTEM SHALL normalize the command id so scenario completions mint one FSRS card per skill (`select_regex`/`split_selection`) rather than one card per literal regex pattern typed | must |

## 4. Non-Functional Requirements

| ID | Category | Requirement |
|----|----------|-------------|
| NFR-001 | Dependency hygiene | The `select_on_matches`/`split_on_matches` implementations come from a new `helix-stdx` dependency, pinned to the same git tag (`25.07.1`) as the existing `helix-core` dependency — no independent version drift |
| NFR-002 | Safety | `#![forbid(unsafe_code)]` MUST be preserved; regex compilation/matching MUST be guarded against pathological patterns via the length cap and match-count truncation (FR-004) |
| NFR-003 | Architectural consistency | The regex prompt reuses the existing command-line typestate prompt-input pattern rather than introducing a parallel one |
| NFR-004 | Correctness | Macro replay MUST NOT resurface a latent bug in repeat-state restore — `a4efc2e` fixed exactly this class of bug as part of this change |

## 5. Data Model

| Entity | Description |
|--------|-------------|
| `MacroRecorder` (`src/helix/macro_recorder.rs`) | Owns `recording: Option<Vec<String>>`, `stored: Vec<String>`, `is_replaying: bool`, `replay_depth: usize` — records/replays a single unnamed `q`/`Q` macro on the simulator |
| Regex prompt input | Transient typestate state collecting a pattern string, shared shape with the existing command-line prompt |

No persistent (serialized) entities — macro state lives on `HelixSimulator`
for the duration of a session only; it does not survive a screen
transition that resets simulator state (see Edge Cases).

## 6. Edge Cases and Error Handling

| Scenario | Expected Behavior |
|----------|-------------------|
| `s`/`S` pattern is invalid regex syntax | `UserError` describing the failure; selection unchanged |
| Pattern matches nothing inside the current selection | Selection unchanged (no-op), no error |
| Match count would exceed `MAX_REGEX_SELECTION_MATCHES` | Truncated to the cap rather than hanging or panicking |
| `q` pressed a second time immediately after the first (nothing captured) | Stored macro is overwritten with an empty macro, discarding any prior one — deliberate, matches real Helix's `q`/`Q` parity (no separate "cancel without saving" gesture) |
| `Q` pressed with nothing stored, already replaying, or at `MAX_MACRO_DEPTH` | `begin_replay` returns `false` and changes nothing |
| `.`-repeat executed while recording | Captured as the *expansion* of `.` (each resulting command individually), not the literal `.` character — deterministic regardless of repeat-buffer state at replay time |
| `Q` triggered while already recording | Deliberate no-op for the replay call itself: `execute_command_any_mode` checks `is_recording_macro()` before calling `execute_macro_replay()`, so a nested replay's effects would otherwise apply to the document without being captured |
| Session paused or ended mid-recording (Task/MiniGame screens consume bare `q` for `QuitApp`/`MiniGameBackToMenu` in some states) | In-progress recording is silently dropped along with the rest of simulator state — known limitation, not addressed by this change |
| `"<reg>d`/`"<reg>c` register-scoped commands executed inside a macro | Handled as part of a later fix, see [[../register-command-mode-support/spec#11. Post-Release Extensions (v0.6.0)\|register-command-mode-support's extensions]] (`efeac9d`) |

## 7. Success Criteria

| ID | Metric | Target |
|----|--------|--------|
| SC-001 | `s`/`S` regression tests pass | `select_regex`/`split_selection` covered in `src/helix/simulator/commands/selection.rs` and dispatch tests in `src/helix/simulator/commands/mod.rs` |
| SC-002 | Macro record/replay regression tests pass | `src/helix/simulator/tests/macro_tests.rs` (new, 161 lines) |
| SC-003 | No regression in `cargo nextest run --workspace --all-features --lib --bins` | 100% pass |
| SC-004 | Issue #198's deferred scope (macros, selection-regex) fully closed | Both capabilities implemented; no remaining reference to #198 as "still open" anywhere in `specs/` |

## 8. Agent Boundaries

### Always (without asking)
- Run `cargo nextest run --workspace --all-features --lib --bins` after any
  change touching `src/helix/macro_recorder.rs` or
  `src/helix/simulator/commands/selection.rs`
- Keep macro replay dispatching through `execute_command_any_mode` — never
  introduce a parallel replay path that bypasses the registry

### Ask First
- Introducing named macro registers (`"<reg>q`) — requires a `RegisterFile`
  data-model change (single `String` per register → `Vec<String>`)
- Raising `MAX_MACRO_LENGTH`/`MAX_MACRO_DEPTH`/`MAX_REGEX_SELECTION_MATCHES`

### Never
- Let macro replay call back into itself unboundedly — always respect
  `MAX_MACRO_DEPTH`
- Feed replayed commands back into the recording tap while a macro is
  simultaneously being recorded (FR-008)

## 9. Open Questions

- [NEEDS CLARIFICATION: should named macro registers (`"<reg>q`) be
  prioritized as a follow-up, given named registers already exist for
  yank/paste via [[../register-command-mode-support/spec|register-command-mode-support]]?]

## 10. See Also

- [[constitution]] — project principles
- [[../register-command-mode-support/spec]] — sibling feature; this spec
  resolves the "Macro recording/playback" and "selection-regex" bullets
  that spec's Out of Scope section deferred to issue #198
- [[MOC-specs]] — all specifications
- [Helix Keymap Reference](https://docs.helix-editor.com/keymap.html)
- `docs/HELIX_KEYBINDINGS.md` — user-facing implemented/not-implemented
  matrix (note: this project's `q`=record-toggle/`Q`=replay assignment is
  a deliberate deviation from real Helix's register-scoped `Q`=record/
  `q`=replay, since only a single unnamed macro slot is implemented, not
  named macro registers)
- Issue #198 — Normal Mode command-coverage gap (closed; this spec plus
  `d66278d` closes it)
