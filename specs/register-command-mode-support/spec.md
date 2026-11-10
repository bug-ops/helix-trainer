---
aliases:
  - Named Registers and Command-Line Mode
  - Register + Goto-Mode Support
tags:
  - sdd
  - spec
  - helix-simulator
  - input-system
  - status/implemented
created: 2026-08-08
status: implemented
related:
  - "[[constitution]]"
  - "[[BRD]]"
  - "[[SRS]]"
  - "[[NFR]]"
  - "[[plan]]"
  - "[[tasks]]"
  - "[[MOC-specs]]"
---

# Feature: Named Register and Command-Line (`:`) Mode Support

> [!info] Metadata
> **Author**: rust-agents:sdd (research spec, from competitive-parity finding)
> **Resolved by**: commit `1ba668d` — "feat(helix-core): add named registers
> and command-line goto mode (#329)", closing issues #282 (implementation)
> **Status**: Implemented, narrower scope than drafted — see [[SRS]] for the
> FR-by-FR verdict. This document is retroactive: preserved below as the
> original research spec, with a Resolution section appended describing
> what actually shipped.

## 1. Overview

### Problem Statement

A competitive-parity research scan compared helix-trainer against reference
Vim/Helix training projects, including
[S-Sigdel/vimhjkl](https://github.com/S-Sigdel/vimhjkl) (524 stars, actively
maintained, terminal-based spaced-repetition Vim trainer, 66 skills / 230
challenges). vimhjkl's curriculum explicitly drills "registers, marks,
macros, `:g` / `:normal` / ranges, regex and substitution" as core skill
categories.

Cross-checking against helix-trainer's own simulator (`src/helix/`) found
two real, documented [Helix keymap](https://docs.helix-editor.com/keymap.html)
features with zero simulator support and zero scenario coverage:

1. **Named registers** — `"` `<reg>` = `select_register` (select a register
   to yank to or paste from). `HelixSimulator<M>` had only a single
   `clipboard: Option<String>` field, no register map keyed by name.
2. **Command-line / ex mode (`:`)** — documented under *Minor modes*,
   supporting `:s` substitute, `:g` global, ranges, `:w`, `:sort`, etc.
   `EditorMode` defined only `NormalMode`/`InsertMode` — no command-line
   mode existed at all.

**Why it matters**: named registers (multi-slot yank/paste) and command-line
commands are among the most commonly used Helix command categories in daily
editing. Their total absence meant helix-trainer taught a strict subset of
Helix's real command surface compared to reference projects drilling the
equivalent Vim concepts.

### Goal

Define named-register support (yank to / paste from a register selected via
`"<reg>`) and a scoped command-line mode as first-class additions to
`HelixSimulator` and the typestate input system, with at least one new
scenario category demonstrating each.

### Out of Scope

> [!danger] Explicit Exclusion — Vim-Style Marks
> Vim-style marks (backtick/apostrophe jump-to-mark) are **explicitly and
> permanently out of scope**. helix-trainer already investigated and
> rejected this in closed issue #104: "Original plan had 'marks (m, '')'
> which is a VIM feature, NOT Helix! In Helix, `m` enters Match Mode for
> brackets/surround/textobjects." This decision is correct and final.

Additional exclusions in the original draft (some became permanent, not
just deferred — see Resolution below):

- Full command-line surface (`:g` global, `:w` write, `:sort`, arbitrary
  line ranges, shell commands, `:normal`)
- Register content inspection UI
- Macro recording/playback — was tracked separately under issue #198;
  **now implemented**, see [[../regex-selection-and-macro-commands/spec|Regex
  Selection and Macro Commands]] and section 11 below
- Scroll commands, view mode extensions, selection-regex — was tracked
  under issue #198; **scroll commands and select-all/replace now have
  scenario coverage** (`d66278d`), **selection-regex now implemented**,
  see [[../regex-selection-and-macro-commands/spec|Regex Selection and
  Macro Commands]] and section 11 below

## 2. User Stories

### US-001: Yank and paste using a named register
AS A helix-trainer user practicing intermediate/advanced Helix workflows
I WANT to select a named register with `"<reg>` before a yank or paste
operation
SO THAT I can hold multiple pieces of text simultaneously and learn Helix's
real multi-register workflow.

**Status**: satisfied as drafted, plus `R` (replace-with-yanked) additionally
scoped to registers. See [[SRS#FR-001]]..[[SRS#FR-004]].

### US-002: Substitute text via command-line mode
AS A helix-trainer user practicing intermediate/advanced Helix workflows
I WANT to enter command-line mode with `:` and run a substitute command
SO THAT I can learn Helix's search-and-replace workflow.

**Status**: **not satisfied as drafted.** No `:s/pattern/replacement/`
substitute was implemented. What shipped instead is `:goto N` / `:g N`
(line-number navigation) — see [[BRD#What Was Implemented]] and
[[SRS#FR-006]].

### US-003: Scenario coverage for both new capabilities
AS A helix-trainer content maintainer
I WANT dedicated scenario categories exercising named registers and
command-line mode
SO THAT the FSRS scheduler can teach and drill these commands.

**Status**: satisfied at minimum viable coverage (1 scenario per category,
not the ≥3-per-category-spanning-difficulty originally targeted by SC-003).

## 3. Functional Requirements

Use EARS notation. Prefix with FR-NNN. See [[SRS]] for the actual verdict of
each.

| ID | Requirement | Priority |
|----|------------|----------|
| FR-001 | WHEN the user presses `"` followed by a register name character in Normal mode THE SYSTEM SHALL enter a register-selection pending state analogous to `GotoPending`/`ViewPending`/`MatchPending` | must |
| FR-002 | WHEN a register has been selected via `"<reg>` and the user subsequently presses `y` THE SYSTEM SHALL yank the current selection into that named register | must |
| FR-003 | WHEN a register has been selected via `"<reg>` and the user subsequently presses `p` or `P` THE SYSTEM SHALL paste the content of that named register after/before the cursor respectively | must |
| FR-004 | WHEN no register is explicitly selected THE SYSTEM SHALL preserve existing default-clipboard yank/paste behavior unchanged | must |
| FR-005 | WHEN the user presses `:` in Normal mode THE SYSTEM SHALL enter a new command-line/prompt mode distinct from `NormalMode` and `InsertMode` | must |
| FR-006 | WHEN the system is in command-line mode and the user types a valid `:s/pattern/replacement/` expression and confirms (Enter) THE SYSTEM SHALL apply the substitution and return to Normal mode | must |
| FR-007 | WHEN the system is in command-line mode and the user presses Escape THE SYSTEM SHALL discard the pending input and return to Normal mode without modifying the document | must |
| FR-008 | WHEN the system is in command-line mode and the user types an unrecognized or malformed command THE SYSTEM SHALL surface a `UserError` rather than panicking or silently no-op-ing | must |
| FR-009 | WHERE new scenario categories are added THE SYSTEM SHALL validate them via `cargo nextest run scenario` with no changes to the validation harness itself | should |
| FR-010 | WHEN a scenario's `metadata.commands_taught` references a register-select or command-line command THE SYSTEM SHALL recognize it as a valid taught command for FSRS tracking | should |

## 4. Non-Functional Requirements

| ID | Category | Requirement |
|----|----------|-------------|
| NFR-001 | Architectural consistency | New register-selection and command-line states MUST use the existing typestate pattern in `src/input/typestate/` |
| NFR-002 | Architectural consistency | Command-line mode MUST be added to `EditorMode` (`src/helix/simulator/mode.rs`) following the sealed-trait typestate pattern |
| NFR-003 | Scope discipline | Vim-style marks MUST NOT be introduced under any name — see issue #104 |
| NFR-004 | Safety | `#![forbid(unsafe_code)]` MUST be preserved |
| NFR-005 | Testability | Both capabilities MUST be covered by scenario TOML fixtures plus unit tests |
| NFR-006 | Error handling | All new user-facing failure modes MUST route through `UserError`/`SecurityError` |
| NFR-007 | Documentation | All new `pub` types/functions MUST carry `///` doc comments |

## 5. Data Model

Described at the WHAT level in the original draft; see [[plan]] for the
concrete as-built types.

| Entity | Description |
|--------|-------------|
| Named Register | A named storage slot for yanked/deleted text, addressable via `"<reg>` |
| Register Selection (pending input state) | Transient typestate marker capturing that the next operation targets a specific register |
| Command-Line Session | Transient state representing an open `:` prompt |
| Substitute Command | A parsed `:s/pattern/replacement/[flags]` expression — **not implemented, see Resolution** |

## 6. Edge Cases and Error Handling

| Scenario | Expected Behavior | Actual (as shipped) |
|----------|-------------------|----------------------|
| User presses `"` then an invalid register-name character | `UserError`, return to `BaseState` | Matches — see [[SRS#FR-001]] |
| User presses `"<reg>` then a key that is neither `y`, `p`, nor `P` | `[NEEDS CLARIFICATION]` in draft | Resolved: also allows `R`; anything else cancels the pending register state |
| User enters command-line mode via `:` then presses `:` again mid-input | Treated as literal input unless a recognized control key | Not applicable — command-line only accepts digits for `:goto`; non-digit input is rejected as malformed |
| `:s/pattern/replacement/` with invalid regex | `UserError` describing regex failure | Not applicable — `:s` was never implemented |
| `:s` invoked with no active selection/range | `[NEEDS CLARIFICATION]` in draft | Moot — `:s` was never implemented |
| Named register read via `"<reg>p` before anything yanked to it | No-op, no panic, mirrors default clipboard | Matches |
| Scenario TOML references an unrecognized register/command-line command | `cargo nextest run scenario` MUST fail loudly | Matches — validation harness unchanged, still fails loudly on unmapped commands |

## 7. Success Criteria

| ID | Metric | Target | Actual |
|----|--------|--------|--------|
| SC-001 | Named-register yank/paste round-trip correctness | 100% pass, no regression | Met |
| SC-002 | Command-line `:s` substitute correctness | 100% pass for `:s/pattern/replacement/` | **Not met — `:s` never implemented; `:goto`/`:g` implemented instead, with its own passing tests** |
| SC-003 | ≥1 scenario category per capability, ≥3 scenarios/category spanning difficulty | ≥3 per category | **Partially met — 1 category + 1 scenario per capability, single difficulty level** |
| SC-004 | No architectural drift — new states follow the typestate pattern | Code review confirms | Met for register states; **not met for command-line, which lives in input-typestate only, not `EditorMode`** — see [[SRS#FR-005]] |
| SC-005 | No scope creep into vim marks | Code review confirms | Met |

## 8. Agent Boundaries

### Always (without asking)
- Run `cargo nextest run --workspace --all-features --lib --bins` and
  `cargo nextest run scenario` after any implementation change here
- Follow the existing typestate pattern in `src/input/typestate/`
- Add doc comments (`///`) to all new `pub` items

### Ask First
- Implementing `:s` substitute or any command-line command beyond
  `:goto`/`:g` (see [[BRD#What Was Implemented]] — this is now a genuinely
  new, unscoped follow-up, not part of what shipped)
- Any change to the existing default/unnamed clipboard behavior

### Never
- Introduce vim-style marks under any name — permanently rejected per issue #104
- Silently swallow malformed command-line input or invalid register names

## 9. Open Questions

All five `[NEEDS CLARIFICATION]` items from the original draft were
implicitly resolved by the implementation — see [[SRS#5.1 Traceability Matrix]]
for how. One new question is introduced by the resolution:

- [NEEDS CLARIFICATION: should `:s` substitute (and `:g` global-with-subcommand,
  as distinct from the `:g N` goto alias that shipped) be scoped as its own
  follow-up feature, given the original US-002/FR-006 were not satisfied?
  `docs/HELIX_KEYBINDINGS.md` already documents this narrowing explicitly to
  users — see [[plan#Documented Scope Narrowing]].]

## 10. Resolution (Retroactive)

Implemented by commit `1ba668d` (2026-08-09), closing issue #282. Named
registers shipped essentially as drafted (FR-001..004), plus `R` support.
Command-line mode shipped **narrower than drafted**: only `:goto N` (line
navigation, alias `:g N`) exists — no `:s` substitute, no `:g` global
command, no ranges. This was an explicit, documented scoping decision (see
`docs/HELIX_KEYBINDINGS.md`'s scope disclaimer), not an oversight. Full
verdict in [[SRS]].

## 11. Post-Release Extensions (v0.6.0)

Three follow-up commits, still within the register/command-mode feature
area, shipped after the retroactive documentation above was written:

- **`d66278d`** (closes issue #198) — added scenario coverage for `Ctrl-b`/
  `Ctrl-f`/`Ctrl-u`/`Ctrl-d` (half-page/page scroll), `%` (select-all), and
  `R` (replace-selection-with-yanked), plus a bug fix recognizing
  Ctrl-modified keys that previously fell through to their unmodified
  bare-key command. Explicitly deferred `q`/`Q` (macros — no
  record/replay logic existed) and `s`/`S` (selection-regex — no-op
  stubs) as "from-scratch feature work, not scenario content."
- **`a4efc2e`** — implemented exactly what `d66278d` deferred: `s`/`S`
  (regex select/split) and `q`/`Q` (macro record/replay). Fully documented
  in its own package, [[../regex-selection-and-macro-commands/spec|Regex
  Selection and Macro Commands]]. This closes issue #198's remaining
  scope; the "still-open content-coverage gap" language in this package's
  original research spec (section 2, US-003 note; section 10 References)
  is now stale and superseded by that package.
- **`fb2e0c5`**, **`dd39300`**, **`efeac9d`** — a trio of register-scoped
  Helix-simulator correctness fixes discovered via kitty-keyboard-protocol
  reachability testing of `Alt-c`/`Alt-d`:
  - `fb2e0c5` registered `change_selection_noyank` for `Alt-c`, initially
    reusing the plain `change_selection` handler (at that point the
    simulator never wrote registers on change/delete, so yank vs. noyank
    were behaviorally identical).
  - `dd39300` split the two apart: `change_selection` (`c`, `d`) now
    deletes the full active selection and writes the deleted text to the
    default register via `yank_to_register` (folding over all selection
    ranges, not just the primary one); `change_selection_noyank` (`Alt-c`)
    keeps the deletion but does not populate the register.
  - `efeac9d` added **blackhole register** (`"_`) support — register-scoped
    delete/change operations targeting `"_` discard text instead of
    overwriting a register, matching real Helix — plus the missing `Alt-d`
    (`delete_selection_noyank`) binding mirroring `Alt-c`, and a fix for
    command-repeat (`.`) silently dropping `"<reg>d`/`"<reg>c`
    register-scoped operations (a gap only reachable once `d`/`c` became
    routable through the register-selection input layer).

These extend FR-002/FR-003 (register-scoped yank on `y`, paste on `p`/`P`)
to also cover register-scoped delete/change (`d`/`c`), and add the
blackhole register as a new addressable register alongside named ones.

## 12. Bug-Fix Batch: Search Prompt, Linewise and Multi-Range Paste, Insert-Mode Keys (#504, #449, #455, #489)

> [!info] Added 2026-11-10 (spec-first update for the P1-P3 bug batch)
> Reference behavior for every criterion is `hx` 25.07.1. Tests MUST assert the resulting selection and cursor as well as buffer text. FR numbers in this section (FR-201 and up) are scoped here; FR-001..FR-010 above are unchanged.

> [!danger] Explicitly deferred
> - Backward search `?` (#504, second half): `?` is the hint key on the Task and MiniGame screens, so it cannot also search without a product decision. Not part of this batch.
> - Insert-mode `Tab` (indent) from #489: needs a per-language indent unit, which the simulator does not have. Not part of this batch.

### 12.1 Search prompt (`/`) (#504, `/` only)

| ID | Requirement | Priority |
|----|------------|----------|
| FR-201 | WHEN the user presses `/` in Normal mode THE SYSTEM SHALL enter a search-prompt pending state (typestate, same pattern as the `s`/`S` regex prompt and the `:` prompt, per [[constitution#I. Architecture]]) and show the typed pattern in the footer; `/` SHALL NOT be recorded as an action on its own and typed characters SHALL NOT be executed as commands | must |
| FR-202 | WHEN the user presses Enter in the search prompt with a non-empty pattern THE SYSTEM SHALL select the next match after the cursor (wrapping at the end of the buffer), execute it as one command carrying the pattern, and increment the action counter once | must |
| FR-203 | WHEN the user presses Escape in the search prompt THE SYSTEM SHALL discard the pattern, leave selection and action counter unchanged, and return to Normal mode | must |
| FR-204 | WHEN the pattern is not a valid regex (checked with the same `helix-stdx` regex engine as `s`/`S`) THE SYSTEM SHALL surface a `UserError`, leave the selection unchanged and return to Normal mode | must |
| FR-205 | WHEN a search completes THE SYSTEM SHALL store the pattern so `n` and `N` continue from it (same state `*` uses) | must |
| FR-206 | WHEN the pattern reaches the maximum pattern length used by the regex prompt THE SYSTEM SHALL reject further input rather than grow unbounded | must |
| FR-207 | WHEN an empty pattern is confirmed THE SYSTEM SHALL return to Normal mode without a selection change and without an action | should |

### 12.2 Linewise and multi-range paste (#449, #455)

| ID | Requirement | Priority |
|----|------------|----------|
| FR-210 | WHEN a register value ENDS WITH a line ending THE SYSTEM SHALL paste it linewise: `p` inserts at the start of the line after the range's last line, `P` at the start of the line of the range's first line (upstream `paste_impl`: linewise = the value ends with a line ending; a multi-line value without a trailing line ending is pasted inline) | must |
| FR-211 | WHEN a linewise paste targets the last line of a buffer with no trailing newline THE SYSTEM SHALL insert the line separator needed so the pasted line does not merge into the last line | must |
| FR-212 | WHEN a register value does not end with a line ending (including a multi-line value without a trailing line ending) THE SYSTEM SHALL paste inline exactly as before this batch | must |
| FR-213 | WHEN a yank runs over several selection ranges THE SYSTEM SHALL store one value per range in the target register (the register value type changes from a single `String` to an ordered list of strings; internal change, no persisted format) | must |
| FR-214 | WHEN `p`/`P` run with N ranges and M stored values THE SYSTEM SHALL paste value i at range i, repeating the last value when M < N and ignoring surplus values when M > N | must |
| FR-214a | `R` (replace with yanked) with several stored values pasting value i into range i is NOT part of this batch (explicitly deferred; needs `RegisterFile::get_values` consumption in `R`); `R` keeps using the first value (FR-215) | deferred |
| FR-215 | WHEN a register holding several values is read by an operation expecting one string THE SYSTEM SHALL use the first value | should |
| FR-216 | WHEN text is pasted THE SYSTEM SHALL select the pasted text in every range (see [[../helix-core-post-25-07-1-upgrade-readiness/spec#3.8 Simulator Fidelity at the Pinned Core (25.07.1)\|section 3.8 FR-024]]) | must |

### 12.3 Insert-mode editing keys (#489)

| ID | Requirement | Priority |
|----|------------|----------|
| FR-220 | WHILE in Insert mode THE SYSTEM SHALL execute `Ctrl-w` (delete word backward), `Alt-Backspace` (delete word backward), `Ctrl-u` (delete to line start), `Ctrl-k` (delete to line end), `Ctrl-d` and `Delete` (delete char forward; at end of line joins the next line), and `Alt-d` (delete word forward) with `hx` 25.07.1 semantics | must |
| FR-221 | WHILE in Insert mode THE SYSTEM SHALL NEVER insert the bare letter of a Ctrl- or Alt-modified key; a modified key with no Insert-mode binding is ignored (no text, no action) | must |
| FR-222 | THE INPUT LAYER SHALL emit these keys as canonical command strings (`Ctrl-w`, `Ctrl-u`, `Ctrl-k`, `Ctrl-d`, `Alt-d`, `Alt-Backspace`, `Delete`) and the simulator SHALL model them as a closed typed set of insert commands rather than treating any unknown string as text | must |
| FR-223 | WHEN an Insert-mode session containing these keys is repeated with `.` THE SYSTEM SHALL replay them like other insert commands | must |

### 12.4 Acceptance Criteria

```
GIVEN the text of scenario 14, cursor at (0,0)
WHEN  the user presses /, types sum, presses Enter, then d
THEN  the first `sum` after the cursor (line 2) is selected and removed
 AND  the action counter is 2 (search, delete)
```
```
GIVEN the search prompt is open with pattern `a(`
WHEN  the user presses Enter
THEN  a UserError is shown, the selection is unchanged, no prompt is pending
```
```
GIVEN scenario 14, the user pressed x (line 1), y, j
WHEN  the user presses p
THEN  `fn add(a: i32, b: i32) -> i32 {` is inserted as a new line between lines 2 and 3
```
```
GIVEN scenario 14, after A (append at end of line 1)
WHEN  the user presses Ctrl-w
THEN  the trailing `{` is removed (not the letter `w` inserted)
```

### 12.5 Edge Cases

| Scenario | Expected Behavior |
|----------|-------------------|
| `/` pressed while a count is pending | [NEEDS CLARIFICATION: hx ignores the count for search; this batch ignores it too, confirm] |
| Search with no match | Selection unchanged, notification shown, no panic |
| `"_` (blackhole) yank over several ranges | Stores nothing, as before |
| `Ctrl-w` at line start in Insert mode | Deletes the preceding newline as hx does |
| `Delete` at the very end of the buffer | No-op |
| Register value `"a\nb"` (multi-line, no trailing line ending) | Pasted inline, not linewise |
| Register value `"a\n"` (ends with a line ending) | Pasted linewise |

### 12.6 Success Criteria

| ID | Metric | Target |
|----|--------|--------|
| SC-201 | Typestate unit tests for the search prompt | Enter, Escape, invalid regex, empty pattern, over-length pattern covered |
| SC-202 | Simulator tests for linewise `p`/`P`, last-line paste, multi-range yank/paste | Match hx 25.07.1 |
| SC-203 | Insert-mode tests for each key in FR-220 plus `.` repeat | Match hx 25.07.1 |

## 13. See Also

- [[constitution]] — project principles
- [[BRD]] — business rationale and what actually shipped
- [[SRS]] — FR-by-FR verdict
- [[NFR]] — non-functional requirement verdicts
- [[plan]] — as-built architecture
- [[tasks]] — retroactive task breakdown
- [[../regex-selection-and-macro-commands/spec|Regex Selection and Macro
  Commands]] — resolves this package's deferred macro/selection-regex
  scope (section 11)
- [[MOC-specs]] — all specifications
- [Helix Keymap Reference](https://docs.helix-editor.com/keymap.html)
- [Helix Command-Line Reference](https://docs.helix-editor.com/command-line.html)
- [S-Sigdel/vimhjkl](https://github.com/S-Sigdel/vimhjkl) — reference project
- Issue #198 — Helix command content-coverage gap — **closed**, see
  section 11 (macros, scroll, selection-regex all landed)
- Issue #104 — vim-style marks explicitly rejected (authoritative prior art)
