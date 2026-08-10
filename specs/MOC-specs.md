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
> was reconciled 2026-08-09 and again 2026-08-10 (v0.6.0 release prep)
> against actual merged implementation state — see each package's
> README.md (where present) for the retroactive-documentation
> methodology.

## Active Specs

(none — all specs below are either fully documented against shipped code or
superseded)

## Completed Specs

| Feature | Phase | Status |
|---------|-------|--------|
| [[fsrs-proptest-coverage-gap/README\|FSRS Scheduler Property-Based Test Coverage Gap]] | review | implemented (partial scope — see [[fsrs-proptest-coverage-gap/BRD#Residual Gap\|Residual Gap]]) |
| [[arcade-game-mode-variety/README\|Arcade Game Mode Variety]] | rejected | NO-GO — full decision-record package, not built |
| [[register-command-mode-support/README\|Named Register and Command-Line (`:`) Mode Support]] | review | implemented (command-line mode narrower than drafted — `:goto`/`:g` only, no `:s` substitute); extended in v0.6.0 with blackhole register and register-scoped delete/change — see [[register-command-mode-support/spec#11. Post-Release Extensions (v0.6.0)\|Post-Release Extensions]] |
| [[arcade-gamification-session-fixes/spec\|Gamification Live-Trigger Wiring, Notifications, and Bookkeeping Fixes]] | specify (lightweight, per SDD scaling guidance) | implemented — broadened in v0.6.0 from one commit (`623e0a8`) to the full seven-commit bug-fix thread |
| [[language-aware-syntax-highlighting/spec\|Language-Aware Syntax Highlighting for Scenario Content]] | specify (lightweight, per SDD scaling guidance) | implemented — shared prerequisite for the two specs below |
| [[writing-markup-scenario-track/spec\|Writing / Markup Scenario Track]] | specify (lightweight, per SDD scaling guidance) | implemented — evaluates GitHub issue #152 (#361) |
| [[multi-language-scenario-content/spec\|Multi-Language Scenario Content]] | specify (lightweight, per SDD scaling guidance) | implemented — pilot shipped as scoped (#362) |
| [[regex-selection-and-macro-commands/spec\|Regex Selection (`s`/`S`) and Macro Record/Replay (`q`/`Q`)]] | specify (lightweight, per SDD scaling guidance) | implemented — closes issue #198's remaining scope (macros, selection-regex) |
| [[custom-helix-keymap-support/spec\|Custom Helix Keymap Remapping (`use_helix_keymap`)]] | specify (lightweight, per SDD scaling guidance) | implemented — closes issue #163 |
| [[end-game-summary-screen/spec\|End-Game Summary Screen on Curriculum Completion]] | specify (lightweight, per SDD scaling guidance) | implemented — closes issue #145 |

## Proposed Specs

Not yet built. Drafted for review/prioritization only.

(none)

## Project Foundation

- [[constitution]] — non-negotiable project principles. Created 2026-08-09,
  synthesized from `.claude/CLAUDE.md` and `.claude/rules/*.md` (which
  already documented these conventions informally).

## Reconciliation Notes (2026-08-09)

This index was retroactively reconciled against actual merged
implementation state, then consolidated from working drafts under
`.local/specs/` into this git-tracked `specs/` tree. Summary of what
changed and why:

- **fsrs-proptest-coverage-gap** — was a `.local/specs/` draft describing a
  then-unresolved testing gap. Resolved by commit `33bdaa1`. Full
  BRD/SRS/NFR/plan/tasks/README package added, documenting a **partial**
  resolution: `src/learning/performance.rs` gained proptest coverage (and a
  real bug was found/fixed as a result); `src/learning/scheduler.rs` did
  not, and remains an open residual gap.
- **arcade-game-mode-variety** — a reflex-drill arcade mechanic (issue
  #264) was proposed twice: once as a `.local/specs/` draft
  (`002-arcade-game-mode-variety`), and once as a full, independently-run
  BRD → SRS → NFR → spec → plan → tasks pipeline that reached NO-GO. Only
  the complete, already-committed package lives here; the earlier draft
  remains in `.local/specs/002-arcade-game-mode-variety/spec.md` (main
  checkout only, gitignored) purely as a historical artifact — this
  package's README/SRS/spec cross-reference it explicitly and it is not
  duplicated in this tree. Commit `623e0a8` (double-XP/streak/level-up
  fixes, #322) was investigated as a possible match for this feature and
  found to be **unrelated** — a separate, previously undocumented set of
  gamification bug fixes, now covered by **arcade-gamification-session-fixes**.
- **register-command-mode-support** — was a `.local/specs/` draft
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

## See Also

- [[constitution]] — project principles
- `.claude/CLAUDE.md` — architecture reference
