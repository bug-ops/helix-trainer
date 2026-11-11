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

## 10. Bug-Fix Batch: Input Chords, Counted Commands, Surround and Primary Selection (#487, #493, #502, #503, #455)

> [!info] Added 2026-11-10 (spec-first update for the P1-P3 bug batch)
> Behavior changes below are specified before implementation. Reference behavior for every criterion is `hx` 25.07.1 (the version vendored as `helix-core`). Tests MUST assert the resulting selection, not only the final buffer text. Related: [[../helix-core-post-25-07-1-upgrade-readiness/spec#3.8 Simulator Fidelity at the Pinned Core (25.07.1)|simulator fidelity requirements]] (shared count mechanism), [[../register-command-mode-support/spec#12. Bug-Fix Batch: Search Prompt, Linewise and Multi-Range Paste, Insert-Mode Keys|register-command-mode-support bug-fix batch]].

### 10.1 Requirements

| ID | Requirement | Issue | Priority |
|----|------------|-------|----------|
| FR-101 | WHEN a Normal-mode key arrives with the Alt modifier THE INPUT LAYER SHALL NOT enter any plain-key pending state (`s`/`S` regex prompt, `g`/`z`/`m`/`[`/`]` leaders, `f`/`t`/`r` argument states, `"`, `:`, `q`, count digits); it SHALL either emit the Alt-bound command or stay in the current state. Chord classification SHALL be a closed type (`Plain`, `Alt`, `Ctrl`) matched exhaustively so an unhandled modifier is a compile error | #487 | must |
| FR-102 | WHEN the user presses `Alt-s` in Normal mode THE SYSTEM SHALL execute `split_selection_on_newline` immediately, increment the action counter once, and leave the footer without a pending prompt; scenario 126 ("Split selection on newlines") SHALL be completable with solution `Alt-s` | #487 | must |
| FR-103 | WHEN a count prefix is followed by `G` or `gg` THE SYSTEM SHALL go to that line (`2G` and `2gg` both go to line 2) and SHALL NOT cancel the command or re-interpret the following keys | #493 | must |
| FR-104 | WHEN a count prefix is followed by `f`, `t`, `F` or `T` and a target character THE SYSTEM SHALL use the Nth occurrence (`3fa` selects up to the 3rd `a`), as a single command and a single action | #493 | must |
| FR-105 | THE INPUT LAYER SHALL carry the count as a non-zero value through the `g`/`f`/`t`/`F`/`T`/`G` pending states and emit one canonical command string that includes the count (for example `3fa`, `2G`); every string it emits SHALL be accepted by the simulator (a unit test iterates the input layer's emittable grammar against the simulator) | #493 | must |
| FR-106 | THE SIMULATOR SHALL be the only place that applies a count (one counted-execution mechanism, see [[../helix-core-post-25-07-1-upgrade-readiness/spec#3.8 Simulator Fidelity at the Pinned Core (25.07.1)\\|section 3.8 FR-025]]); the game layer SHALL NOT repeat the base command to emulate a count | #493 | must |
| FR-107 | WHEN `ms<char>` (surround add) completes THE SYSTEM SHALL leave each range covering the surrounded span including both inserted delimiters (`w m s [ d` deletes the whole `[fn ]`) | #502 | must |
| FR-108 | WHEN `md<char>` (surround delete) completes THE SYSTEM SHALL leave each range covering the former inner text with the delimiters removed (not collapse to a point) | #502 | must |
| FR-109 | WHEN `mr<old><new>` (surround replace) completes THE SYSTEM SHALL leave each range covering the span including the new delimiters | #502 | must |
| FR-110 | WHEN `,` (`keep_primary_selection`) is executed with several ranges THE SYSTEM SHALL reduce the selection to the primary range only; with one range it is a no-op | #503 | must |
| FR-111 | WHEN `Alt-,` (`remove_primary_selection`) is executed with several ranges THE SYSTEM SHALL remove the primary range, promoting the next one; WHEN only one range exists THE SYSTEM SHALL leave it unchanged without error | #503 | must |
| FR-112 | WHEN several ranges are yanked and pasted THE SYSTEM SHALL yank one value per range and paste value N at range N (see [[../register-command-mode-support/spec#12. Bug-Fix Batch: Search Prompt, Linewise and Multi-Range Paste, Insert-Mode Keys\|register-command-mode-support FR-213/FR-214]]) | #455 | must |

### 10.2 Acceptance Criteria

```
GIVEN scenario 126 is active with a pre-set multi-line selection
WHEN  the user presses Alt-s
THEN  the selection becomes [[0,0,0,10],[1,0,1,10],[2,0,2,10]]
 AND  no prompt is pending
 AND  the action counter is 1
```
```
GIVEN the text of scenario 14 and the cursor at (0,0)
WHEN  the user presses 2, G, d
THEN  the first character of line 2 is deleted and nothing else changes
```
```
GIVEN the text of scenario 14 and the cursor at (0,0)
WHEN  the user presses 3, f, a, d
THEN  the text from the start through the 3rd `a` is deleted (identical to hx 25.07.1)
 AND  the action counter is 2 (one counted find, one delete)
```
```
GIVEN the text of scenario 14 and the cursor at (0,0)
WHEN  the user presses w, m, s, [, d
THEN  the buffer starts with `add(a: i32, ...`
```
```
GIVEN the text of scenario 14 and `% s a Enter` produced two ranges
WHEN  the user presses `,` then d
THEN  only the first `a` is deleted (`fn dd(a: i32, ...`)
 AND  with `Alt-,` instead of `,` only the second `a` is deleted
```
```
GIVEN `% s um Enter` produced two ranges (the `um` in `sum` on lines 2 and 3)
WHEN  the user presses y then p
THEN  line 2 is `let sumum = a + b;` and line 3 is `    sumum`
```

### 10.3 Edge Cases

| Scenario | Expected Behavior |
|----------|-------------------|
| Alt-modified digit, `g`, `f`, `t`, `r`, `m`, `z`, `"`, `:`, `q` keys | Never open a pending state; Alt-bound commands execute, unbound Alt chords are ignored (no action recorded) |
| Count larger than the available matches for `f`/`t` (`9fa`) | Selection unchanged, no error, same as the uncounted failure path |
| `{n}G` / `{n}gg` with `n` beyond the last line | Clamped to the last line as in hx |
| Bare `G` with no count | No-op (verified against helix-term 25.07.1); bare `gg` goes to the file start |
| `md` / `mr` with no surrounding pair found | Selection and buffer unchanged, `UserError` per the existing path |
| `,` / `Alt-,` with a single range | No-op, no error |
| Surround add/delete/replace undo granularity | One undo step per operation (see helix-core spec 3.8 FR-027) |

### 10.4 Out of Scope (this batch)

- Named macro registers (unchanged from section 1)
- `Alt-K`, `K`, `&` and backward search `?` were deferred here and are specified in section 11 (#510, #513)

### 10.5 Success Criteria

| ID | Metric | Target |
|----|--------|--------|
| SC-101 | A `BaseState` handler test sends Alt-modified keys for every pending-state-opening key | No pending state entered; Alt-s emits `split_selection_on_newline` |
| SC-102 | `cargo nextest run scenario` | Scenario 126 passes with solution `Alt-s` |
| SC-103 | Counted-command tests through the real input layer and `HelixSimulator` | `2G`, `3gg`, `3fa`, `2ti`, `2Fi` match the hx 25.07.1 outcomes in 10.2 |
| SC-104 | Selection assertions after `ms`, `md`, `mr`, `,`, `Alt-,` | Match hx 25.07.1 |

### 10.6 Agent Boundaries (additions)

- Always: land simulator acceptance of a new command string before the input layer starts emitting it (unknown strings are errors in the registry, and recoverable errors must not end the session, see [[constitution#V. Security]])
- Never: emulate a count by repeating the base command outside the simulator (FR-106)

## 11. Bug-Fix Batch: Backward Search, Keep/Remove Selections, Hint Key (#509, #510, #513)

> [!info] Added 2026-11-11 (spec-first update for the P2-P3 bug batch)
> Reference behavior is `hx` 25.07.1. FR numbers continue the scoped series of section 10. Product decision (#513): the hint key on the Task screen is `F1` only (the arcade has no hint), and `?` is released for backward search and for typing.

### 11.1 Requirements

| ID | Requirement | Issue | Priority |
|----|------------|-------|----------|
| FR-120 | THE HINT SHALL be opened and closed by `F1` only on the Task screen (the arcade has no hint feature, so `F1` is ignored there); no other key, and in particular not `?`, SHALL toggle it; the user-visible footer/help text SHALL read `Press F1 for hint` | #513, #509 | must |
| FR-121 | WHEN the user presses `?` in Normal mode THE SYSTEM SHALL enter a backward-search prompt (a backward variant of the search-prompt pending state of [[../register-command-mode-support/spec#12.1 Search prompt (`/`) (#504, `/` only)\|register spec 12.1]], same typestate pattern, same pattern-length cap and invalid-regex `UserError` handling); WHEN confirmed with a non-empty pattern THE SYSTEM SHALL select the previous match before the cursor (wrapping at the start of the buffer), execute it as one command carrying the pattern under the command id `?` (the shared constant), and count one action; `n`/`N` continue from the stored pattern with `hx` 25.07.1 semantics | #513 | must |
| FR-122 | WHEN a printable character `?` arrives in Insert mode, in any prompt (`/`, `?`, `s`, `S`, `K`, `Alt-K`, `:`) or as the argument character of a pending command (`f`, `t`, `F`, `T`, `r`, `m`) THE SYSTEM SHALL treat it as text or argument and SHALL NOT trigger any screen-level special key; special keys that are printable (`?`) SHALL be handled only while the input state machine is in its Base state, after Insert-mode and prompt states had the chance to consume the key; non-printable special keys (`F1`, `Ctrl-Q`) stay active in every input state; `Shift-/` is normalized to `?` | #509 | must |
| FR-123 | WHEN the user presses `K` (`keep_selections`) or `Alt-K` (`remove_selections`) in Normal mode with an active selection THE SYSTEM SHALL open a regex prompt (same typestate pattern, caps and error handling as `s`/`S`, FR-001..FR-004) and on confirm keep (`K`) or remove (`Alt-K`) the selection ranges whose text matches the pattern; when no range would remain `hx` behavior applies and is asserted by a differential test Bare `K`/`Alt-K` with no pattern SHALL return a `UserError`; the prompts emit the command strings `? <pat>`, `K <pat>` and `Alt-K <pat>` | #510 | must |
| FR-124 | WHEN the user presses `&` (`align_selections`) THE SYSTEM SHALL execute it immediately as one command and one action, without a prompt, with `hx` 25.07.1 alignment semantics | #510 | must |
| FR-125 | WHEN `K`/`Alt-K` completions are recorded for review THE SYSTEM SHALL normalize the typed-pattern forms to the skill ids `keep_selections` and `remove_selections` (as FR-009 does for `s`/`S`), and `? sum` to the command `?` (as `/ sum` to `/`, see [[../fsrs-proptest-coverage-gap/spec#11. Follow-Up Requirements: Review Queue Ordering, Eq/Ord Agreement, Reviewable Commands (#433, #434, #505)\|FSRS spec FR-015]]) | #510, #513 | must |

### 11.2 Acceptance Criteria

```
GIVEN a Training scenario
WHEN  the user presses i, ?, Escape
THEN  `?` is inserted into the buffer and the hint stays closed
```
```
GIVEN the Training screen
WHEN  the user presses F1
THEN  on the Task screen the hint toggles (on arcade screens F1 is ignored); pressing ? in Normal mode opens the backward-search prompt instead
```
```
GIVEN a buffer with two occurrences of `sum` and the cursor after the second
WHEN  the user presses ?, types sum, Enter
THEN  the second occurrence is selected as one action; Escape instead discards the prompt with no action
```
```
GIVEN three selection ranges, two of which contain `a`
WHEN  K, a, Enter are pressed  THEN only the two ranges containing `a` remain
WHEN  Alt-K, a, Enter are pressed  THEN only the range without `a` remains
```

### 11.3 Edge Cases

| Scenario | Expected Behavior |
|----------|-------------------|
| `?` typed inside the `/` prompt | Appended to the pattern (regex `?` quantifier) |
| `?` pressed with a pending count prefix (`3?`) | Count is handled as in `hx` for `/` (verified by differential test); never opens the hint |
| `K`/`Alt-K` with an invalid regex or no matching range | `UserError` per FR-204 style handling or the `hx` outcome; selection unchanged on error, no panic |
| Scenarios and docs that mention `?` as the hint key | Updated to `F1` in the same change (`hint_key` locale string, README and key reference) |

### 11.4 Success Criteria

| ID | Metric | Target |
|----|--------|--------|
| SC-111 | Input-layer tests: `?` in Insert, in each prompt and as an argument is text; `F1` is the only hint key (Task screen; ignored in the arcade) | Pass |
| SC-112 | Simulator tests for `?`, `K`, `Alt-K`, `&` assert buffer and selection against `hx` 25.07.1 | Pass |

## 12. See Also

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
