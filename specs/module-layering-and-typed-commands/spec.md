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
> **Issues**: #555 (typed `NormalCommand`), #557 (shared key resolution), #558 (module layering), #564 (visibility policy), #598 (locale keys). Extended 2026-11-11 (follow-up batch of PR #607) with #614 (parsed `CanonicalKeys`, FR-002), #613 (one key-history policy, FR-013), #606 (error hub, FR-010) and #612 and #602 (locale keys for gamification text and mode names, FR-012). The input-side behavior fixes of the same batch (#592 `Ctrl-r`, #599 `Ctrl-c`) are specified in [[helix-core-post-25-07-1-upgrade-readiness/spec]] (FR-058, FR-042).
> **Status**: Proposed
> **Depth**: Lightweight spec. Architecture refactors with no intended gameplay change, except #598 (text moves to locale keys with identical English output).

## 1. Overview

### Problem Statement

Commands travel from the keyboard to the simulator as strings that are re-parsed at several call sites (`CHAR_ARGUMENT_COMMANDS`, two `FindType` enums, `NativeCounted`). The Task and Arcade screens each carry a copy of the key-resolution pipeline. The crate has module dependency cycles (`game -> ui`, `config -> gamification`, `gamification -> sound/minigame/input`, `config -> input`, `input -> ui`) that nothing prevents from growing. About 100 `pub` functions have no production caller because the library exports everything. User-visible English remains hard-coded outside the locale files.

### Goal

One closed typed command from input to simulator, one shared resolution pipeline, a strict layer order enforced by a permanent test, an explicit visibility policy, and all user-visible text behind locale keys.

### Out of Scope

- Typed Insert-mode keys (the Insert path carries typed text as `GameplayKey::Typed`, FR-002, and never parses it)
- Any change to what a command does, with ONE user-approved, documented exception: after #555 a count typed before a command that does not accept one (`.`, `r`, surround, text objects, prompts, command-line mode, `Escape`) is `CountNotAccepted`, and a count before `Q` or before register operations is ignored. Before #555 `.` and similar commands repeated. This is a KNOWN DIVERGENCE from hx, which repeats `.` and `Q` N times and passes the count to `mi` and `ma` (helix-term `ui/editor.rs` 929-930, `commands.rs` 6618-6620). The UI cannot produce any of these counts today (`is_count_compatible_command` excludes them), so only the crate-internal API is affected; no code change in this PR

## 2. Functional Requirements

