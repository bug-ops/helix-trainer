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

> [!info] Re-verified 2026-11-12 (#438)
> Against the `25.07.1` tag (`a05c151`) and master `a714a81`: still no release tag after `25.07.1` (2025-07-18); the compare is ahead 1004 and behind 2 (diverged; the earlier figure was 991). No version bump follows from this, the decision rule of Section 3.2 keeps the `25.07.1` pin. CORRECTIONS to the first research: the Insert-mode `Tab` (`smart_tab`) and `Shift-Tab` (`insert_tab`) bindings (`default.rs` 393-394), `gD` goto_declaration (48) and normal-mode `Ctrl-z` suspend (335) are ALREADY in `25.07.1`, not master changes; `:set-register` is NOT in `25.07.1`. The default keymap diff `25.07.1..master` is only `[x`/`]x` XML element motions, `space E` renamed `space .`, and `space s`/`S` mapped to `lsp_or_syntax_symbol_picker` and `lsp_or_syntax_workspace_symbol_picker`. Confirmed: the `find_nth_pairs_pos` API break (master `surround.rs` 160), `|` added to `PAIRS`, and diffs in `case_conversion.rs` (92 lines), `selection.rs` (64), `transaction.rs` (208), `indent.rs` (1008) and `surround.rs` (56). Not re-verified: jumplist saving for goto next/prev and goto_word, the `<n>G` documentation, and the window-mode additions.

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

Candidates come from default keymap and typable-command changes on master, except the three rows marked as already released in `25.07.1` (re-verification of 2026-11-12). None is committed to the scenario catalog by this spec. Each accepted candidate needs its own spec/issue, and MUST NOT be added until it exists in a released tag the project pins. Rows are on unreleased master.

| Candidate | Specific upstream change | Teachable in simulator? |
|-----------|-------------------------|-------------------------|
| XML element motions | `]x` / `[x` motions; XML/HTML/JSX textobjects (upstream #11158) | `[NEEDS CLARIFICATION: needs tree-sitter syntax; the simulator uses no syntax trees today, and the new `find_nth_pairs_pos` `syntax` parameter suggests more tree-sitter coupling]` |
| Goto declaration | `gD` goto_declaration (already bound in `25.07.1`, `default.rs` 48; not a master change) | Likely LSP-only; probably not simulable (no LSP in the trainer) |
| Goto line / file start docs | docs now describe `<n>G` / `<n>gg` as goto_line and `<n>g` / `g` split for goto_file_start | Check whether existing goto-line coverage already matches; possible hint/doc fix instead of a new scenario |
| Jumplist saving | goto next/prev and goto_word now save to the jumplist | Scenarios using `Ctrl-o` / `Ctrl-i` after those motions would change; check jumplist support in the simulator |
| Window mode additions | `t` transpose_view, `ns` / `nv` new splits, goto_file_hsplit/vsplit | Out of scope: windows and splits are not simulated |
| Insert-mode Tab behavior | `Tab` smart_tab, `Shift-Tab` insert_tab (already in `25.07.1`, `default.rs` 393-394; not a master change) | Simulated at the pin: indentation by FR-031, smart-tab approximation by FR-059; `Shift-Tab` (`insert_tab`) is not modeled |
| Suspend | normal-mode `Ctrl-z` suspend (already in `25.07.1`, `default.rs` 335; not a master change) | Not teachable (process-level) |
| Symbol pickers | `space s` / `S` now `lsp_or_syntax_*_symbol_picker` | Picker UI is not simulated |
| `:set-register` | new typable command, NOT in `25.07.1` (verified 2026-11-12) | Candidate: aligns with the existing named-register and command-line support, which today covers only `:goto` / `:g` ([[register-command-mode-support/spec]]) |

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
- `[x` / `]x` (XML element motions) are new on master; they need syntax trees and are not simulated.
- Insert-mode `Tab` / `Shift-Tab` and normal-mode `Ctrl-z` are NOT master changes: they are already default in `25.07.1` (re-verification of 2026-11-12), so the stock keymap table must already hold them.

### 3.8 Simulator Fidelity at the Pinned Core (25.07.1)

> [!info] Added 2026-11-10 (spec-first update for the P1-P3 bug batch)
> Differential runs of the trainer against `hx` 25.07.1 (the exact version vendored as `helix-core`) found commands whose simulated result differs from the pinned core. These are defects at the CURRENT pin, independent of any future bump, and are in scope of the fixes below (they do not change `Cargo.toml`). The reference for every requirement is the behavior of `hx` 25.07.1 on the same text and keys; tests MUST assert the resulting selection as well as buffer text. Issue #500 (language-aware indent unit for `>`/`<`) and the follow-ups #510, #514, #517, #519, #520 and #524 (found while fixing the first set) were added to the batch on 2026-11-11 (FR-028..FR-035 and the FR-020 amendment). Tree-sitter extra indent was deferred at first; on 2026-11-11 the batch of #534, #535, #521 and #516 (FR-036..FR-040 below) was added, with #516 specified as a per-language rule without tree-sitter (see 3.8.4). On 2026-11-11 the batch of #592, #593, #594, #595, #596, #597 and #599 (FR-042 rewrite, FR-050..FR-058, the FR-025, FR-039, FR-040 and FR-047 amendments, sections 3.8.6 and 3.8.7) was added; the reference is the E0 differential run recorded in 3.8.6. On 2026-11-11 the batch of #607, #608, #609, #611, #583, #584, #585, #600, #541, #542, #617, #604, #603, #610, #616 and #544 (FR-028, FR-029, FR-040, FR-041, FR-042, FR-050 and FR-053 amendments, FR-059..FR-065, section 3.8.8) was added.

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
| FR-025 | THE SIMULATOR SHALL expose ONE counted-execution mechanism (a counted execute taking the base command and a non-zero count; it is crate-internal and not part of the public API) driven by per-command metadata `CountBehavior::{Repeat, Ignore, Native}` in the command registry: `Repeat` applies the command n times (the default, e.g. `h`, `j`, `x`), `Ignore` applies it once (`d`, `Alt-d`, `c`, `Alt-c`, `i`, `a`, `I`, `A`, `Alt-,` (`remove_primary_selection`), `J`, `Alt-J`, `~`, `` ` ``, ``Alt-` ``, `Alt-;` (`flip_selections`), `v`, `Ctrl-c`, `.`, `Q`, `q`, and the register-scoped forms `"<r>d` and `"<r>c`, decided by the rule in `CommandRegistry::count_behavior`; hx 25.07.1 reads or ignores the count there per the `helix-term` sources, so `3d` on `abcdef` gives `bcdef`), `Native` passes the count into the command (`G`, `gg`, `f`, `t`, `F`, `T`, `o`, `O`); counted native commands are carried as one closed typed `NormalCommand` (`GotoLine`, `Find`, `OpenBelow`, `OpenAbove`, see [[module-layering-and-typed-commands/spec]] FR-001) parsed by ONE parser, not as strings re-parsed per call site. Commands where hx reads the count and repetition is equivalent (`h`/`j`/`k`/`l`, `w`/`b`/`e`, `x`/`X`, `u`/`U`, `p`/`P`/`R`, `>`/`<`, `n`/`N` and similar) stay `Repeat`. A count SHALL be accepted by the input layer before `y`, `r`, `v`, `i`, `a`, `I`, `A` and `C`: `y`, `v`, `i`, `a`, `I` and `A` are `Ignore`, `C` is `Repeat` (`copy_selection_on_next_line` reads the count) and `r` ignores the count (it replaces with the following character once). A `Repeat` loop SHALL stop as soon as a command has switched the simulator into Insert mode, so a counted Insert-entering command never types the base command as text. `o` and `O` are NOT counted-equivalent and are `Native`: `No` opens N lines below each range with N cursors (one per opened line), `NO` N lines above, as hx `open` does (FR-053, #596; the earlier single-line, swallowed-count divergence of #577 is removed). The authoritative per-command list is the registry; this table is checked by a registry test. The game and minigame layers SHALL call this mechanism and SHALL NOT repeat the base command themselves | #493, #499, #577, #596 | must |
| FR-027 | WHEN `ms`, `md` or `mr` completes THE SIMULATOR SHALL record it as ONE undo step (one transaction, as hx), so `m s (` followed by `u` restores the text before the surround in one press | #502 | must |
| FR-026 | WHEN `o`, `O` or Enter in Insert mode creates a new line THE SIMULATOR SHALL copy the leading whitespace of the current line onto it (`j o xyz Enter abc Escape` on scenario 14 keeps the 4-space indent), amended by FR-039 for lines that open a block | #490, #516 | must |

| FR-028 | THE SIMULATOR SHALL resolve one indent unit per document from the effective language (the scenario's `language`, falling back to the default language `rs` when absent) through a typed indent-unit value (spaces of a given width, or a tab) backed by a minimal static table whose entries are copied from the `hx` 25.07.1 language definitions and verified against a local `hx`; the table contains only the languages that bundled scenarios use. Amended by #544 (2026-11-11): the effective language is a closed `Language` enum (`rs`, `md`, `py`) in `helix::language`, serialized as those tokens; `Setup.language` is `Option<Language>` and `None` means `rs`; an unknown or empty `language` value is a scenario load error (see [[language-aware-syntax-highlighting/spec]] FR-004); `Language::rules()` is an exhaustive match returning `&'static LanguageRules`, so it is total (no `Option`, no string lookup `for_language(&str)`); the free-form language validator in `src/security.rs` is removed because the enum parse is the validation | #500, #544 | must |
| FR-029 | A `Language` variant without a rules entry SHALL be a compile error (the exhaustive `rules()` match), and a unit test over the embedded scenarios SHALL load every bundled scenario, so a language the enum does not know fails at scenario load (no silent fall-through to an editor-wide default such as tabs). `LanguageRules::PLAIN` (renamed from `UNLISTED`) remains only as the rules of a bare `HelixSimulator::new` with no language; no scenario can reach it | #500, #544 | must |
| FR-030 | WHEN `>` runs THE SIMULATOR SHALL add one indent unit to every non-empty line touched by every selection range (not only the head line); WHEN `<` runs it SHALL remove at most one indent unit of leading whitespace from every touched line; counted forms follow FR-025 (`2>` indents twice). Scenario hints that teach the fixed 2-space behavior are corrected in the same change | #500 | must |
| FR-031 | WHEN `Tab` is pressed in Insert mode THE SIMULATOR SHALL insert one indent unit (FR-028) at each cursor as `hx` 25.07.1 does, modeled as a typed Insert command (see [[../register-command-mode-support/spec#12.3 Insert-mode editing keys (#489)\|register spec 12.3]]) and emitted by the input layer under the constant for the Tab key; it SHALL NOT insert the literal control character | #514 | must |
| FR-032 | THE SIMULATOR SHALL implement `&` (`align_selections`), `K` (`keep_selections`) and `Alt-K` (`remove_selections`) with `hx` 25.07.1 semantics instead of silent no-op handlers; `K` and `Alt-K` collect their regex through the regex prompt of [[../regex-selection-and-macro-commands/spec#11. Bug-Fix Batch: Backward Search, Keep/Remove Selections, Hint Key (#509, #510, #513)\|regex spec section 11]] | #510 | must |
| FR-033 | WHEN `.` replays a recorded Insert sequence THE SIMULATOR SHALL replay cursor movements and edits in their original order (one ordered list in the recorded sequence, not edits followed by all movements) | #519 | must |
| FR-034 | WHEN `R` runs with N selection ranges and M stored register values THE SIMULATOR SHALL replace range i with value i, repeating the last value when M < N and ignoring surplus values when M > N (the same rule as `p`/`P`, FR-214 of the register spec); with a single value every range receives it. On a point cursor (a one-grapheme range) `R` SHALL replace the one grapheme under the cursor (not the empty range before it), and the resulting selection SHALL cover the replaced (pasted) text as in FR-024 | #520, #578 | must |
| FR-035 | WHILE in Select mode (`v`) THE SIMULATOR SHALL extend the selection with motions as `hx` 25.07.1 does (`v l l` covers the two characters moved over), and `d`, `c`, `y`, `r` and `Esc` in Select mode SHALL behave as in `hx` (`Esc` returns to Normal, `c` enters Insert); the verification set is recorded from a differential run against `hx` before the fix | #524 | must |

| FR-036 | WHILE in Select mode (`v`) THE SIMULATOR SHALL, for `/`, `?`, `n` and `N`, push a new range for the found match as `hx` 25.07.1 does (the existing ranges stay), and SHALL extend the current ranges for `Ctrl-d`, `Ctrl-u`, `]p`, `[p` and `mm` as `hx` does | #534 | must |
| FR-037 | THE INDENT UNIT of a document SHALL first be auto-detected from the initial text with `helix_core::indent::auto_detect_indent_style` (consistent indentation wins, so a scenario indented with 8 spaces uses 8), and fall back to the per-language table of FR-028 when detection yields nothing. `align_selections` (`&`) SHALL pad by display width (grapheme display columns, as hx), not by character count | #535 | must |
| FR-038 | WHEN `J` joins lines THE SIMULATOR SHALL remove the leading comment token of the joined line as `hx` `join_selections_impl` does, using one per-language comment-token table (`rs`: `///`, `//!`, `//`; `py`: `#`; `md`: none) that lives in the same module as the indent table of FR-028 (one language table, not two) | #521 | must |
| FR-039 | WHEN `o`, `O` or Enter creates a new line THE SIMULATOR SHALL derive its indent with a per-language rule, with no new dependency and no syntax tree: the current line's indent plus one indent unit when the code before the cursor (trailing comment ignored) has a bracket (`{`, `(` or `[`) opened on the line that is closed later in the document (strings and comments skipped; `let a = foo(1,` without a closer stays at 0, and so does `fn f() -> u8 {` with no `}` anywhere: an unclosed opener with no closer yields 0, as hx, because hx auto-pairs typed openers; the earlier "unclosed opener at the end of the line" approximation is REMOVED, #597), or, for `py`, when the line ends in `:` (also before a trailing comment); a line that closes a bracket opened above is indented relative to the opener's line; blank lines walk up to the nearest non-blank line; for `py`, a line starting with `return`, `pass`, `break`, `continue` or `raise` (whole word, no bracket left open) dedents the next line by one unit (floor 0); on a comment line (`rs`: `///`, `//!`, `//`; `py`: `#`; longest token) `o`, `O` and Enter continue the comment at the comment line's indent without applying the opener rule, and a trailing comment after code is not a continuation; the comment continuation is checked BEFORE the auto-pair split (`// foo(` / `// y)`); Enter between an auto-pair (`()[]{}` and double quote and backtick everywhere, single quote for `py` and `md` only, `<>` never) splits it onto three lines (mismatched halves such as `x(]` give `x(` / `    y]`); a count prefix (`3o`, `2O`) opens N lines with N cursors as hx does (FR-053); the rule is applied to every range (multi-cursor Insert, FR-053); otherwise copy the current line's indent (FR-026); for languages without a rule (`md`) copy. For `O` the rule is the indent `o` would give below the PREVIOUS line (this matches the recorded goldens); on the first line, and for languages without openers, `O` copies the current line. The rule applies to every input; correctness is claimed only for the recorded golden cases of FR-040. Amendments (#546): (1) for `py`, a `:` opens a block only at bracket depth 0 on a line whose first token is a compound-statement keyword (`if`, `elif`, `else`, `for`, `while`, `def`, `class`, `try`, `except`, `finally`, `with`, `match`, `case`); `lambda`, dict and slice colons never open; (2) a `:` that lies inside a string at the cursor does not open; (3) brackets inside block comments, raw strings and triple-quoted or multi-line strings are not counted, and the lexer state is carried from the start of the document; (4) an unclosed opener with no closer later in the document yields indent 0 (hx auto-pairs typed openers, so a loaded text with an unmatched `{` is a parse error); confirmed by the goldens and IMPLEMENTED by #597 together with typed auto-pair (FR-055); (5) comment continuation takes the comment token from the WHOLE line (so `x = 1  # note` is not a continuation, while an indented `# note` line is); (6) multi-cursor `o`/`O`/Enter apply the rule to every range, and counted `o`/`O` are supported, by FR-053 (#596) | #516, #546, #596, #597 | must |
| FR-040 | THE GOLDEN TRACES for FR-039 SHALL be recorded from a local `hx` 25.07.1 started with an empty configuration (`hx -c <empty file>`, no user `languages.toml`, so `indent-heuristic` and other overrides cannot change the result) that only turns `auto-format` off (otherwise rustfmt rewrites valid Rust on `:w` and contaminates the result) and `insert-final-newline` off (it only affects writes, so the `:w` output equals the buffer bytes; the test file is written byte-identical to the scenario `file_content`, with no trailing newline) for every bundled scenario that uses `o`, `O` or Enter, plus canonical cases per language. Any recorded case where the rule differs from `hx` is listed in 3.8.4 and tracked by the follow-up defect issue. The goldens for #593-#597 and #599 are recorded in 3.8.6 and are the reference for FR-050..FR-058. Amended by #603 (2026-11-11): the recorded cases live in ONE golden data table that the library differential test reads (no values duplicated in test code); a dev-only example `examples/hx_differential.rs` replays every case in a local `hx` through tmux and reports drift against the table, and its `--write` flag regenerates the table from hx. The live run is opt-in (it needs `hx` and tmux), never part of CI or of the required gate, and is the drift check named by the upgrade checklist (FR-008, FR-011) | #516, #593, #594, #595, #596, #597, #599, #603 | must |

| FR-041 | AFTER every command THE SIMULATOR SHALL hold every selection range inside the document (`0 <= from <= to <= len_chars`); a transaction's selection SHALL be the one the transaction carries and, when it carries none, the previous selection mapped through the transaction's changes. No key sequence SHALL panic (property test over key sequences asserting in-bounds ranges and no panic). Amended by #604 (2026-11-11): the property test has two passes. The CI pass runs a pinned seed (`rng_seed` of the proptest configuration) over a FIXED command alphabet that does not grow with the registry, so runs are comparable across commits, and a committed `proptest-regressions` file keeps every shrunk failure. The full-registry randomized pass (alphabet derived from the registry, random seed) runs only behind an explicit environment opt-in, so a rare finding never looks like a flaky CI test | #569, #604 | must |
| FR-042 | `Ctrl-c` on a gameplay screen in an editing state (the Task screen, and the Arcade screen while Playing) SHALL never quit the application. Per input state (every row below was recorded in `hx` 25.07.1, see 3.8.6 table A): the Normal base state and a pending count (`2 Ctrl-c`, the count is consumed) run `toggle_comments`; Insert mode INSERTS the character `c` and stays in Insert (the hx insert keymap has no `C-c`, and the insert fallback ignores key modifiers; this replaces the former no-op row); Select mode (`v`) runs `toggle_comments` AND returns to Normal mode; the register-name state `"` takes `c` as the register name (`" Ctrl-c y` yanks into register `c`); a selected register awaiting a command (`"a` then `Ctrl-c`) FORWARDS to `toggle_comments` and the pending register is DROPPED (the next `y` goes to the default register and register `a` is unchanged; this replaces the former cancel row); the other argument-taking pending states (`f`, `t`, `F`, `T`, counted `2f`, `r`, `ms`, `md`, `mr` (both arguments), `mi`, `ma`) treat `Ctrl-c` as the plain character `c`, because hx ignores key modifiers there (helix-term `commands.rs` 1558-1561 and 1696-1699); keymap-leader pending states (`g`, counted `2g`, `z`, `m`, `[`, `]`, `space`) follow hx: an unmatched key cancels the pending state and is dropped (the next key is plain); prompts (`/`, `?`, `s`, `S`, `K`, `Alt-K`, `:`) close the prompt and leave the buffer untouched; menus and all non-gameplay screens keep `Ctrl-c` as quit. The sticky view popup (`Z`) ignores `Ctrl-c` in hx; the trainer has no `Z` state. REGISTER-OPERATION STATE (#610): in the register-operation state (`"a` typed, awaiting a command) ANY key with a CONTROL or ALT modifier is handed to the Base handler (as hx does: the key runs its own command) and the pending register is dropped; this generalizes the `Ctrl-c` arm above and replaces the former known divergence (`"a` then `Ctrl-d` ran `"ad`; it now drops the register and runs `half_page_down`, so a Ctrl key never triggers a destructive plain command with the register). Tests: every Ctrl key after `"a`, plus `"a Alt-c`. MINIGAME NON-EDITING STATES (#616): the Arcade countdown, pause and game-over states map `Ctrl-c` to `QuitApp`, exactly like every other non-gameplay screen; this is a TRAINER DECISION (hx has no equivalent), not a hx fact. The countdown counts the game as played, the same as `Ctrl-q`: a Daily Challenge records a result of 0 (the attempt was already consumed at launch, [[challenge-attempt-limit-not-enforced/spec]] FR-004), and the quit follows the existing `Ctrl-q` bookkeeping path. The mapping is one exhaustive function over the screen and the input state, with one test per changed row (Insert, `"<reg>`, Select) and one end-to-end test per MiniGame non-editing state (countdown, pause, game over) | #570, #599, #610, #616 | must |
| FR-043 | WHEN `mm` runs THE SIMULATOR SHALL take the character under the head's block cursor (not the character before the head) as the bracket to match, and SHALL collapse onto the matching bracket as hx `match_brackets` does | #579 | must |
| FR-044 | WHEN `Alt-_` (`merge_consecutive_selections`) runs THE SIMULATOR SHALL merge only ranges that touch or overlap; WHEN `Alt--` (`merge_selections`) runs it SHALL merge all ranges into one spanning range | #580 | must |
| FR-045 | WHEN `*` runs THE SIMULATOR SHALL behave as `search_selection_detect_word_boundaries`: the escaped primary selection becomes the search pattern, `\b` is added at each end where the selection starts or ends at a word boundary, the search register is set, and the selection is unchanged (`n` moves to the next match). The scenarios `search_word_001`, `search_word_002`, `search_next_001` and `search_prev_001` are reworked so that their solutions select the word first (for example `miw *`); the in-game `*` help text is updated | #581 | must |
| FR-046 | WHEN `c` runs on a whole-line selection (one that covers the line ending) THE SIMULATOR SHALL keep the line break and enter Insert on an empty line indented by the FR-039 rule | #587 | must |
| FR-047 | WHEN `I` runs THE SIMULATOR SHALL move to the first non-whitespace character of the line and enter Insert; on a truly empty line (no characters before the line ending; a whitespace-only line is NOT empty and gets no auto-indent, as hx) it SHALL auto-indent as hx `insert_with_indent` does, with the indent `o` would give below that line (FR-039); `A` follows the same rule (FR-051, #594) | #588, #594 | must |
| FR-048 | WHEN `a` is pressed, text typed and `Escape` pressed THE SIMULATOR SHALL leave a selection that runs from the ORIGINAL selection start through the end of the inserted text | #589 | must |
| FR-049 | THE commands `f`, `F`, `t`, `T`, `r`, `ms`, `md`, `mr`, `mi` and `ma` SHALL accept non-ASCII argument characters; an unparsable key token in a command string SHALL be dropped and logged, never replaced with another character | #556 | must |
| FR-050 | WHEN `i` runs on ANY selection (forward, backward, or after `Alt-;`) THE SIMULATOR SHALL insert at the selection START (the simulator flips every range with `Range::new(r.to(), r.from())` and collapses it to a point cursor at the head while Insert mode is active), so `w i a Escape` on scenario 14 gives `afn add(...` exactly as `i` on a flipped range does; the pre-command selection is still the one restored by `u` (undo spec FR-003). `a` keeps inserting at the selection end (`w a a Escape` gives `fn aadd(`). AFTER `Escape` (#611, E0 3.8.6 table B): hx keeps the REVERSED original range as the selection (`w i a Escape` leaves the primary selection `fn ` with the cursor at 1:2), so the simulator SHALL NOT collapse to a point cursor. Each cursor's Insert entry stores the hx range (`Range::new(original to(), entry cursor)` for a non-empty original range; a point cursor stores a zero-width range). DELIBERATE DEVIATION from the design plan, which stored `Vec<Option<Range>>` with `None` for a point: the implementation stores `Vec<Range>`, with the same observable behavior in every golden below; every session change maps each stored range with helix-core's own `Range::map`, which re-chooses the association per change from `anchor.cmp(head)` (a reversed range maps its anchor BeforeSticky; once a range collapses to anchor == head both ends map AfterSticky), so the mapping is never re-implemented; any Insert-mode cursor motion (arrow keys) drops that cursor's stored range, as hx replaces the range on motion; on `Escape` a stored range is restored as is and a dropped one stays a point cursor. Goldens, recorded from hx 25.07.1 before implementation: forward range, backward range (`Alt-;`), several ranges, two ADJACENT ranges `[0,2)` and `[2,4)` with `i x Escape` (BOTH ranges are kept), and on scenario 14 `w i Delete Delete Delete x Escape` (the selection is a point after `x`) | #593, #611 | must |
| FR-051 | WHEN `A` runs on an EMPTY line (no characters before the line ending) THE SIMULATOR SHALL auto-indent as hx `insert_with_indent` does: the line is the cursor line of the range (the block-cursor line, which differs from the head line after `x`: `j x A y Escape` appends on line 2 even though the head is on line 3), the indent is the one `o` would give below that line (FR-039), and a whitespace-only line gets no auto-indent. `I` and `A` SHALL share one implementation. A following `Escape` keeps the auto-indent whitespace (`A Escape` on an empty line inside a block leaves four spaces; it is NOT stripped, see FR-054). CRLF line endings are preserved (`A x`, `A Enter`) | #594 | must |
| FR-052 | WHEN `Ctrl-k` (delete to line end) runs in Insert mode THE SIMULATOR SHALL NOT panic on any line ending, and SHALL behave as hx: with the cursor before the line end it deletes to the line end and KEEPS the line break (`ab\r\ncd`, `l i C-k Escape` gives `a\r\ncd`); with the cursor AT the line end it JOINS the next line (`ab\r\ncd`, `A C-k Escape` gives `abcd`: a CRLF is removed as ONE line break; the LF twin joins the same way; on the last line it is a no-op; `A C-k C-k` on three CRLF lines removes both breaks). `A Delete` and `A C-d` join the same way; `A Backspace` deletes one character (not the line break) and `Backspace` at a line start removes the whole line break (`ab\r\ncd`, `j i Backspace Escape` gives `abcd`). The CRLF property test (carriage returns in the key and text alphabet) asserts no panic and in-bounds ranges (FR-041). This differs from the issue text (\"no-op or delete to line end\"): the guard is only against the panic, the join is the hx behavior | #595 | must |
| FR-053 | THE SIMULATOR SHALL model multi-cursor Insert mode as hx does. Every range stays a point cursor while Insert mode is active; typed characters, Enter (with FR-039 indent), Backspace, Delete, `Ctrl-w`, `Ctrl-u`, `Ctrl-k`, `Tab` and the entry commands `i`, `a`, `I`, `A` apply to EVERY cursor in one transaction (`l C i x Escape` on `ab cd` / `ef gh` / `ij kl` gives `axb cd` / `exf gh` / `ij kl`, exact buffers in 3.8.6 table C). `No` opens N lines below each range and `NO` N lines above, with N cursors per range, one cursor on each opened line, the primary index preserved, indent per FR-039 for every opened line (`j 2 o x Escape` on `fn f() {` / `    a` / `    b` / `}` gives two `    x` lines after `    a`, primary cursor on the first new line). The whole session, across all cursors, is ONE undo step ([[undo-insert-session-granularity/spec]] FR-013) and `.` replays a recorded multi-cursor session. `a` on a reversed (flipped) selection inserts after the selection's last character, never at the head (hx 25.07.1: `abcd`, `v l Alt-; a x Escape` gives `abxcd`; the earlier simulator result `axbcd` was wrong); `a` on adjacent ranges merges them as hx does (`abc`, `% "s [ab]" a x Escape` gives `abxc` with ONE range), and on `Escape` every cursor is transformed back into a range individually, so cursors that merged during the session (`Backspace`, `Ctrl-w`, `Ctrl-u`) do not lose the remaining ranges (`abcdef`, `% "s [abd]" a Backspace Escape` gives `acef` with 2 ranges: the ranges for `a` and `b` merge on entry as in hx, `Backspace` deletes before each cursor, and `Escape` keeps every remaining range; likewise `abcd\nef`, `% "s [ace]" a Ctrl-u Escape` gives `d\nf` with two ranges at `(0,0)` and `(2,2)`: the two cursors on line 1 merge when `Ctrl-u` deletes to the line start, and the cursor on line 2 survives). AMENDED 2026-11-11, three former known divergences are now requirements: (1) a typed `.` in Insert mode INSERTS a period (`ab`, `l i . Escape` gives `a.b`); the repeat command `.` is interpreted only in the Normal arm of the command dispatcher, and replaying a recorded session (`.` or macro `q`) types a recorded `.` as text (#607); (2) `Backspace` is a typed Insert command handled by the same path as every other Insert command (no special branch), so `.` and macro replay record and replay it, plain and between an auto-pair (`i ( Backspace Escape j .` leaves no `()` behind; on `ab` / `cd`, `A x Backspace y Escape j .` gives `cdy`) (#608); (3) `Backspace` when only whitespace precedes the cursor on its line is the hx `delete_char_backward` rule (helix-term `commands.rs` 4340-4417): a preceding tab is deleted alone, otherwise the whitespace width (tabs counted at the language tab width, FR-063) is rounded down to the previous multiple of the indent width, a whole unit when already aligned, and that many spaces are removed; the indent width is the simulator's EFFECTIVE indent unit (detected from the text first, then the language table, FR-037), with tests for spaces, tabs and a detected 2-space unit (#609). `C` (`copy_selection_on_next_line`) and `Alt-C` copy only the PRIMARY range, a user-approved simplification kept as a known divergence (hx `copy_selection_on_line` copies EVERY range: `% "s [ae]" C` gives 3 ranges in the simulator and 4 in hx); the line walk itself follows hx per FR-061 | #596, #577, #546, #607, #608, #609 | must |
| FR-054 | WHEN Insert mode is left with `Escape` THE SIMULATOR SHALL remove the auto-indent of an unused new line by the hx 25.07.1 `try_restore_indent` rule (helix-view `editor.rs` 2287-2315, `inserted_a_new_blank_line`), evaluated on the composition of every transaction of the session including the entry edit (it mirrors hx, including hx's own mix of units: the length of the inserted text is compared with the cursor offset the way hx compares them, which agrees with a pure character comparison for ASCII whitespace; the whitespace test runs over graphemes and the line break is the document's own, so `\r\n` followed by whitespace strips like `\n` followed by whitespace): the composed change set is exactly `[Retain(m), Insert(s), Retain(_)]`; `s` is one line break followed only by whitespace characters; the offset `m + len(s)` equals the primary cursor; and the cursor is at the end of its line. When all hold, the whitespace after the line break is removed (one session edit, folded into the single undo entry); otherwise nothing is removed. Goldens (3.8.6 table C): single `o`/`O` `Escape` strips; `o x Backspace Escape` strips; `A Enter Escape` strips; `2o`, `2O` and two-cursor `o` do NOT strip (the change set has several insertions); `o Escape` on the LAST line of a buffer WITHOUT a trailing newline does NOT strip (the composed set is `[Retain, Insert]` with no trailing retain, so `fn f() {` / `    a` gives `fn f() {\n    a\n    `). This rule, ported from hx 25.07.1 and recorded on ASCII goldens only, REPLACES the earlier draft \"strip only when every range qualifies\". Text typed on the new line (`o x Escape`) leaves the indent in place | #596, #516 | must |
| FR-055 | WHILE typing in Insert mode THE SIMULATOR SHALL simulate hx auto-pairs one typed character at a time. The `helix_core::auto_pairs::AutoPairs` hook is fed one character per key and its result is normalized to a point cursor at the head (`Range::point(head)`), PER range, so several cursors or ranges each pair independently. A language uses its pair table (`rs`: `()` `[]` `{}` `\"\"` and the backtick, no single quote; `py` and `md`: those plus the single quote; an unlisted language: `helix_core::auto_pairs::DEFAULT_PAIRS`). An opener is NOT paired when the next character is alphanumeric (`i ( Escape` before `ab` gives `(ab`; before ` ab` gives `() ab`), and an apostrophe after an alphanumeric is never paired in `md` (`i d o n ' Escape` gives `don'`; `i ' Escape` on an empty `md` buffer gives `''`; `rs` gives `'`). Typing the closer when the next character is that closer skips over it (`i ( x ) y Escape` gives `(x)y`; `i \" x \" y Escape` gives `\"x\"y`; `i ( ) y Escape` gives `()y`). `i ( x Escape` gives `(x)`. `i { Enter x Escape` gives `{` / `    x` / `}`. Two cursors on empty lines each pair (`C i ( Escape` on `\n\n` gives `()\n()\n`), and two ranges pair at each range start when the next character is not alphanumeric (`v l C i ( Escape` on `  a\n  b` gives `()  a\n()  b`). The auto-paired backtick makes three typed backticks give FOUR in `md`, which is why the code-fence scenario needs FR-057 | #597 | must |
| FR-056 | WHEN `Backspace` runs in Insert mode with the cursor between an auto-pair (`()`, `[]`, `\"\"`) THE SIMULATOR SHALL delete BOTH halves (`i ( Backspace Escape` gives the empty buffer; also `[` and `\"`), as hx does | #597 | must |
| FR-057 | THE scenario `writing/code-fence.toml` SHALL use the solution `O`, backtick, backtick, backtick, `Delete`, `Escape`, `j`, `o`, backtick, backtick, backtick, `Delete`, `Escape` with `optimal_count` 13 (was 11): with auto-pair (FR-055) three typed backticks leave a fourth that `Delete` removes. The target text is unchanged. The scenario's parity claim is SIMULATOR OUTPUT versus the scenario target (Task and MiniGame completion, `cargo nextest run scenario`), NOT byte parity with hx: in hx the same keys leave a trailing `\n` after the final `Delete` (a hx quirk, see 3.8.6 table E), which the simulator does not reproduce and the spec does not claim to | #597 | must |
| FR-058 | WHEN `Ctrl-r` is pressed in the Base input state on a gameplay screen (Task and Arcade) THE SYSTEM SHALL execute redo (the existing `TrainerAlias` path) instead of dropping the key. The Ctrl-key table (`b f u d c r`) is ONE table shared by the single-key mapping and the base-state Ctrl handler, with an end-to-end test (`handle_key_event(Ctrl-r)` after an undo redoes) | #592 | must |
| FR-059 | WHEN Insert-mode `Tab` is pressed and ANY cursor has a non-whitespace character before it on its line THE SIMULATOR SHALL insert nothing for any cursor (hx `smart_tab`, on by default in 25.07.1, moves to the end of the parent syntax node instead of indenting; with a cursor in leading whitespace only, FR-031 applies unchanged). For `rs`, each cursor with text before it moves to just after the closer of the INNERMOST bracket pair (`()` `[]` `{}`) that encloses it, found with the lexer of the newline-indent rule (FR-039: strings and comments are skipped); when no pair encloses it, it moves to just after the closing `}` of the depth-0 block that opens later in the cursor line's item; otherwise, and for `py` and `md` ALWAYS, the cursor stays (hx without a syntax tree). This is a deterministic approximation of hx's tree-sitter parent-node jump without a syntax tree dependency (the user's choice for FR-039 applies); the three repro cases of the issue (`a Tab Escape`, `A Tab Escape`, `w a Tab Escape` on scenario 14) are golden-checked against hx and every mismatch is recorded in 3.8.4 (known: for `py` and `md` hx moves to the end of the ROOT node, so `A Tab` goes to the document end, where this rule stays; and for `rs` `f( a Tab` jumps to the function end in hx and to after `)` here). Exact tree-sitter parent-node fidelity is tracked by a follow-up issue | #583 | must |
| FR-060 | WHEN `/`, `?`, `n` or `N` searches THE SIMULATOR SHALL port hx `search_impl` (helix-term `commands.rs` 2148-2203): the forward search starts at the primary range's `to()` (the next grapheme boundary for a point range) and the backward search at its `from()` (the previous grapheme boundary); the match range keeps `with_direction(primary.direction())`; a zero-width match is widened to one grapheme; a match with `end == 0` is skipped; the search wraps around; the match is smart-case: a pattern without an uppercase letter matches case-insensitively and a pattern with one matches case-sensitively (#624). The same smart-case rule applies to the `s`, `S`, `K` and `Alt-K` regex prompts, which share one `regex_prompt` in hx. All of them compile through one shared function (`compile_smart_case`), so `^` and `$` are line-anchored (multi-line) in `s`, `S`, `K` and `Alt-K` as in hx. So `/ $ Enter d` on scenario 14 selects the line break at the end of line 1 and joins it with line 2 (`fn add(a: i32, b: i32) -> i32 {    let sum = a + b;`) and `/ ^ Enter d` deletes the first character of line 2 (`   let sum = a + b;`) | #584 | must |
| FR-061 | WHEN `C` or `Alt-C` runs THE SIMULATOR SHALL port hx `copy_selection_on_line` for the primary range (the primary-only simplification of FR-053 stays): the copy goes to the same VISUAL column (tabs counted at the language tab width, FR-063) on the next (`C`) or previous (`Alt-C`) line, and a line shorter than the column is SKIPPED (no cursor is created there and the walk continues to the following line). So on scenario 14 `l C C C d` creates no cursor on the shorter closing-brace line and deletes the character at column 1 of lines 1-3 only, as hx does. A SECOND `d` keeps the closing `}` as in hx: `delete_selection_impl` no longer clamps a cursor at the document end onto the last character (#625, closed). The scenario `copy_selection_next_002` is reworked against hx. `Alt-;` (`flip_selections`) flips EVERY range, as hx does (`Selection::transform`), not only the primary one (#623) | #585 | must |
| FR-062 | THE simulator command registered as `search_word_under_cursor` SHALL be renamed `search_selection_detect_word_boundaries` (the behavior of FR-045 since #581), and `Alt-*` (`search_selection`) on a POINT range SHALL search for the escaped character under it instead of selecting the word; the helpers `select_word_as_pattern` and `extract_word_at_cursor` are deleted. The in-game help text and the keymap command-name tables use the new name (a user remap naming the old name is reported through the unknown-command path, FR-014, no alias before 1.0) | #600 | must |
| FR-063 | THE language rules SHALL carry the tab width as a typed `TabWidth` newtype: `rs` 4, `py` 4, `md` 2 (the hx 25.07.1 `languages.toml` values), and 4 for the bare-simulator rules `LanguageRules::PLAIN` only. `&` (FR-037), `<` (FR-030), the whitespace-only Backspace (FR-053), and `C`/`Alt-C` (FR-061) read the tab width from the active language rules, never from a constant. Tests cover `md` (width 2) next to `rs` | #542 | must |
| FR-064 | THE simulator SHALL model ONE logical viewport so that `Ctrl-d`, `Ctrl-u`, `Ctrl-f`, `Ctrl-b`, `zj` and `zk` and the view after every command match hx 25.07.1 on a fixed 24-row view (user decision D-541 = B: a fixed logical view height, a port of hx scrolling; the rendered terminal height is NOT used and no real-height input exists). The full model, formulas and golden setup are in section 3.8.8. The fixed `PAGE_SIZE` is deleted. | #541 | must |
| FR-065 | WHILE an Insert session records its changes THE SIMULATOR SHALL keep the cost of recording one key independent of the number of keys already typed in the session (amortized constant): composing the session's changes into the accumulated set SHALL NOT clone and re-compose the whole accumulated set per key. The composed result SHALL equal sequential composition by op structure (`changes()` slice, `len`, `len_after`), checked by a property test that compares batched against sequential composition (not by applied text). Acceptance: a bench with 10 cursors where the cost per key at key 8000 is at most 1.5 times the cost per key at key 200; a failing measurement reopens the issue before the PR is merged. NFR-003 of [[undo-insert-session-granularity/spec]] is the standing rule | #617 | must |
| FR-066 | THE SIMULATOR SHALL model `.` as hx 25.07.1 `repeat_last_insert`: it replays ONLY the last Insert-mode command (the entry command `i`, `a`, `I`, `A`, `o`, `O`, `c` or a count-bearing form, the keys typed in Insert mode, and the closing `Escape`; FR-053, FR-065, [[undo-insert-session-granularity/spec]]). Non-insert edits (`d`, `Alt-d`, `x` followed by `d`, `>`, `<`, `J`, `~`, `p`, `P`, `R`, `y`, `r`, `u`, `U`) SHALL NOT record or replace the repeat buffer, and selection plus operator compounds are NOT recorded as one compound action. WHEN `.` runs and no Insert session has been recorded THE SIMULATOR SHALL do nothing (no buffer change, no selection change, no error). WHEN a non-insert edit follows an Insert session THEN `.` SHALL still replay that earlier Insert session (`i Z Escape g g x d .` re-inserts `Z`). A count before `.` stays ignored (known divergence of [[module-layering-and-typed-commands/spec]]) and is out of scope here. The module documentation of `src/helix/repeat.rs` (section "Compound Actions" and the `is_selection_command` / `is_operator_command` notes), `docs/HELIX_KEYBINDINGS.md` (`.` rows: "Repeat last insert operation") and the `CMD_REPEAT` doc comment in `src/grammar/commands.rs` ("Repeat the last insert or change") SHALL state this rule and SHALL NOT claim compound-action or change repetition. Decision (recommended direction, 2026-11-12): match hx 25.07.1, no documented divergence; see Open Questions | dot-repeat parity bug | must |

> [!info] Former explicit gaps, closed 2026-11-11
> Counted `o`/`O` (#577), multi-cursor `o`/`O`/Enter newline indent (#546 case 6) and the unclosed-opener indent 0 (#546 case 4, needs typed auto-pair) were explicit gaps while Insert mode was single-cursor. They are specified as IMPLEMENTED by FR-053, FR-055 and the FR-039 amendments (#596, #597). The simulator still does not model `Ctrl-s` (`commit_undo_checkpoint`), tree-sitter indentation, or the hx trailing-newline quirk of 3.8.6 table E.

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
WHEN  a selection range would end past the document end after any command
                                      THEN every range is within 0..=len_chars and nothing panics (#569)
WHEN  the user presses Ctrl-c in Normal mode on a gameplay screen
                                      THEN toggle_comments runs and the game is not quit; in Insert mode the character `c` is inserted (#570, #599)
WHEN  the user presses ", a, Ctrl-c  THEN toggle_comments runs and the register is dropped (#599)
WHEN  the user presses v, Ctrl-c     THEN toggle_comments runs and the mode is Normal (#599)
WHEN  the user presses Ctrl-r after u THEN the change is redone (#592)
WHEN  the user presses ", a, Ctrl-d THEN the register is dropped and half_page_down runs (#610)
WHEN  the user presses Ctrl-c on the Arcade countdown, pause or game-over screen
                                      THEN the application quits and the countdown counts the game as played (#616)
WHEN  the user presses f, Ctrl-c      THEN the target is the character `c` (modifier ignored, as hx)
WHEN  the user presses Ctrl-c in the `/` prompt
                                      THEN the prompt closes and the game continues
WHEN  the user presses m, m on `(a)` with the cursor on `)`
                                      THEN the selection collapses onto the matching `(` (#579)
WHEN  three adjacent ranges and one separate range exist and Alt-_ runs
                                      THEN only the touching ranges merge; Alt-- merges all four (#580)
WHEN  the user presses m, i, w, *     THEN the word is the search pattern with \b boundaries, the selection is unchanged and n finds the next match (#581)
WHEN  the user presses x, c on a line THEN the line break stays and the cursor is on an indented empty line (#587)
WHEN  the user presses I on `    foo` THEN the cursor is before `foo`; on an empty line inside a block it is auto-indented (#588)
WHEN  the user presses a, x, y, Escape on a selection `ab`
                                      THEN the selection runs from the original start through the inserted `xy` (#589)
WHEN  the user presses f followed by a non-ASCII character
                                      THEN the find uses that character (#556)
WHEN  the user types a count before r, v, i, a, I, A, y or C
                                      THEN the count is accepted (ignored, or repeated for C) (#577)
WHEN  the user presses j, o, xyz, Enter, abc, Escape
                                      THEN the inserted lines are indented by 4 spaces
WHEN  the user presses w, i, a, Escape on scenario 14
                                      THEN the first line starts with `afn add(` (#593)
WHEN  the user presses A on an empty line inside a block
                                      THEN it is auto-indented; A, Escape keeps the indent (#594)
WHEN  the user presses A, Ctrl-k at the end of `ab\r\ncd`
                                      THEN the buffer is `abcd` and nothing panics (#595)
WHEN  the user presses j, 2, o, Escape inside a block
                                      THEN two indented lines are opened and the indent is kept (#596)
WHEN  the user presses j, o, Escape inside a block
                                      THEN the opened line is stripped (#596)
WHEN  the user types ( x ) y in Insert mode on an empty rs buffer
                                      THEN the buffer is `(x)y` (#597)
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
WHEN  the user presses l, i, ., Escape on `ab`  THEN the buffer is `a.b` (#607)
WHEN  the user records A, x, Backspace, y, Escape on a line and presses . on `cd`
                                      THEN the replay gives `cdy` (#608)
WHEN  the user presses Backspace after only an indent of 4 spaces
                                      THEN one indent unit is removed; with tabs one tab (#609)
WHEN  the user presses w, i, a, Escape on scenario 14 and then d
                                      THEN d acts on the reversed range `fn ` as in hx (#611)
WHEN  the user presses a, Tab, Escape on scenario 14
                                      THEN no indentation is inserted and the cursor follows FR-059 (#583)
WHEN  the user presses /, $, Enter, d on scenario 14
                                      THEN lines 1 and 2 are joined (#584)
WHEN  the user presses l, C, C, C, d on scenario 14
                                      THEN lines 1-3 each lose the character at column 1 and the closing brace line is untouched (#585; a second d keeps the closing brace, #625)
WHEN  the user presses Alt-* on a point cursor
                                      THEN the escaped character under it is the search pattern (#600)
WHEN  the user presses Ctrl-d / Ctrl-u / Ctrl-f / Ctrl-b on a 60-line document
                                      THEN the selection and the view match the hx goldens of 3.8.8 (#541)
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
| Counted `o`/`O` (`3o`), multi-cursor `o`/`O`/Enter | Supported (FR-053): N lines per range, one cursor per opened line; the count never leaks into following keys |
| `o`, `Escape` on the last line of a buffer without a trailing newline | The new indented line is NOT stripped (FR-054, hx composed change set has no trailing retain) |
| `A Ctrl-k` / `A Delete` at the end of the last line | No-op, no panic (FR-052) |
| Insert-mode `Ctrl-c` | Inserts `c` (FR-042) |
| An unparsable key token in a command string (#556) | Dropped and logged, never substituted by another key |
| `o`/`O`/Enter after a line that ends in an unclosed `{`, `(` or `[`, opens a bracket closed later (`rs`, `py`) or ends in `:` (`py`) | One extra indent unit by the rule of FR-039; constructs the rule does not see (`lambda:`, block comments, multi-line strings, partially parsed code) follow the remaining divergences in 3.8.4 |
| `>` on an empty line | The line is left unchanged (hx skips empty lines) |
| `<` on a line with less leading whitespace than one unit | Removes only what exists, no error |
| `J` where the next line has no comment token, or the language has no comment tokens (`md`) | Whitespace-only join as before |
| Scenario `language` outside `rs`, `md`, `py` (or empty) | Scenario load error (FR-028, #544), never defaulted silently |
| `Tab` after text in a line with no enclosing bracket pair and no later block | Nothing is inserted and the cursor stays (FR-059) |
| `C` on a line whose next line is shorter than the cursor column | That line is skipped, the copy lands on the next long enough line, or no copy is made (FR-061) |
| `Ctrl-d` near the document end | The view stops at the last line; every range still moves the clamped distance (3.8.8) |
| `&` or `K`/`Alt-K` where `hx` reports an error (no match, invalid regex) | Surface a `UserError` through the existing path; selection unchanged |
| Existing scenarios whose target was authored against the old (wrong) behavior | Target and `optimal_count` are corrected to hx behavior in the same change; `cargo nextest run scenario` must pass |
| Command strings with counts (`3fa`, `2G`) | Must be accepted by the simulator before the input layer emits them (see [[regex-selection-and-macro-commands/spec#10. Bug-Fix Batch: Input Chords, Counted Commands, Surround and Primary Selection (#487, #493, #502, #503, #455)\|regex-selection-and-macro-commands section 10]]) |

### 3.8.3 Accepted deviations from hx (this batch)

| Deviation | Rationale |
|-----------|-----------|
| `;` collapses all ranges to a block cursor | Accepted simplification of `collapse_selection` for the trainer |
| `state()` reports a cursor on the empty line after a final `\n` as the end of the last real line | Display/target comparison uses real lines only |
| `Escape` immediately after `o`, `O` or Enter removes the auto-inserted indent | Same as hx `try_restore_indent`, by the exact rule of FR-054 (not for counted, multi-cursor or last-line-without-newline sessions) |

### 3.8.4 Deferred and residual divergences

- Full tree-sitter indentation for `o`/`O`/Enter is NOT implemented. The user chose the per-language rule of FR-039 over adding tree-sitter and grammar dependencies. The residual fidelity gap (newline indent in situations outside the recorded golden cases) is tracked by one follow-up defect issue (P3); the divergences found while recording the goldens are listed here and attached to that issue.

| Case | hx 25.07.1 result | Simulator result | Status |
|------|-------------------|------------------|--------|
| `rs`: `let a = foo(1,` then `o` (bracket closed later) | 4 spaces (argument continuation) | 4 | matches |
| `rs`: blank line inside a block, `o`/`O` on the blank line | 4 | 4 | matches |
| `rs`/`py`: loaded, syntactically broken text with an unclosed trailing opener (`fn f() -> u8 {`, no `}`), Enter at line end | 0 (parse error) | 0 (FR-039 amendment 4, FR-055) | matches (after #597) |
| `rs`/`py`: `let a = foo(1,` with no closer, `o` | 0 | 0 | matches |
| `rs`: `// foo (` then `o` (any comment line; `py`: `# note:`) | continues the comment (`// y`, `# y`) | continues the comment; `Escape` keeps `// ` | matches |
| `rs`/`py`/`md`: Enter between an auto-pair (`fn f() {}`) | `}` on its own line (`{` / `    y` / `}`) | same | matches |
| `py`: `return`, `pass`, `break`, `continue`, `raise` line then `o` | dedents to 0 | dedents one unit (floor 0) | matches |
| `py`: `x = {`, `x = [`, `x = foo(` (closed later) then `o` | +1 unit | +1 unit | matches |
| `py`: `f = lambda:` then `o`; a `:` in a dict or slice | 0 | 0 (FR-039 amendment 1: only compound-statement keyword lines at bracket depth 0 open) | matches (after #546) |
| `py`/`rs`: `:` inside a string at the cursor | no block | no block (amendment 2) | matches (after #546) |
| `rs`/`py`: brackets inside block comments, raw strings, triple-quoted or multi-line strings | not counted | not counted (lexer state carried from document start, amendment 3) | matches (after #546) |
| Mismatched or partially parsed code (unclosed inner bracket with a closed outer one, `if x:` unclosed, `'` pairing for other languages) | tree-sitter error recovery | approximated by bracket matching | open |
| Multi-cursor `o`/`O`/Enter, counted `o`/`O` (`3o`) | the rule applies to every range; `3o` opens 3 lines with 3 cursors | same (FR-053) | matches (after #596) |
| `O` right after the closing line of a multi-line statement | not exhaustively probed | `o` rule below the previous non-blank line | open (not probed) |
| `rs`/`py`: Enter between mismatched halves (`x(]`) | `x(` / `    y]` | same | matches |
| `md`: `- a`, `1. a`, `> q` then `o` | copies indent | copies indent | matches |

All `open` rows above for newline indent are tracked by one follow-up issue, #546; its cases 1-5 are implemented by the FR-039 amendments, and case 6 (multi-cursor Insert, together with counted `o`/`O`, #577) by FR-053 (#596).

The former note on `Ctrl-d`/`Ctrl-u`/`Ctrl-f`/`Ctrl-b` (fixed page size, so Select-mode extension by these keys, FR-036, could not be golden-compared) and on the fixed tab width of 4 is replaced by FR-064 and FR-063. The residual rows of those requirements are listed in the table "Residuals of the 2026-11-11 batch" below.

**Residuals of the 2026-11-11 batch** (recorded after implementation; hx 25.07.1 versus the simulator):

| Case | hx 25.07.1 result | Simulator result | Status |
|------|-------------------|------------------|--------|
| `rs`: `f( a Tab` (FR-059, #583) | cursor jumps to the end of the enclosing function (tree-sitter parent node) | cursor moves to just after the `)` of the innermost enclosing pair | open: exact tree-sitter parent-node fidelity needs a syntax tree (follow-up issue filed with the PR) |
| `rs`: `l l i Tab z` on `x(a)` (FR-059, #583) | `x(az)` (the cursor is already at the end of the word inside the brackets, so `Tab` leaves it in place and `z` is typed before `)`) | `x(a)z` (the cursor moves after the closer) | open, same follow-up |
| `py` and `md`: `A Tab` after text (FR-059) | moves to the end of the root node (document end) | stays | open, same follow-up |
| Viewport: soft-wrap, horizontal view, jumplist and picker centering (FR-064, #541) | view scroll and centering follow soft-wrapped rows, the horizontal offset, jumps and pickers | not modeled (soft-wrap off, no horizontal view, no jumplist or picker) | accepted residual; paging by the RENDERED editor height is the P4 follow-up #629 (open, deferred by user decision: the fixed 24-row logical height of D-541 = B stays on every screen) |
| View update while typing a search (FR-064, FR-060) | hx updates the view on every keystroke of the `/` or `?` prompt, so `G ? L10 Enter Ctrl-f` ends on line 34 | the view is updated only when the search is confirmed, so the same keys end on line 29 | open (residual M1 of the implementation critique) |
| Macro keys (FR-025) | `Q` starts and stops recording and `q` replays (`Q w i x Escape Q q q` gives `xxx`) | same | closed, #631 (the keys were inverted before) |
| `zz` (FR-064) | `align_view(Center)` with `relative = (H-1)/2`, no clamp at the document end | same (`ViewState::center_on_line`: top = line - (H-1)/2, no end clamp) | closed, #626 |
| `l C C C d d` (FR-061, #585) | the second `d` keeps the closing `}` | same (no cursor clamp in `delete_selection_impl`) | closed, #625 |
| `Alt-;` on a multi-range selection | flips every range | same (`flip_selections` uses `Selection::transform`) | closed, #623 |
| `/`, `s`, `S`, `K` and `Alt-K` with an all-lowercase pattern (FR-060) | smart-case: case-insensitive match | same (smart-case) | closed, #624 |
| Macro length (FR-025) | a recorded macro keeps every command | the recorder silently stops accepting commands at `MAX_MACRO_LENGTH` (100), so a longer macro is truncated; NO parity is claimed | open, pre-existing defect, #635 |
| Named macro registers and a shared `.`/macro recorder (FR-025) | `"<reg>Q` records into a register; one recording mechanism | one unnamed macro slot; separate `.` and macro recorders | open, tracking only: #619 and #620 |
| Long Insert sessions replayed with `.` (FR-053) | the full session replays | same (the caps are removed; the whole recorded session is replayed in order) | closed, #622 |

Scenario corrections: `dedent_001` and `dedent_line_001` were rewritten with 4-space nesting, because hx auto-detects an 8-space unit for an 8-space file (FR-037) and `<` removes all 8 spaces.

### 3.8.5 Success Criteria

| ID | Metric | Target |
|----|--------|--------|
| SC-101 | Each criterion in 3.8.1 exists as a simulator test asserting buffer and selection | Pass |
| SC-102 | A test over the registry confirms every command has a `CountBehavior` and the game layer contains no count-by-repeat loop | Pass |
| SC-103 | `cargo nextest run scenario` after target corrections | Pass |
| SC-104 | Differential tests for FR-028..FR-035 (indent, Tab, `&`/`K`/`Alt-K`, replay order, `R`, Select mode) assert buffer and selection against recorded `hx` 25.07.1 results; the FR-029 language-coverage test passes | Pass |
| SC-105 | Goldens for FR-039 recorded with `hx -c <empty config>` pass; the FR-037 detection, `&` display-width and FR-038 comment-token cases and the FR-036 Select-mode cases assert buffer and selection; the divergence table of 3.8.4 is complete | Pass |
| SC-106 | Every golden of 3.8.6 (tables A-D) exists as a simulator test asserting the exact buffer; the FR-052 CRLF property test and the FR-055 one-char-at-a-time auto-pair cases pass; the code-fence scenario completes with 13 commands (Task and MiniGame) | Pass |
| SC-107 | Every golden of FR-059..FR-064 and of the FR-050, FR-053 amendments is a simulator test asserting buffer AND selection; the viewport goldens of 3.8.8 exist for every listed case; the FR-065 bench ratio is at most 1.5; the opt-in live differential of FR-040 reports no drift | Pass |
| SC-108 | The differential check of 3.8.9 (every sequence of the table, run against real `hx` 25.07.1 and the simulator on the same text) shows identical buffers; `cargo nextest run scenario` and the solve-all scenario test pass with the rewritten repeat scenarios | Pass |

### 3.8.6 Recorded hx goldens for the #592-#599 batch (E0)

Recorded 2026-11-11 against `hx` 25.07.1 (a05c151b, the vendored tag) in tmux, started with `-c <config>` where the config sets only `auto-format = false` and `insert-final-newline = false`; each test file is byte-identical to the scenario content (no trailing newline) and `:w` output equals the buffer. Position is the hx statusline `line:col` (1-based, primary cursor). `C` is `copy_selection_on_next_line`; `Bs` is Backspace. Buffers are shown with `\n` and `\r\n` escapes.

**Table A: `Ctrl-c` (#599)**, content `fn calc(a: i32) -> i32 {\n    a + 1\n}` (`rs`)

| State | Keys | hx result |
|-------|------|-----------|
| Normal | `Ctrl-c` | line commented (`// fn calc...`) |
| Count | `2 Ctrl-c`, then `l` | commented, count consumed (`l` moves one column) |
| Register-name `"` | `" Ctrl-c`, then `y` | register `c` selected (`yanked to register c`), `"cp` pastes it |
| Register selected | `x "a Ctrl-c` | line commented, selection kept, register DROPPED: `x "ay j x "a Ctrl-c y ge "ap` pastes line 1 (`a` unchanged); a plain `y` afterwards fills the default register |
| `g`, `2g`, `z`, `m`, `[` | key, `Ctrl-c`, then `l`/`j`/`x` | no comment toggle; pending cancelled, the next key is plain |
| `ms` | `m s Ctrl-c` | surrounds with `c` |
| `md`, `mr` (from), `mr` (to), `ma`, `mi` | `m d Ctrl-c` etc. | behave as the plain character `c` (`md`, `mr` from: error "Surround pair not found around all cursors"; `mr` to: `fn calcca: i32c -> i32 {`) |
| `f`, `t`, `F`, `T`, `2f`, `r` | key, `Ctrl-c` | equal to `fc`, `tc`, `Fc`, `Tc`, `2fc`, `rc` |
| Prompts | `: Ctrl-c`, `: w Ctrl-c`, `/ Ctrl-c`, `/ a Ctrl-c`, `? Ctrl-c` | prompt closed, buffer untouched, next key plain |
| Insert | `i Ctrl-c x Escape` on `fn calc...` | `cxfn calc...` (inserts `c`, stays Insert); `o Ctrl-c Escape` opens `    c` |
| Select | `v Ctrl-c`, `v w Ctrl-c` | line commented AND mode Normal |
| `Z` sticky view | `Z Ctrl-c` | popup stays open (trainer has no `Z`) |
| MiniGame countdown, pause, game over | - | no hx equivalent: trainer decision |

**Table B: #593, #594, #595**

| Content | Keys | Buffer |
|---------|------|--------|
| scenario 14 (`combined.toml`) | `w i a Escape` | `afn add(a: i32, b: i32) -> i32 {\n    let sum = a + b;\n    sum\n}`; `w a a Escape` gives `fn aadd(`; `w Alt-; i a Escape` gives the same as `w i a Escape` |
| `fn f() {\n    x\n\n}` | `j j A y Escape` / `j j A Escape` / `j j I y Escape` | `fn f() {\n    x\n    y\n}` / `fn f() {\n    x\n    \n}` / `fn f() {\n    x\n    y\n}` |
| `a\n\nb` | `j A x Escape` | `a\nx\nb` (depth 0) |
| `fn f() {\n    x\n\n    y\n}` | `j x A y Escape` | `fn f() {\n    xy\n\n    y\n}` (cursor line, not head line) |
| `fn f() {\n    x\n\n}` | `j v j A y Escape` / `j j v k A y Escape` | `    y` on line 3 / `    xy` on line 2 |
| CRLF `ab\r\ncd` | `A x Escape` / `A Enter Escape` / `j A x Escape` | `abx\r\ncd` / `ab\r\n\r\ncd` / `ab\r\ncdx` |
| CRLF `ab\r\n\r\ncd` | `j A x Escape` | `ab\r\nx\r\ncd` |
| CRLF `fn f() {\r\n    x\r\n\r\n}` | `j j A x Escape` | `fn f() {\r\n    x\r\n    x\r\n}` |
| `ab\r\ncd` (also LF twin `ab\ncd`) | `A C-k Escape` | `abcd` |
| `ab\r\ncd` | `j A C-k Escape` (last line) | `ab\r\ncd` |
| `ab\r\ncd\r\nef` | `A C-k C-k Escape` | `ab\r\nef` |
| `ab\r\ncd` | `l A C-k Escape` / `l i C-k Escape` / `i C-k Escape` | `abcd` / `a\r\ncd` / `\r\ncd` |
| `ab\r\ncd` | `A C-d Escape`, `A Delete Escape` | `abcd` |
| `ab\r\ncd` | `A Bs Escape` / `j i Bs Escape` | `a\r\ncd` / `abcd` |
| `ab\r\ncd` | `A C-u Escape`, `A C-w Escape` | `\r\ncd` |
| `ab cd\nef gh\nij kl`, two cursors | `l C i C-k Escape` / `l C A C-k Escape` | `a\ne\nij kl` / `ab cdef ghij kl` |

**Table C: #596 `o`/`O` and the Escape strip**, `I1` = `fn f() {\n    a\n    b\n}`

| Keys | Buffer |
|------|--------|
| `j 2 o Escape` | `fn f() {\n    a\n    \n    \n    b\n}` (indent kept, 2 cursors) |
| `j 2 O Escape` | `fn f() {\n    \n    \n    a\n    b\n}` (kept) |
| `j 2 o x Escape` | two `    x` lines after `    a`, primary on the first (3:6) |
| `j 2 o x Bs Escape` | `...a\n    \n    \n    b...` (kept) |
| `2 o Escape` on `a\nb` | `a\n\n\nb` |
| `j C o Escape` (two cursors) | `fn f() {\n    a\n    \n    b\n    \n}` (kept) |
| `j C o x Escape` | `fn f() {\n    a\n    x\n    b\n    x\n}` |
| `j C O Escape` | `fn f() {\n    \n    a\n    \n    b\n}` (kept) |
| `j o Escape` (single) | `fn f() {\n    a\n\n    b\n}` (STRIPPED) |
| `j o x Bs Escape` | `fn f() {\n    a\n\n    b\n}` (stripped) |
| `j O Escape` | `fn f() {\n\n    a\n    b\n}` (stripped) |
| `j A Enter Escape` | `fn f() {\n    a\n\n    b\n}` (stripped) |
| `j o x Escape` / `j A Enter x Escape` | `...a\n    x\n    b...` |
| `j o Enter Escape` | `fn f() {\n    a\n\n    \n    b\n}` |
| last line without EOL, `fn f() {\n    a`: `j o Escape` | `fn f() {\n    a\n    ` (NOT stripped) |
| same: `j o x Bs Escape` / `j 2 o Escape` / `j O Escape` | `fn f() {\n    a\n    ` / `fn f() {\n    a\n    \n    ` / `fn f() {\n\n    a` |
| `j C o x Escape u` | back to `I1`, two ranges (one undo step) |
| `j C o x Escape gg .` | `fn f() {\n    x\n    a\n    x\n    b\n    x\n}` |
| `2 o x Escape j .` on `a\nb\nc` | `a\nx\nx\nx\nb\nx\nc` |

Two-cursor entry goldens on `ab cd\nef gh\nij kl` after `l C`: `i x Escape` gives `axb cd\nexf gh\nij kl`; `a x` gives `abx cd\nefx gh\nij kl`; `A x` gives `ab cdx\nef ghx\nij kl`; `I x` gives `xab cd\nxef gh\nij kl`; `i Enter` gives `a\nb cd\ne\nf gh\nij kl`; `i Bs` gives `b cd\nf gh\nij kl`; `a Bs` gives `a cd\ne gh\nij kl`; `i C-w` gives `b cd\nf gh\nij kl`; `A C-w` gives `ab \nef \nij kl`.

**Table D: #597 newline indent, auto-pair, pair Backspace**

| Content (language) | Keys | Buffer |
|--------------------|------|--------|
| `fn f() -> u8 {` (`rs`) | `o x Escape` / `A Enter x Escape` | `fn f() -> u8 {\nx` (indent 0) |
| `fn f() -> u8 {\n}` | `o x Escape` | `fn f() -> u8 {\n    x\n}` |
| `fn f() -> u8 {\n    a` | `o x Escape` on line 1 | `fn f() -> u8 {\nx\n    a` (indent 0) |
| `def f():` (`py`) | `A Enter x Escape` | `def f():\n    x` |
| `{` (`json`) | `o x Escape` | `{\nx` |
| empty (`rs`) | `i ( Bs Escape`, `i [ Bs Escape`, `i " Bs Escape` | empty |
| empty (`rs`) | `i ( x ) y Escape` / `i ( ) y Escape` / `i ( x Escape` | `(x)y` / `()y` / `(x)` |
| empty (`rs`) | `i " x " y Escape` / `i { Enter x Escape` | `"x"y` / `{\n    x\n}` |
| empty (`rs`, `md`) | `i d o n ' Escape` | `don'` |
| empty (`txt`, `md`) | `i ' Escape` | `''` |
| empty (`rs`) | `i ' Escape` | `'` |
| `ab` / ` ab` (`rs`) | `i ( Escape` | `(ab` / `() ab` |
| `ab\ncd` (`rs`) | `C i ( Escape` / `A ( Escape` / `C A ( Escape` | `(ab\n(cd` / `ab()\ncd` / `ab()\ncd()` |
| `\n\n` (`rs`) | `C i ( Escape` | `()\n()\n` |
| `  a\n  b` (`rs`, `md`) | `v l C i ( Escape` | `()  a\n()  b` |
| `ab cd\nef gh` | `v l C i ( Escape` | `(ab cd\n(ef gh` (next character alphanumeric: no pair) |
| `print("hello")` (`md`) | `O` + two or three backticks + `Escape` | two backticks give two; three give FOUR on line 1 |
| same | `O` three backticks `Escape j o` three backticks `Escape` | four backticks on each fence line |

**Table E: code-fence quirk.** `O`, three backticks, `Delete`, `Escape`, `j`, `o`, three backticks, `Delete`, `Escape` on `print("hello")` (`md`) gives the hx buffer and `:w` output `` ```\nprint("hello")\n```\n `` with a TRAILING `\n` that the scenario target lacks. Isolation: `A a Left Delete Escape` on `x` gives `x\n`; `i a Left Delete Escape` on an empty buffer gives `\n`; the quirk needs `Delete` of a character inserted in the same session that is the last buffer character (not seen with `Backspace`, `d`, a pre-existing character, or a non-last character); one `u` undoes it together with the session. Root cause not determined. The spec therefore claims simulator-output versus scenario-target parity for this scenario only (FR-057).

### 3.8.7 Scenario and test impact

- `scenarios/en/writing/code-fence.toml`: solution and `optimal_count` per FR-057; target unchanged.
- No other bundled solution changes. `cargo nextest run scenario` confirms, including scenarios that use `o`, `O`, `i` on a selection or `A`.
- Tests assert buffer AND selection; each golden of 3.8.6 becomes a simulator test; carriage return joins the FR-041 property-test alphabet.
- #541, #583, #584, #585, #600: the targets of `scenarios/en/movement/scroll.toml` and its header comment are recomputed against hx; `search_*` and `copy_selection_next_002` scenarios are re-validated or reworked against hx; `cargo nextest run scenario` stays green.
- #544: no bundled scenario changes (the nine `md` and three `py` scenarios stay valid; the rest use the default `rs`).

### 3.8.8 Viewport model (FR-064, #541)

User decision D-541 = B (2026-11-11): a fixed logical view height with a port of the hx scroll, not the rendered terminal height. Reference: `hx` 25.07.1 (helix-term `commands.rs` 1810-1917 `scroll`, `default.rs` 185-188, helix-view `view.rs` 231-300 and 335-347). The residual rows (soft-wrap, horizontal view, jumplist and picker centering) are listed in 3.8.4; `zz` is `align_view(Center)` with `relative = (H-1)/2` and no end-of-document clamp (#626).

One logical viewport: `ViewState` with H = 24 inner rows (the existing `DEFAULT_VISIBLE_LINES`). `PAGE_SIZE` is deleted, and `ViewState.top_line` plays the role of the hx view anchor line. Soft-wrap is off, so visual rows equal lines. One ported `scroll(offset, direction, sync_cursor)` serves every key below:

- `Ctrl-d` and `Ctrl-u` are hx `page_cursor_half_down` and `page_cursor_half_up` (`default.rs` 187-188), that is `scroll(H/2, direction, sync_cursor = true)`. The view moves by 12 lines, clamped at the document ends. Every range moves 12 lines vertically and keeps its column (hx `move_vertically_visual`); Select mode extends.
- `Ctrl-f` and `Ctrl-b` are hx `page_down` and `page_up` (`default.rs` 185-186), that is `scroll(H, direction, sync_cursor = false)`. The view moves by 24 lines. The early returns of the hx tail are kept: forward when `band_top <= cursor`, backward when `band_bottom >= cursor`. Otherwise the PRIMARY range is replaced by a point at column 0 of the band edge: row `top + so` going forward, row `top + H - so - 1` going backward. Select mode keeps the anchor.
- `zj` and `zk` (simulator `scroll_down` and `scroll_up`) are hx `scroll(count, direction, false)` and are routed through the same function, so there is only one viewport. `Ctrl-e` and `Ctrl-y` are NOT bound in hx 25.07.1 (the keymap has only `zj`, `zk` and the sticky `Z` view mode, which the trainer does not model), so the simulator binds no scroll to them.
- After EVERY command (Normal, Select and Insert keys, also inside `.` and macro replay) the simulator applies hx `ensure_cursor_in_view` (`view.rs` 335; 231-300 with CENTERING = false). With `so_top = min(5, (H-1)/2)` and `so_bottom = min(5, H/2)`: if `cursor_line < top + so_top` then `top = cursor_line - so_top` (saturating); if `cursor_line + so_bottom >= top + H` then `top = cursor_line - (H - so_bottom - 1)` (saturating). The view moves, never the cursor.
- Search centering (hx `search_impl` `commands.rs` 2211, `ensure_cursor_in_view_center`, `view.rs` 341-347): after `/`, `?`, `n` or `N` place a match, if the primary cursor line lies outside `[top, top + H - 1]` (no scrolloff) then `top = cursor_line - (H-1)/2` (saturating; hx `align_view` Center with `relative = (H-1)/2`, helix-view `lib.rs` 51-74; 11 for H = 24). Golden evidence on a 70-line document: `/ L50 Enter C-f` leaves the cursor on line 68 (top 39 = 50 - 11), and `/ L50 Enter C-b` gives 33; otherwise nothing changes. The post-command ensure then runs. `*` and `Alt-*` do not move the cursor, so they do not center. hx also centers on jumplist jumps and pickers, which the simulator does not have (a residual row).
- Golden setup: hx 25.07.1 started with `hx -c <empty config>`, an 80x26 terminal (24 inner rows, one status line, one command line, bufferline off), and a document of at least 60 lines. Cases: `Ctrl-d`, `Ctrl-u`, `Ctrl-f` and `Ctrl-b` from the top, the middle and near the end; `v` then `Ctrl-d`; two cursors then `Ctrl-d`; `/<far match> Enter Ctrl-u`; `n Ctrl-d`; `zj` then `Ctrl-f`; `/ L50 Enter C-f` and `/ L50 Enter C-b` on a 70-line document. Each case asserts the selection (and the text). The partial closure of #541 (paging by the RENDERED editor height) is the P4 follow-up #629, open and deferred by user decision.

### 3.8.9 Dot-repeat replays only the last insert (FR-066)

> [!bug] Finding
> P2, Helix fidelity. The simulator records and replays selection plus operator compounds and other edit commands under `.`, so three bundled repeat scenarios teach `d .`, `x d .` and `> j .`, which do nothing in hx 25.07.1.

Recorded hx goldens. Text `fn add(a: i32, b: i32) -> i32 {\n    let sum = a + b;\n    sum\n}`; keys are prefixed by `~` (case switch, not an insert) so no insert precedes. hx behavior is the oracle; the "Trainer today" column is the defect.

| Keys | hx 25.07.1 | Trainer today |
|------|-----------|---------------|
| `~ x d .` | `.` does nothing | second line removed |
| `~ x d . .` | text untouched by `.` | everything deleted |
| `~ d .`, `~ w d .`, `~ w d j .` | `.` does nothing | repeats the delete |
| `~ > .` | one indent only | second indent |
| `~ J .` | one join only | second join |
| `~ y p .`, `~ x y p .` | one paste only | extra paste |
| `i Z Escape g g x d .` | `.` re-inserts `Z` (last insert) | repeats the delete |
| `i ab Escape j .`, `a`, `A`, `o`, `O`, `c <text> Escape` forms | replay of the insert session | already matches (#607, #608, #622, #641); MUST stay green |

**Scenarios to rewrite** (all in `scenarios/en/repeat/basic-repeat.toml`, which keeps 5 scenarios):

| Scenario id | Current solution | Problem |
|-------------|------------------|---------|
| `repeat_delete_char_001` | `d .` | `.` does not repeat `d` in hx |
| `repeat_delete_line_001` | `x d .` | `.` does not repeat `x d` in hx |
| `repeat_indent_001` | `> j .` | `.` does not repeat `>` in hx |

Unchanged and correct: `repeat_insert_001` (`a x Escape .`). `repeat_replace_001` (`r_ llll .`) is NOT confirmed by the finding: the differential check MUST include it, and it is rewritten too if hx does not repeat `r`.

Requirements for the rewrite:

- Every canonical solution of every scenario in the file SHALL produce the target in real hx 25.07.1, not only in the simulator.
- Each rewritten scenario SHALL still teach `.` through an Insert-session repeat (for example append or open-line text on one place, move, `.` on another) or SHALL be re-expressed with a count, `d` per line, or a non-`.` solution; the scenario IDs and the scenario count of the file are kept so progress and FSRS cards keyed by id survive; `name`, `description`, `solution.description`, `optimal_count` and `target` are updated to match.
- `commands_taught` coverage of [[scenario-coverage-for-implemented-commands/spec]] stays intact: `.` remains taught by at least one scenario, and `x` and `d` stay covered by the scenarios of that package (the removal of `["." , "x", "d"]` from `repeat_delete_line_001` SHALL NOT create an allow-list entry; if `x`/`d` lose their only teaching scenario elsewhere, the rewritten scenario keeps them in `commands_taught`).
- Tags drop `delete`/`indent` repeat claims that no longer hold.

**Acceptance criteria**

```
GIVEN the text above and no earlier Insert session
WHEN the keys `~ x d .` run
THEN the buffer equals the buffer after `~ x d` (the `.` is a no-op), as in hx

GIVEN an Insert session `i Z Escape` followed by `g g x d`
WHEN `.` runs
THEN `Z` is inserted again at the cursor, as in hx

GIVEN every sequence of the table above
WHEN run against the simulator and against real hx 25.07.1
THEN buffers and primary selections match

GIVEN the rewritten basic-repeat.toml
WHEN the solve-all scenario test and each solution replayed in hx run
THEN every scenario reaches its target and the file still has 5 scenarios
```

**Verification method**: (1) simulator tests, one per table row, asserting buffer AND selection; (2) differential check by driving real `hx` 25.07.1 through tmux with the same text and keys; (3) `cargo nextest run scenario` plus the solve-all test; (4) `cargo nextest run --workspace --all-features --lib --bins`; (5) docs build with broken intra-doc links denied (the doc edits of FR-066).


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
> - [NEEDS CLARIFICATION: whether upstream will publish a tag before the master changes listed here are altered or reverted; re-verify the compare at bump time.] Re-verified 2026-11-12: no tag yet, master diverged (ahead 1004, behind 2).

> [!question] Dot-repeat parity (FR-066)
> - [NEEDS CLARIFICATION: confirm no documented divergence. Recommended and specified: match hx 25.07.1 exactly. A divergence (repeating `x d`, `>`) would be friendlier but teaches commands that fail in real Helix, defeating the trainer's purpose, so none is proposed.]
> - [NEEDS CLARIFICATION: does `r` (`repeat_replace_001`) repeat in hx 25.07.1? Settle by the differential check; the finding says replace already matches.]

## 9. See Also

- [[constitution]] - project principles, including the pinned-core invariant
- [[custom-helix-keymap-support/spec]] - command-name resolution keys affected by renames
- [[register-command-mode-support/spec]] - register and `:` mode coverage relevant to `:set-register`
- [[undo-insert-session-granularity/spec]] - one undo step per Insert session, including multi-cursor (FR-013)
- [[module-layering-and-typed-commands/spec]] - typed `NormalCommand` and module layering
- [[duplicate-scenario-ids/spec]] - scenario ID uniqueness guard
- [[MOC-specs]] - all specifications
