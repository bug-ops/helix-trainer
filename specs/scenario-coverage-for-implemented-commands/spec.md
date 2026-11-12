---
aliases:
  - Scenario Coverage for Implemented Commands
  - Untaught Helix Commands
tags:
  - sdd
  - spec
  - scenarios
  - content
  - status/proposed
created: 2026-11-12
status: proposed
related:
  - "[[constitution]]"
  - "[[regex-selection-and-macro-commands/spec]]"
  - "[[helix-core-post-25-07-1-upgrade-readiness/spec]]"
  - "[[optimal-count-action-counter-unit/spec]]"
  - "[[training-progress-indicator/spec]]"
  - "[[duplicate-scenario-ids/spec]]"
  - "[[MOC-specs]]"
---

# Feature: Scenario Coverage for Implemented Helix Commands

> [!info] Metadata
> **Issues**: #642 (movement), #643 (selection manipulation), #644 (select mode and search), #645 (editing), #646 (view commands, P4). Group A to D are P3, group E is P4. Research-sourced; follows the closed coverage issue #198, which covered only regex select/split and macros.
> **Status**: Proposed
> **Depth**: Lightweight spec, per this project's SDD scaling guidance. Scenario content plus one guard test. No simulator, scoring or UI change, except where [NEEDS CLARIFICATION] items in section 7 say otherwise.

## 1. Overview

### Problem Statement

At HEAD `bc064f4` the simulator implements and unit-tests about 35 real Helix 25.07.1 commands that no bundled scenario teaches: no `solution.commands` token and no `metadata.commands_taught` entry under `scenarios/en/**` references them. FSRS review items, daily quests, achievements and the adaptive arcade selection are all driven by `commands_taught`, so these commands can never be practised, scheduled or reviewed. A player who learns them from Helix itself gets no feedback in the trainer, and a regression in any of them is caught only by simulator unit tests, never by the scenario validation suite.

Verified at HEAD by parsing all 160 `[[scenarios]]` entries (100 distinct solution tokens, 80 distinct `commands_taught` values) and diffing against the `CMD_*` constants of `src/grammar/commands.rs` that are registered in `src/helix/registry/definitions/*.rs` or used by `src/helix/simulator/**`.

| Group | Untaught commands (key) | Registry name |
|-------|------------------------|---------------|
| A. Movement | `W`, `B`, `E`; `G`; `Alt-.` | long-word motions, `goto_line`, `repeat_last_motion` |
| B. Selection manipulation | `;`, `Alt-;`, `Alt-,`, `&`, `Alt--`, `Alt-_`, `K`, `Alt-K` | `collapse_selection`, `flip_selections`, `remove_primary_selection`, `align_selections`, `merge_selections`, `merge_consecutive_selections`, `keep_selections`, `remove_selections` |
| C. Select mode and search | `v`; `?`; `Alt-*` | `select_mode`, `rsearch`, `search_selection` |
| D. Editing | `Alt-d`, `Alt-c`, `` Alt-` ``, `Ctrl-c`; Insert-mode `Ctrl-w`, `Ctrl-k`, `Alt-Backspace` | `delete_selection_noyank`, `change_selection_noyank`, `switch_to_uppercase`, `toggle_comments`, Insert deletions |
| E. View (P4) | `zz`, `zt`, `zb`, `zm`, `zj`, `zk` | `align_view_*`, `scroll_down`, `scroll_up` |

Exact registry names are taken from `src/helix/registry/definitions/` and MUST be re-read before implementation.

> [!note] Verified exclusions from the list
> `^` (`goto_first_nonblank`) and `Ctrl-r` are `Binding::TrainerAlias` entries with no upstream command and are out of scope. `Alt-x` (`shrink_to_line_bounds`) is untaught but already tracked by #141 and is out of scope here. `"` (register select) and `:` (command line) have no scenario of their own but are taught through the combined tokens `"ay`, `"aP` and `:goto`, so they count as covered. The header comment of `scenarios/en/editing/advanced-editing.toml` names `` Alt-` ``, but no scenario in that file uses it: it counts as untaught.