| ID | Requirement | Issue | Priority |
|----|------------|-------|----------|
| FR-001 | THE SYSTEM SHALL carry a normal-mode command as a closed typed `NormalCommand` (`helix::normal_command`) from the input layer to the simulator. The find grammar (`FindKind`, `FindDirection`, `FindMotion`, `takes_char_argument`) lives in `grammar::find` with no crate dependencies. There is ONE parser from a command string to `NormalCommand`, a `Display` that renders the CANONICAL spelling of the command (identical to the string form used today except for aliases: `:g 3` displays as `:goto 3`; card ids are unaffected because `normalize_command_id` canonicalizes the same way), a `keys()` accessor, and an exhaustive `execute_normal`. `execute_command(&str)` stays as the parse facade; both sessions get a `pub(crate)` `apply_command`; `HandlerResult::Execute` carries a `NormalCommand`. `CHAR_ARGUMENT_COMMANDS`, both `FindType` enums and `NativeCounted` SHALL be deleted | #555 | must |
| FR-002 | THE SYSTEM SHALL parse each key text ONCE into a typed `CanonicalKeys { text: Cow<'static, str>, keys: Keys }` with private fields, where `Keys` is private and is `One(KeyEvent)` or `Many(Box<[KeyEvent]>)` with the `Many` length at least 2, enforced by the only constructor. The events come from the same `parse_helix_key_string` that `resolve_keys` uses today, run once at construction. `Eq`, `Hash` and the keymap `fingerprint` use `text` ONLY: equality and fingerprints stay byte-identical to the string form, raw `KeyEvent` equality is never used and nothing formats keys back to text. Constructors: `parse(text) -> Result<_, ParsePhysicalKeyError>` for keymap load (a failure goes to the existing `KeymapWarningReason`, no `expect`); `from_event(KeyEvent) -> Option<Self>` for the Normal-mode stock and arrow path only, returning `None` exactly where today's resolution drops the key (its text comes from `key_to_command_string`, which moves unchanged from `ui/key_dispatch/handlers.rs` into `input/typestate/key_mapping.rs` next to `parse_helix_key_string`, rank 9, and the ui imports it from there; the arrow-to-`h/j/k/l` mapping goes with it as `arrow_movement`). `from_static`, `from_owned` and `From<Cow>` are removed; a `#[cfg(test)]` `crate::testing::keys(&str)` helper serves unit tests and integration tests use `parse(..).unwrap()`. The first commit of the change is a test that pins `KeymapOverlay::fingerprint` for a sample custom overlay and passes on the code before the change. INSERT PATH: typed text never goes through `CanonicalKeys` (`key_to_command_string` drops non-ASCII); `Message::{TaskCommand, MiniGameCommand}` carry `keys: GameplayKey` with `enum GameplayKey { Typed(Cow<'static, str>), Keys(CanonicalKeys) }` and `From<CanonicalKeys>`; the ui sends `Typed(handle_insert_mode_input(..))` in Insert mode and `Keys(..)` otherwise. Tests: every registry key and every `CMD_*` constant parses; `from_event` succeeds for every Normal-mode event that `key_to_command_string` maps; typing ' ', '-', '<', uppercase, non-ASCII and the `map_macos_composed_char` outputs in Insert mode reaches the simulator unchanged. Supersedes the earlier cut of this requirement | #555, #614 | must |
| FR-003 | Tests SHALL cover: a canonical-only round trip (`NormalCommand` -> `Display` -> parse), and a card-id oracle comparing the old `CommandId::from_action` with the new derivation over every scenario solution and the generated corpus | #555 | must |
| FR-004 | THE Task and Arcade screens SHALL share ONE crate-internal function `resolve_keys(&mut InputStateMachine, ModeKind, &GameplayKey) -> Resolution` with `Resolution { Insert(Cow<'static, str>), Execute { command, from: ResolvedFrom }, Pending, Discarded }` and `ResolvedFrom { Base, Pending }` (in `src/input/resolve.rs`). In Insert mode `Typed(t)` gives `Insert(t)` and `Keys(k)` gives `Insert(k.text)`; in Normal mode `Keys` parses nothing further, and `Typed` is a stale message: `Discarded`, logged at debug. Key history follows ONE policy shared by both screens (FR-013) | #557, #614, #613 | must |
| FR-005 | KEY-EVENT DISPATCH SHALL be part of `ui`: `handle_key_event`, the quarantine-banner shortcuts, the screen handlers and the `Ctrl-c` routing live in `src/ui/key_dispatch/{mod,handlers,ctrl_c}.rs` (moved from `src/input/{mod,handlers,ctrl_c}.rs` as a pure move, no logic change), exported as `ui::handle_key_event`; `input` keeps only the typestate machine, keymap overlay and key resolution. No `input` re-export is left behind, so `crate::input::{handle_key_event, handlers}` disappear (breaking change in `CHANGELOG.md`) | #558 | must |
| FR-006 | THE library modules SHALL obey the layer order of the [[constitution]] (Section I): a module depends only on a strictly lower rank. The cycles are removed by moving code: `game -> ui` by a `CompletionEvent` converted into the notification type in `ui`; `config -> gamification` by moving `QuarantineError` to `config::json_store`, `scenario_collection` to `game` and `From<&QuestSpec> for Quest` into `gamification`; `gamification -> sound/minigame` by moving `SoundConfig` to `config`, `ChallengeProgress` to `gamification` and adding `Difficulty::base_time_limit()`; `config -> input` by moving the keymap TOML resolution to `input/keymap`; `gamification -> input` by moving the keymap fingerprint types to `config/keymap/fingerprint.rs` ([[custom-helix-keymap-support/spec]]) | #558 | must |
| FR-007 | THE moved modules SHALL leave NO forwarding re-exports (constitution: no shims): `helix::commands` moves to `grammar::commands` and `limits` to `limits` (`src/limits.rs`), and every importer (in the crate, tests, benches and doc-tests) is rewritten to the new path; the fingerprint importers are rewritten likewise. The CHANGELOG lists the moved public paths | #558 | must |
| FR-008 | A permanent layering test (`src/layering_tests.rs`, `#[cfg(test)] mod layering_tests;` in `lib.rs`, so local gate and CI both run it) SHALL parse every library source file with `syn` (a test-only dev-dependency, `syn` 3.0.9 with the `full` and `visit` features, plus `proc-macro2` for the macro-token scan) and fail on any upward or same-rank edge. Edges come from `use` trees (groups included), from every path whose first segment is `crate`, and from `crate :: <ident>` token runs inside macro bodies. It skips every item or statement carrying `#[cfg(test)]` or `cfg(all(test, ..))` at ANY nesting level (modules, `impl` and trait items, functions, statements, fields, arms), ignores doc comments (attributes), skips `src/testing/`, and treats the bin-crate modules (found from the `mod` items in `src/main.rs`) as above every library layer. A module missing from the rank table, or a source file `syn` cannot parse, SHALL fail the test loudly; file paths come from `env!("CARGO_MANIFEST_DIR")`. The rank table in the test is: `grammar`, `limits`, `constants`, `numeric`, `time` = 0; `ids`, `security` = 1; `helix` = 2; `config` = 3; `learning`, `sound` = 4; `gamification` = 5; `game` = 6; `minigame` = 7; `error` = 8 (the user-facing error hub of FR-010, #606); `input` = 9; `async_state` = 10; `ui` = 11; `data_loader` = 12 (the same table as the constitution). The allow-list holds the four edges `security -> {config, gamification, helix, learning}` until #606 lands (FR-010) and is EMPTY afterwards; the test ALSO fails when an allow-listed edge no longer exists, so the list can only shrink | #558 | must |
| FR-009 | THE layering test SHALL carry 11 self-checks over in-memory sources, each asserting the stated outcome: (1) a production edge placed after a `#[cfg(test)] mod tests;` declaration is reported; (2) an edge inside a `#[cfg(test)]` block, and inside `#[cfg(all(test, ..))]`, is not reported; (3) a doc link to `crate::ui::X` is not reported; (4) a `use` group reports every target edge; (5) an expression path `crate::ui::f()` in a function body is reported; (6) `crate::ui::X` and `crate::game::g()` inside `matches!(..)`, `assert!(..)` and `tracing::warn!(..)` are reported; (7) a same-rank edge is reported; (8) an unranked module is an error; (9) a `#[cfg(test)] fn` inside an `impl` that uses `crate::ui` is not reported; (10) an unparsable source fails loudly; (11) a lower-rank edge is accepted. Two real-tree tests accompany them: the layer-order scan (including the allow-list staleness check) and a check that every library module has a rank | #558 | must |
| FR-010 | THE user-facing error hub SHALL live above the domain modules: `src/error.rs` (rank 8) holds `UserError`, `ErrorCause` and the `From` conversions into `UserError`. `error_chain` STAYS in `src/security.rs`: `config::json_store` and `gamification::storage` (ranks 3 and 5) call it in production code, so moving it to rank 8 would add upward edges. Domain modules return their own typed errors converted with `?` in the upper layers: a `GameError` shared by `game` and `minigame` (variants `Security`, `Simulator`, `NoActiveScenario`), the existing `GamificationError` and `LearningError`, and a `ConfigError` for the config loaders and `QuestTemplateRegistry::from_embedded` (wrapped as `ErrorCause::Config(ConfigError)`, replacing the former `Security(SecurityError::QuestTemplate/Scenario)` causes; the logged chain text is unchanged); `SecurityError` loses its `QuestTemplate` and `Scenario` variants. `security` (rank 1) keeps validators and `SecurityError` and imports none of `config`, `gamification`, `helix` or `learning`. The work lands as one commit per domain, and when it ends the layering allow-list is EMPTY (FR-008). The fuzz target of `ConfigStorage::load` is re-run after the move (its error type changes). The constitution hub path (Section V) and the rank-8 row are updated by the same change. Supersedes the earlier cut of this requirement | #558, #606 | must |
| FR-011 | PUBLIC VISIBILITY: the library SHALL export only what the binary, the integration tests and the benches use. `#![warn(unreachable_pub)]` is set in `lib.rs` (not `[lints.rust]`, which would also hit the bin, test and bench targets); an item with no external consumer is `pub(crate)`; a function with no production caller is deleted after a RE-SCAN following #558 (the issue's list is stale: `learning::ReviewSession` is live), only when verified unused. A doc-test on a narrowed item is deleted when an existing test covers the item, and converted to a unit test only when nothing else covers it | #564 | should |
| FR-012 | ALL user-visible text SHALL resolve through locale keys in `locales/` (`rust-i18n`): `Notification::title()` and `message()` and the hard-coded text in `ui/render/{profile,popups,achievements}.rs`. A test asserts that each `NotificationType` renders exactly the English it rendered before; a missing key (which would render as the key name) fails a test . Amended by #612 and #602 (2026-11-11): the names and descriptions of achievements and quests resolve through locale keys `achievements.<id>.{name,description}` and `quests.<template_id>.{name,description}`, with `%{target}`, `%{command}` and `%{minutes}` filled from the quest params at the ui call site; `Achievement`, `Quest` and `QuestTemplate` hold ids and params only (no text fields), and domain events carry ids only (`CompletionEvent::AchievementUnlocked { id }`); the ui-layer notification shapes keep their `String`s, filled from `t!` at the conversion points (`From<CompletionEvent>`, the profile-load and quest handlers); a test checks that every achievement id and quest template id resolves and no `%{` placeholder remains. KNOWN GAP: the speed-run quest descriptions keep the scenario id and the seconds LITERAL in the locale text, because the placeholder set above is only `target`, `command` and `minutes`; those two values duplicate params of the quest template data and can drift. Adding `%{scenario}` and `%{seconds}` placeholders is the follow-up #628 (P4); `MiniGameMode::name` and `description` are deleted (they duplicate the `mode_selection.*` keys) and their tests are fixed. A profile written by an older build that lacks `Achievement.name` is quarantined (CHANGELOG breaking note) | #598, #612, #602 | must |
| FR-013 | BOTH screens SHALL apply ONE key-history policy, the Task screen's (user decision 2026-11-11): every physical key is recorded in the key history BEFORE resolution, and the last-command value is set AFTER the command is accepted. The bookkeeping lives in one shared helper next to `resolve_keys` (`src/ui/state/handlers/key_history.rs`) used by `execute_on_task` and `handle_minigame_command`. The history is display-only (no card, scoring or quest effect), so no gameplay result changes; the Arcade key-history display now shows pending-state keys as the Task screen does | #613 | must |

### 2.1 Acceptance Criteria

```
GIVEN the command string "2fa"
WHEN  it is parsed and displayed
THEN  NormalCommand::Find { count 2, kind, direction } displays "2fa" in canonical spelling

GIVEN the layering test and a planted `use crate::ui::X;` in src/helix/foo.rs
WHEN  `cargo nextest run layering` runs
THEN  it fails naming the edge helix -> ui

GIVEN the key-dispatch move
WHEN  `git diff -M` is reviewed
THEN  src/ui/key_dispatch/* is a rename of the old input files plus import changes only
```
```
GIVEN a pending count or register prefix on the Arcade screen
WHEN  a physical key arrives
THEN  it is recorded in the key history before resolution, as on the Task screen (FR-013)
```
```
GIVEN the Insert-mode keys ' ', '-', '<', an uppercase letter and a non-ASCII character
WHEN  they are typed on the Task or Arcade screen
THEN  each reaches the simulator unchanged and none passes through CanonicalKeys (FR-002)
```
```
GIVEN the error hub move
WHEN  `cargo nextest run layering` runs
THEN  the allow-list is empty and security imports none of config, gamification, helix, learning (FR-010)
```

## 3. Edge Cases

| Scenario | Expected Behavior |
|----------|-------------------|
| A new top-level module is added | The layering test fails until it is added to the rank table (and the constitution table) |
| A back edge appears only inside `#[cfg(test)]` code | Allowed (test code may reach upward) |
| A `crate::` path built by `macro_rules!` expansion | Not seen by `syn` (false negative only, never a false failure); literal `crate ::` text in macro bodies is scanned |

## 4. Scope and Order of Cuts

First batch (2026-11-11, PR for #592): one PR, one conventional commit per issue (#558 got two: the key-dispatch move and the rest). The `CanonicalKeys` step (FR-002) and the error hub E1 (FR-010) were cut from it, with the layering allow-list holding the four `security` edges, and were filed as #614 and #606.

Follow-up batch (2026-11-11, PR for #607): FR-002 (#614), FR-010 (#606) and FR-013 (#613) are specified for implementation, with #612 and #602 in FR-012. If the budget is exceeded the remaining items are cut in this order (the user decides): #603, then #537, then #612, then the tail of the #606 stages (one commit per domain; the allow-list shrinks with each stage). On a cut the `sdd` agent adjusts this spec and the cut work is closed with a note and a follow-up issue filed immediately.

## 5. Success Criteria

| ID | Metric | Target |
|----|--------|--------|
| SC-001 | Round trip and card-id oracle pass; all scenario solutions complete | Pass |
| SC-002 | The 11 layering self-checks and the two real-tree tests pass and the real-tree scan reports no edge | Pass |
| SC-003 | `cargo clippy --all-targets --all-features --workspace -- -D warnings` with `unreachable_pub` (if FR-011 lands) | Pass |
| SC-004 | `NotificationType` English output unchanged | Pass |
| SC-005 | Fingerprint pin test passes before and after #614; every registry key parses; Insert typing of the listed characters is unchanged | Pass |
| SC-006 | Layering allow-list empty after #606; `ConfigStorage::load` fuzz target builds against the new error type | Pass |
| SC-007 | Every achievement id and quest template id resolves to a non-empty name and description with no `%{` left | Pass |

## 6. Agent Boundaries

### Always
- Review the key-dispatch move as a rename; a logic change there is a defect
- Rewrite importers rather than leave a re-export

### Never
- Add a forwarding re-export, any new allow-list entry (the four `security` edges of FR-008 only shrink), or an `input -> ui` edge
- Edit specs by hand outside the `sdd` agent workflow

## 7. See Also

- [[constitution]] - layer order table and no-shims rule
- [[helix-core-post-25-07-1-upgrade-readiness/spec]] - FR-042 (`Ctrl-c`) and FR-058 (`Ctrl-r`)
- [[scenario-cursor-spec/spec]] - closed `CursorSpec` enum (#553)
- [[MOC-specs]] - all specifications
