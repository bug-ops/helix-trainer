---
aliases:
  - Module Layering and Typed Commands
  - NormalCommand and layering
tags:
  - sdd
  - spec
  - architecture
  - input
  - refactoring
  - status/proposed
created: 2026-11-11
status: proposed
related:
  - "[[constitution]]"
  - "[[helix-core-post-25-07-1-upgrade-readiness/spec]]"
  - "[[scenario-cursor-spec/spec]]"
  - "[[custom-helix-keymap-support/spec]]"
  - "[[MOC-specs]]"
---

# Feature: Typed Normal Commands, One Key Resolution, Enforced Module Layering

> [!info] Metadata
> **Issues**: #555 (typed `NormalCommand`), #557 (shared key resolution), #558 (module layering), #564 (visibility policy), #598 (locale keys). The input-side behavior fixes of the same batch (#592 `Ctrl-r`, #599 `Ctrl-c`) are specified in [[helix-core-post-25-07-1-upgrade-readiness/spec]] (FR-058, FR-042).
> **Status**: Proposed
> **Depth**: Lightweight spec. Architecture refactors with no intended gameplay change, except #598 (text moves to locale keys with identical English output).

## 1. Overview

### Problem Statement

Commands travel from the keyboard to the simulator as strings that are re-parsed at several call sites (`CHAR_ARGUMENT_COMMANDS`, two `FindType` enums, `NativeCounted`). The Task and Arcade screens each carry a copy of the key-resolution pipeline. The crate has module dependency cycles (`game -> ui`, `config -> gamification`, `gamification -> sound/minigame/input`, `config -> input`, `input -> ui`) that nothing prevents from growing. About 100 `pub` functions have no production caller because the library exports everything. User-visible English remains hard-coded outside the locale files.

### Goal

One closed typed command from input to simulator, one shared resolution pipeline, a strict layer order enforced by a permanent test, an explicit visibility policy, and all user-visible text behind locale keys.

### Out of Scope

- Unifying the Task and Arcade key-history policy (Task records every typed key, Arcade only executed ones; display-only, tracked as a P4 follow-up)
- Typed Insert-mode keys
- Any change to what a command does

## 2. Functional Requirements