### Goal

Every registered Helix-bound command is either taught by at least one scenario that passes `cargo nextest run scenario`, or is on an explicit, reasoned allow-list, and a test keeps it that way.

### Out of Scope

- Trainer-only aliases (`Binding::TrainerAlias`): `^`, `Ctrl-r`.
- `Alt-x` (`shrink_to_line_bounds`): tracked by #141.
- Commands the simulator does not implement (window mode `Ctrl-w`, jumplist, pickers, `Z` sticky view mode, soft-wrap and horizontal view). New simulator behavior is a separate spec; a coverage scenario MUST NOT be written against behavior that differs from `hx` 25.07.1.
- Any change to scoring, progress, FSRS, quest or achievement logic. Newly taught commands flow into them through the existing `commands_taught` mechanism.
- Translations of scenario content: `scenarios/` contains only the `en` locale today.

## 2. User Stories

### US-001: Practise every command the simulator supports
AS A learner of Helix
I WANT a scenario for each Helix command the trainer simulates
SO THAT I can practise it and have it scheduled for review.

**Acceptance criteria:**
```
GIVEN the bundled scenarios
WHEN  every registered Helix-bound command is looked up in solution.commands and commands_taught
THEN  each is found, or is on the documented allow-list with a reason
```

### US-002: Review newly taught commands
AS A returning player
I WANT commands such as `W`, `Alt-d` or `K` to appear in my FSRS review queue after I complete their scenario
SO THAT I retain them.

**Acceptance criteria:**
```
GIVEN a completed scenario whose commands_taught contains "W"
WHEN  the completion is recorded
THEN  "W" is a tracked card in the profile and is surfaced by the scheduler when due
```

### US-003: Catch a silently untaught command
AS A maintainer
I WANT a failing test when someone registers a new Helix-bound command without a scenario
SO THAT the coverage gap cannot reopen.

**Acceptance criteria:**
```
GIVEN a new Binding::Helix entry in the registry and no scenario or allow-list entry for it
WHEN  cargo nextest run is executed
THEN  the guard test fails and names the command key and its Helix name
```

## 3. Functional Requirements

### 3.1 Common rules (apply to every group)

| ID | Requirement | Priority |
|----|------------|----------|
| FR-001 | EVERY new scenario SHALL be a `[[scenarios]]` entry with `setup`, `target`, `solution`, `scoring` and `metadata` in the existing format, and SHALL pass `cargo nextest run scenario`, in particular `test_all_scenarios_execute_solution` (the canonical `solution.commands`, typed key by key as the UI does, completes the scenario) | must |
| FR-002 | EVERY new scenario SHALL have `scoring.optimal_count == solution.commands.len()` with each entry one command in the counter's unit ([[optimal-count-action-counter-unit/spec]] FR-001, FR-004: a combined token such as `md(` or `K foo` is one entry), `max_points = 100`, and `tolerance` of 0 unless a documented alternative solution of equal intent exists (then 1). Higher tolerance values are not used | must |
| FR-003 | EVERY new scenario `id` SHALL be unique across all of `scenarios/en/**` (the existing `test_all_scenario_ids_are_unique` enforces it) and follow the `<topic>_<nnn>` convention of the neighbouring files, for example `long_word_forward_001` | must |
| FR-004 | EVERY new scenario SHALL list the taught command in `metadata.commands_taught` in its normalized form (`test_bundled_commands_taught_are_normalized`: `normalize_command_id(c) == c`); a typed-pattern form such as `K foo` is normalized by the input layer to the skill id, so the implementer SHALL write the id that `normalize_command_id` returns and verify it with that test. Prerequisite commands used only as setup (for example `x` before `K`) are NOT listed unless the scenario also teaches them | must |
| FR-005 | EVERY new scenario SHALL set `metadata.category` to a value of the closed `ScenarioCategory` enum (`movement`, `editing`, `clipboard`, `search`, `selection`, `text_objects`, `advanced`), `metadata.difficulty` to `beginner`, `intermediate` or `advanced`, and `metadata.tags` to lowercase kebab-case words: the topic, the command name and the group (for example `["selection", "merge", "multi-cursor"]`); tags are free-form and are not validated | must |
| FR-006 | EVERY new scenario SHALL have two or three `hints` that name the command and what it does without giving the full key sequence away, and a `solution.description` stating why the sequence is optimal | must |
| FR-007 | EVERY new scenario's `target` SHALL differ from its `setup` in text, cursor or selection, so the progress indicator starts below 100% and reaches 100% only on completion ([[training-progress-indicator/spec]] FR-001, FR-002); a scenario whose target equals its setup is a defect | must |
| FR-008 | EVERY new scenario SHALL be unambiguous: the canonical solution is the shortest sequence that reaches the target, and the expected selection SHALL match `hx` 25.07.1 for the same keys (the reference of [[helix-core-post-25-07-1-upgrade-readiness/spec]] section 3.8); a scenario that depends on a known simulator deviation is not written until the deviation is fixed | must |
| FR-009 | EVERY new scenario file SHALL be registered in `src/config/scenarios/embedded.rs` (`include_str!` constant plus the per-locale list), since the binary loads only embedded scenarios; a scenario file that is on disk but not embedded SHALL fail a test (see FR-031) | must |
| FR-010 | EVERY new scenario SHALL use source text in a language of the closed `Language` enum (`rs`, `md`, `py`; default `rs`) and set `setup.language` when it is not Rust; `Ctrl-c` scenarios (FR-033) set it explicitly | must |

