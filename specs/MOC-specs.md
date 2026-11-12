---
aliases:
  - Specifications Index
  - Specs Overview
tags:
  - moc
  - sdd
created: 2026-08-08
status: moc
---

# Specifications

> [!abstract]
> Map of Content for all helix-trainer project specifications. Each entry
> links to a feature spec with its current phase and status. This index
> was reconciled 2026-08-09, 2026-08-10 (v0.6.0 release prep) and
> 2026-11-10 (migration of the remaining untracked bug-spec drafts)
> against actual merged implementation state — see each package's
> README.md (where present) for the retroactive-documentation
> methodology.

## Active Specs

Specified 2026-11-10 for the P1-P3 bug batch (PR for #487); implementation
follows these packages. The batch also appends "Bug-Fix Batch" sections to
[[regex-selection-and-macro-commands/spec]],
[[register-command-mode-support/spec]],
[[custom-helix-keymap-support/spec]],
[[arcade-gamification-session-fixes/spec]],
[[fsrs-proptest-coverage-gap/spec]] and amends [[constitution]] Section V.
The 2026-11-11 spec update for the #546-#589 batch further amends [[helix-core-post-25-07-1-upgrade-readiness/spec]] (FR-025, FR-034, FR-039, FR-041..FR-049), [[regex-selection-and-macro-commands/spec]] (FR-126, #576), [[undo-insert-session-granularity/spec]], [[profile-screen-xp-progress-label/spec]], [[custom-helix-keymap-support/spec]] (FR-008 fingerprint, #560), [[multi-language-scenario-content/spec]] (#554) and adds [[fsrs-review-rating-and-session/spec]].
The 2026-11-11 P2-P3 batch (PR for #509) further extends [[helix-core-post-25-07-1-upgrade-readiness/spec]] (section 3.8), [[regex-selection-and-macro-commands/spec]] (section 11: `?`, `K`/`Alt-K`/`&`, F1 hint), [[register-command-mode-support/spec]] (deferral pointers), [[custom-helix-keymap-support/spec]] (data-directory pointer), [[fsrs-proptest-coverage-gap/spec]] (section 12: #511, #464, #469, #470), [[data-and-runtime-hardening/spec]] (#467, #484, #515, #518), [[optimal-count-action-counter-unit/spec]] (#512) and [[arcade-gamification-session-fixes/spec]] (section 9: #468, #469, #461; section 10: #527, #528, #529, #530).
The 2026-11-11 batch for #553, #555, #557, #558, #564, #592-#599 (PR for #592) further amends [[helix-core-post-25-07-1-upgrade-readiness/spec]] (FR-025, FR-039, FR-040, FR-042, FR-047; new FR-050..FR-058; sections 3.8.6 and 3.8.7 with the recorded hx goldens), [[undo-insert-session-granularity/spec]] (multi-cursor sessions, FR-013 and FR-014), [[constitution]] (module layer order; the error hub stays in `src/security.rs`, #606), [[custom-helix-keymap-support/spec]] (fingerprint path), and updates moved paths in [[backtick-key-misparsed-as-alt/spec]], [[register-command-mode-support/SRS]] (also plan and tasks) and [[writing-markup-scenario-track/spec]]; it adds [[scenario-cursor-spec/spec]] and [[module-layering-and-typed-commands/spec]].
The 2026-11-11 batch of 33 issues (PR for #607) amends [[helix-core-post-25-07-1-upgrade-readiness/spec]] (FR-028, FR-029, FR-040, FR-041, FR-042, FR-050, FR-053; new FR-059..FR-065 and section 3.8.8 for #583, #584, #585, #600, #542, #541 and #617: #607, #608, #609, #611, #604, #603, #610, #616 and #544 are amendments), [[module-layering-and-typed-commands/spec]] (FR-002 #614, FR-004, FR-010 #606, FR-012 #612 and #602, new FR-013 #613), [[constitution]] (error hub rank 8 and Section V, lint allow-list #537), [[language-aware-syntax-highlighting/spec]] (closed `Language` enum, unknown language is a load error, #544), [[multi-language-scenario-content/spec]] (#605 unknown metadata keys, #544), [[custom-helix-keymap-support/spec]] (FR-015, #601), [[challenge-attempt-limit-not-enforced/spec]] (FR-010, FR-011, #615), [[fsrs-7-model-adoption/spec]] (FR-003, FR-006 thresholds, FR-011 corpus validity, FR-012 evaluation example, #437), [[data-and-runtime-hardening/spec]] (section 8: #543 fuzz targets, #539 sort cost) and [[undo-insert-session-granularity/spec]] (NFR-003, #617); it adds [[code-health-modern-apis-and-tech-debt/spec]] (#566, #568) and [[review-screen-margin-consistency/spec]] (#586). The research outcomes of #549 (mutation baseline) and #438 (upgrade checklist re-verification) are recorded in their packages after the runs.

| Feature | Phase | Status |
|---------|-------|--------|
| [[helix-core-post-25-07-1-upgrade-readiness/spec\|Helix Core Upgrade Readiness and Simulator Fidelity at the Pinned Core]] | specify (lightweight) | proposed — upgrade checklist (sections 3.1-3.7) not built; section 3.8 (simulator fidelity, #490-#499, #501; extended 2026-11-11 with #500, #510, #514, #517, #519, #520, #524, #534, #535, #521, #516; extended 2026-11-11 with #569, #570, #556, #577, #578-#581, #587-#589, #546 amendments; extended 2026-11-11 with #592-#597 and #599, FR-050..FR-058 and the 3.8.6 hx goldens; extended 2026-11-11 with #607, #608, #609, #611, #583, #584, #585, #600, #541, #542, #617, #604, #603, #610, #616, #544: FR-059..FR-065, section 3.8.8 viewport model) specified for the bug batches |
| [[cli-version-and-help-flags/spec\|`--version` and `--help` Command-Line Flags]] | specify (lightweight bug-fix spec) | proposed — #452 |
| [[training-progress-indicator/spec\|Training Progress Indicator Reflects Cursor and Selection]] | specify (lightweight bug-fix spec) | proposed — #451 |
| [[optimal-count-action-counter-unit/spec\|`optimal_count` and Action Counter Use the Same Unit]] | specify (lightweight bug-fix spec) | proposed — #488; section 7 adds the opt-in `after_taught_commands` completion rule (#512) |
| [[fsrs-review-rating-and-session/spec\|One Review Session Owner and a Deterministic Rating Mapping]] | specify (lightweight bug-fix spec) | proposed — #550, #551 |
| [[profile-screen-xp-progress-label/spec\|One Definition of Level XP Progress on Every Screen]] | specify (lightweight bug-fix spec) | proposed — #572; Profile label showed XP remaining instead of level span; shared helper (P2); #574 Profile quest total; span-0 label decided |
| [[scenario-cursor-spec/spec\|Scenario Cursor Spec Is a Closed Enum]] | specify (lightweight spec) | proposed — #553; 66 legacy TOML entries migrated |
| [[module-layering-and-typed-commands/spec\|Typed Normal Commands, One Key Resolution, Enforced Module Layering]] | specify (lightweight spec) | proposed — #555 (the `CanonicalKeys` parse-once step is cut, #614), #557, #558 (the `UserError` hub move was cut and is re-specified as FR-010, #606; four `security` edges allow-listed until then), #564, #598; extended 2026-11-11 with #614, #613, #606, #612, #602 |
| [[code-health-modern-apis-and-tech-debt/spec\|Modern std APIs Within the MSRV and a Tech-Debt Batch]] | specify (lightweight spec) | proposed — #566, #568 |
| [[review-screen-margin-consistency/spec\|The Review Screen Uses the Same Margin as Every Other Screen]] | specify (lightweight bug-fix spec) | proposed — #586 |
| [[sound-setting-persistence/spec\|Sound Toggle Persists Across Restarts]] | specify (lightweight bug-fix spec) | proposed — #459 |
| [[data-and-runtime-hardening/spec\|User-Data Safety and Runtime Robustness Hardening]] | specify (lightweight spec) | proposed — #440, #441, #442, #443, #444; extended 2026-11-11 with #467, #484, #515, #518, #460, #457, #456, #531, #454, #523, #463 (section 7) |

## Completed Specs

| Feature | Phase | Status |
|---------|-------|--------|
| [[fsrs-proptest-coverage-gap/README\|FSRS Scheduler Property-Based Test Coverage Gap]] | review | implemented (residual: ignored properties tied to #433/#434 — see [[fsrs-proptest-coverage-gap/BRD#Residual Gap\|Residual Gap]]) |
| [[arcade-game-mode-variety/README\|Arcade Game Mode Variety]] | rejected | NO-GO — full decision-record package, not built |
| [[register-command-mode-support/README\|Named Register and Command-Line (`:`) Mode Support]] | review | implemented (command-line mode narrower than drafted — `:goto`/`:g` only, no `:s` substitute); extended in v0.6.0 with blackhole register and register-scoped delete/change — see [[register-command-mode-support/spec#11. Post-Release Extensions (v0.6.0)\|Post-Release Extensions]] |
| [[arcade-gamification-session-fixes/spec\|Gamification Live-Trigger Wiring, Notifications, and Bookkeeping Fixes]] | specify (lightweight, per SDD scaling guidance) | implemented — broadened in v0.6.0 from one commit (`623e0a8`) to the full seven-commit bug-fix thread; section 10 (2026-11-11) specifies #527-#530 |
| [[language-aware-syntax-highlighting/spec\|Language-Aware Syntax Highlighting for Scenario Content]] | specify (lightweight, per SDD scaling guidance) | implemented — shared prerequisite for the two specs below |
| [[writing-markup-scenario-track/spec\|Writing / Markup Scenario Track]] | specify (lightweight, per SDD scaling guidance) | implemented — evaluates GitHub issue #152 (#361) |
| [[multi-language-scenario-content/spec\|Multi-Language Scenario Content]] | specify (lightweight, per SDD scaling guidance) | implemented — pilot shipped as scoped (#362) |
| [[regex-selection-and-macro-commands/spec\|Regex Selection (`s`/`S`) and Macro Record/Replay (`q`/`Q`)]] | specify (lightweight, per SDD scaling guidance) | implemented — closes issue #198's remaining scope (macros, selection-regex) |
| [[custom-helix-keymap-support/spec\|Custom Helix Keymap Remapping (`use_helix_keymap`)]] | specify (lightweight, per SDD scaling guidance) | implemented — closes issue #163 |
| [[end-game-summary-screen/spec\|End-Game Summary Screen on Curriculum Completion]] | specify (lightweight, per SDD scaling guidance) | implemented — closes issue #145 |
| [[duplicate-scenario-ids/spec\|Fix Duplicate Scenario IDs]] | specify (lightweight bug-fix spec) | implemented — closes issue #363 via PR #372; uniqueness guard in `tests/scenario_validation.rs` |
| [[key-history-popup-training-overlap/spec\|Key-History Popup Corrupts Task Screen Panel Borders in Training Mode]] | specify (lightweight bug-fix spec) | implemented — closes issue #364 via PR #372 |
| [[challenge-attempt-limit-not-enforced/spec\|Daily Challenge Attempt Limit and Best-Score Tracking Enforcement]] | specify (lightweight bug-fix spec) | implemented — closes issue #376 via PR #384 (also part of [[arcade-gamification-session-fixes/spec\|the gamification fix thread]]) |
| [[backtick-key-misparsed-as-alt/spec\|Bare Backtick Keypress Misparsed as Alt-Backtick]] | specify (lightweight bug-fix spec) | implemented — closes issue #377 via PR #380 |
| [[arcade-stats-bar-second-line-clipped/spec\|Arcade/Survival/Challenge Stats Bar's Second HUD Line Clipped]] | specify (lightweight bug-fix spec) | implemented — closes issue #393 via PR #397 |

## Proposed Specs

Not yet built. Drafted for review/prioritization only.

| Feature | Phase | Status |
|---------|-------|--------|
| [[fsrs-7-crate-migration/spec\|Migrate `fsrs` Crate from 6.6 to 7.0 (FSRS-6 Behavior Preserved)]] | specify (lightweight, per SDD scaling guidance) | proposed — dependency major bump; model change tracked in [[fsrs-7-model-adoption/spec]] |
| [[fsrs-7-model-adoption/spec\|Evaluate Switching the Scheduler from the FSRS-6 Model to the FSRS-7 Model]] | specify (lightweight, per SDD scaling guidance) | evaluated 2026-11-12 (#437): DEFER (synthetic corpus, elapsed form not declared in advance; re-run on real logs) — decision record in section 11; go/no-go evaluation (research, P4); prerequisite: [[fsrs-7-crate-migration/spec]] |
| [[mutation-testing-critical-paths/spec\|Adopt Mutation Testing on Scoring, FSRS Scheduling and Quest-Generation Critical Paths]] | specify (lightweight, per SDD scaling guidance) | proposed — non-gating scheduled cargo-mutants run (research, P4); setup built (#549), baseline pending; related: #536 |
| [[undo-insert-session-granularity/spec\|One Undo Step per Insert Session (`u` Undoes the Whole `i ... Escape`)]] | specify (lightweight, per SDD scaling guidance) | proposed — #571, #589, #596; FR-003 amended with the per-command hx table; FR-013 and FR-014 add multi-cursor sessions; related: #501 |

## Project Foundation

- [[constitution]] — non-negotiable project principles. Created 2026-08-09,
  synthesized from the project's architecture guide and rules (which
  already documented these conventions informally).

## Reconciliation Notes (2026-08-09)

This index was retroactively reconciled against actual merged
implementation state, then consolidated from untracked local working drafts into this
git-tracked `specs/` tree. Summary of what
changed and why:

- **fsrs-proptest-coverage-gap** — was an untracked local draft describing a
  then-unresolved testing gap. Resolved by commit `33bdaa1`. Full
  BRD/SRS/NFR/plan/tasks/README package added, documenting a **partial**
  resolution: `src/learning/performance.rs` gained proptest coverage (and a
  real bug was found/fixed as a result); `src/learning/scheduler.rs` did
  not, and remains an open residual gap.
- **arcade-game-mode-variety** — a reflex-drill arcade mechanic (issue
  #264) was proposed twice: once as an untracked local draft
  (`002-arcade-game-mode-variety`), and once as a full, independently-run
  BRD → SRS → NFR → spec → plan → tasks pipeline that reached NO-GO. Only
  the complete, already-committed package lives here; the earlier draft
  was never committed and is not duplicated in this tree; this package's
  README/SRS/spec refer to it only as the superseded draft. Commit `623e0a8` (double-XP/streak/level-up
  fixes, #322) was investigated as a possible match for this feature and
  found to be **unrelated** — a separate, previously undocumented set of
  gamification bug fixes, now covered by **arcade-gamification-session-fixes**.
- **register-command-mode-support** — was an untracked local draft
  researching named registers and a command-line mode. Resolved by commit
  `1ba668d`. Full BRD/SRS/NFR/plan/tasks/README package added. Named
  registers shipped essentially as drafted; command-line mode shipped
  **narrower** than drafted (`:goto`/`:g` line navigation only — no `:s`
  substitute, no `EditorMode::CommandMode` variant), a deliberate,
  documented scope decision made during implementation.
- **arcade-gamification-session-fixes** (new) — commit `623e0a8` (double
  XP on arcade replay, zero-streak freeze guard, missing review-session
  level-up notification) had no spec anywhere in the repository prior to
  this pass. Given a lightweight spec.md per this project's SDD scaling
  guidance for small, independent bug fixes (3 single-guard-clause
  changes, 6 files, no new architecture) — a full BRD/SRS/NFR/plan/tasks
  package would be disproportionate.

## Reconciliation Notes (2026-08-10, v0.6.0 release prep)

Reviewed every commit merged since the `v0.5.12` tag (61 commits) and
updated this tree so it stays in sync with what actually shipped. Summary:

- **arcade-gamification-session-fixes** (broadened) — was scoped to a
  single commit (`623e0a8`, 3 bugs). Six more commits in the same release
  turned out to be the same continuous bug-fix thread on
  streaks/freezes/achievements/quests/level-ups/Daily Challenge
  (`573fab8`, `1b1a8bb`, `04f0e8b`, `71ed504`, `a2b0185`, `e637c30`), each
  triggered by adversarial review of the one before it, none previously
  documented anywhere. Expanded the existing package in place (title
  broadened, old title kept as an alias) rather than creating six new
  micro-packages, per this project's guidance against excessive package
  proliferation for small related fixes.
- **register-command-mode-support** (extended) — its original research
  spec deferred macros and selection-regex to "issue #198, separate,
  still open." Issue #198 is now closed: `d66278d` added scroll/
  select-all/`R` scenario coverage and explicitly deferred macros/regex;
  `a4efc2e` then implemented exactly that deferral. A trio of further
  commits (`fb2e0c5`, `dd39300`, `efeac9d`) extended register-scoped
  behavior itself (blackhole register `"_`, register-scoped delete/change,
  `Alt-c`/`Alt-d` noyank variants, a command-repeat register-drop fix).
  Added as a new "Post-Release Extensions" section rather than a new
  package, since these are direct extensions of FR-002/FR-003 in the
  existing spec; stale "#198 still open" references corrected throughout
  (spec.md and README.md).
- **regex-selection-and-macro-commands** (new) — `a4efc2e` implemented
  `s`/`S` (regex select/split) and `q`/`Q` (macro record/replay), a
  substantial standalone feature (new `helix-stdx` dependency, new
  `MacroRecorder`) with no prior spec coverage beyond a deferred bullet in
  register-command-mode-support.
- **custom-helix-keymap-support** (new) — `999e63e` (issue #163) plus
  follow-up fixes `463d363` and `d7003ca` had no spec anywhere in the
  repository despite being a substantial multi-file addition
  (`src/input/keymap/`, `src/config/keymap/`).
- **end-game-summary-screen** (new) — `0dbfcbe` (issue #145) had no spec
  anywhere in the repository.
- **No change needed** — the majority of the 61 commits are pure bug
  fixes, internal refactors, or chores with no existing spec-level
  behavior description to go stale: e.g. the `Clock` abstraction
  (`ba8e0d3`), the `ScenarioFilter`/`DifficultyController` RNG-seam
  refactor (`8100ebc`), the `MiniGameStats` field-to-accessor restructuring
  (`e119429`), the `get_`-prefix getter renames (`34208d0`, `90eccbe`),
  and the MSRV 1.89→1.91 bump (`fc95b07`) are all internal/architectural —
  none were described by name in any existing spec, so nothing there is
  now stale. The MSRV bump in particular is already correctly reflected in
  [[constitution]] (Section II), written after that change landed.

## Reconciliation Notes (2026-11-10, local drafts migration)

Five remaining bug-spec drafts kept in an untracked local directory
(numbered `001`-`005`, all `status: draft`) were migrated into this tree
and checked against the code, `CHANGELOG.md`, and `gh issue list`. All five
bugs were already fixed; the drafts had simply never been updated. Each
package keeps the original problem analysis and adds a "Resolution (as
shipped)" and "Resolved Questions" section. The numeric prefixes were
dropped to match the slug-only naming used by every other package here.

- **duplicate-scenario-ids** — fixed by `7f34268` (PR #372, issue #363).
  The shipped fix renamed the `basic/delete.toml` and `editing/join.toml`
  scenarios (`delete_current_line_001`, `join_lines_basic_001`), the
  opposite of the draft's default. Adds `test_all_scenario_ids_are_unique`
  and `test_quest_scenario_references_exist`.
- **key-history-popup-training-overlap** — fixed by `7f34268` (PR #372,
  issue #364): popup bounded to the Target panel's inner area, skipped when
  too small.
- **challenge-attempt-limit-not-enforced** — fixed by `e637c30` (PR #384,
  issue #376). The `MiniGameData.challenge_progress` snapshot was removed;
  attempts show as `(N/3)` in the mode submenu. The malformed
  `last_challenge_date` risk (CR-013) was initially left open and later
  resolved by #427 (`Option<NaiveDate>` with a lenient deserializer).
- **backtick-key-misparsed-as-alt** — fixed by `45bbd1f` (PR #380, issue
  #377). Root cause was the app's own `map_macos_composed_char` table, not
  crossterm or tmux as the draft suspected.
- **arcade-stats-bar-second-line-clipped** — fixed by `f7bf0fe` (PR #397,
  issue #393): stats bar chunk widened from 3 to 4 rows at all three call
  sites.
- **language-aware-syntax-highlighting** — corrected a stale path
  (it cited a nonexistent `validators.rs` file; the validator is
  `validators::validate_language_field` in `src/security.rs`).
- **Follow-up updates**: #428 implemented FR-004/FR-005 as proptests in
  `src/learning/scheduler.rs`; three properties are `#[ignore]`d pending real
  defects #433 and #434 (see [[fsrs-proptest-coverage-gap/BRD#Residual Gap|Residual Gap]]). #427 resolved the CR-013 TODO in `src/minigame/challenge.rs`.

## See Also

- [[constitution]] — project principles
- `CONTRIBUTING.md` — Conventional Commits and the pre-commit check suite