| ID | Requirement | Issue | Priority |
|----|------------|-------|----------|
| FR-001 | THE SYSTEM SHALL carry a normal-mode command as a closed typed `NormalCommand` (`helix::normal_command`) from the input layer to the simulator. The find grammar (`FindKind`, `FindDirection`, `FindMotion`, `takes_char_argument`) lives in `grammar::find` with no crate dependencies. There is ONE parser from a command string to `NormalCommand`, a `Display` that is byte-identical to the string form used today (card ids, key history and macros are unchanged), a `keys()` accessor, and an exhaustive `execute_normal`. `execute_command(&str)` stays as the parse facade; both sessions get a `pub(crate)` `apply_command`; `HandlerResult::Execute` carries a `NormalCommand`. `CHAR_ARGUMENT_COMMANDS`, both `FindType` enums and `NativeCounted` SHALL be deleted | #555 | must |
| FR-002 | CUT from this PR (the first half of this requirement is not implemented). Originally: parse key events once into `CanonicalKeys` at keymap load. Reason: `CanonicalKeys` is a string newtype built infallibly (`from_static(CMD_*)`) in about 50 places including tests and benches and compared as a string (`Message::ExecuteCommand` equality, overlay maps, keymap parse tests); storing `KeyEvent`s would make construction fallible or lazy and rewrite all of them with no behavior change, since `resolve_keys` parses one short token per keystroke. WHAT LANDS: the `command_to_key_event` alias is deleted and its callers use `parse_helix_key_string`. REMAINING FOLLOW-UP (P4 enhancement): store parsed key events in `CanonicalKeys` | #555 | cut |
| FR-003 | Tests SHALL cover: a canonical-only round trip (`NormalCommand` -> `Display` -> parse), and a card-id oracle comparing the old `CommandId::from_action` with the new derivation over every scenario solution and the generated corpus | #555 | must |
| FR-004 | THE Task and Arcade screens SHALL share ONE crate-internal function `resolve_keys(&mut InputStateMachine, ModeKind, &CanonicalKeys) -> Resolution` with `Resolution { Insert(Cow<'static, str>), Execute { command, from: ResolvedFrom }, Pending, Discarded }` and `ResolvedFrom { Base, Pending }` (in `src/input/resolve.rs`). Each screen keeps its own key-history policy | #557 | must |
| FR-005 | KEY-EVENT DISPATCH SHALL be part of `ui`: `handle_key_event`, the quarantine-banner shortcuts, the screen handlers and the `Ctrl-c` routing live in `src/ui/key_dispatch/{mod,handlers,ctrl_c}.rs` (moved from `src/input/{mod,handlers,ctrl_c}.rs` as a pure move, no logic change), exported as `ui::handle_key_event`; `input` keeps only the typestate machine, keymap overlay and key resolution. No `input` re-export is left behind, so `crate::input::{handle_key_event, handlers}` disappear (breaking change in `CHANGELOG.md`) | #558 | must |
| FR-006 | THE library modules SHALL obey the layer order of the [[constitution]] (Section I): a module depends only on a strictly lower rank. The cycles are removed by moving code: `game -> ui` by a `CompletionEvent` converted into the notification type in `ui`; `config -> gamification` by moving `QuarantineError` to `config::json_store`, `scenario_collection` to `game` and `From<&QuestSpec> for Quest` into `gamification`; `gamification -> sound/minigame` by moving `SoundConfig` to `config`, `ChallengeProgress` to `gamification` and adding `Difficulty::base_time_limit()`; `config -> input` by moving the keymap TOML resolution to `input/keymap`; `gamification -> input` by moving the keymap fingerprint types to `config/keymap/fingerprint.rs` ([[custom-helix-keymap-support/spec]]) | #558 | must |
| FR-007 | THE moved modules SHALL leave NO forwarding re-exports (constitution: no shims): `helix::commands` moves to `grammar::commands` and `security::limits` to `limits` (`src/limits.rs`), and every importer (in the crate, tests, benches and doc-tests) is rewritten to the new path; the fingerprint importers are rewritten likewise. The CHANGELOG lists the moved public paths | #558 | must |
| FR-008 | A permanent layering test (`src/layering_tests.rs`, `#[cfg(test)] mod layering_tests;` in `lib.rs`, so local gate and CI both run it) SHALL parse every library source file with `syn` (a test-only dev-dependency; the version is chosen by the project's dependency check, preferring the current major already in `Cargo.lock`) and fail on any upward or same-rank edge. Edges come from `use` trees (groups included), from every path whose first segment is `crate`, and from `crate :: <ident>` token runs inside macro bodies. It skips every item or statement carrying `#[cfg(test)]` or `cfg(all(test, ..))` at ANY nesting level (modules, `impl` and trait items, functions, statements, fields, arms), ignores doc comments (attributes), skips `src/testing/`, and treats the bin-crate modules (found from the `mod` items in `src/main.rs`) as above every library layer. A module missing from the rank table, or a source file `syn` cannot parse, SHALL fail the test loudly; file paths come from `env!("CARGO_MANIFEST_DIR")`. The allow-list is EMPTY (the only exception is the cut case in section 4) | #558 | must |
| FR-009 | THE layering test SHALL carry self-checks over in-memory sources, each asserting the stated outcome: (1) a `use crate::game::X;` placed after `#[cfg(test)] mod tests;` in a `helix` file is reported; (2) a `use crate::ui::X;` inside a `#[cfg(test)] mod t { .. }` block is not reported; (3) a doc link to `crate::ui::X` is not reported; (4) `use crate::{game::A, ui::B};` in `helix` reports both edges; (5) an expression path `crate::ui::f()` in a function body is reported; (6) `crate::ui::X` inside `matches!(..)` or `tracing::warn!(..)` is reported; (7) a same-rank edge (`ids -> security`) is reported; (8) an unranked module is reported; (9) a `#[cfg(test)] fn` inside an `impl` that uses `crate::ui` is not reported | #558 | must |
| FR-010 | WHERE the user-facing error hub (E1, decided at cut trigger T2, section 4) is built THE SYSTEM SHALL hold `UserError`, `ErrorCause`, `error_chain` and every domain `From` in `src/error.rs` (rank 8, above every domain module); `SecurityError` loses its config variants; `game::GameError` serves both the game and minigame sessions; `QuestTemplateRegistry::from_embedded` returns `GamificationError`; upper layers convert with `?`; the doc-tests that named the old path are updated | #558 | should |
| FR-011 | PUBLIC VISIBILITY: the library SHALL export only what the binary, the integration tests and the benches use. `#![warn(unreachable_pub)]` is set in `lib.rs` (not `[lints.rust]`, which would also hit the bin, test and bench targets); an item with no external consumer is `pub(crate)`; a function with no production caller is deleted after a RE-SCAN following #558 (the issue's list is stale: `learning::ReviewSession` is live), only when verified unused. A doc-test on a narrowed item is deleted when an existing test covers the item, and converted to a unit test only when nothing else covers it | #564 | should |
| FR-012 | ALL user-visible text SHALL resolve through locale keys in `locales/` (`rust-i18n`): `Notification::title()` and `message()` and the hard-coded text in `ui/render/{profile,popups,achievements}.rs`. A test asserts that each `NotificationType` renders exactly the English it rendered before; a missing key (which would render as the key name) fails a test | #598 | must |

### 2.1 Acceptance Criteria

```
GIVEN the command string "2fa"
WHEN  it is parsed and displayed
THEN  NormalCommand::Find { count 2, kind, direction } displays "2fa" byte for byte

GIVEN the layering test and a planted `use crate::ui::X;` in src/helix/foo.rs
WHEN  `cargo nextest run layering` runs
THEN  it fails naming the edge helix -> ui

GIVEN the key-dispatch move
WHEN  `git diff -M` is reviewed
THEN  src/ui/key_dispatch/* is a rename of the old input files plus import changes only
```

## 3. Edge Cases

| Scenario | Expected Behavior |
|----------|-------------------|
| A new top-level module is added | The layering test fails until it is added to the rank table (and the constitution table) |
| A back edge appears only inside `#[cfg(test)]` code | Allowed (test code may reach upward) |
| A `crate::` path built by `macro_rules!` expansion | Not seen by `syn` (false negative only, never a false failure); literal `crate ::` text in macro bodies is scanned |

## 4. Scope and Order of Cuts

User decisions (2026-11-11): one PR, one conventional commit per issue (#558 gets two: the key-dispatch move and the rest). If the budget is exceeded the remaining items are cut in this order: #564 (FR-011), then the #558 error hub E1 (FR-010), then the #555 `CanonicalKeys` step (FR-002). Status: the `CanonicalKeys` step was cut (2026-11-11, see FR-002); #564 and E1 are not cut. On a cut the `sdd` agent adjusts this spec: for E1 the constitution hub path reverts to the previous location and the layering allow-list gets the `security -> {config, gamification, helix, learning}` edges with the follow-up issue number; the cut work is closed with a note and a follow-up issue is filed immediately.

## 5. Success Criteria

| ID | Metric | Target |
|----|--------|--------|
| SC-001 | Round trip and card-id oracle pass; all scenario solutions complete | Pass |
| SC-002 | The nine layering self-checks pass and the real-tree scan reports no edge | Pass |
| SC-003 | `cargo clippy --all-targets --all-features --workspace -- -D warnings` with `unreachable_pub` (if FR-011 lands) | Pass |
| SC-004 | `NotificationType` English output unchanged | Pass |

## 6. Agent Boundaries

### Always
- Review the key-dispatch move as a rename; a logic change there is a defect
- Rewrite importers rather than leave a re-export

### Never
- Add a forwarding re-export, an allow-list entry (outside the cut case) or an `input -> ui` edge
- Edit specs by hand outside the `sdd` agent workflow

## 7. See Also

- [[constitution]] - layer order table and no-shims rule
- [[helix-core-post-25-07-1-upgrade-readiness/spec]] - FR-042 (`Ctrl-c`) and FR-058 (`Ctrl-r`)
- [[scenario-cursor-spec/spec]] - closed `CursorSpec` enum (#553)
- [[MOC-specs]] - all specifications