### 3.2 Group A: movement (#642)

| ID | Requirement | Priority |
|----|------------|----------|
| FR-011 | THE SYSTEM SHALL provide at least one scenario each for `W`, `B` and `E`, where the text contains punctuation-joined tokens such as `foo.bar(baz)` so that the long-word motion (whitespace-delimited) and the short-word motion (`w`/`b`/`e`) give different targets; difficulty `beginner` for `W`, `intermediate` for `B` and `E`. File: `scenarios/en/movement/long-word.toml` (new), category `movement`, tags include `word`, `long-word` | must |
| FR-012 | THE SYSTEM SHALL provide at least one scenario for `G` using a count (`12G` as one entry, as `3h` is today), difficulty `intermediate`, in a document of at least 20 lines so that the target is not reachable cheaply with `j`. File: `scenarios/en/movement/goto-commands.toml` (append), category `movement`. Reference: `G` with a count is `goto_line`; a bare `G` is a different command and MUST NOT be conflated | must |
| FR-013 | THE SYSTEM SHALL provide at least one scenario for `Alt-.` (`repeat_last_motion`): a `f<char>` or `t<char>` motion followed by `Alt-.` that lands on the next occurrence; difficulty `intermediate`. File: `scenarios/en/movement/find-till.toml` (append), category `movement` | must |

### 3.3 Group B: selection manipulation (#643)

All group B scenarios go into `scenarios/en/selection/selection-manipulation.toml` (new), category `selection`, tags include `multi-cursor` where several ranges are involved.

| ID | Requirement | Priority |
|----|------------|----------|
| FR-014 | THE SYSTEM SHALL provide at least one scenario each for `;` (collapse to a cursor) and `Alt-;` (flip anchor and head), difficulty `beginner` and `intermediate` respectively; the target selection or cursor SHALL make the two ends distinguishable (head position, not only extent) | must |
| FR-015 | THE SYSTEM SHALL provide at least one scenario each for `Alt-,` (remove the primary selection) and `Alt--` (merge selections) and `Alt-_` (merge consecutive selections), difficulty `intermediate`, starting from a multi-selection produced by existing taught commands (`s foo`, `C`, `%`); the `Alt-_` scenario SHALL contain ranges for which `Alt--` and `Alt-_` give different results, otherwise the scenario teaches nothing specific to `Alt-_` | must |
| FR-016 | THE SYSTEM SHALL provide at least one scenario for `&` (`align_selections`) with a text whose target differs from the setup in whitespace only, difficulty `advanced` | must |
| FR-017 | THE SYSTEM SHALL provide at least one scenario each for `K` (keep matching) and `Alt-K` (remove matching), difficulty `intermediate`, using one combined entry per prompt command (`K foo`, `Alt-K foo`, same convention as `s foo` in `select-all-replace.toml`) and the selection semantics of [[regex-selection-and-macro-commands/spec]] FR-123 | must |

