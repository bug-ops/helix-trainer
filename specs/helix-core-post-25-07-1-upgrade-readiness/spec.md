---
aliases:
  - Helix Core Upgrade Readiness
  - Post-25.07.1 Upgrade Readiness
tags:
  - sdd
  - spec
  - research
  - helix-core
  - dependencies
  - status/proposed
created: 2026-11-10
status: proposed
related:
  - "[[constitution]]"
  - "[[custom-helix-keymap-support/spec]]"
  - "[[MOC-specs]]"
---

# Feature: Helix Core Upgrade Readiness (Post-25.07.1)

> [!info] Metadata
> **Source**: research CI cycle finding (priority P4, parity with upstream)
> **Status**: Proposed for the upgrade checklist (sections 3.1-3.7, not yet built). Section 3.8 (simulator fidelity at the pinned core) is specified for implementation in the P1-P3 bug batch (#487 and related issues).
> **Depth**: Lightweight spec per this project's SDD scaling guidance. It is a readiness checklist and decision record, not a feature. The upgrade itself is a non-goal. Section 3.8 is the exception: it specifies simulator fixes at the current pin.

> [!abstract]
> helix-trainer pins `helix-core` and `helix-stdx` to upstream tag `25.07.1` (released 2025-07-18). As of 2026-11-10 upstream `master` is 991 commits ahead with no newer tag. This spec records the known breaking and behavioral drift, and the criteria that must hold before the pin moves, so the future bump is mechanical rather than exploratory.

## 1. Overview

### Problem Statement

The simulator wraps real `helix-core` ([[constitution]] Section I). The project promises that scenarios teach genuine Helix behavior, so drift between the pinned core and the current upstream editor makes some canonical solutions and taught keybindings stale. Upstream has accumulated roughly 16 months of changes, including at least one compile-breaking API change affecting this project. When a new tag is published, the bump will land on top of that backlog unless it is understood in advance.

Facts below were verified on 2026-11-10 against the upstream compare of `25.07.1...master` (master HEAD `57d86412`). Everything is on unreleased master, so details may change before a tag is cut.

### Goal

When a new Helix tag is published, a maintainer can execute the bump by working through a pre-agreed checklist: known API breaks fixed, scenario solutions re-validated against the bumped core, keymap and command renames reconciled, new-command scenario candidates triaged, and the pin strategy chosen by an explicit rule.

### Out of Scope

- Performing the bump now, or changing `Cargo.toml` or `Cargo.lock`. (Source changes described in section 3.8 are in scope; they fix simulator parity at the current pin.)
- Writing the new scenarios (each accepted candidate gets its own spec/issue per the spec-first workflow).
- Tracking upstream master continuously.
- Upgrading the MSRV (handled by the normal dependency flow, though a core bump may force it; see Open Questions).

## 2. User Stories

### US-001: Maintainer executes the bump from a checklist
AS A maintainer
I WANT a list of known breaks, behavior changes and validation steps
SO THAT the bump is a bounded task and not an open-ended investigation.

**Acceptance criteria:**
```
GIVEN a new Helix tag has been published
WHEN the maintainer follows this spec's checklist (Section 3)
THEN the project builds against the new tag, all scenario tests pass,
     and every deviation is either fixed or recorded in an issue
```

### US-002: Learner is never taught stale behavior
AS A learner
I WANT scenario solutions and hints to match the Helix version I install today
SO THAT drilled muscle memory transfers to my real editor.

**Acceptance criteria:**
```
GIVEN the pin has moved to a new core version
WHEN scenarios run in the simulator
THEN each canonical solution reaches its target state under the new core,
     and no hint describes a binding that upstream renamed or removed
```

### US-003: Custom keymap users keep working remaps
AS A user with `use_helix_keymap` enabled
I WANT my `config.toml` remaps to keep resolving after the bump
SO THAT the upgrade does not silently drop my bindings.

**Acceptance criteria:**
```
GIVEN a user config that remaps commands by name
WHEN the trainer runs against the new core
THEN renamed upstream commands either resolve via an alias or produce
     a clear user-facing error naming the old and new command
```

## 3. Functional Requirements

Requirements are on the upgrade process and its outcome. "Readiness" means the checklist is complete.

### 3.1 Trigger and pin strategy

| ID | Requirement | Priority |
|----|------------|----------|
| FR-001 | WHEN a new tag is published in `helix-editor/helix` newer than `25.07.1` THE MAINTAINER SHALL open an upgrade issue referencing this spec and start the checklist | must |
| FR-002 | WHEN choosing between a tag pin and a git `rev` pin THE MAINTAINER SHALL apply the decision rule in Section 3.2 and record the choice and reason in the upgrade issue | must |
| FR-003 | WHEN the pin changes THE MAINTAINER SHALL update every place that states the pinned version: `Cargo.toml` (`helix-core` and `helix-stdx` kept on the same tag/rev), [[constitution]] Section I, and project docs and architecture notes that cite `25.07.1` | must |

### 3.2 Decision rule: tag vs `rev` pin

| Condition | Decision |
|-----------|----------|
| A tag newer than `25.07.1` exists and its release notes contain no regression affecting simulated commands | Pin the tag (default; matches current practice and the "teach what users have installed" goal) |
| No new tag, and nothing in the known-break list blocks a needed feature or fix | Stay on `25.07.1` |
| No new tag, and a specific required fix exists only on master (tracked by an issue) | Pin a git `rev` of a master commit, only with a named issue justifying it and an expiry condition ("move to the next tag") |
| Maintainers consider a `rev` pin for convenience or for new scenarios alone | Reject: scenarios must teach behavior users can install from a release |

A `rev` pin MUST NOT be used for features that are not in any released Helix, because learners would be drilled on commands their installed editor lacks. `[NEEDS CLARIFICATION: should distro-packaged Helix versions (often older than the latest tag) influence which tag is the minimum supported teaching target?]`

### 3.3 API-break remediation list

| ID | Requirement | Priority |
|----|------------|----------|
| FR-004 | WHEN bumping THE PROJECT SHALL adapt both call sites of `helix_core::surround::find_nth_pairs_pos` (`src/helix/simulator/commands/editing.rs` and `src/helix/mod.rs`) to the new leading `syntax: Option<&Syntax>` parameter | must |
| FR-005 | WHEN adapting FR-004 THE PROJECT SHALL pass `None` unless a document syntax tree is already available, keeping behavior equivalent to 25.07.1 for plain-text scenarios | should |
| FR-006 | WHEN bumping THE PROJECT SHALL re-run a compile against the new core and record any further API breaks in the upgrade issue before fixing them | must |
| FR-007 | WHEN a function used by this project changed signature THE PROJECT SHALL NOT keep a compatibility shim for the old signature (pre-1.0 policy: no backward compatibility) | must |

Known facts:

- `surround::find_nth_pairs_pos` gains a new first parameter `syntax: Option<&Syntax>`. This project calls it at the two sites above.
- `movement::goto_treesitter_object` lost a parameter. It is not used by this project; no action unless a later feature adopts it.
- `[NEEDS CLARIFICATION: full list of other changed or removed helix-core/helix-stdx items this project imports; a compile against the new tag is the authoritative check, not the diff summary]`

### 3.4 Scenario-solution re-validation

| ID | Requirement | Priority |
|----|------------|----------|
| FR-008 | WHEN the pin changes THE PROJECT SHALL run `cargo nextest run scenario` and the full suite (`cargo nextest run --workspace --all-features --lib --bins`) against the bumped core | must |
| FR-009 | WHEN a scenario fails after the bump THE PROJECT SHALL classify it as (a) upstream behavior change, so the scenario's setup, target or solution is updated to the new behavior, or (b) simulator regression, so a defect is filed and fixed in the simulator | must |
| FR-010 | WHEN a scenario is updated under classification (a) THE PROJECT SHALL confirm the new behavior against the corresponding released upstream editor, not only against the simulator | must |
| FR-011 | WHEN the bump lands THE PROJECT SHALL review the scenarios in each at-risk area in Section 3.5 even if tests pass, because tests may assert only the final buffer state and not a changed selection | should |

### 3.5 Behavior changes to re-validate

These are changes in `helix-core` between `25.07.1` and master that can alter outcomes of existing solutions. Each needs a targeted review of the scenarios whose `commands_taught` or solution exercises it.

| # | Upstream change | Scenarios at risk |
|---|-----------------|-------------------|
| 1 | Surround with identical open/close characters (`"`, `'`, `` ` ``): when the cursor is on the quote, the matching pair is now resolved instead of returning `CursorOnAmbiguousPair` | Surround delete/replace/select scenarios on quotes, especially cursor-on-quote setups and any test asserting the old failure |
| 2 | `to_camel_case` now lowercases the first character and the rest of each word (e.g. `OTTO_BOTTO` becomes `ottoBotto`; previously inner case of mixed-case input was kept) | Case-conversion scenarios using camel-case helpers; mixed-case or SCREAMING_CASE input |
| 3 | `match_brackets`: `\|` added to PAIRS (Rust closure params); pair lookup now requires the cursor within the pair range | `m m` bracket-match scenarios, particularly with `\|` in the buffer or the cursor adjacent to, not inside, a pair |
| 4 | `selection.rs` (+62 lines), `transaction.rs` (+161/-35), `indent.rs` (+507/-319, indent detection weights changed) | Selection-manipulation scenarios; auto-indent/`o`/`O`/`=`-style scenarios; any scenario relying on detected indent width |

`[NEEDS CLARIFICATION: which existing scenario files exercise items 1-4? Enumerate by grepping solutions and commands_taught at bump time; this spec does not list file names because the scenario set changes.]`

### 3.6 New-command scenario candidates

Candidates come from default keymap and typable-command changes on master. None is committed to the scenario catalog by this spec. Each accepted candidate needs its own spec/issue, and MUST NOT be added until it exists in a released tag the project pins. Rows are on unreleased master.

| Candidate | Specific upstream change | Teachable in simulator? |
|-----------|-------------------------|-------------------------|
| XML element motions | `]x` / `[x` motions; XML/HTML/JSX textobjects (upstream #11158) | `[NEEDS CLARIFICATION: needs tree-sitter syntax; the simulator uses no syntax trees today, and the new `find_nth_pairs_pos` `syntax` parameter suggests more tree-sitter coupling]` |
| Goto declaration | new `gD` goto_declaration | Likely LSP-only; probably not simulable (no LSP in the trainer) |
| Goto line / file start docs | docs now describe `<n>G` / `<n>gg` as goto_line and `<n>g` / `g` split for goto_file_start | Check whether existing goto-line coverage already matches; possible hint/doc fix instead of a new scenario |
| Jumplist saving | goto next/prev and goto_word now save to the jumplist | Scenarios using `Ctrl-o` / `Ctrl-i` after those motions would change; check jumplist support in the simulator |
| Window mode additions | `t` transpose_view, `ns` / `nv` new splits, goto_file_hsplit/vsplit | Out of scope: windows and splits are not simulated |
| Insert-mode Tab behavior | `Tab` smart_tab, `Shift-Tab` insert_tab | Candidate: indentation-in-insert-mode scenario, if the simulator exposes smart_tab behavior |
| Suspend | normal-mode `Ctrl-z` suspend | Not teachable (process-level) |
| Symbol pickers | `space s` / `S` now `lsp_or_syntax_*_symbol_picker` | Picker UI is not simulated |
| `:set-register` | new typable command | Candidate: aligns with the existing named-register and command-line support, which today covers only `:goto` / `:g` ([[register-command-mode-support/spec]]) |

### 3.7 Keymap and command rename handling (`use_helix_keymap`)

| ID | Requirement | Priority |
|----|------------|----------|
| FR-012 | WHEN the pin changes THE PROJECT SHALL diff the command identifiers used as resolution keys (`CommandMetadata.name`, see [[custom-helix-keymap-support/spec]]) against the new upstream command list and report additions, removals and renames | must |
| FR-013 | WHEN an upstream command was renamed THE PROJECT SHALL accept the new name as canonical and SHALL accept the old name as an alias only if it is still valid in the user's installed Helix generation `[NEEDS CLARIFICATION: how to decide which Helix versions users run; likely the old name is accepted with a deprecation note until the next minor release]` | should |
| FR-014 | WHEN a user's config names a command absent in the pinned core THE SYSTEM SHALL report the unresolved command name through the existing user-facing error path (`src/security.rs`) and keep the remaining remaps working | must |
| FR-015 | WHEN stock default bindings change upstream THE PROJECT SHALL update the stock keymap table and the scenario hints that quote those bindings | must |

Known keymap changes on master relevant here:

- `space E` was renamed to `space .` (file explorer in the current buffer's directory). The explorer is not simulated, so this affects at most hint text and any binding-name table.
- `space s` / `S` now map to `lsp_or_syntax_*_symbol_picker`; names resolved by `use_helix_keymap` must tolerate the new identifiers.
- Insert-mode `Tab` / `Shift-Tab` and normal-mode `Ctrl-z` have new defaults; verify the stock keymap does not shadow them wrongly.

### 3.8 Simulator Fidelity at the Pinned Core (25.07.1)

> [!info] Added 2026-11-10 (spec-first update for the P1-P3 bug batch)
> Differential runs of the trainer against `hx` 25.07.1 (the exact version vendored as `helix-core`) found commands whose simulated result differs from the pinned core. These are defects at the CURRENT pin, independent of any future bump, and are in scope of the fixes below (they do not change `Cargo.toml`). The reference for every requirement is the behavior of `hx` 25.07.1 on the same text and keys; tests MUST assert the resulting selection as well as buffer text. Issue #500 (language-aware indent unit for `>`/`<`) and the follow-ups #510, #514, #517, #519, #520 and #524 (found while fixing the first set) were added to the batch on 2026-11-11 (FR-028..FR-035 and the FR-020 amendment). Tree-sitter extra indent was deferred at first; on 2026-11-11 the batch of #534, #535, #521 and #516 (FR-036..FR-040 below) was added, with #516 specified as a per-language rule without tree-sitter (see 3.8.4).

| ID | Requirement | Issue | Priority |
|----|------------|-------|----------|
| FR-016 | WHEN `x` runs THE SIMULATOR SHALL select the current line as a forward range (anchor at line start, head at the end of the line including its line ending); WHEN `x` runs on an existing selection that already covers whole lines it SHALL extend by one line; `{n}x` SHALL select n lines; `%` then `x` SHALL keep the whole buffer selected. `X` SHALL follow hx `extend_to_line_bounds` | #495 | must |
| FR-017 | WHEN a vertical motion (`j`, `k`) follows a line selection THE SIMULATOR SHALL compute the target from the range head as hx does (`x j d` removes the newline at the end of line 2) | #496 | must |
| FR-018 | WHEN `f`, `t`, `F` or `T` finds its target THE SIMULATOR SHALL extend/select from the previous position to the found position as hx does (`f a d` removes `fn a`; `t ( d` removes `fn add`), not collapse to a one-character cursor | #494 | must |
| FR-019 | WHEN `r<char>` runs THE SIMULATOR SHALL replace every character of every selection range with `<char>` | #497 | must |
| FR-020 | WHEN `~`, `` ` `` or `Alt-`` ` `` runs THE SIMULATOR SHALL apply the case change to every character of every range and keep the selection. `~` SHALL toggle case (`switch_case`), `` ` `` SHALL lowercase (`switch_to_lowercase`) and `Alt-`` ` `` SHALL uppercase (`switch_to_uppercase`); each key is bound to a dedicated handler matching its registry name, and the lowercase and uppercase keys SHALL NOT toggle | #498, #517 | must |
| FR-021 | WHEN `J` runs THE SIMULATOR SHALL join per hx `join_selections`: for each range replace each line break with a single space and strip the leading whitespace of the joined line (and the leading comment token, FR-038); a count SHALL NOT repeat the join (`2J` equals `J`); `Alt-J` follows the same rule with its hx semantics | #499 | must |
| FR-022 | WHEN `u`/`U` undo or redo a change THE SIMULATOR SHALL restore the selection that existed with that history state (history entries store the text and the selection) so `x d u d` removes line 1 again | #501 | must |
| FR-023 | WHEN `gl` runs THE SIMULATOR SHALL move to the last character of the line (not the line ending, and not past the end on the last line) | #492 | must |
| FR-024 | WHEN `p` or `P` completes THE SIMULATOR SHALL select the pasted text in each range (`w y P d` restores the original text) | #491 | must |
| FR-025 | THE SIMULATOR SHALL expose ONE counted-execution mechanism (a counted execute taking the base command and a non-zero count; it is crate-internal and not part of the public API) driven by per-command metadata `CountBehavior::{Repeat, Ignore, Native}` in the command registry: `Repeat` applies the command n times (the default, e.g. `h`, `j`, `x`), `Ignore` applies it once (`d`, `Alt-d`, `c`, `Alt-c`, `i`, `a`, `I`, `A`, `Alt-,` (`remove_primary_selection`), `J`, `Alt-J`, `~`, `` ` ``, ``Alt-` ``, `Alt-;` (`flip_selections`), `v`, `Ctrl-c`, `.`, `q`, and the register-scoped forms `"<r>d` and `"<r>c`, decided by the rule in `CommandRegistry::count_behavior`; hx 25.07.1 reads or ignores the count there per the `helix-term` sources, so `3d` on `abcdef` gives `bcdef`), `Native` passes the count into the command (`G`, `gg`, `f`, `t`, `F`, `T`); native counted strings are parsed by one registry parser into a closed `NativeCounted` type (`GotoLine` or `Find`). Commands where hx reads the count and repetition is equivalent (`h`/`j`/`k`/`l`, `w`/`b`/`e`, `x`/`X`, `u`/`U`, `p`/`P`/`R`, `>`/`<`, `o`/`O`, `n`/`N` and similar) stay `Repeat`; A `Repeat` loop SHALL stop as soon as a command has switched the simulator into Insert mode, so `3o` never types the base command as text. The authoritative per-command list is the registry; this table is checked by a registry test. The game and minigame layers SHALL call this mechanism and SHALL NOT repeat the base command themselves | #493, #499 | must |
| FR-027 | WHEN `ms`, `md` or `mr` completes THE SIMULATOR SHALL record it as ONE undo step (one transaction, as hx), so `m s (` followed by `u` restores the text before the surround in one press | #502 | must |
| FR-026 | WHEN `o`, `O` or Enter in Insert mode creates a new line THE SIMULATOR SHALL copy the leading whitespace of the current line onto it (`j o xyz Enter abc Escape` on scenario 14 keeps the 4-space indent), amended by FR-039 for lines that open a block | #490, #516 | must |

| FR-028 | THE SIMULATOR SHALL resolve one indent unit per document from the effective language (the scenario's `language`, falling back to the default language `rs` when absent) through a typed indent-unit value (spaces of a given width, or a tab) backed by a minimal static table whose entries are copied from the `hx` 25.07.1 language definitions and verified against a local `hx`; the table contains only the languages that bundled scenarios use | #500 | must |
| FR-029 | A unit test over the embedded scenarios SHALL fail when any effective language used by a bundled scenario has no indent-unit table entry (no silent fall-through to an editor-wide default such as tabs) | #500 | must |
| FR-030 | WHEN `>` runs THE SIMULATOR SHALL add one indent unit to every non-empty line touched by every selection range (not only the head line); WHEN `<` runs it SHALL remove at most one indent unit of leading whitespace from every touched line; counted forms follow FR-025 (`2>` indents twice). Scenario hints that teach the fixed 2-space behavior are corrected in the same change | #500 | must |
| FR-031 | WHEN `Tab` is pressed in Insert mode THE SIMULATOR SHALL insert one indent unit (FR-028) at each cursor as `hx` 25.07.1 does, modeled as a typed Insert command (see [[../register-command-mode-support/spec#12.3 Insert-mode editing keys (#489)\|register spec 12.3]]) and emitted by the input layer under the constant for the Tab key; it SHALL NOT insert the literal control character | #514 | must |
| FR-032 | THE SIMULATOR SHALL implement `&` (`align_selections`), `K` (`keep_selections`) and `Alt-K` (`remove_selections`) with `hx` 25.07.1 semantics instead of silent no-op handlers; `K` and `Alt-K` collect their regex through the regex prompt of [[../regex-selection-and-macro-commands/spec#11. Bug-Fix Batch: Backward Search, Keep/Remove Selections, Hint Key (#509, #510, #513)\|regex spec section 11]] | #510 | must |
| FR-033 | WHEN `.` replays a recorded Insert sequence THE SIMULATOR SHALL replay cursor movements and edits in their original order (one ordered list in the recorded sequence, not edits followed by all movements) | #519 | must |
| FR-034 | WHEN `R` runs with N selection ranges and M stored register values THE SIMULATOR SHALL replace range i with value i, repeating the last value when M < N and ignoring surplus values when M > N (the same rule as `p`/`P`, FR-214 of the register spec); with a single value every range receives it | #520 | must |
| FR-035 | WHILE in Select mode (`v`) THE SIMULATOR SHALL extend the selection with motions as `hx` 25.07.1 does (`v l l` covers the two characters moved over), and `d`, `c`, `y`, `r` and `Esc` in Select mode SHALL behave as in `hx` (`Esc` returns to Normal, `c` enters Insert); the verification set is recorded from a differential run against `hx` before the fix | #524 | must |

| FR-036 | WHILE in Select mode (`v`) THE SIMULATOR SHALL, for `/`, `?`, `n` and `N`, push a new range for the found match as `hx` 25.07.1 does (the existing ranges stay), and SHALL extend the current ranges for `Ctrl-d`, `Ctrl-u`, `]p`, `[p` and `mm` as `hx` does | #534 | must |
| FR-037 | THE INDENT UNIT of a document SHALL first be auto-detected from the initial text with `helix_core::indent::auto_detect_indent_style` (consistent indentation wins, so a scenario indented with 8 spaces uses 8), and fall back to the per-language table of FR-028 when detection yields nothing. `align_selections` (`&`) SHALL pad by display width (grapheme display columns, as hx), not by character count | #535 | must |
| FR-038 | WHEN `J` joins lines THE SIMULATOR SHALL remove the leading comment token of the joined line as `hx` `join_selections_impl` does, using one per-language comment-token table (`rs`: `///`, `//!`, `//`; `py`: `#`; `md`: none) that lives in the same module as the indent table of FR-028 (one language table, not two) | #521 | must |
| FR-039 | WHEN `o`, `O` or Enter creates a new line THE SIMULATOR SHALL derive its indent with a per-language rule, with no new dependency and no syntax tree: for `rs`, the current line's indent plus one indent unit when the line (ignoring trailing whitespace) ends in `{`, `(` or `[`; for `py`, plus one unit when it ends in `:`; otherwise copy the current line's indent (FR-026); for languages without a rule (`md`) copy. The rule applies to every input; correctness is claimed only for the recorded golden cases of FR-040 | #516 | must |
| FR-040 | THE GOLDEN TRACES for FR-039 SHALL be recorded from a local `hx` 25.07.1 started with an empty configuration (`hx -c <empty file>`, no user `languages.toml`, so `indent-heuristic` and other overrides cannot change the result) for every bundled scenario that uses `o`, `O` or Enter, plus canonical cases per language. Any recorded case where the rule differs from `hx` is listed in 3.8.4 and tracked by the follow-up defect issue | #516 | must |

### 3.8.1 Acceptance Criteria

```
GIVEN the text of scenario 14, cursor at (0,0)
WHEN  the user presses x, x, d        THEN lines 1 and 2 are removed
WHEN  the user presses 3, x, d        THEN lines 1 to 3 are removed
WHEN  the user presses %, x, d        THEN the whole buffer is removed
WHEN  the user presses x, j, d        THEN the newline at the end of line 2 is removed
WHEN  the user presses f, a, d        THEN `fn a` is removed
WHEN  the user presses w, r, z, d     THEN `add(a: i32, ...` remains (the replaced `fn ` is deleted)
WHEN  the user presses x, ~, d        THEN line 1 is removed (toggled then deleted)
WHEN  the user presses J              THEN line 1 is `fn add(a: i32, b: i32) -> i32 { let sum = a + b;`
WHEN  the user presses 2, J           THEN the result equals the result of J
WHEN  the user presses x, d, u, d     THEN line 1 is removed
WHEN  the user presses g, l, d        THEN the trailing `{` of line 1 is removed
WHEN  the user presses w, y, P, d     THEN the buffer equals the original text
WHEN  the user presses j, o, xyz, Enter, abc, Escape
                                      THEN the inserted lines are indented by 4 spaces
WHEN  the user presses x, x, >        THEN lines 1 and 2 each gain 4 spaces (Rust indent unit), line 3 is untouched
WHEN  the user presses x, >, then x, <   THEN line 1 is back to its original indentation
WHEN  the user presses 2, >           THEN the cursor line gains 8 spaces
WHEN  the user presses ` on `aBc`     THEN the text is `abc`; Alt-` gives `ABC`; ~ gives `AbC`
WHEN  the user presses i, Tab         THEN one indent unit of the document language is inserted
WHEN  the user records `i a <Left> b <Esc>` and presses . elsewhere
                                      THEN the replay reproduces the original order (a, Left, b)
WHEN  three ranges are yanked and R runs
                                      THEN range i receives value i
WHEN  the user presses v, l, l        THEN the selection covers the two characters moved over
WHEN  the user presses v, /foo, Enter THEN a new range covers the match and the previous range stays (as hx)
WHEN  a Rust scenario line ends in `{` and the user presses o
                                      THEN the new line is indented one unit deeper than the current line
WHEN  a Python line ends in `:` and the user presses o
                                      THEN the new line is indented one unit deeper
WHEN  the user presses J on `// a` joined with `// b`
                                      THEN the result is `// a b` (the comment token of the next line is stripped)
WHEN  a scenario is indented with 8 spaces and the user presses >
                                      THEN the line gains 8 spaces (auto-detected unit)
```

### 3.8.2 Edge Cases

| Scenario | Expected Behavior |
|----------|-------------------|
| `x` on the last line without a trailing newline | Selects through the end of the buffer; a second `x` is a no-op |
| `J` on the last line | No-op, no error |
| `r` with a newline target or on a range containing line endings | Follows hx; verify against `helix-term` 25.07.1 [NEEDS CLARIFICATION: newline handling] |
| `u` with no history | No-op, selection unchanged |
| `o`/`O`/Enter after a line ending in `{`, `(` or `[` (`rs`) or `:` (`py`) | One extra indent unit by the rule of FR-039; constructs the rule does not see (multi-line expressions, closing-bracket dedent, other languages) follow the recorded divergences in 3.8.4 |
| `>` on an empty line | The line is left unchanged (hx skips empty lines) |
| `<` on a line with less leading whitespace than one unit | Removes only what exists, no error |
| `J` where the next line has no comment token, or the language has no comment tokens (`md`) | Whitespace-only join as before |
| Scenario language with no indent-unit table entry | Rejected by the FR-029 unit test, never defaulted silently |
| `&` or `K`/`Alt-K` where `hx` reports an error (no match, invalid regex) | Surface a `UserError` through the existing path; selection unchanged |
| Existing scenarios whose target was authored against the old (wrong) behavior | Target and `optimal_count` are corrected to hx behavior in the same change; `cargo nextest run scenario` must pass |
| Command strings with counts (`3fa`, `2G`) | Must be accepted by the simulator before the input layer emits them (see [[regex-selection-and-macro-commands/spec#10. Bug-Fix Batch: Input Chords, Counted Commands, Surround and Primary Selection (#487, #493, #502, #503, #455)\|regex-selection-and-macro-commands section 10]]) |

### 3.8.3 Accepted deviations from hx (this batch)

| Deviation | Rationale |
|-----------|-----------|
| `;` collapses all ranges to a block cursor | Accepted simplification of `collapse_selection` for the trainer |
| `state()` reports a cursor on the empty line after a final `\n` as the end of the last real line | Display/target comparison uses real lines only |
| `Escape` immediately after `o`, `O` or Enter removes the auto-inserted indent | Same as hx `try_restore_indent` |

### 3.8.4 Deferred and residual divergences

- Full tree-sitter indentation for `o`/`O`/Enter is NOT implemented. The user chose the per-language rule of FR-039 over adding tree-sitter and grammar dependencies. The residual fidelity gap (newline indent in situations outside the recorded golden cases) is tracked by one follow-up defect issue (P3); the divergences found while recording the goldens are listed here and attached to that issue.

| Case | hx 25.07.1 result | Simulator result | Status |
|------|-------------------|------------------|--------|
| (filled in from the golden recording, FR-040) | | | |

### 3.8.5 Success Criteria

| ID | Metric | Target |
|----|--------|--------|
| SC-101 | Each criterion in 3.8.1 exists as a simulator test asserting buffer and selection | Pass |
| SC-102 | A test over the registry confirms every command has a `CountBehavior` and the game layer contains no count-by-repeat loop | Pass |
| SC-103 | `cargo nextest run scenario` after target corrections | Pass |
| SC-104 | Differential tests for FR-028..FR-035 (indent, Tab, `&`/`K`/`Alt-K`, replay order, `R`, Select mode) assert buffer and selection against recorded `hx` 25.07.1 results; the FR-029 language-coverage test passes | Pass |
| SC-105 | Goldens for FR-039 recorded with `hx -c <empty config>` pass; the FR-037 detection, `&` display-width and FR-038 comment-token cases and the FR-036 Select-mode cases assert buffer and selection; the divergence table of 3.8.4 is complete | Pass |

## 4. Non-Functional Requirements

| ID | Category | Requirement |
|----|----------|-------------|
| NFR-001 | Reproducibility | The pin is exact (tag or full commit hash), never a branch; `Cargo.lock` is committed with the change |
| NFR-002 | Supply chain | A `rev` or tag change passes `cargo deny check` before merge |
| NFR-003 | Quality gate | The bump PR passes the full pre-commit suite per `CONTRIBUTING.md`, including `cargo nextest run scenario`, with `--no-default-features` and the `audio` feature both compiling |
| NFR-004 | Documentation | Breaking changes (renamed commands, changed scenario behavior) are recorded in `CHANGELOG.md` under `[Unreleased]` |

## 5. Edge Cases and Error Handling

| Scenario | Expected Behavior |
|----------|-------------------|
| New tag adds further API breaks beyond the two known | Compile errors are authoritative; list them in the upgrade issue and fix; the known list is not exhaustive |
| A behavior change makes a scenario unbeatable | Treated as classification (a)/(b) in FR-009; never silently deleted. See [[duplicate-scenario-ids/spec]] for the uniqueness guard that stays in force |
| `helix-core` and `helix-stdx` on different tags | Disallowed; both move together (FR-003) |
| Upstream tag changes `helix-core`'s MSRV above the project's | Bump MSRV as part of the same change and note it in `CHANGELOG.md` |
| Upstream never cuts a tag | Stay on `25.07.1` (Section 3.2); revisit at the cadence in Open Questions |
| Persisted FSRS review data refers to scenario IDs changed by the bump | Scenario IDs stay stable; only content changes. Renaming an ID needs a migration decision `[NEEDS CLARIFICATION: confirm FSRS card ids are keyed by command, as stated for keymaps, not scenario id]` |

## 6. Success Criteria

| ID | Metric | Target |
|----|--------|--------|
| SC-001 | Build against the new tag with both `audio` and `--no-default-features` | passes |
| SC-002 | `cargo nextest run scenario` and full suite on the bumped core | 100% pass, no `#[ignore]` added to hide failures |
| SC-003 | Known API breaks (FR-004) resolved | 2 of 2 call sites adapted |
| SC-004 | Behavior-change areas in Section 3.5 reviewed | 4 of 4 areas have a recorded review outcome in the upgrade issue |
| SC-005 | Keymap resolution diff (FR-012) produced | recorded before merge |
| SC-006 | Time from tag publication to an upgrade issue opened | `[NEEDS CLARIFICATION: target latency, e.g. 2 weeks]` |

## 7. Agent Boundaries

### Always (without asking)
- Compile against the new core first and treat compiler output as the source of truth for breaks.
- Run `cargo nextest run scenario` and the full suite after any scenario or simulator change.
- Verify any changed expected behavior against upstream documentation or the released editor.
- Update this spec when upstream facts change before the bump.

### Ask First
- Choosing a `rev` pin instead of a tag.
- Adding a new scenario from the Section 3.6 candidate list.
- Raising the project MSRV.
- Adding a compatibility alias for a renamed command.

### Never
- Modify `Cargo.toml`, `Cargo.lock` or source as part of this spec's authoring.
- Pin a branch name or an untagged master commit without an issue and expiry condition.
- Weaken or delete a failing scenario just to make the suite pass.
- Bypass `HelixSimulator` with bespoke logic to mimic new upstream behavior.

## 8. Open Questions

> [!question] Unresolved
> - [NEEDS CLARIFICATION: minimum Helix version the trainer should teach (latest tag only, or the version in common distro packages)?]
> - [NEEDS CLARIFICATION: which existing scenarios exercise the Section 3.5 behavior changes (enumerate at bump time)?]
> - [NEEDS CLARIFICATION: does the new core raise `rust-version` above the project's MSRV?]
> - [NEEDS CLARIFICATION: should there be an automated trigger (scheduled check of upstream tags, e.g. in the continuous-improvement cycle's dependency monitoring) or is manual detection enough?]
> - [NEEDS CLARIFICATION: how long to wait before filing an issue to move to a `rev` pin if no tag appears (it has been about 16 months since 25.07.1)?]
> - [NEEDS CLARIFICATION: whether upstream will publish a tag before the master changes listed here are altered or reverted; re-verify the compare at bump time.]

## 9. See Also

- [[constitution]] - project principles, including the pinned-core invariant
- [[custom-helix-keymap-support/spec]] - command-name resolution keys affected by renames
- [[register-command-mode-support/spec]] - register and `:` mode coverage relevant to `:set-register`
- [[duplicate-scenario-ids/spec]] - scenario ID uniqueness guard
- [[MOC-specs]] - all specifications
