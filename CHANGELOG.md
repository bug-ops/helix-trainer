# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `install.sh` and `install.ps1` at the repo root: one-line installers that resolve the latest (or a pinned) GitHub release, download the archive matching the current OS/architecture, verify its SHA-256 checksum, and install the `helix-trainer` binary. `install.sh` covers Linux (glibc and static musl builds via `--static`) and macOS on x86_64/aarch64; `install.ps1` covers Windows on x86_64/aarch64. README's Installation section documents both as the recommended path alongside manual archive download and building from source. `install.sh` extracts the downloaded archive with [exarch](https://github.com/bug-ops/exarch) when available or bootstrappable on the host, falling back to the system `tar` otherwise. CI runs `shellcheck` on `install.sh` when it changes.
- End-game summary screen: reached from the Results screen's `(n)` key once every scenario in the curriculum has been completed at least once (`ScenarioCollection::is_curriculum_complete`/`curriculum_stats`); shows total scenarios, perfected count, lifetime completions, level/XP, command success rate, commands mastered, a per-category mastery breakdown, and up to four suggested next steps (due reviews, open quests, imperfect scenarios, always ending in Arcade mode). The Results screen shows a discoverability hint when the curriculum is complete and this is the last scenario in the current filtered list — matching exactly when `(n)` leads to the summary; re-reachable indefinitely by replaying the last scenario in the list and pressing `(n)` again (#145)
- Optional custom Helix keymap support (#163): setting `use_helix_keymap: true` in `~/.config/helix-trainer/config.json` translates gameplay keypresses through the user's real Helix `config.toml` (`dirs::config_dir()/helix/config.toml`, a fixed path — not user-configurable). Disabled by default.
  - New `src/input/keymap/` module: `PhysicalKey` (normalized, hashable keypress — the sole overlay lookup key, matching crossterm's own SHIFT/case normalization convention) and `CanonicalKeys` (one or more canonical Helix key tokens, with a `tokens()` tokenizer shared with `parse_helix_key_string`) and `KeymapOverlay` (forward-only `(KeyContext, PhysicalKey) -> CanonicalKeys` translation table, keyed by `InputState::key_context()`).
  - New `src/config/keymap/` module: parses `[keys.normal]` (single key -> command name) and nested `[keys.normal.<g|m|z|[|]>]` minor-mode remaps, resolving each against a new `helix_name -> key` reverse index on `CommandRegistry` (`key_for_name`). Unsupported forms (key sequences, `@`-macros, `:`-typable commands, minor-mode relocation across prefixes, tables other than `g`/`m`/`z`/`[`/`]`) are reported as ignored bindings, not silently dropped; a malformed config, an oversized file, or too many bindings falls back entirely to the stock keymap.
  - Translation happens once, at the `KeyEvent` boundary in `handle_gameplay_input` (`src/input/handlers.rs`) — only in the normal-mode branch, never in insert mode. `Message::ExecuteCommand`/`MiniGameCommand` gained a second field (`{ keys, typed }`) so `KeyHistory` still shows the physically-pressed key, not the translated command.
  - A multi-token expansion (one physical key resolving to more than one canonical key, e.g. remapping a key to `goto_last_line` -> `"ge"`) is applied only when the input state machine is in `Base`, via `InputStateMachine::apply_canonical_expansion`'s clone-and-commit-or-discard: every intermediate token must transition and the final token must execute, or the attempt is discarded and the machine is left untouched.
  - `UserProfile` gained `keymap_fingerprint: Option<u64>` (`#[serde(default)]`), a hash over the resolved keymap's bindings; a mismatch against the currently active keymap at profile-load time raises a notification that review history was recorded under a different mapping.
  - Gameplay-only: menus, results, filter screens, and scenario hint prose keep the stock keymap.
- Fixed a pre-existing bug where `AppConfig`/its on-disk wrapper had no `#[serde(default)]`, so any config file missing a field (e.g. one written before a new field was added) failed to parse entirely and silently discarded the user's whole config back to defaults; both now default missing fields individually.
- Corrected ~35 wrong `CommandMetadata.name` values across `src/helix/registry/definitions/*.rs` to their real Helix 25.07.1 command names (e.g. `move_left` -> `move_char_left`, `search_forward` -> `search`, `view_center` -> `align_view_center`); added `CommandMetadata::alias_only` for trainer-internal convenience bindings with no real upstream Helix command name (`^`, `Ctrl-r`, dot-repeat); normalized the `CMD_CTRL_R` constant to `"Ctrl-r"` casing, consistent with `CMD_PAGE_UP`/`CMD_PAGE_DOWN`. Checked in `src/helix/registry/helix-25.07-commands.txt`, a reference list of real Helix 25.07.1 command names, plus a test asserting every non-alias registered name is a member.
- New `src/time.rs` module: `Clock` trait abstraction over the current time, with a `SystemClock` production implementation and a `FakeClock` for deterministic tests (#269)
- A persistent, scrollable Achievements screen (`p`/`s`/`a` cycles between Profile, Statistics, and Achievements from the mode selection, menu, profile, statistics, and paused-arcade screens) listing every achievement with its unlocked/locked status, so players can review the full roster outside the transient unlock toast (#292)
- Named registers: `"<reg><op>` (e.g. `"ay`, `"ap`) scopes `y`/`p`/`P`/`R` to a named register instead of the default one, via a new `RegisterFile` on the simulator; unnamed `y`/`p`/`P`/`R` are unchanged and remain equivalent to `""y`/`""p`/`""P`/`""R` (#282)
- Command-line mode: `:` opens a command-line prompt; only `:goto N` (alias `:g N`) is implemented, transcribed from real Helix `:goto` semantics (1-based line number, clamped to the last real line, `:goto 0` is a silent no-op, cursor moves to line start) — no other typed command (`:w`, `:sort`, `:clear-register`, shell commands, etc.) is implemented, since the trainer has no file/buffer/LSP/shell concepts to back them (#282)
- New scenarios: one named-register scenario (`scenarios/en/registers/named-registers.toml`) and one `:goto` scenario (`scenarios/en/movement/command-line-goto.toml`); new `ScenarioCategory::Registers`
- New scenarios covering previously-untested Normal Mode commands (#198): `scenarios/en/movement/scroll.toml` (`Ctrl-d`/`Ctrl-u`/`Ctrl-f`/`Ctrl-b` half-page/page movement, including document-boundary clamping) and `scenarios/en/selection/select-all-replace.toml` (`%` select-all, `R` replace-selection-with-yanked). At the time, `zz`/`zt`/`zb` (view alignment) and `q`/`Q` (macros) were not scoreable/implemented in this simulator and were intentionally omitted, and `s`/`S` (regex select/split) shipped as commented-out placeholders pending their still-stubbed simulator implementation - `q`/`Q` and `s`/`S` are now implemented, see the macro recording/replay and regex-selection entries below (#337, #338). `x`/`X` (line selection) and `ge` (Helix's "go to last line", the equivalent of vim's `G`, which this keymap does not bind) already had scenario coverage
- Fixed the input layer so `Ctrl-b`/`Ctrl-f`/`Ctrl-u`/`Ctrl-d` (page/half-page movement) and bare `R` (replace-selection-with-yanked) are actually reachable from a real key press in the TUI: `map_single_key_command` (`src/input/typestate/key_mapping.rs`) previously read only SHIFT/ALT from modifiers, so a Ctrl-modified key fell through to its unmodified bare-key command (e.g. `Ctrl-d` resolved to delete-selection instead of half-page-down, `Ctrl-f` was dropped entirely), and had no `R` arm at all; `BaseState`'s handler (`src/input/typestate/handlers/base.rs`) separately swallowed all CONTROL-modified keys except `Ctrl-r`/`Ctrl-c`. Both layers now recognize `Ctrl-b`/`Ctrl-f`/`Ctrl-u`/`Ctrl-d`, and bare `R` is now bound
- `helix::commands::normalize_command_id` canonicalizes register ops and command-line invocations (`"ay` -> `"y`, `:g 3` -> `:goto`) to a stable FSRS/quest card id, applied at all three places a raw command string is recorded for learning/quest tracking
- Regex-based selection commands `s` (select regex matches within the current selection) and `S` (split selection on a regex delimiter) (#337): `s`/`S` now open a prompt (`RegexPromptPending`, mirroring the existing `:` command-line prompt), assembled atomically as `"s <pattern>"` / `"S <pattern>"` on Enter and dispatched via `helix_core::selection::{select_on_matches, split_on_matches}` against the new `helix-stdx` dependency's `rope::Regex`. An invalid pattern cancels the prompt rather than erroring. Two new scenarios in `scenarios/en/selection/select-all-replace.toml`, previously commented out pending this implementation
- Macro recording and replay via `q` (toggle recording) and `Q` (replay) (#338): a new `MacroRecorder` on the simulator records successfully-executed command strings while recording is active and replays them through the same dispatch path as live input (`execute_command_any_mode`). Single unnamed macro register (named registers deferred). New `scenarios/en/macros/basic-macros.toml` with two scenarios; new `is_recording_macro()` indicator in the Task/Arcade instructions bar
- Fixed a latent bug in `.`-repeat's mode-restore (`AnyModeSimulator::execute_repeat_impl`): the `is_repeating`/`repeat_depth` reset was gated on the Normal-mode arm only and assigned rather than restored the prior value, which macro replay (able to end in Insert mode, unlike `.`) made newly reachable; the restore is now unconditional across both mode arms and uses `saturating_sub`

### Dependencies

- Added `helix-stdx` (git, tag `25.07.1`, same source/tag already trusted for `helix-core`) for `rope::Regex`, required by `select_on_matches`/`split_on_matches`'s signature for the new `s`/`S` regex-selection commands (#337)

### Changed

- `test_failed_save_does_not_corrupt_existing_profile` no longer forces a save failure by `chmod`-ing the target directory read-only, which silently no-ops when the test runs as root; it now pre-occupies the exact temp-file path `ProfileStorage::save` will use with a directory, so the write fails deterministically via a hard OS type constraint regardless of privilege level (#295)
- Added property-based test coverage (`proptest`) for FSRS scheduling invariants in `src/learning/performance.rs`: `PerformanceTracker::record_attempt`'s resulting card state is deterministic for a given rating sequence and simulated clock, and `stability`/`difficulty`/`scheduled_days`/`due` stay within valid bounds across arbitrary rating sequences (#263)
- Minor internal cleanup batch (#278): deduped the category/difficulty filter-toggle logic in `handlers/filters.rs` behind a shared `toggle_filter_set` helper; named the repeated "scenario count + 4" menu-item-count expression as `handlers/menu.rs::total_menu_items()`; `command_to_key_event` now logs a `tracing::warn!` when it falls back to a Space keypress for an unparseable key command instead of failing silently; replaced `use crate::security::limits::*` wildcard imports with explicit named imports in `config/scenarios.rs` and `config/quests/mod.rs`; added a proptest covering `select_challenge_scenarios` determinism, boundedness, and pool-membership
- **BREAKING**: Introduced a `Clock` abstraction (`helix_trainer::time::Clock`/`SystemClock`/`FakeClock`) so day-boundary and scheduling logic no longer calls `chrono::Utc::now()` directly, enabling deterministic tests (#269):
  - `PerformanceTracker` and `Scheduler` now hold an injected `clock: Arc<dyn Clock>`; `new()` defaults to `SystemClock`, `with_clock()` accepts an explicit clock. `PerformanceTracker::from_stats_with_clock` added alongside `from_stats`; `PerformanceTracker::set_clock` added so an existing tracker can be re-pointed at a shared clock.
  - `ProgressState` gained a private `clock: Arc<dyn Clock>` field (accessed via `now()`/`today()`/`clock()`, not exposed directly — reassigning it post-construction would not re-point `performance_tracker`/`scheduler`) and a `with_clock()` constructor that propagates the same clock instance to both; `AppState::with_clock` and `TestAppStateBuilder::with_clock` plumb it through from the composition root.
  - The following now take a mandatory trailing `now: DateTime<Utc>` (or `today: NaiveDate`) parameter instead of reading the system clock internally: `StreakManager::update_streak`, `UserProfile::reset_daily_quests`, `Achievement::unlock`, `QuestGenerator::generate_quests`, `ScenarioCompletion::new`/`record_attempt`, `ScenarioHistory::record_completion`, `Analytics::get_progress_over_time`, `ChallengeProgress::{can_attempt, attempts_remaining, start_attempt, is_today}`, `ChallengeConfig::is_today`, `ScenarioCompletionService::record_and_scale_xp`.
  - `ChallengeConfig::for_today()` and `impl Default for ChallengeConfig` were removed; use `ChallengeConfig::for_date(today)` instead.
  - `UserProfile::new()` is unchanged (still the one sanctioned `Utc::now()` call site); added `UserProfile::new_at(now)`.
  - As part of this refactor, `PerformanceTracker::record_attempt`, `ScenarioCompletion::record_attempt`/`check_and_reset_daily`, and the profile-load streak/quest-refresh checks in `data_handling.rs` now read the clock once per operation instead of twice (or more) independently — a deliberate fix for latent midnight-boundary races, not just a mechanical rename.
  - Monotonic/`Instant`-based timing (arcade/survival countdowns, notification durations) is unaffected and deferred to a follow-up issue.
- **BREAKING**: `ScenarioFilter`'s `completed_only`/`not_completed_only` booleans are replaced by a single `completion: CompletionFilter` field (`Any`/`CompletedOnly`/`NotCompletedOnly`), making the previously-representable-but-unreachable both-true state — which would have silently filtered out every scenario — structurally impossible (#270)
- **BREAKING**: `DifficultyController::next_scenario` takes a mandatory trailing `rng: &mut R` (`R: rand::RngExt + ?Sized`) instead of calling `rand::rng()` internally, so scenario selection can be seeded deterministically in tests; production behavior is unchanged (#279)
- **BREAKING**: `ScenarioLoader::available_locales` is now a `&self` method scanning `allowed_base_paths` instead of an associated function hardcoding `./scenarios`; `QuestLoader::load_for_locale` similarly now scans `allowed_base_paths` (trying `<base>/<locale>/daily.toml` in order) instead of hardcoding `./quests` (#279)
- `QuestLoader::load_for_locale` now returns the underlying error from the last attempted candidate path (path-traversal rejection, oversized file, malformed TOML, or template validation failure) when every path fails, instead of a generic `ScenarioLoadError` that discarded the real cause (#279)
- `ScenarioLoader` and `QuestLoader` filesystem-loading paths are now documented as intentionally retained for a planned "custom scenario/quest packs from disk" feature; production loads exclusively from embedded data (#279)
- `Setup` and `TargetState` now share their cursor/selection fields and logic through a single `CursorSpec` type (`#[serde(flatten)]`) instead of duplicating both; scenario TOML files are unaffected (#268)
- Quest templates now use a single adjacently-tagged `QuestSpec` enum instead of a separate `type` discriminator and untagged `params` enum, making a `type`/`params` mismatch a deserialization error instead of a runtime-only check; quest TOML files are unaffected (#274)
- **BREAKING**: `MiniGameStats` now embeds `MultiplierState` as its sole owner of the streak/multiplier tier table instead of duplicating it; the `multiplier`, `streak`, and `best_streak` fields are removed (read them via the new `multiplier()`, `streak()`, `best_streak()` accessors), and `lives`/`level` are now private with `lives()`/`level()` accessors. `MiniGameStats`'s `Serialize`/`Deserialize` wire format changes accordingly (three scalar fields become a nested `multiplier_state` object); nothing in-tree persists `MiniGameStats` today. `MiniGameStats::increase_streak`/`reset_streak`/`calculate_multiplier` and `MiniGameSession::multiplier_state`/`streak_for_next_tier` are removed as dead duplicate paths (#259)
- Dependency hygiene: bumped `thiserror` to 2.0.20; removed the stale `RUSTSEC-2024-0436` (paste) ignore entry from `deny.toml`, no longer matched by any dependency; corrected the `RUSTSEC-2025-0141` (bincode) ignore comment to reflect its actual transitive path (`bincode -> syntect`) (#261)
- **BREAKING**: `StreakChange::Protected`'s vestigial `used_freeze` field (always `true` in practice — the `false` case was never constructed) is dropped; it is now a unit variant (#310)

### Fixed

- The custom Helix keymap's startup "N bindings applied" count now matches the number of bindings actually live in the overlay: two TOML keys normalizing to the same physical key (e.g. `G` and `S-g`, or a bare `` ` `` and `` A-` `` on macOS) previously both counted as applied even though the overlay only keeps one of them - whichever `toml::Table` resolves last, which is lexicographic key order, not file-declaration order. The shadowed binding is now reported as an ignored binding (`KeymapWarningReason::Shadowed`) instead of silently inflating the applied count (#347)
- `keymap_fingerprint` now treats the stock keymap as its own distinct mapping (it fingerprints identically regardless of *why* it's active - disabled, never configured, or a malformed config falling back) instead of being skipped from tracking entirely. Previously, disabling a custom keymap left the last custom fingerprint untouched, so a disable -> stock-play -> re-enable cycle could see the stale value silently "match" the re-enabled mapping and hide that FSRS review history was partly recorded under the stock keymap in between; every genuine mapping transition (stock <-> custom, or between two different custom mappings) is now flagged (#348)
- `StreakManager::update_streak` no longer lets a single streak freeze protect an arbitrarily long absence; a freeze now only covers a gap of up to `STREAK_FREEZE_MAX_GAP_DAYS` (3 days) since the last recorded activity, matching a Friday-to-Monday weekend absence. Beyond that cap the streak breaks normally (`StreakChange::Broken`) and the freeze is left unconsumed, even if one is available and the prior streak was non-zero (#325)
- `StreakManager::use_freeze` previously consumed a streak freeze unconditionally, with no gap-length check, inconsistent with `update_streak`'s cap-aware freeze policy above; both now share a `try_consume_freeze` helper. `use_freeze` takes the gap in days as a parameter and now returns a new `GamificationError::StreakFreezeGapOutOfRange` (gap out of the coverable range) or `GamificationError::StreakFreezeNothingToProtect` (no active streak) instead of silently no-oping when the freeze can't be applied (#346)
- `StreakChange::Broken` and the `StreakBroken` notification now carry a `freeze_could_not_cover_gap` flag, true exactly when a break happened despite a held freeze because the gap exceeded the freeze's cap (freeze left unconsumed), so that case reads distinctly from a plain break with no freeze available instead of both saying "reset after a missed day". The `StreakFreezeGranted`/`StreakFreezeUsed` notifications now use "away" (elapsed-day) phrasing consistent with the freeze's actual gap-based coverage, and `StreakFreezeGranted` states the day-cap instead of implying unlimited coverage (#345)
- The Current/Target editor panels no longer soft-wrap long lines; a line (including an appended past-EOL cursor block) that exactly filled a panel's inner width could soft-wrap in one panel but not the other, desyncing every subsequent rendered row between the two panels (#333). Both panels now truncate instead and share a single cursor-following horizontal scroll offset, derived from the Current panel's primary cursor column and applied identically to both panels, so long lines and past-EOL cursors stay reachable and Current/Target columns stay aligned. Known limitation: because the shared offset follows only the Current panel's cursor, a long Current-panel line can drive the offset far enough that the Target panel's own cursor/selection scrolls off-screen (reachable via `scenarios/en/movement/line-navigation.toml`); this self-corrects on the next keystroke and is a trade-off of the column-aligned design, not a regression.
- The Current/Target editor panels now scroll vertically to follow the cursor instead of always rendering from the top of the document; on a document taller than the visible pane the cursor's line could previously fall outside the rendered window and never become visible (#339). Mirrors the existing horizontal follow-cursor scroll: derived from the Current panel's primary cursor row and the panel's inner height, applied identically to both panels, centering the cursor in the viewport (clamped to the document's last page) instead of pinning it to the pane's bottom row, so page-up motions (`Ctrl-u`/`Ctrl-b`) move the cursor within the pane rather than gluing it to the bottom edge. Known limitation, same trade-off as #333's horizontal offset: because the shared offset follows only the Current panel's cursor, inserting lines into Current so it grows taller than Target can drive the offset past Target's own line count and blank the entire Target panel until the cursor moves back up; not reachable from any shipped scenario's initial state.
- `calculate_menu_scroll` no longer underflows/panics when the menu's visible height is 0 (e.g. a very short terminal collapsing the menu area), returning the caller's scroll offset unchanged instead (#321)
- Arcade mode's Esc key now cancels a pending prefix state (count, `g`/`m`/`z`, register selection, command-line buffer) instead of always pausing the game — **behavior change**: previously the first Esc while e.g. a count prefix was building would pause the mini-game; it now cancels the prefix on the first Esc and pauses on the second. Without this fix the new command-line buffer was an unrecoverable trap in arcade mode, draining the scenario timer until Enter or a lucky escape (#282)
- Arcade mode's input state (pending count/prefix/register/command-line buffer) is now reset when advancing to the next scenario, instead of potentially leaking into the next scenario's fresh input (#282)
- An unmapped Alt- or Ctrl-modified key reaching the fallback key-to-command path is now dropped instead of being silently serialized as the bare character — previously an unmapped Alt-x reached the input state machine as plain `x`, executing whatever `x` resolves to instead of being ignored (#282)
- `parse_key_code`'s single-key check now counts chars instead of bytes, so a non-ASCII character (or a macOS Option-composed character not otherwise mapped) is correctly rejected instead of silently falling through to a literal space (#282)
- The Current/Target editor panels now always split into equal-width columns instead of `Constraint::Percentage(50)`/`Percentage(50)`, which on odd-width terminals handed the leftover column to one side unpredictably; the resulting width mismatch made the same line soft-wrap in one panel but not the other, desyncing every subsequent rendered row between the two panels and making some scenarios look unsolvable even though the underlying scoring was unaffected (#192)
- Arcade mode now fires the `LevelUp` notification for level-ups driven by quest XP, scenario-completion XP, and end-of-game XP, matching the training-mode path; previously all three `add_xp` calls in the arcade handlers discarded their `leveled_up` result (#309)
- Breaking a daily streak (a missed day with no freeze available) now surfaces a `StreakBroken` notification instead of only being logged, mirroring the existing `StreakFreezeGranted`/`StreakFreezeUsed` notifications (#310)
- Returning to the menu from the arcade game-over screen no longer re-runs game-over XP/stat/FSRS bookkeeping a second time (#317; the original per-call-site `GameOver`-state check this shipped with was superseded by #323's session-level idempotency guard, see below)
- A streak freeze is no longer consumed (and no `StreakFreezeUsed` notification pushed) when `current_streak` is already 0, mirroring the existing `was_streak > 0` guard on the `StreakBroken` notification (#319)
- Finishing a review session now fires the `LevelUp` notification when its XP award crosses a level threshold, matching the training-mode and arcade-mode paths (#318)
- Quitting the app with a live arcade session now awards the same XP/score/FSRS game-over bookkeeping that quitting to the menu or an in-game timeout does, instead of silently discarding the in-progress session. This covers both ways to quit - the global `QuitApp` message (Ctrl-C) and the main-menu "Quit" item, which previously set `ui.running` directly and skipped bookkeeping entirely; `handle_quit_app` is now the single sink both paths route through (#324)
- Arcade game-over bookkeeping (`handle_minigame_game_over`) is now idempotent per session via `MiniGameSession::try_begin_game_over`, instead of relying on callers to separately check `MiniGameState::GameOver` before invoking it; this removes the implicit invariant introduced by #317's fix, under which any future producer of `GameOver` not paired with an external guard could silently double-award (#323)
- `PerformanceTracker`'s FSRS `retrievability` calculation passed the decay parameter to `fsrs::current_retrievability` with the wrong sign, which could drive the underlying forgetting-curve formula's base negative and yield `NaN` (e.g. after just two failed reviews a day apart) instead of a valid recall probability. This was more than a dead-field concern: `serde_json` serializes a `NaN` `f32` as `null`, and `CommandPerformance`'s derived `Deserialize` failed outright on that `null`, so any profile that had accumulated one silently became permanently unloadable. `retrievability` now also deserializes tolerantly (a `null` recovers as `1.0`) so an already-affected `profile.json` can load again. Discovered via the #263 proptest coverage added by this same release, not itself tied to an issue
- Scenario completion checking now requires the editor to be back in Normal mode, not just matching content/cursor/selections; scenarios whose solution is a bare mode-entry command followed by `Escape` (e.g. `o`, `Escape` for "Insert line below") no longer complete on the mode-entry keystroke alone, before `Escape` is pressed (#283). `open_below_001`/`open_above_001` now hint that Escape is required, the progress bar no longer reads 100% while still in Insert mode, and the hint panel no longer swallows the first Escape needed to exit Insert mode
- Completing a scenario that also completes a daily quest (`ScenarioCompletion`/`SpeedRun`/`TimeInvested`) no longer double-counts the quest's XP reward; it was applied once inside `award_quest_completion_xp` and a second time via `record_scenario_completion`, since both fed off independent "newly completed" trackers. The results screen's XP breakdown now reads the same authoritative bonus list `award_quest_completion_xp` returns instead of independently re-deriving "newly completed" from a session-local set that didn't survive a mid-day restart (which previously could show phantom quest bonuses, or lose them entirely, after a restart) (#292)
- Arcade mode no longer silently drops XP for a `CommandPractice`/`Exploration` quest that completes on a keystroke which doesn't also finish the current scenario; quest completion XP is now awarded on every keystroke, not only when the scenario also completes (#292)
- An account level-up caused purely by quest XP (with the scenario's own XP not reaching the next threshold on its own) now still fires the `LevelUp` notification and triggers an immediate save, instead of being missed because quest XP and scenario XP are applied via two separate `add_xp` calls (#292)
- Consuming a streak freeze to protect a missed day now surfaces a `StreakFreezeUsed` notification instead of only being logged, mirroring the existing `StreakFreezeGranted` notification (#292)
- The achievements screen's list now scrolls (`j`/`k`/arrow keys) and shows a position indicator instead of silently truncating with no indicator when the terminal is too short to fit all achievements; its layout margin was also reduced so the instructions bar itself doesn't get squeezed to zero content height at the minimum supported terminal size (80x24) (#292)
- CI now runs the full `cargo nextest` test surface (`--workspace --all-features --lib --bins --tests`) instead of `--lib` only, so integration tests under `tests/` (e.g. `tests/scenario_validation.rs`, which verifies every scenario's documented solution actually completes it) are no longer silently skipped in CI (#288)
- Added `.gitattributes` (`* text=auto eol=lf`) to force LF checkouts on every platform; without it, Windows checkouts converted `scenarios/`/`quests/` TOML files to CRLF, and since those are embedded verbatim via `include_str!()`, the TOML parser preserved `\r\n` literally inside multi-line strings, corrupting scenario content and breaking cursor/column math for roughly half of all scenarios on Windows only — surfaced immediately once CI began running `tests/scenario_validation.rs` on `windows-latest` (#288)
- Pausing an arcade/survival/challenge mini-game session no longer lets the active scenario's countdown timer keep draining in real time; elapsed time now excludes accumulated paused duration (#271)
- Arcade mode's key-history popup is now gated behind a visibility flag reset on scenario transitions instead of staying permanently visible after the first keypress, matching Training mode's behavior; the popup is also repositioned/capped to avoid overlapping the Target/Timer/Score HUD or corrupting editor pane borders in both modes (#272)
- Scenario `id` fields with an empty string now fail validation, matching quest template behavior (#275)
- `yank` (`y`) now copies the full `anchor..head` range of the primary selection instead of a single character, so text objects and extend motions yank the entire selected text (#266)
- `paste_after`/`paste_before` (`p`/`P`) now insert at the correct end of a range selection regardless of its direction, instead of always using the raw (possibly backward) `head` position (#266)
- `redo` (`U`, `Ctrl-r`) now restores the most recently undone change via a redo stack built on the existing document history, instead of being a no-op; repeated redo walks forward through history and round-trips correctly with undo (#265)
- Applying a transaction with no actual document changes no longer records a spurious undo step or discards pending redo history (#265)
- Wire up daily-streak, streak-freeze, and achievement triggers that were fully implemented but never invoked outside of unit tests (#267, #257, #256):
  - Quest completion now marks `UserProfile::completed_quests_today`, so `StreakManager::update_streak` (still evaluated once per app launch, at profile load) can increment `current_streak` on the following day's session instead of the set staying permanently empty (#267)
  - Streak-freeze eligibility now requires completing every quest generated for the day rather than a fixed count of 5, which the quest generator can never produce (max 4/day) and was therefore unreachable; eligible completion grants a freeze via `StreakManager::grant_freeze`, surfaced with a new `StreakFreezeGranted` notification (#257)
  - Scenario completion and profile load now check `AchievementEngine::check_achievements` and unlock newly satisfied achievements — including retroactively unlocking every tier a profile already qualifies for, not just the highest one — surfaced via the existing `Achievement` notification (#256)
- FSRS review history, XP, level, and quest progress are no longer lost on a crash, forced kill, or terminal close mid-session (#258, #273). Every mid-session profile save — scenario completion, XP award, mini-game game-over, and review-session completion/abandon — now syncs the live FSRS tracker state before writing, routed through a single centralized `ProgressState::save_immediate`/`save_debounced` path instead of five duplicated (and partly stale) save call sites.
- Profile saves (`~/.config/helix-trainer/profile.json`) are now atomic: the write goes to a temp file, is `fsync`ed, and is renamed into place, so a crash or power loss mid-write can no longer leave a truncated or corrupted profile behind.
- A failed profile save after scenario completion — including the level-up save and the achievement-unlock save — no longer terminates the app; it now logs and continues, matching the review-session and mini-game handlers' existing policy, so all profile save-error handling is consistent across the codebase (#296)
- The achievement-unlock save in scenario completion now goes through `ProgressState::save_immediate` instead of a raw `storage.save`, so it no longer skips the FSRS tracker sync and persist stale `performance_data` when it lands outside a level-up (#296)
- Leveling up the account is now surfaced with a `LevelUp` notification through the live scenario-completion path, pushed after any mastery/achievement notifications from the same completion so it isn't evicted from the notification queue's fixed-size visible window; the dead `Message::AwardXP`/`handle_award_xp` path (unreachable in production) has been removed (#293)
- Arcade/Survival/Challenge mini-game scenario completion now calls `AchievementEngine::check_and_unlock`, surfacing an `Achievement` notification the moment a condition is satisfied mid-session instead of only retroactively on the next profile load; it also now feeds `ScenarioCompletionService::update_profile_counters` (`scenarios_completed`/`perfect_scenarios`), which it previously never did, so mastery/perfect-run achievements (`FirstPerfect`, `Perfect10`, `Perfect100`, `Centurion`, `Veteran`, `Legend`) are reachable from arcade play too (#291)
- `AchievementEngine::check_achievements` now evaluates `SpeedDemon`, `Speedrunner`, `Flash`, and `Polyglot`, which previously had no evaluation arm and were unreachable through play (#290):
  - `SpeedDemon`/`Speedrunner`/`Flash` are driven by new `UserProfile::speed_run_count`/`flash_run_count` counters, incremented on scenario completions finished under 50%/25% of the scenario difficulty's base time budget, in both Training and mini-game modes (via the new shared `gamification::speed_time_ratio` helper)
  - `Polyglot` is driven by a new `UserProfile::difficulties_completed` set, populated on scenario completion in both Training and mini-game modes
- Mid-session profile saves (review answers, scenario completion, mini-game game-over) no longer block the TUI event-loop thread on the blocking `fsync` syscall — `ProgressState::save_immediate`/`save_debounced` now dispatch the write to a single serialized save-writer task (`data_loader::spawn_save_writer`) rather than the event-loop thread, and report the outcome via the previously-unused `DataLoadMessage::ProfileSaved`/`ProfileSaveError` messages, so a slow fsync can no longer introduce input latency or stutter during gameplay (#294). Every save — mid-session and exit-time — is funneled through this one writer and applied strictly in the order requested, so a save can never be raced by, and silently lose to, an earlier one still in flight; the application's exit path enqueues its final snapshot last and drains the writer before the process exits, guaranteeing it is the one left on disk and that the exit-time log line reflects whether that save actually succeeded (not just that the writer task didn't panic). A failed off-thread save now also surfaces a notification and re-dirties the profile so the next save retries, instead of being logged only and silently treated as successful; if the writer's queue is ever backed up, a save is skipped and the profile re-dirtied rather than written out of order ahead of older queued saves.
- `ProfileStorage::save` now also `fsync`s the profile directory itself (Unix only) after the atomic rename, so the directory-entry update — not just the file's contents — survives a power loss, not only a process crash or kill; the doc comment on `save` no longer overclaims what the pre-existing atomic-rename protection covered (#297)
- Two instances of the trainer running against the same profile now warn the user on startup instead of silently overwriting each other's progress: `ProfileStorage` maintains an advisory PID lock file (`profile.json.lock`) refreshed on every save; a stale lock left by a crashed process is reclaimed silently, but a lock held by a genuinely live process surfaces a notification, checked both when the profile loads successfully and when it fails and falls back to a fresh profile — the latter being the highest-stakes case, since a fresh fallback profile can otherwise clobber a genuinely live instance's real profile on the next save (#298). `ProfileStorage::delete` now also removes the lock file so a deleted profile doesn't leave a stale one behind.
- Arcade mode now enforces its advertised 60-second session time limit instead of running until all 3 lives are lost regardless of elapsed time; a new session-level clock on `MiniGameSession` (mirroring the existing per-scenario pause-freeze pattern) is checked on every tick, including during the between-scenario transition window, so a session can't overrun its budget by lingering in `Transition`. Expiry ends the session via the same XP/score/FSRS/high-score bookkeeping path as a lives-depleted game-over, without consuming a life (#327)
- Replaced the non-exhaustive `_ =>`/`matches!` fallthroughs across `MiniGameMode` mode-selection code with exhaustive matches, closing a silent-fallthrough hazard where adding a new mode variant would compile cleanly while some call sites kept resolving it to Arcade-equivalent behavior or an empty label — 9 sites total across `src/minigame/modes.rs`, `src/ui/state/screen.rs`, and the arcade mode-select menu's render path (`src/ui/render/mode_selection.rs`, not enumerated by the original report but the same hazard class); `has_session_timer()` now derives from `session_duration()` instead of duplicating its own independently-exhaustive match (#328)

### Performance

- `CanonicalKeys::tokens()` no longer heap-allocates a `Vec` on the common single-token keystroke path (a bare, un-remapped key): new `CanonicalKeys::is_single_token()` answers "is this one token?" without allocating, and the three normal-mode keystroke call sites (`src/input/handlers.rs`, `src/ui/state/handlers/gameplay.rs`, `src/ui/state/handlers/minigame.rs`) check it before falling back to `tokens()` only for actual multi-token keymap expansions (#349)

### CI

- Added a `changes` job (`dorny/paths-filter`) to `ci.yml` that skips the 3-OS `test` matrix, `msrv`, the 3-OS `build` matrix, and `coverage` when a push/PR only touches docs/metadata (no `src/`, `scenarios/`, `locales/`, manifests, or workflow files changed); `fmt`/`clippy`/`security` remain always-on since they're cheap and also cover config files like `Cargo.toml`. Any change under `.github/workflows/**` still forces every job to run. The `gate` job's pass/fail check now accepts a `skipped` result for the gated jobs only when the path filter deliberately excluded them, so a real failure or an unexpected skip still fails the gate.

### Changed

- **BREAKING**: `ScoringConfig.optimal_count` is now `NonZeroUsize` instead of `usize`; TOML layout is unchanged, but zero rejection now happens at parse time, so `optimal_count = 0` now surfaces as "Failed to load scenario file..." instead of "Operation failed..." (#277)
- **BREAKING**: Removed the now-unreachable `SecurityError::InvalidScoringConfig` variant (#277)
- Unified scenario and quest ID validation into `security::validators::validate_id_field` (#275)
- Consolidated scenario/quest TOML parsing, count-limit enforcement, and per-item validation into a shared `config::loader::parse_and_validate` pipeline (#276)
- Collapsed the duplicated `PlayableScenario` trait implementations for `GameSession<Active>` and `GameSession<Completed>` into a single `impl<S: SessionState> PlayableScenario for GameSession<S>` block; the per-state `elapsed()` behavior is now dispatched through a new `SessionState::session_elapsed` associated function (#280)
- **BREAKING**: Removed `AppState::save_profile_debounced`, `AppState::save_profile_immediate`, and `ProgressState::save_blocking` — the serialized save-writer path introduced for #294 left them with no production caller; the application's exit-time save now goes through `AppState::prepare_final_save_request` instead (#294)
- `ScenarioCompletionService::record_and_scale_xp` now returns a named `XPScalingResult` struct instead of an unlabeled `(u64, ScenarioMastery, f64, f64, f64)` tuple, removing the risk of a silent positional swap between its three `f64` fields (#281)

## [0.5.12] - 2026-07-27

### Dependencies

- `serde`: 1.0.228 → 1.0.229 (#242)
- `regex`: 1.12.4 → 1.13.1 (#241, #249)
- `toml`: 1.1.2 → 1.1.3 (#247)
- `tokio`: 1.52.3 → 1.53.1 (#244, #252)
- `anyhow`: 1.0.103 → 1.0.104 (#245)
- `rust-i18n`: 4.1.0 → 4.2.1 (#248)
- `futures`: 0.3.32 → 0.3.33 (#243)
- `thiserror`: 2.0.18 → 2.0.19 (#246)
- `serde_json`: 1.0.150 → 1.0.151 (#253)

### CI

- Bump `lewagon/wait-on-check-action` from 1.8.1 to 1.9.0 (#250)
- Bump `actions/labeler` from 6 to 7 (#251)

## [0.5.11] - 2026-07-07

### Dependencies

- `tokio`: 1.52.1 → 1.52.3 (#220)
- `serde_json`: 1.0.149 → 1.0.150 (#221)
- `rust-i18n`: 4.0.0 → 4.1.0 (#222)
- `fsrs`: 5.2.0 → 6.6.1 (#223, #225, #230)
- `ratatui`: 0.30.0 → 0.30.2 (#226, #234)
- `chrono`: 0.4.44 → 0.4.45 (#227)
- `tui-big-text`: 0.8.4 → 0.8.8 (#229, #233)
- `regex`: 1.12.3 → 1.12.4 (#231)
- `anyhow`: 1.0.102 → 1.0.103 (#236)
- `rand`: 0.10.1 → 0.10.2 (#238)
- `crossbeam-epoch`: 0.9.18 → 0.9.20 (transitive, fixes RUSTSEC-2026-0204) (#238)

### CI

- Bump `codecov/codecov-action` from 6 to 7 (#224)
- Bump `lewagon/wait-on-check-action` from 1.7.0 to 1.8.1 (#228, #237)
- Bump `actions/checkout` from 6 to 7 (#232)
- Bump `actions/cache` from 5 to 6 (#235)

## [0.5.10] - 2026-04-21

### Fixed

- `Excellent` performance tier (80-99%) is now reachable with all existing scenarios by lowering the threshold from 90% to 80% (#201)
- PerformanceTracker (FSRS) data now persisted between sessions (#196)
- `commands_executed` counter now incremented on scenario completion (#196)
- Daily quests now generated on first launch (#195)
- Escape key no longer counts as a game action when dismissing the hint popup (#194)
- Arcade sub-mode headers now display the correct title: Survival shows `SURVIVAL MODE`, Daily Challenge shows `DAILY CHALLENGE` (#197)
- Collapse nested if into match guard for clippy compliance in gamification module

### Dependencies

- `rust-i18n`: 3.1.5 → 4.0.0 (#217)
- `tokio`: 1.49.0 → 1.52.x (#187, #209, #216)
- `rand`: 0.10.0 → 0.10.1 (#215)
- `toml`: 1.0.3 → 1.1.2 (#188, #193, #207, #208)
- `tui-big-text`: 0.8.2 → 0.8.4 (#205, #210)
- `proptest`: 1.10.0 → 1.11.0 (#206)
- `tempfile`: 3.26.0 → 3.27.0 (#189)
- `tracing-subscriber`: 0.3.22 → 0.3.23 (#190)
- `rodio`: 0.22.1 → 0.22.2 (#186)

### CI

- Bump `codecov/codecov-action` from 5 to 6 (#203)
- Bump `lewagon/wait-on-check-action` from 1.5.0 to 1.7.0 (#204, #212, #218)
- Bump `softprops/action-gh-release` from 2 to 3 (#211)
- Bump `actions/github-script` from 8 to 9 (#214)
- Bump `dependabot/fetch-metadata` from 2 to 3 (#213)

## [0.5.9] - 2026-03-07

### Fixed

- Fix double keypress on Windows by filtering `KeyEventKind::Release` events (#182)

## [0.5.8] - 2026-02-09

### Added

- **Arrow key cursor movement** — Optional user configuration to enable arrow keys for cursor movement (#153)

### Changed

- CI: Added dependabot auto-merge workflow for patch-level dependency updates
- CI: Fixed codecov upload to not fail workflow on upload errors
- CI: Fixed labeler workflow to use `pull_request_target` event

### Dependencies

- `rand`: 0.9.2 → 0.10.0 (adapted to `RngExt` trait rename)
- `anyhow`: 1.0.100 → 1.0.101
- `proptest`: 1.9.0 → 1.10.0
- `regex`: 1.12.2 → 1.12.3
- `criterion`: 0.8.1 → 0.8.2
- `tui-big-text`: 0.8.1 → 0.8.2
- `chrono`: 0.4.42 → 0.4.43
- `thiserror`: 2.0.17 → 2.0.18
- `time`: bumped to latest compatible
- `bytes`: bumped to latest compatible

### Quality

- **Tests**: 1857 (up from 1842)
- **Clippy**: Zero warnings

## [0.5.7] - 2026-01-15

### Added

- **Category Filters Screen** — Filter scenarios by category with popup UI (#143):
  - Quick category selection with `f` key from scenario list
  - 9 categories: Movement, Editing, Selection, Clipboard, Search, Text Objects, Surround, Multi-cursor, Insert
  - Toggle categories with Space/Enter, navigate with j/k

- **Multi-cursor Support** — Full support for Helix multi-cursor operations (#142):
  - `C` (copy_selection_on_next_line) and `Alt-C` (copy_selection_on_prev_line)
  - Multi-cursor scenarios for learning cursor multiplication

- **Bracket Preview Highlight** — Visual preview for surround commands (#136):
  - Shows bracket type before confirming surround operation
  - Supports all bracket pairs: `()`, `[]`, `{}`, `<>`, quotes

- **Scenario Progress Tracking** — Display current scenario number in title (#135)

### Fixed

- **Search Command `*` Behavior** — Now correctly matches real Helix behavior (#150):
  - `*` selects current word and sets search pattern without jumping
  - Use `n`/`N` to navigate between matches after `*`

- **Search `N` Command** — Fixed `find_prev` returning same match at cursor end (#148):
  - Proper backward navigation with correct match boundary handling

- **Removed Invalid `#` Command** — Removed Vim-ism that doesn't exist in Helix (#146):
  - `#` (search_word_backward) is not a Helix command
  - Use `*` followed by `N` for backward word search

- **Alt Modifier Key Support** — Proper handling of Alt-key combinations (#140)

### Quality

- **Tests**: 1842 (up from 1626)
- **Clippy**: Zero warnings

## [0.5.6] - 2026-01-10

### Added

- **Embedded Scenarios & Quests** — All 136 scenarios and 55 quests now compiled into binary:
  - Zero runtime file I/O for scenarios/quests
  - Instant startup with no file loading
  - Single-binary distribution (no assets folder needed)
  - `include_str!` macro for compile-time embedding

- **Menu Position Persistence** — Scenario list remembers position when returning from training:
  - Selected item and scroll offset preserved
  - Seamless navigation between training and menu
  - Session-scoped memory (resets on restart)

- **GitHub Auto Labeler** — Automatic PR labeling in CI:
  - File-path based labels (ui, helix-core, game, etc.)
  - Conventional commit prefix labels (feat→enhancement, fix→bug, etc.)

### Changed

- **README** — Added training mode GIF demo in hero section

### Quality

- **Tests**: 1626 (up from 1355)
- **Coverage**: 91% (up from 86%)
- **Clippy**: Zero warnings

## [0.5.5] - 2026-01-10

### Added

- **Audio Feedback System** — Sound effects for mini-game events:
  - Complete/Failed scenario sounds
  - Multiplier up, Level up, Life lost sounds
  - Game over and countdown sounds
  - Timer warning alerts
  - OGG Vorbis format via rodio library (CC0 licensed from Kenney.nl)

- **Game Mode Selection** — Three distinct mini-game modes:
  - **Arcade**: Classic 60-second timed session with 3 lives
  - **Survival**: Endless mode with 1 life, escalating time pressure (10s → 3s per level)
  - **Daily Challenge**: Fixed 10 scenarios, 3 attempts per day, same for all players

- **Sound Toggle** — Global sound on/off with `M` key across all screens

- **Mode Selection Submenu** — Visual popup for choosing game mode when starting Arcade

### Changed

- CI: Added `libasound2-dev` installation for Linux audio support

### Quality

- **Tests**: 1355 (up from 1347)
- **Clippy**: Zero warnings

## [0.5.4] - 2026-01-10

### Added

- **FSRS-based Arcade Mode** — Intelligent scenario selection prioritizes commands needing practice:
  - Overdue commands increase scenario urgency
  - Weak commands (low success rate) appear more often
  - Novel commands get moderate priority for discovery
  - Weighted random selection maintains variety

- **Statistics Screen Improvements** — Real FSRS data integration:
  - Command Mastery Distribution with progress bars
  - Review Status showing due reviews and success rates
  - Commands Needing Practice section (top 5 weak commands)
  - Scenario Mastery distribution (Learning/Proficient/Mastered)

- **Profile Screen Learning Progress** — New section showing FSRS learning metrics

- **Review Session UX** — Improved review experience:
  - Command descriptions from registry
  - Session complete notifications with XP earned
  - "No reviews due" informative message

- **Mastery Level-Up Notifications** — Visual feedback when command mastery improves

- **Navigation Shortcuts** — `m` and `Esc` keys navigate back to mode selection from scenario list

- **Review Shortcut** — `r` key starts review session from all main screens

### Changed

- **Codebase Architecture** — Comprehensive refactoring:
  - Split `input/typestate.rs` (2747 → ~400 lines max per file)
  - Centralized test utilities in `src/testing/` module
  - Split simulator tests into 8 organized modules
  - Removed 21 dead code items

### Quality

- **Tests**: 1268 (up from 1239)
- **Clippy**: Zero warnings
- **Coverage**: 86%

## [0.5.3] - 2026-01-09

### Added

- **Syntax highlighting for scenario code** — Rust code in scenarios now displays with full syntax highlighting using syntect
- **Adaptive difficulty system** — Mini-game difficulty adjusts based on player performance metrics

### Changed

- **136 scenarios with realistic Rust code** — All scenarios updated from minimal synthetic text to real, idiomatic Rust code snippets
- **Paragraph commands use Helix keybindings** — Changed from Vim-style `{`/`}` to Helix-style `[p`/`]p` paragraph navigation

### Fixed

- **Paragraph scenario selection visibility** — Target states now correctly show selection range for `]p`/`[p` commands
- **Cursor rendering at end-of-line** — Cursor now displays correctly when positioned at the end of a line

### Quality

- **Tests**: 1239+ (up from 1116)
- **Clippy**: Zero warnings
- **Scenarios**: 136 total (realistic Rust code content)

## [0.5.2] - 2026-01-07

### Added

- **141 new training scenarios** covering all command categories:
  - Movement: basic, word, line, paragraph, search, text objects
  - Editing: insert, delete, change, replace, case toggle, surround
  - Selection: expand, shrink, line selection, text object selection
  - Clipboard: yank, paste, registers
  - Advanced: macros, multiple cursors, search & replace

- **Quest system** with 5 quest types:
  - Command practice, scenario completion, speed runs
  - Time invested, exploration quests

### Fixed

- **Helix behavior accuracy improvements**
  - `~` (switch_case) now keeps cursor in place and toggles entire selection
  - `p`/`P` (paste) now keeps cursor on last pasted character, not after it
  - `i` (insert mode) now collapses selection when entering insert mode
  - Word movements (`w`, `b`) now correctly create selections in target state

- **XP breakdown display**
  - Separated mastery factor and repeat penalty in results screen
  - Fixed misleading "Learning: -30%" display (was showing combined reduction)

- **Scenario fixes**
  - Fixed scenario sorting to sort by difficulty first
  - Added missing selection fields to 13 movement scenarios
  - Fixed switch_case scenarios to match new cursor behavior
  - Fixed clipboard scenarios cursor positions

### Changed

- CI: Switched to `Swatinem/rust-cache@v2` for proper target directory caching (faster builds)

### Security

- Updated `lru` crate to 0.16.3 (RUSTSEC-2026-0002 soundness fix)
- Added ignore for RUSTSEC-2025-0141 (bincode unmaintained, transitive dep)

## [0.5.1] - 2026-01-07

### Changed

- **Refactored to helix-core primitives** (#110)
  - Migrated text objects (`miw`, `maw`, `mip`, `map`) to `helix_core::textobject`
  - Migrated character search (`f`, `F`, `t`, `T`) to `helix_core::search`
  - Migrated surround helpers to `helix_core::surround::find_nth_pairs_pos`
  - Migrated bracket matching (`mm`) to `helix_core::match_brackets`
  - Migrated paragraph movement (`{`, `}`) to `helix_core::movement`
  - Migrated toggle comments (`Ctrl-c`) to `helix_core::comment`
  - Migrated split selection on newlines (`Alt-s`) to `helix_core::selection`

- **Code cleanup and deduplication** (#111)
  - Extracted `extract_word_at_cursor` helper for search commands
  - Removed ~400 lines of excessive comments and documentation
  - Added 14 new unit tests for helper functions

### Fixed

- `join_selections_space` (`Alt-J`) now correctly selects inserted space (#110)

## [0.5.0] - 2026-01-07

### Added

**Major Feature: 45+ New Commands with Typestate Input Architecture**

This release significantly expands command coverage with a complete typestate-based input system for handling multi-key sequences.

- **Selection Commands (12 new)** (#100)
  - <kbd>s</kbd> select_regex: Select regex matches in selection
  - <kbd>S</kbd> split_selection: Split selection on regex
  - <kbd>Alt</kbd>-<kbd>s</kbd> split_selection_newlines: Split on newlines
  - <kbd>&</kbd> align_selections: Align selections to columns
  - <kbd>_</kbd> trim_selections: Trim whitespace from selections
  - <kbd>Alt</kbd>-<kbd>-</kbd> merge_selections: Merge all selections
  - <kbd>Alt</kbd>-<kbd>_</kbd> merge_consecutive: Merge adjacent selections
  - <kbd>C</kbd> copy_selection_next_line: Copy selection down
  - <kbd>Alt</kbd>-<kbd>C</kbd> copy_selection_prev_line: Copy selection up
  - <kbd>K</kbd> keep_selections_matching: Keep matching selections
  - <kbd>Alt</kbd>-<kbd>K</kbd> remove_selections_matching: Remove matching
  - <kbd>Ctrl</kbd>-<kbd>c</kbd> toggle_comments: Toggle line comments

- **Search Commands (6 new)** (#100)
  - <kbd>/</kbd> search_forward: Search forward with regex
  - <kbd>?</kbd> search_backward: Search backward with regex
  - <kbd>n</kbd> search_next: Jump to next match
  - <kbd>N</kbd> search_prev: Jump to previous match
  - <kbd>*</kbd> search_word_under_cursor: Search word forward
  - <kbd>Alt</kbd>-<kbd>*</kbd> search_selection: Search selection text

- **View Commands (6 new)** (#100)
  - <kbd>z</kbd>/<kbd>zz</kbd> view_center: Center view on cursor
  - <kbd>zt</kbd> view_top: Scroll cursor to top
  - <kbd>zb</kbd> view_bottom: Scroll cursor to bottom
  - <kbd>zm</kbd> view_center_horizontal: Center horizontally
  - <kbd>zj</kbd> scroll_down: Scroll view down
  - <kbd>zk</kbd> scroll_up: Scroll view up

- **Movement Commands (8 new)** (#105, #106)
  - <kbd>{</kbd> goto_prev_paragraph: Jump to previous paragraph
  - <kbd>}</kbd> goto_next_paragraph: Jump to next paragraph
  - <kbd>^</kbd> goto_first_nonblank: Alias for <kbd>gs</kbd>
  - <kbd>Ctrl</kbd>-<kbd>b</kbd> page_up: Scroll page up
  - <kbd>Ctrl</kbd>-<kbd>f</kbd> page_down: Scroll page down
  - <kbd>Ctrl</kbd>-<kbd>u</kbd> half_page_up: Scroll half page up
  - <kbd>Ctrl</kbd>-<kbd>d</kbd> half_page_down: Scroll half page down
  - <kbd>Alt</kbd>-<kbd>.</kbd> repeat_last_motion: Repeat last f/F/t/T

- **Editing Commands (3 new)** (#106)
  - <kbd>Alt</kbd>-<kbd>`</kbd> switch_to_uppercase: Convert to uppercase
  - <kbd>R</kbd> replace_with_yanked: Replace selection with yanked text
  - <kbd>Alt</kbd>-<kbd>J</kbd> join_selections_space: Join with spaces

- **Selection Management (3 new)** (#106)
  - <kbd>,</kbd> keep_primary_selection: Keep only primary selection
  - <kbd>Alt</kbd>-<kbd>,</kbd> remove_primary_selection: Remove primary selection
  - <kbd>Alt</kbd>-<kbd>x</kbd> shrink_to_line_bounds: Shrink selection to line bounds

- **Surround Commands (Match Mode)** (#107)
  - <kbd>ms</kbd> + char: Add surround (wrap selection with brackets/quotes)
  - <kbd>md</kbd> + char: Delete surround (remove enclosing pair)
  - <kbd>mr</kbd> + char + char: Replace surround (change pair type)

- **Text Object Selection** (#108)
  - <kbd>ma</kbd> + object: Select around (includes delimiters)
  - <kbd>mi</kbd> + object: Select inside (excludes delimiters)
  - Supported objects: `w`, `W`, `(`, `[`, `{`, `<`, `"`, `'`, `` ` ``, `p`

- **Lesson Navigation** (#103)
  - Next/Previous scenario navigation after completion
  - <kbd>n</kbd> to go to next lesson from Results screen
  - <kbd>p</kbd> to go to previous lesson
  - <kbd>l</kbd> to return to scenario list

### Changed

- **Typestate Input Architecture** (#100, #102)
  - Replaced `command_buffer: String` with `InputStateMachine`
  - Added `InputStateAccess` trait for uniform state access
  - 8 pending states: `FindCharPending`, `TillCharPending`, `SurroundAddPending`, `SurroundDeletePending`, `SurroundReplacePending`, `TextObjectAroundPending`, `TextObjectInsidePending`, `ViewPending`
  - Type-safe state transitions at compile time
  - `FindState` module to track last find/till motion for repeat

- **Command System**
  - Extended command constants for all new commands
  - Extended `cmd_to_key_events()` for multi-key command sequences
  - Unified handler pattern across all pending states

### Fixed

- **Alt-, Command** (#106)
  - Fixed `Alt-,` to correctly call `remove_primary_selection`
  - Was incorrectly mapped to `repeat_last_motion_reverse` (Vim feature, not Helix)

- **Empty Document Handling** (#108)
  - Fixed panic in `find_surrounding_pair()` when document is empty
  - Fixed panic in `select_around_paragraph()` / `select_inside_paragraph()` for empty documents
  - Added early returns with safe defaults for edge cases

### Quality

- **Tests**: 1116 (was 845, +271 new tests)
- **Clippy**: Zero warnings
- **Commands**: 45+ new commands (total ~90 commands supported)
- **Architecture**: Complete typestate input system for all multi-key sequences

## [0.4.8] - 2026-01-06

### Added

- **Code Coverage Integration** — Codecov workflow with coverage reports (#93)
- **UI Screenshots** — Added visual examples to README (#88)
- **Comprehensive Test Coverage** — 845 tests (up from ~760)
  - Handler tests: minigame.rs, scenario.rs, navigation.rs, profile.rs
  - Render module tests: helpers.rs, editor.rs, screen integration tests
  - Edge case coverage: Unicode, empty content, small/large terminals

### Changed

- **Architecture Refactoring** — 5-phase cleanup for better maintainability
  - Phase 1: Eliminate `Rc<RefCell<>>` from ProgressState (#91)
  - Phase 2: Remove GameState.session duplication (#92)
  - Phase 3: Standardize error handling with `UserError` (#94)
  - Phase 4: Introduce Game Service Layer (#95)
  - Phase 5: Migrate minigame handlers to HandlerContext pattern (#96)

- **CI/CD Improvements**
  - Removed sccache, kept cargo cache for simplicity (#98)
  - Merged coverage.yml into main ci.yml workflow
  - Bumped actions: upload-artifact v6, download-artifact v7, cache v5
  - Increased Miri job timeout to 90 minutes

- **Documentation**
  - Improved README structure and content
  - Reformatted keybindings with `<kbd>` tags
  - Simplified installation section (removed hardcoded version links)

### Dependencies

- `tokio`: 1.48.0 → 1.49.0
- `tui-big-text`: 0.7.3 → 0.8.1
- `toml`: 0.9.8 → 0.9.10
- `tempfile`: 3.23.0 → 3.24.0
- `serde_json`: 1.0.145 → 1.0.148
- `tracing`: 0.1.43 → 0.1.44
- `criterion`: 0.8.0 → 0.8.1

### Quality

- **Tests**: 845 (was ~760, +85 new tests)
- **Clippy**: Zero warnings
- **Architecture**: Cleaner separation of concerns with HandlerContext pattern

## [0.4.7] - 2025-12-03

### Changed

- **Constants Module** — Extracted hardcoded values to `src/constants.rs` (#77)
  - Centralized timing constants (tick rate, animation delays)
  - UI dimension constants (key history size, popup dimensions)
  - Game balance constants (XP values, streak multipliers)
  - Improved maintainability and configurability

- **Unified Command Handling** — Refactored command parsing between modes (#76)
  - Shared `CommandContext` trait for training and arcade modes
  - Eliminated code duplication in key handling
  - Consistent behavior across all game modes

### Fixed

- **Match Mode Multi-Key Handling** — Added <kbd>m</kbd> prefix to multi-key commands (#75)
  - <kbd>mm</kbd> now correctly detected as partial command (was incorrectly treating <kbd>m</kbd> as complete)
  - Fixes issue where Match Mode was not accessible in certain contexts

## [0.4.6] - 2025-12-02

### Added

- **Helix-style Menu Navigation** — Navigate scenario list with vim-like commands
  - <kbd>j</kbd>/<kbd>k</kbd> for up/down movement
  - <kbd>gg</kbd> to jump to first item, <kbd>G</kbd> to jump to last
  - Count prefixes: <kbd>5j</kbd> moves 5 items down, <kbd>10k</kbd> moves 10 up
  - <kbd>15G</kbd> or <kbd>15gg</kbd> jumps directly to item 15

- **Numeric Count Prefixes for Commands** — Execute commands multiple times
  - <kbd>3h</kbd> moves left 3 times, <kbd>5j</kbd> moves down 5 times
  - <kbd>2w</kbd> moves forward 2 words
  - Counts as single action for scoring (not N separate actions)

### Fixed

- **Match Mode Implementation** — <kbd>m</kbd> now correctly enters Match Mode
  - <kbd>mm</kbd> jumps to matching bracket (was incorrectly just <kbd>m</kbd>)
  - Matches official Helix keymap documentation

- **Count Prefix Scoring** — Commands like <kbd>3w</kbd> now count as 1 action, not 3
  - Added `record_action_with_count` method for proper scoring

- **Results Screen Progression Panel** — Improved XP and stats display
  - Added "Your Stats" section with Level, Total XP, Scenarios, Streak
  - Shows earned XP from scenario (+N) next to total
  - Fixed data source (was reading cleared state)

- **Unified Exit Keymap** — <kbd>Ctrl</kbd>-<kbd>Q</kbd> now works consistently across all screens
  - Menu/ModeSelection: exits application
  - Task/Results/Profile/Stats/Review/MiniGame: returns to previous screen

## [0.4.5] - 2025-12-02

### Added

- **Command Registry Architecture** — Type-safe O(1) command dispatch system
  - `CommandRegistry<M>` with mode-specific command registration
  - `CommandMetadata` for rich command documentation (name, key, description, category)
  - `KeyTrie` for efficient multi-key command resolution (<kbd>gg</kbd>, <kbd>ge</kbd>, <kbd>gh</kbd>, <kbd>gl</kbd>, <kbd>gs</kbd>)
  - Category-based organization (Movement, Editing, Selection, Clipboard)
  - Compile-time mode safety via PhantomData markers

### Fixed

- **Cursor display on empty lines** — Use block character (█) instead of space to prevent visual line duplication when cursor is on empty line
- **Append after word movement** — Fix `append()` to handle forward selections correctly (<kbd>e</kbd> + <kbd>a</kbd> now works properly)
- **Scenario corrections**:
  - `append_mode_001`: Fixed expected content from "hello !world" to "hello! world"
  - Removed non-existent <kbd>G</kbd> command (Helix uses <kbd>ge</kbd> for goto last line)
  - Removed duplicate `document_end_001` scenario (use `goto_last_line_001` with <kbd>ge</kbd>)
  - Updated hints to reference correct <kbd>ge</kbd> command instead of <kbd>G</kbd>

### Changed

- Simplified command dispatch from O(n) string matching to O(1) registry lookup
- Reduced cyclomatic complexity in command handling code
- 86 scenarios (removed 1 duplicate)

## [0.4.4] - 2025-12-01

### Added

- **Notification System** — Real-time notifications for level-ups, quest completions, and achievements
  - Auto-dismissing popups (3 seconds) in top-right corner
  - Color-coded by type (yellow/magenta/green/cyan)
  - Queue system for multiple simultaneous notifications

- **Filtering System** — Full implementation of scenario filters
  - Category filtering (toggle Movement, Editing, Clipboard, Advanced)
  - Difficulty filtering (toggle Beginner, Intermediate, Advanced)
  - Completion filtering (cycle: All → Completed → Not Completed)

- **Learning Scheduler Improvements** — Mix new content with reviews
  - 50/50 split between review items and new content in practice sessions
  - Weak command detection (low mastery, high difficulty, many lapses)
  - Untried essential command suggestions

- **Repeat Command Enhancement** — Full support for insert mode entry commands
  - Correctly replay <kbd>a</kbd>, <kbd>A</kbd>, <kbd>I</kbd>, <kbd>o</kbd>, <kbd>O</kbd> with proper entry point
  - `entry_command` field in `RepeatableAction::InsertSequence`

### Changed

- **Type-Safe Handler Architecture** — Compile-time guarantees for screen handlers
  - `HandlerContext<'a>` struct for borrowing non-screen state
  - `HandlerOutcome` enum (Stay/Transition) for explicit screen changes
  - `extract_screen!` macro for type-safe screen data extraction
  - 30+ handlers refactored to use new patterns

- **Unified Command Parsing** — Extracted shared logic into `CommandContext` trait
  - `ParsedCommand` enum (Complete/Partial/Invalid)
  - Single source of truth for command buffer parsing
  - Reduced code duplication between training and arcade modes

### Removed

- 23 redundant tests (3.2% reduction, coverage maintained)
- Phase/Stage/Iteration comments from documentation
- Invalid <kbd>dd</kbd> command references (replaced with <kbd>x</kbd> + <kbd>d</kbd> pattern)

### Fixed

- Unused import warnings after test cleanup
- Damaged "Add" comments from bulk replacement

### Quality

- **Tests**: 685 (was 708, -23 redundant)
- **Clippy**: Zero warnings
- **Code**: Cleaner documentation without development phase references

## [0.4.3] - 2025-12-01

### Added

- **MSRV Check** — CI now verifies build with minimum supported Rust version (1.89)
- **Miri Workflow** — Scheduled weekly undefined behavior detection (Sundays)
- **`#![forbid(unsafe_code)]`** — Enforced in lib.rs and main.rs

### Changed

- **MSRV bumped to 1.89** — Required by dependencies (bytemuck avx512_simd)
- **Float rounding** — XP calculations now use `.round()` for cross-platform consistency
- **Modern Rust syntax** — Refactored to use `if let && condition` chains

### Fixed

- Float-to-int conversion differences between native and Miri execution
- Async tests excluded from Miri (too slow, 300+ seconds each)

## [0.4.2] - 2025-11-30

### Added

**Expanded Commands** (#52)

- **14 New Commands**: Expanded from 31 to 45+ supported commands
  - Find/till character: <kbd>f</kbd>, <kbd>F</kbd>, <kbd>t</kbd>, <kbd>T</kbd> (jump to/before character)
  - Match brackets: <kbd>m</kbd> (jump to matching bracket)
  - Goto commands: <kbd>gh</kbd> (line start), <kbd>gl</kbd> (line end), <kbd>gs</kbd> (first non-whitespace), <kbd>ge</kbd> (last line)
  - Selection: <kbd>x</kbd> (select line), <kbd>X</kbd> (extend to line), <kbd>v</kbd> (select mode), <kbd>;</kbd> (collapse selection)
  - Case switching: <kbd>~</kbd> (toggle case)
  - Delete selection: <kbd>d</kbd> (delete current selection)

- **New Scenario Files**:
  - `find-till.toml`: <kbd>f</kbd>/<kbd>F</kbd>/<kbd>t</kbd>/<kbd>T</kbd> character search (5 scenarios)
  - `goto-commands.toml`: <kbd>gh</kbd>/<kbd>gl</kbd>/<kbd>gs</kbd>/<kbd>ge</kbd> navigation (4 scenarios)
  - `match-brackets.toml`: bracket matching (4 scenarios)
  - `line-selection.toml`: <kbd>x</kbd>/<kbd>X</kbd> line selection (3 scenarios)

### Changed

- **Helix-correct command behavior**:
  - <kbd>d</kbd> now executes immediately as delete selection (not waiting for <kbd>dd</kbd>)
  - <kbd>x</kbd> selects current line only (not including next line)
  - Idiomatic Helix: use <kbd>xd</kbd> to delete line (select + delete)

- **Selection visualization**: Selected text now highlighted with blue background

### Fixed

- Selection not being displayed visually after selection commands
- <kbd>d</kbd> command waiting for second <kbd>d</kbd> instead of executing immediately
- <kbd>x</kbd> command selecting two lines instead of one
- Selection end boundary including next line when it shouldn't

## [0.4.1] - 2025-11-30

### Added

**Expanded Content** (#51)

- **Training Scenarios**: Expanded from 25 to 78 scenarios (3.1x increase)
  - `basic-movement.toml`: <kbd>h</kbd>, <kbd>j</kbd>, <kbd>k</kbd>, <kbd>l</kbd> character navigation (9 scenarios)
  - `word-basics.toml`: <kbd>w</kbd>, <kbd>b</kbd> word navigation (7 scenarios)
  - `line-navigation.toml`: <kbd>0</kbd>, <kbd>gg</kbd> line/document navigation (7 scenarios)
  - `combined.toml`: multi-key navigation challenges (8 scenarios)
  - `precision.toml`: single-step precision movements (8 scenarios)
  - `deletion.toml`: <kbd>x</kbd>, <kbd>dd</kbd> deletion operations (8 scenarios)
  - `delete-advanced.toml`: advanced deletion patterns (6 scenarios)

- **Daily Quests**: Expanded from 12 to 55 quest templates (4.6x increase)
  - Easy (14): Movement commands (<kbd>h</kbd>, <kbd>j</kbd>, <kbd>k</kbd>, <kbd>l</kbd>, <kbd>w</kbd>, <kbd>b</kbd>, <kbd>0</kbd>, <kbd>gg</kbd>, <kbd>G</kbd>), editing (<kbd>x</kbd>, <kbd>dd</kbd>, <kbd>yy</kbd>)
  - Medium (23): Word navigation, insert/append modes, clipboard, undo/redo
  - Hard (18): Marathons, speed runs, time challenges, exploration quests

## [0.4.0] - 2025-11-30

### 🎮 Phase 2.1: Mini-Games Mode (Arcade Training)

This release introduces Mini-Games mode - an arcade-style training experience with time pressure and gamification mechanics designed to build muscle memory through fast-paced repetition.

### Added

**Mini-Games Mode** (#50)

- **Arcade Training Experience**
  - Mode selection screen (Training vs Arcade)
  - 60-second timed sessions with countdown
  - Automatic scenario progression with 2-second transitions
  - 5-10 second time limits per scenario based on difficulty

- **Scoring & Progression**
  - Real-time score tracking with streak multiplier (1.0x → 5.0x)
  - XP awards per scenario (15 base + streak bonus)
  - "+X XP" notification popup during transitions
  - High score tracking per session

- **Lives System**
  - Start with 3 lives
  - Lose life on timeout or excessive actions
  - Lives displayed with ❤️ indicators
  - Bonus life at 1000, 2500, 5000 points

- **UI/UX Improvements**
  - Key history display (last 5 keys, big text)
  - Pause menu (Esc) with profile/stats navigation
  - Game over screen with final score and stats
  - Countdown animation (3... 2... 1... GO!)

- **Integration**
  - Full XP/quest/FSRS integration
  - Profile and statistics accessible from pause menu
  - Proper navigation flow (arcade → pause → profile → arcade)

**Technical Implementation**:

- New `MiniGameSession` struct with state machine (Countdown → Playing → Transition → Paused → GameOver)
- `MiniGameData` screen state with command buffer and key history
- `ReturnDestination` enum for context-aware back navigation
- Unified `render_key_history_popup` (DRY refactoring)
- 82 new tests for mini-game functionality

### Changed

- BackToMenu now returns to ModeSelection (main mode menu)
- XP display fixed to show progress within current level (not total XP)
- Refactored render functions for code reuse between training and arcade modes

### Quality

- All 645 tests passing
- Zero clippy warnings
- Code formatted with rustfmt nightly
- Security audit passed (cargo deny check)

## [0.3.0] - 2025-11-30

### 🚀 Async Non-Blocking Architecture

This release introduces a complete async architecture overhaul, making the application instantly responsive with background data loading.

### Added

**Async Architecture** (#49)

- Tokio async runtime with non-blocking event loop
- `AsyncState<T>` enum for type-safe loading states (Loading/Ready/Failed)
- `DataLoadMessage` enum for channel-based communication
- Background data loaders using `tokio::task::spawn_blocking`
- Parallel loading of scenarios and profile at startup
- Biased `tokio::select!` for responsive keyboard handling

**New Modules**:
- `src/async_state.rs` (179 lines) - Type-safe async state management
- `src/data_loader.rs` (239 lines) - Background data loading functions

**Quest TOML System** (#48)

- Quest templates extracted to TOML files (`quests/en/daily.toml`)
- `QuestLoader` with security validation and limits
- `QuestTemplateRegistry` for template management
- 12 quest templates across 3 difficulty levels
- Strict TOML parsing with `#[serde(deny_unknown_fields)]`
- Validation for IDs, versions, locales, and level ranges

### Changed

**Performance Optimizations**:
- Runtime limited to 2 worker threads (sufficient for TUI workload)
- Instant UI display (~10-50ms to first render)
- Scenarios and profile load in parallel background tasks
- Keyboard events prioritized with biased select

**Code Quality** (#47, #45):
- Large functions split into focused helpers
- Hardcoded commands replaced with constants
- CI/CD improvements with cargo doc tests and benchmark checks

### Performance

- **Cold start**: 50-80ms (UI displays immediately)
- **Background loading**: Scenarios + profile load in parallel
- **Event loop**: Non-blocking, remains responsive during all operations
- **Memory overhead**: 1-2 MB for async runtime

### Quality

- All 563 tests passing (9 new async tests)
- Zero clippy warnings
- Security audit passed (cargo deny check)
- Comprehensive test coverage for `handle_data_message()`

### Security

- Bounded channels (32 messages) prevent memory exhaustion
- Error sanitization chain intact
- No race conditions (single-threaded event loop)
- Path validation layer for quest templates
- All input bounded by strict limits

## [0.2.1] - 2025-11-29

### Fixed

**Esc Key Conflict** (#41)
- Fixed critical conflict where <kbd>Esc</kbd> key was used for both abandoning scenarios and exiting Helix insert mode
- Changed scenario abandonment from <kbd>Esc</kbd> to <kbd>Ctrl</kbd>+<kbd>Q</kbd>
- Now users can properly use <kbd>Esc</kbd> to exit insert mode (native Helix behavior)
- Updated UI instructions to show <kbd>Ctrl</kbd>-<kbd>Q</kbd> instead of <kbd>Esc</kbd>

**FSRS Command Tracking** (#40)
- Fixed Review Session being non-functional due to missing command tracking
- Commands from completed scenarios now properly recorded in FSRS scheduler
- First reviews are immediately available (not delayed until next day)
- Review Commands menu now shows badge [N] when reviews are due

### Changed

**Major Code Quality Improvements** (#41)
- Refactored 6 complex functions into 20+ focused, maintainable functions
- Improved code readability and testability throughout codebase

**execute_command** (src/helix/simulator/commands/mod.rs):
- Split 193-line function into 4 focused functions (26+9+56+95+28 lines)
- Separated Insert mode and Normal mode logic
- Isolated repeat command recording
- Reduced cyclomatic complexity significantly

**handle_task_keys** (src/main.rs):
- Split 112-line function into 5 focused functions
- Clear separation: special keys, Insert mode input, Normal mode mapping
- Better maintainability for keyboard handling

**Configuration & Learning**:
- visit_toml_files: Functional style, no side effects (38 lines)
- update_fsrs_state: Separated FSRS calculation from state mutation (3 functions)
- generate_quests: Data-driven design with QuestDistribution struct
- validate_scenario: DRY principle with reusable validators

**State Handlers** (from v0.2.0, documented):
- Modularized state.rs into focused handler modules
- 8 handler modules: navigation, menu, scenario, gameplay, profile, quests, review, filters
- Reduced main state file from 2,084 to 1,486 lines

### Performance

**CI/CD Improvements**:
- Added cargo registry caching (crates.io index, .crate files, git deps)
- Combined with sccache: full build cache (registry + compilation)
- Dependency downloads: ~10-30s faster on cache hits
- Fixed sccache configuration for 88-100% cache hit rate

### Quality

- All 352 tests passing
- Zero clippy warnings
- Code formatted with rustfmt nightly
- Single Responsibility Principle applied throughout
- Improved testability and maintainability

## [0.2.0] - 2025-11-28

### 🎉 Phase 1 COMPLETE - Smart Learning System

This release completes Phase 1 of the helix-trainer roadmap by implementing the Interactive Review Session UI, the final missing piece of the FSRS spaced repetition system.

### Added

**Interactive Review Session UI** (#39)

- Review session screen with progress tracking ("Reviewing 3/5 commands")
- Command mastery display (Beginner → Intermediate → Advanced → Master)
- Next review date indicator based on FSRS scheduling
- Simple MVP interaction: <kbd>s</kbd> (success), <kbd>f</kbd> (failed), <kbd>Esc</kbd> (abandon)
- Menu integration with yellow badge `[N]` showing count of due reviews
- XP rewards system:
  - Base: 10 XP per command reviewed
  - Success rate bonus: 0-20 XP (Example: 5 reviews at 80% = 66 XP total)
- Keyboard shortcuts: Press <kbd>r</kbd> from menu to start review session

**Technical Implementation**:

- New `Screen::Review` state and `ReviewSessionState` structure
- Review screen rendering module (`src/ui/render/review.rs`)
- Message handlers for review session workflow
- Event handling for review interactions
- 33 comprehensive tests (3 unit tests passing, 30 integration tests documented)

### Performance

All performance targets exceeded:
- **Rendering**: 0.5ms per frame (60+ FPS, 32x faster than 16ms target)
- **Memory**: 440 bytes review state (56% of 1KB budget)
- **State updates**: O(1) complexity
- **Event handling**: <0.1ms (10x faster than 1ms target)

### Security

- **Security Rating**: A+ (rust-security-maintenance audit)
- Zero unsafe blocks
- All input validated through safe Rust enums
- Bounds checking on all array access
- Tamper-proof XP calculations
- Zero vulnerabilities found

### Quality

- All 352 library tests passing
- Zero clippy warnings (`clippy --all-targets --all-features -- -D warnings`)
- Code review approved by rust-code-reviewer agent
- Performance reviewed by rust-performance-engineer agent
- Security audited by rust-security-maintenance agent

### Changed

- Updated README to highlight Phase 1 completion
- Reorganized features section to emphasize Smart Learning System
- Menu now shows "Review Commands (r)" with due count badge

### Phase 1 Status: 100% Complete ✅

With this release, all Phase 1 components are fully implemented:
- ✅ FSRS spaced repetition backend
- ✅ Interactive review sessions (NEW in this release)
- ✅ Daily quest system
- ✅ XP/leveling progression
- ✅ Profile & Statistics screens
- ✅ Scenario mastery tracking
- ✅ Anti-farming protection

## [0.1.5] - 2025-11-28

### Added

**Phase B: Profile & Statistics Menu Integration**

- Main menu integration for Profile and Statistics screens
  - Added "View Profile (<kbd>p</kbd>)" menu item with keyboard shortcut
  - Added "Statistics (<kbd>s</kbd>)" menu item with keyboard shortcut
  - Profile and Statistics now accessible from main menu with arrow navigation
  - Visual separator grouping system options (Profile, Statistics, Quit)
  - Keyboard hints displayed in menu items

- Enhanced navigation
  - Press <kbd>p</kbd> for instant Profile screen access (1-keypress shortcut)
  - Press <kbd>s</kbd> for instant Statistics screen access (1-keypress shortcut)
  - Arrow keys / <kbd>j</kbd>/<kbd>k</kbd> navigation through all menu items
  - Number keys (<kbd>1</kbd>-<kbd>9</kbd>) still work for scenario shortcuts

### Changed

- Menu structure: Scenarios → Separator → Profile → Statistics → Quit
- Updated menu navigation logic to support +3 items (Profile, Statistics, Quit)
- Quit option moved to bottom after Profile and Statistics

### Fixed

- Profile and Statistics screens now discoverable (previously only accessible from results screen)
- Menu scrolling works correctly with new items

## [0.1.4] - 2025-11-28

### Added

**Phase 1.5: Scenario Metadata & Filtering System**

- Rich scenario metadata system
  - `ScenarioMetadata` with category, difficulty, tags, and taught commands
  - 25 scenarios fully categorized: Movement (5), Editing (11), Clipboard (3), Advanced (6)
  - Difficulty levels: Beginner (20), Intermediate (5)
  - Each scenario tagged with taught commands and practice focus areas

- Flexible filtering and sorting system
  - `ScenarioCollection` with six sort modes (alphabetical, difficulty, category, completion, recent, random)
  - Filter by category (Movement, Editing, Clipboard, Advanced)
  - Filter by difficulty (Beginner, Intermediate, Advanced)
  - Filter by taught commands (e.g., show all scenarios teaching <kbd>w</kbd> or <kbd>dd</kbd>)
  - Filter by completion status (completed vs. uncompleted)
  - Chainable filters for complex queries
  - 100% backward compatible with existing scenarios (metadata optional)

- Enhanced UI indicators
  - Difficulty badges: 🟢 Beginner / 🟡 Intermediate / 🔴 Advanced
  - Completion status: ✅ for completed scenarios
  - Visual feedback in scenario menu

- Automated scenario validation tests
  - `test_all_scenarios_load_successfully`: validates all 25 scenarios load without errors
  - `test_all_scenarios_execute_solution`: executes solution commands and verifies target state
  - Catches cursor position errors, invalid command formats, and other issues
  - Runs during CI to prevent scenario regressions

- Performance benchmarks
  - Criterion benchmarks in `benches/filtering_sorting.rs`
  - <1ms filtering/sorting for collections of 1000 scenarios
  - Memory-efficient metadata storage

### Changed

- Integrated `ScenarioCollection` into `AppState` (replacing raw `Vec<Scenario>`)
- Menu now displays difficulty and completion indicators for each scenario
- Updated all 25 scenarios with complete metadata (category, difficulty, tags, taught commands)

### Fixed

- Corrected cursor positions in multiple scenarios:
  - `repeat_insert_001`: Fixed target cursor [1, 17] → [1, 16]
  - `paste_before_001`: Fixed target cursor [0, 1] → [0, 2]
  - `delete_line_001`: Start cursor on line 1 instead of line 0
  - `document_end_001`: Cursor at [4, 9] (end of document)
  - `select_word_001`: Cursor at [0, 9] (after word end)
  - `repeat_indent_001`: Cursor [2, 2] → [2, 4] (after second indent)

- Fixed command format in scenarios:
  - Changed multi-key commands from arrays to strings (e.g., `["d", "d"]` → `["dd"]`)
  - Applies to: <kbd>dd</kbd>, <kbd>gg</kbd>, <kbd>r</kbd>_, and other multi-key sequences

- Fixed repeat command repeatability:
  - Made all printable ASCII characters and space repeatable in `is_repeatable_command()`
  - Allows replace commands like <kbd>r</kbd>_ to work with repeat (<kbd>.</kbd>)

- Fixed indentation in `repeat_indent_001`:
  - Changed from 4-space to 2-space indentation to match simulator behavior

- Fixed append mode behavior in `append_mode_001`:
  - Adjusted target content to match actual <kbd>e</kbd> command behavior (cursor after word, not on last char)

### Performance

- Filtering 1000 scenarios: ~50-200 µs (well under 1ms budget)
- Sorting 1000 scenarios: ~100-400 µs depending on sort mode
- Zero overhead for scenarios without metadata (backward compatible)

### Security

- Added `walkdir = "2.5"` dev-dependency for safe directory traversal in tests
- All scenario validation tests run with bounds checking

### Tests

- Added 16 comprehensive unit tests for `ScenarioCollection`
- Added 2 integration tests for scenario validation
- Total test count: 392 (all passing)
- Zero clippy warnings

## [0.1.3] - 2025-11-28

### Fixed

- **Hint system completely non-functional**: Hints were stored in wrong TOML section and never loaded
  - Moved `hints` arrays from `[scenarios.solution]` to `[[scenarios]]` level in all 11 scenario files
  - Fixed 24 scenarios containing 55 total hints
  - Hints now properly deserialize and display when requested
- **Hint key conflict**: UI showed `[h: Show Hint]` but <kbd>h</kbd> is Helix left movement command
  - Added <kbd>?</kbd> as primary hint key (intuitive, no conflicts)
  - Kept <kbd>F1</kbd> as alternative for accessibility
  - Handles both `Char('?')` and `Char('/')` + <kbd>Shift</kbd> for cross-platform support
  - Updated UI to show `[?: Hint | F1]`

### Added

- **Hint toggle behavior**: Press <kbd>?</kbd> to open hint, press again to close (improved UX)
- **Cross-platform hint key support**: Properly handles different keyboard layouts and modifier keys

## [0.1.2] - 2025-11-28

### Fixed

- Indent/dedent commands (<kbd>></kbd>, <kbd><</kbd>) not working in TUI (#35)

## [0.1.1] - 2025-11-28

### Added

- Scenario mastery tracking system to prevent XP farming
  - Three-tier mastery levels: Learning → Proficient → Mastered
  - Graduated XP scaling: 100% → 50% → 20%
  - Session spam protection (same-day repeat penalties: 100% → 70% → 30%)
  - Prevents farming by reducing XP for mastered scenarios by 80%
  - New module: `src/learning/scenario_history.rs` (600+ lines)
  - 23 comprehensive tests for mastery tracking

- Bounded scenario tracking (MAX_SCENARIOS_TRACKED = 10,000)
  - Prevents unbounded memory growth
  - Defense in depth against DoS attacks

- Scenario ID validation at storage boundary
  - Validates alphanumeric + underscore + hyphen only
  - MAX_SCENARIO_ID_LENGTH = 100 characters
  - Blocks path traversal and injection attempts

- Performance benchmarks and profiling
  - Criterion benchmarks: `benches/xp_scaling.rs`
  - DHAT memory profiling: `examples/memory_profile.rs`

### Changed

- Replaced `String` dates with `chrono::NaiveDate` for type safety
- Consistent `.round()` usage in XP calculations (prevents truncation)
- Mastery level displayed in results screen with emoji indicators

### Fixed

- Floating-point precision issues in XP calculations
- Potential memory exhaustion from unbounded HashMap growth

### Performance

- XP calculation overhead: 260-460 ns (2000x under 1ms budget)
- Memory per scenario: ~288 bytes
- Profile serialization: ~40 µs for 100 scenarios

### Security

- Added MAX_SCENARIOS_TRACKED limit (defense in depth)
- Added scenario ID validation (alphanumeric + `_` + `-` only)
- Type-safe date handling with NaiveDate
- All arithmetic uses saturating operations

## [0.1.0] - 2025-11-28

### Added

**Phase 1: Smart Learning & Gamification System**

- FSRS-based spaced repetition learning system for scientifically-optimized practice
  - Card state management (New, Learning, Review, Relearning)
  - Performance tracking per scenario (Again, Hard, Good, Easy ratings)
  - Intelligent review scheduling (20-30% fewer reviews than traditional methods)
  - Review analytics and statistics
  - Persistent storage in `~/.config/helix-trainer/learning.json`

- Daily quests gamification system with Duolingo-style mechanics
  - Three quest types: Practice (3 scenarios), Learning (2 new), Review (5 cards)
  - Daily quest rotation at midnight local time
  - Streak tracking with loss aversion mechanics
  - Quest completion rewards (XP bonuses)
  - Persistent storage in `~/.config/helix-trainer/quests.json`

- XP and leveling progression system
  - Experience points for scenario completion
  - Performance-based XP multipliers (1.0x to 2.0x)
  - Quest completion bonuses (50-150 XP)
  - Level progression with exponential scaling
  - Profile screen with detailed statistics

- User profile and statistics tracking
  - Total scenarios completed
  - Average performance score
  - Current streak and longest streak
  - Total XP and current level
  - Review history and analytics
  - Persistent storage in `~/.config/helix-trainer/profile.json`

- Enhanced UI screens
  - Profile screen with level, XP, and statistics
  - Statistics screen with review calendar and performance graphs
  - Quest tracker in main menu
  - XP breakdown in results screen
  - Success animations with XP rewards

**Phase A: Foundation (Complete)**

- Core TUI framework with ratatui 0.29
  - Main menu with scenario selection
  - Task screen with editor simulation
  - Results screen with performance rating
  - Keyboard-driven navigation (Vim-style)

- Helix simulator using official helix-core library (v25.07.1)
  - 30+ commands: <kbd>h</kbd>,<kbd>j</kbd>,<kbd>k</kbd>,<kbd>l</kbd>,<kbd>w</kbd>,<kbd>b</kbd>,<kbd>e</kbd>,<kbd>0</kbd>,<kbd>$</kbd>,<kbd>x</kbd>,<kbd>dd</kbd>,<kbd>i</kbd>,<kbd>a</kbd>,<kbd>I</kbd>,<kbd>A</kbd>,<kbd>o</kbd>,<kbd>O</kbd>,<kbd>r</kbd>,<kbd>c</kbd>,<kbd>y</kbd>,<kbd>p</kbd>,<kbd>P</kbd>,<kbd>u</kbd>,<kbd>U</kbd>,<kbd>gg</kbd>,<kbd>G</kbd>,<kbd>J</kbd>,<kbd>></kbd>,<kbd><</kbd>
  - Repeat command (<kbd>.</kbd>) for efficient editing workflows
  - Insert mode with text input, <kbd>Backspace</kbd>, arrow keys
  - Multi-key command buffer (<kbd>dd</kbd>, <kbd>gg</kbd>)
  - Yank/paste clipboard support
  - Automatic completion detection
  - Cursor and selection visualization

- Beautiful UI components
  - Large key history display (tui-big-text, 8-line tall characters)
  - Success popup with 1.5s delay
  - Diff highlighting (red/green) in results
  - Action count indicators
  - Performance rating with emoji (Perfect/Excellent/Good/Fair/Poor)
  - Hint system (<kbd>F1</kbd> key)

- TOML scenario system with security validation
  - 20 training scenarios (basic to intermediate)
  - Configurable setup, target, solution, hints
  - Performance scoring (optimal count, max points, tolerance)
  - Input validation and sanitization
  - Path traversal prevention
  - Content size limits

- Testing and quality infrastructure
  - 164 passing tests (unit, integration, property-based)
  - Zero clippy warnings policy
  - cargo-nextest for fast test execution
  - Proptest for property-based testing
  - 100% test coverage for core modules

- Cross-platform CI/CD pipeline
  - GitHub Actions workflows for Linux, macOS, Windows
  - Automated testing with cargo-nextest
  - Rustfmt (nightly) formatting checks
  - Clippy lints with -D warnings
  - Security audit with cargo-deny
  - Rust 2024 edition support
  - MSRV: 1.85

- Internationalization (i18n) support
  - rust-i18n integration for multi-language support
  - English locale (default)
  - Infrastructure for future language additions

- Build optimization
  - sccache integration for fast incremental builds
  - Cargo cache optimization
  - 5x faster rebuilds (10s vs 54s)

- Release automation
  - Cross-platform binary builds (9 platforms)
  - Linux x86_64 (GNU and musl)
  - Linux ARM64 (aarch64 GNU and musl)
  - macOS x86_64 (Intel)
  - macOS ARM64 (Apple Silicon M1/M2/M3)
  - Windows x86_64
  - Windows ARM64
  - Automatic GitHub releases on tags
  - Changelog generation and extraction
  - SHA256 checksums for all binaries

- Documentation improvements
  - Problem-solution framing in README
  - Quick Links navigation section
  - Consolidated Features section
  - Code examples with real output
  - CONTRIBUTING.md extraction
  - CI status badge
  - Visual TUI example

### Changed

- Upgraded to Rust 2024 edition (latest stable)
- Migrated to helix-core 25.07.1 (from 24.07)
- Optimized dependencies (removed tokio, unnecessary features)
- Refactored codebase into 31 modular files
- Enhanced error messages with context and suggestions
- Improved scoring algorithm for better feedback

### Fixed

- Replace command cursor initialization bugs
- Multi-key command handling edge cases
- Insert mode arrow key navigation
- Yank/paste clipboard state management
- Undo/redo transaction handling

### Security

- All input validation with bounds checking
- Path traversal prevention in scenario loading
- Content size limits (1MB scenarios, 100KB files)
- Safe arithmetic with checked operations
- No unsafe blocks in codebase
- Dependency security audits with cargo-deny

## [0.0.1] - 2024-10-01

### Added

- Initial project structure
- Basic TUI skeleton with ratatui
- Prototype scenario loader
- Proof of concept demo

---

[Unreleased]: https://github.com/bug-ops/helix-trainer/compare/v0.5.12...HEAD
[0.5.12]: https://github.com/bug-ops/helix-trainer/compare/v0.5.11...v0.5.12
[0.5.11]: https://github.com/bug-ops/helix-trainer/compare/v0.5.10...v0.5.11
[0.5.10]: https://github.com/bug-ops/helix-trainer/compare/v0.5.9...v0.5.10
[0.5.9]: https://github.com/bug-ops/helix-trainer/compare/v0.5.8...v0.5.9
[0.5.8]: https://github.com/bug-ops/helix-trainer/compare/v0.5.7...v0.5.8
[0.5.7]: https://github.com/bug-ops/helix-trainer/compare/v0.5.6...v0.5.7
[0.5.6]: https://github.com/bug-ops/helix-trainer/compare/v0.5.5...v0.5.6
[0.5.5]: https://github.com/bug-ops/helix-trainer/compare/v0.5.4...v0.5.5
[0.5.4]: https://github.com/bug-ops/helix-trainer/compare/v0.5.3...v0.5.4
[0.5.3]: https://github.com/bug-ops/helix-trainer/compare/v0.5.2...v0.5.3
[0.5.2]: https://github.com/bug-ops/helix-trainer/compare/v0.5.1...v0.5.2
[0.5.1]: https://github.com/bug-ops/helix-trainer/compare/v0.5.0...v0.5.1
[0.5.0]: https://github.com/bug-ops/helix-trainer/compare/v0.4.8...v0.5.0
[0.4.8]: https://github.com/bug-ops/helix-trainer/compare/v0.4.7...v0.4.8
[0.4.7]: https://github.com/bug-ops/helix-trainer/compare/v0.4.6...v0.4.7
[0.4.6]: https://github.com/bug-ops/helix-trainer/compare/v0.4.5...v0.4.6
[0.4.5]: https://github.com/bug-ops/helix-trainer/compare/v0.4.4...v0.4.5
[0.4.4]: https://github.com/bug-ops/helix-trainer/compare/v0.4.3...v0.4.4
[0.4.3]: https://github.com/bug-ops/helix-trainer/compare/v0.4.2...v0.4.3
[0.4.2]: https://github.com/bug-ops/helix-trainer/compare/v0.4.1...v0.4.2
[0.4.1]: https://github.com/bug-ops/helix-trainer/compare/v0.4.0...v0.4.1
[0.4.0]: https://github.com/bug-ops/helix-trainer/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/bug-ops/helix-trainer/compare/v0.2.1...v0.3.0
[0.2.1]: https://github.com/bug-ops/helix-trainer/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/bug-ops/helix-trainer/compare/v0.1.5...v0.2.0
[0.1.5]: https://github.com/bug-ops/helix-trainer/compare/v0.1.4...v0.1.5
[0.1.4]: https://github.com/bug-ops/helix-trainer/compare/v0.1.3...v0.1.4
[0.1.3]: https://github.com/bug-ops/helix-trainer/compare/v0.1.2...v0.1.3
[0.1.2]: https://github.com/bug-ops/helix-trainer/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/bug-ops/helix-trainer/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/bug-ops/helix-trainer/releases/tag/v0.1.0
[0.0.1]: https://github.com/bug-ops/helix-trainer/releases/tag/v0.0.1
