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
> was last reconciled 2026-08-09 against actual merged implementation
> state — see each package's README.md for the retroactive-documentation
> methodology.

## Active Specs

(none — all specs below are either fully documented against shipped code or
superseded)

## Completed Specs

| Feature | Phase | Status |
|---------|-------|--------|
| [[fsrs-proptest-coverage-gap/README\|FSRS Scheduler Property-Based Test Coverage Gap]] | review | implemented (partial scope — see [[fsrs-proptest-coverage-gap/BRD#Residual Gap\|Residual Gap]]) |
| [[arcade-game-mode-variety/README\|Arcade Game Mode Variety]] | rejected | NO-GO — full decision-record package, not built |
| [[register-command-mode-support/README\|Named Register and Command-Line (`:`) Mode Support]] | review | implemented (command-line mode narrower than drafted — `:goto`/`:g` only, no `:s` substitute) |
| [[arcade-gamification-session-fixes/spec\|Arcade Gamification Session Bookkeeping Fixes]] | specify (lightweight, per SDD scaling guidance) | implemented |
| [[language-aware-syntax-highlighting/spec\|Language-Aware Syntax Highlighting for Scenario Content]] | specify (lightweight, per SDD scaling guidance) | implemented — shared prerequisite for the two specs below |
| [[writing-markup-scenario-track/spec\|Writing / Markup Scenario Track]] | specify (lightweight, per SDD scaling guidance) | implemented — evaluates GitHub issue #152 (#361) |
| [[multi-language-scenario-content/spec\|Multi-Language Scenario Content]] | specify (lightweight, per SDD scaling guidance) | implemented — pilot shipped as scoped (#362) |

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

## See Also

- [[constitution]] — project principles
- `.claude/CLAUDE.md` — architecture reference