### 3.4 Group C: select mode and search (#644)

| ID | Requirement | Priority |
|----|------------|----------|
| FR-018 | THE SYSTEM SHALL provide at least two scenarios for `v` (`select_mode`): one that extends a selection with a single motion in Select mode and leaves with `Escape` or `v`, and one that combines Select mode with a text edit (for example `v w w d`); difficulty `beginner` and `intermediate`. File: `scenarios/en/selection/select-mode.toml` (new), category `selection`, tags include `select-mode` | must |
| FR-019 | THE SYSTEM SHALL provide at least one scenario for `?` (backward search) from a cursor past a match, using the combined entry `? <pat>` of [[regex-selection-and-macro-commands/spec]] FR-121 and FR-123, followed by an `n` or `N` step only when it demonstrates the fixed `n`-forward, `N`-backward rule (FR-126 there); difficulty `intermediate`. File: `scenarios/en/search/basic-search.toml` (append) or a new `scenarios/en/search/backward-search.toml`, category `search` | must |
| FR-020 | THE SYSTEM SHALL provide at least one scenario for `Alt-*` (`search_selection`, no word boundaries added, unlike `*`), difficulty `intermediate`, whose target differs from the `*` result so the two cannot be confused (a substring that is part of a longer word). File: `scenarios/en/search/basic-search.toml` (append), category `search` | must |

### 3.5 Group D: editing (#645)

| ID | Requirement | Priority |
|----|------------|----------|
| FR-021 | THE SYSTEM SHALL provide at least one scenario each for `Alt-d` and `Alt-c` (delete or change without touching the default register): the scenario SHALL yank something first and paste after, so the target is reached only if the yank register was NOT overwritten; difficulty `intermediate`. File: `scenarios/en/editing/no-yank-edits.toml` (new), category `editing`, tags include `register`, `no-yank` | must |
| FR-022 | THE SYSTEM SHALL provide at least one scenario for `` Alt-` `` (to uppercase), difficulty `beginner`; it SHALL NOT be solvable by `~` alone, so the target is an all-uppercase result over a mixed-case selection. File: `scenarios/en/editing/advanced-editing.toml` (append), category `editing` | must |
| FR-023 | THE SYSTEM SHALL provide at least one scenario for `Ctrl-c` (`toggle_comments`) on a selection spanning several lines, one in Rust (`//`) and one in Python (`#`, `setup.language = "py"`) so that the comment token comes from the language; difficulty `intermediate`. File: `scenarios/en/editing/comments.toml` (new), category `editing`. `Ctrl-c` is the only comment key; the block-comment variant is not implemented and is out of scope | must |
| FR-024 | THE SYSTEM SHALL provide at least one scenario each for the Insert-mode keys `Ctrl-w` (delete word backward), `Ctrl-k` (delete to line end) and `Alt-Backspace` (delete word backward as bound by the simulator), difficulty `intermediate`, each entered after `i`, `a`, `A` or `c` and left with `Escape`; the target SHALL be reachable more cheaply with the Insert key than with `Escape` plus a Normal command, otherwise the scenario does not teach it. File: `scenarios/en/editing/insert-mode-editing.toml` (new), category `editing`, tags include `insert-mode` | must |

### 3.6 Group E: view commands (#646, P4)

| ID | Requirement | Priority |
|----|------------|----------|
| FR-025 | THE SYSTEM SHALL provide at least one scenario each for `zj` and `zk` (scroll the view, with the cursor pushed to the scrolloff band edge as in `hx`), difficulty `intermediate`, in a document of at least 60 lines so that the fixed 24-row logical viewport of [[helix-core-post-25-07-1-upgrade-readiness/spec]] section 3.8.8 can scroll; the expected cursor SHALL come from the recorded hx goldens, not from the simulator. File: `scenarios/en/movement/view.toml` (new), category `movement`, tags include `view`, `viewport` | should |
| FR-026 | `zz`, `zt`, `zb` and `zm` change only the view and leave the cursor and text unchanged, so they cannot be asserted by the current `target` format (section 7, Q-001). UNTIL Q-001 is decided they are on the guard allow-list (FR-029) with that rationale | should |

### 3.7 Guard (all groups)

| ID | Requirement | Priority |
|----|------------|----------|
| FR-027 | THE SYSTEM SHALL have a test that fails WHEN a registered command with `Binding::Helix(..)` in `normal_registry()` (`src/helix/registry/`) has its key neither in the set of normalized `solution.commands` tokens nor in the set of `metadata.commands_taught` values of the bundled `en` scenarios, and is not on the allow-list. The failure message SHALL name the command key and its Helix name | must |
| FR-028 | THE guard SHALL compare keys after the same normalization the input layer applies (`normalize_command_id`, count prefix, typed-pattern suffix, register prefix), so that `12G` covers `G`, `s foo` covers `s`, `"ay` covers `"`, `:goto 3` covers `:` and `3h` covers `h`; it SHALL read the keys from the registry and the scenarios from the embedded loader, never from a hand-kept copy of either list | must |
| FR-029 | THE allow-list SHALL be a single constant in the guard test (`key`, Helix name, one-line reason, tracking issue where one exists), sorted, and the guard SHALL ALSO fail when an allow-listed key IS taught (a stale entry), so the list can only shrink. Initial content: `Alt-x` (#141) and the group E keys of FR-026 while Q-001 is open. `Binding::TrainerAlias` entries are excluded by the filter, not by the allow-list | must |
| FR-030 | THE guard SHALL cover Insert-mode commands that exist only as simulator constants and are not in `normal_registry()` (`CMD_INSERT_DELETE_WORD_BACKWARD`, `CMD_INSERT_DELETE_TO_LINE_END`, `CMD_INSERT_ALT_BACKSPACE`) by an explicit list in the same test, until a registry for Insert mode exists ([NEEDS CLARIFICATION: Q-002]) | should |
| FR-031 | A test SHALL fail WHEN a `*.toml` file under `scenarios/en/` is not embedded by `src/config/scenarios/embedded.rs` (FR-009), since a scenario file that is not embedded is silently never taught | must |
| FR-032 | THE guard and the new tests SHALL live in `tests/scenario_validation.rs` (integration test, real `ScenarioLoader` and real registry, no mocks, per [[constitution]] Section III) and run under `cargo nextest run scenario` and the full suite | must |

## 4. Non-Functional Requirements

| ID | Category | Requirement |
|----|----------|-------------|
| NFR-001 | Correctness | Every expected target is derived from `hx` 25.07.1 behavior (a recorded golden, or the existing simulator test for the command, which already carries one), not from what the simulator currently prints |
| NFR-002 | Uniqueness | Scenario ids stay unique across all files; quest scenario references still resolve (`test_quest_scenario_references_exist`) |
| NFR-003 | i18n | Scenario content is English under `scenarios/en/`; no UI string is added, so `locales/` is unchanged. A future locale directory repeats the files and the guard runs per locale ([NEEDS CLARIFICATION: Q-003]) |
| NFR-004 | Progress and scoring | New scenarios use the existing scoring model unchanged: the progress indicator ([[training-progress-indicator/spec]]) reflects cursor and selection distance, completion is `optimal_count` actions with `tolerance` per FR-002, and the result enum is unchanged |
| NFR-005 | Determinism | Scenario texts contain no content that depends on locale, time or randomness; arcade selection ordering is not asserted by these scenarios |
| NFR-006 | Documentation | `CHANGELOG.md` `[Unreleased]` records the added scenario files and the guard, one line each, with the PR link; docs that list taught commands (the user guide command reference), if any, are updated in the same change |
| NFR-007 | Performance | The guard parses the embedded scenarios once and runs within the budget of the other `scenario_validation` tests (a few seconds) |

## 5. Data Model

| Entity | Description | Key Attributes |
|--------|-------------|----------------|
| Scenario entry | One `[[scenarios]]` TOML entry | `id`, `setup`, `target`, `solution.commands`, `scoring`, `metadata.commands_taught` |
| Registered Helix command | Entry of `normal_registry()` with `Binding::Helix(name)` | key, Helix name, category |
| Allow-list entry | Deliberate exclusion from the guard | key, Helix name, reason, issue |

No new types are introduced in `src/`.

## 6. Acceptance Criteria and Edge Cases

```
GIVEN the new long-word scenario with text "let x = foo.bar(baz);" and the cursor on "let"
WHEN  the canonical solution ["W", "W", "W"] is typed through the UI
THEN  the target selection is reached in 3 actions, and the same target is NOT reached by "w" in 3 actions
```
```
GIVEN the guard test and an allow-list entry for "Alt-x"
WHEN  a scenario that teaches "Alt-x" is added
THEN  the guard fails with a stale-allow-list message naming "Alt-x"
```
```
GIVEN a new file scenarios/en/editing/comments.toml that is not in embedded.rs
WHEN  cargo nextest run scenario is executed
THEN  the not-embedded test fails and names the file
```

| Scenario | Expected Behavior |
|----------|-------------------|
| A key is taught only as a count form (`12G`) | Counts as taught (FR-028) |
| `Alt-d` is both the Normal-mode no-yank delete and the Insert-mode forward word delete (the same `CMD_*` string, two commands) | The key id is one string, so one scenario covers the guard; the Insert-mode meaning is NOT claimed as taught ([NEEDS CLARIFICATION: Q-004]) |
| A scenario's `commands_taught` is an alias or a not-yet-normalized form | `test_bundled_commands_taught_are_normalized` fails; the scenario is fixed, the test is not relaxed |
| A command behaves differently in the simulator than in `hx` 25.07.1 | No scenario is written; the deviation is filed against [[helix-core-post-25-07-1-upgrade-readiness/spec]] and the key is allow-listed with the issue number until fixed |
| The new scenario is solvable by a shorter unintended sequence | `test_all_scenarios_execute_solution` still passes, but the scenario is rejected in review (FR-008); add `tolerance = 0` and tighten the text |

## 7. Open Questions

> [!question] Open items
> - [NEEDS CLARIFICATION: Q-001 Viewport assertions] The scenario `target` is `file_content` plus one `CursorSpec` (cursor, selection, cursors or selections); it has no view field, although `ViewState.top_line` exists in the simulator. `zz`, `zt`, `zb` leave the cursor unchanged, and `zm` is a horizontal alignment the simulator does not model (no horizontal view, [[helix-core-post-25-07-1-upgrade-readiness/spec]] section 3.8.4), so none can be asserted. `zj` and `zk` can, because the cursor is pushed to the scrolloff band edge (FR-025). Options: (a) exclude `zz`, `zt`, `zb`, `zm` on the allow-list with the rationale "view-only, not assertable by target" (recommended; no format change; `zm` stays excluded permanently); (b) assert indirectly by chaining a following command whose result depends on the view, for example `zt` then `Ctrl-f` (rejected: teaches an unrelated trick and breaks when the viewport model changes); (c) add an optional `view_top` key to `[scenarios.target]` plus completion, progress and renderer support (a separate spec amending [[training-progress-indicator/spec]] FR-001, because completion would then depend on view state, which `GameSession` does not compare today). Decide before #646 is implemented.
> - [NEEDS CLARIFICATION: Q-002 Insert-mode registry] Insert-mode commands are simulator constants, not registry entries, so the guard cannot derive them (FR-030). Options: (a) a hard-coded list in the test (cheap, can drift); (b) register Insert-mode Helix commands with `Binding::Helix` in a second registry (consistent with FR-027, larger change, belongs to [[module-layering-and-typed-commands/spec]]). Default: (a).
> - [NEEDS CLARIFICATION: Q-003 Locales] Only `scenarios/en/` exists. Should the guard run per available locale, or only `en`? Default: `en` only until a second locale ships.
> - [NEEDS CLARIFICATION: Q-004 One id, two modes] `commands_taught` has no mode qualifier, so `Alt-d` (Normal no-yank delete and Insert forward word delete) is one id and one FSRS card. Options: keep one card (default; the Insert variant is documented in a hint only) or introduce a mode-qualified id (a profile and FSRS data change, out of scope here).
> - [NEEDS CLARIFICATION: Q-005 Non-listed untaught keys] The Insert-mode `Tab`, `Backspace` and the arrow keys (`Left`, `Right`, `Up`, `Down`) also appear untaught in the diff and were not in the issue list. Default: allow-list them as duplicates of taught commands (`hjkl`, `d`, `>`) unless a decision says to teach them.
> - [NEEDS CLARIFICATION: Q-006 Scenario volume] The minimum count is 24 scenarios in groups A to D (5 + 8 + 4 + 7, one per command, `v` two) plus 2 in group E. Should each group ship as one PR (matching #642 to #646), with the guard landing in the first PR with the not-yet-covered keys allow-listed and each later PR removing its keys from the list? Default: yes.

## 8. Success Criteria

| ID | Metric | Target |
|----|--------|--------|
| SC-001 | Registered Helix-bound normal-mode commands (`Binding::Helix`) with neither a `solution.commands` token nor a `commands_taught` entry, excluding the allow-list | 0 |
| SC-002 | Allow-list size once groups A to D are merged | `Alt-x`, `zz`, `zt`, `zb`, `zm` and the Q-005 keys only; no key of groups A to D |
| SC-003 | New scenarios in groups A to D | At least 24, each passing `cargo nextest run scenario` |
| SC-004 | The guard fails on a deliberately removed scenario and on a stale allow-list entry (checked once when the guard is added) | Both fail with a message naming the key |
| SC-005 | `test_all_scenario_ids_are_unique`, `test_bundled_commands_taught_are_normalized`, `test_quest_scenario_references_exist` after the change | Pass |
| SC-006 | Every new target differs from its setup (FR-007), checked by a test over the new scenario ids or by the existing validation | Pass |

## 9. Agent Boundaries

### Always (without asking)
- Run `cargo nextest run scenario` and the full pre-commit suite after adding scenarios.
- Derive each expected target from `hx` 25.07.1 semantics and an existing simulator test, and follow the layout of the neighbouring scenario file.
- Update `CHANGELOG.md` `[Unreleased]` and this spec's status when a group lands.

### Ask first
- Adding an optional field to the scenario TOML format (Q-001 option c) or an Insert-mode registry (Q-002 option b).
- Changing a simulator command so that a scenario can be written (the deviation is a bug against the upgrade-readiness spec first).
- Adding an allow-list entry that is not listed in FR-029 or Q-005.

### Never
- Weaken or skip `test_all_scenarios_execute_solution`, raise `tolerance` above 1, or write a scenario whose target equals its setup.
- Teach a trainer-only alias (`^`, `Ctrl-r`) as if it were a Helix command, or claim `Alt-x` before #141 lands.
- Put scenario or spec drafts outside `scenarios/en/` and `specs/`, or reference untracked paths from the scenarios or this spec.

## 10. See Also

- [[constitution]] - testing and scenario rules (Section III)
- [[regex-selection-and-macro-commands/spec]] - `s`, `S`, `?`, `K`, `Alt-K`, `&`, macros and the prompt-command entry convention
- [[helix-core-post-25-07-1-upgrade-readiness/spec]] - `hx` reference behavior, viewport model (section 3.8.8)
- [[optimal-count-action-counter-unit/spec]] - `optimal_count` unit and combined-token entries
- [[training-progress-indicator/spec]] - progress for movement and selection scenarios
- [[duplicate-scenario-ids/spec]] - id uniqueness guard
- [[register-command-mode-support/spec]] - register-scoped `Alt-c` and `Alt-d` behavior
- [[multi-language-scenario-content/spec]] - `setup.language` and Python variants
- [[module-layering-and-typed-commands/spec]] - typed commands and the registry
- [[MOC-specs]] - all specifications
