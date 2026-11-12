---
aliases:
  - Mutation Testing on Critical Paths
  - cargo-mutants Adoption
tags:
  - sdd
  - spec
  - research
  - testing
  - ci-cd
  - status/proposed
created: 2026-11-11
status: proposed
related:
  - "[[constitution]]"
  - "[[fsrs-proptest-coverage-gap/README]]"
  - "[[MOC-specs]]"
---

# Feature: Adopt Mutation Testing on the Scoring, FSRS Scheduling and Quest-Generation Critical Paths

> [!info] Metadata
> **Source**: research CI cycle finding (priority P4, research)
> **Status**: Setup built (#549): configuration and workflow committed. baseline RECORDED 2026-11-12 (section 11: 76.0% on 233 viable mutants, gaps tracked by #633); the PR gating threshold stays pending until those gaps are covered.
> **Depth**: Lightweight research spec, per this project's SDD scaling guidance. It specifies WHAT is missing and WHY it matters, plus the decisions that must be made. It does not prescribe workflow YAML or tool flags beyond what is needed to state a requirement.

> [!abstract]
> The project lists several "Critical Paths" where silent breakage is not caught by unit tests: scenario completion detection (`src/game/scorer.rs`), FSRS scheduling and persistence (`src/learning/scheduler.rs`, `src/learning/performance.rs`) and daily quest generation (`src/gamification/quests.rs`). Line coverage (`cargo llvm-cov` in CI) shows that code ran, not that tests would fail if it were wrong. Issue #536 already documents vacuous test asserts, which coverage cannot see. Mutation testing with `cargo-mutants` (27.1.0 at time of writing) measures assertion strength directly by injecting small code changes and reporting "missed" mutants that no test kills. No mutation-testing setup exists in the repository today. This spec defines a non-gating, scheduled, scoped mutation run with a triaged baseline.

## 1. Overview

### Problem Statement

The four target files total about 3,900 lines (`scorer.rs` 555, `scheduler.rs` 885, `performance.rs` 1414, `quests.rs` 1039). They carry the logic whose failure is invisible in play: a scorer that accepts a wrong editor state, a scheduler that mis-orders due items, a quest generator that mis-resets at midnight. The suite is large (nextest over lib and bins, integration targets under `tests/`, proptest, a weekly miri workflow, render tests), yet its ability to detect a behavioural change in those files is unmeasured.

Evidence of the gap:

- Line coverage cannot distinguish an asserting test from one that merely executes code. Issue #536 records tests that pass without constraining behaviour.
- Recent changes touched adjacent logic (the FSRS-6 to FSRS-7 migration in PR #545 and Helix newline/indent fidelity fixes in #547) with no measure of whether the existing tests would catch a regression there.
- The repository has no `.cargo/mutants.toml`, no `mutants` reference in `Cargo.toml`, `src/`, `tests/` or `.github/`. Existing scheduled workflows are `miri.yml` and `security-audit.yml` (weekly); there is no mutation workflow.

### Goal

A non-gating, scheduled or manually dispatched mutation-testing run, scoped to the critical-path files, with checked-in configuration, a documented baseline of missed mutants triaged into follow-up issues, and a recorded decision on whether mutation testing may ever gate pull requests.

### Out of Scope

- Making mutation testing a required PR check (decision is recorded under FR-010; default expectation is no).
- Fixing the missed mutants themselves; each triaged mutant becomes a separate follow-up issue.
- Mutation testing of rendering, i18n, audio, input typestate, or the Helix simulator wrapper, except where later added by an explicit scope decision.
- Replacing or changing `cargo llvm-cov`, proptest, or miri.
- Rewriting the vacuous asserts tracked by #536 (related, separate).
- Choosing an alternative mutation tool. [NEEDS CLARIFICATION: is any alternative to cargo-mutants worth a comparison, or is the tool fixed?]

## 2. User Stories

### US-001: Know whether critical-path tests actually constrain behaviour

AS A maintainer
I WANT a measure of which behavioural changes in scorer, scheduler and quest code no test detects
SO THAT I can strengthen tests where silent breakage is possible.

**Acceptance criteria:**
```
GIVEN the scoped mutation run has completed on the default branch
WHEN the maintainer opens its result artifact
THEN the list of missed, caught, unviable and timed-out mutants per target file is available
```

### US-002: Keep PR feedback fast

AS A contributor
I WANT mutation testing not to slow down or block my pull request
SO THAT the existing CI feedback loop is unchanged.

**Acceptance criteria:**
```
GIVEN a pull request is opened or updated
WHEN the PR checks run
THEN no mutation run is part of the required checks
```

### US-003: Act on the baseline

AS A maintainer
I WANT the first run's missed mutants classified and filed
SO THAT genuine test gaps become tracked work and acceptable survivors are not re-triaged each week.

**Acceptance criteria:**
```
GIVEN the baseline run results
WHEN triage completes
THEN each missed mutant is either linked to a follow-up issue or recorded as accepted with a reason
```

## 3. Functional Requirements

| ID | Requirement | Priority |
|----|------------|----------|
| FR-001 | WHEN the mutation run is triggered THE SYSTEM SHALL generate and test mutants only for the critical-path files: `src/game/scorer.rs`, `src/learning/scheduler.rs`, `src/learning/performance.rs`, `src/gamification/quests.rs`. [NEEDS CLARIFICATION: should adjacent files (`src/learning/session.rs`, `src/learning/urgency.rs`, `src/learning/scenario_history.rs`) be in scope? The finding lists only the four.] | must |
| FR-002 | THE repository SHALL contain a checked-in mutation configuration that excludes noise mutants: logging and tracing calls, `Display`/`Debug` formatting, and rendering code | must |
| FR-003 | THE mutation run SHALL build and test without an audio device, using a feature set that excludes the `audio` feature (or equivalent) | must |
| FR-004 | THE mutation run SHALL be triggered on a schedule (weekly) and by manual dispatch, and SHALL NOT run on pull requests by default | must |
| FR-005 | WHEN the run completes THE SYSTEM SHALL upload its result artifacts (per-mutant outcomes and a human-readable summary) so they are retrievable after the job ends | must |
| FR-006 | THE run SHALL have an explicit job timeout and a per-mutant test timeout so a hanging mutant cannot stall the job. [NEEDS CLARIFICATION: numeric values; depends on measured baseline duration.] | must |
| FR-007 | THE project SHALL record a baseline: counts of caught, missed, unviable and timed-out mutants per file, with the tool version and commit it was measured on | must |
| FR-008 | THE baseline's missed mutants SHALL be triaged into (a) genuine test gap, filed as a follow-up issue linked to this package, (b) equivalent mutant (no observable behaviour change), recorded as accepted with a reason, or (c) noise, handled by extending the configuration exclusions | must |
| FR-009 | THE triage process SHALL be documented so a maintainer can repeat it on later runs, including how accepted survivors are recorded to avoid re-triage | must |
| FR-010 | THE project SHALL record an explicit decision on whether mutation results may ever gate pull requests, with the runtime evidence behind it | must |
| FR-011 | WHERE a run is limited to a subset (for example changed files only), THE procedure SHOULD be documented for local use by contributors. [NEEDS CLARIFICATION: is an incremental or diff-scoped mode wanted for contributors, or is the weekly run sufficient?] | should |
| FR-012 | THE run's pass/fail status SHALL NOT fail the workflow on missed mutants (non-gating); only infrastructure failure (build error, tool crash, timeout) SHALL fail it | must |

## 4. Non-Functional Requirements

| ID | Category | Requirement |
|----|----------|-------------|
| NFR-001 | Runtime budget | The scheduled job SHALL complete within a declared timeout. [NEEDS CLARIFICATION: budget; to be set from a measured full-scope run, since each mutant needs a rebuild and test run.] |
| NFR-002 | Environment | The run SHALL work on a headless CI runner with no sound device and no TTY; it SHALL NOT require a system audio library. |
| NFR-003 | Dependency hygiene | The tool SHALL be installed in the CI job or developer machine only and SHALL NOT be added as a dependency of the shipped crate; `cargo deny check` is unaffected. |
| NFR-004 | Supply chain | WHERE the workflow uses third-party actions or installs the tool, actions SHALL be pinned to full commit SHAs and the tool version SHALL be pinned, per project conventions. |
| NFR-005 | Determinism | Flaky tests SHALL NOT be counted as missed mutants. THE baseline procedure SHALL confirm the unmutated suite passes before mutating (the tool's baseline check). |
| NFR-006 | Isolation | The run SHALL operate on a copy of the source tree so the working checkout is never left mutated. |
| NFR-007 | Reproducibility | THE procedure, tool version and configuration SHALL be documented so the baseline can be re-measured. |

## 5. Data Model

| Entity | Description | Key Attributes |
|--------|-------------|----------------|
| Mutation configuration | Checked-in file controlling scope and exclusions | file scope, exclusion patterns, timeouts, feature set |
| Mutation result | Per-run output | per-mutant outcome (caught, missed, unviable, timeout), source location, mutation description |
| Baseline record | First triaged measurement | tool version, commit, per-file counts, triage disposition per missed mutant |
| Accepted-survivor list | Missed mutants judged equivalent or acceptable | mutant identity, reason, date |

[NEEDS CLARIFICATION: where the baseline record and accepted-survivor list live, given that tracked files may not reference gitignored paths (for example a tracked `docs/` page or this package).]

## 6. Edge Cases and Error Handling

| Scenario | Expected Behavior |
|----------|-------------------|
| Unmutated test suite fails or is flaky | Run aborts at the baseline check and fails as infrastructure failure (FR-012); no mutation results are published as valid. |
| Mutant causes an infinite loop | Per-mutant timeout (FR-006) classifies it as timeout, not missed. |
| Mutant does not compile | Classified unviable and excluded from the missed count. |
| Mutation reaches code covered only by an ignored or slow test | Reported as missed; triage decides whether the test should be included. [NEEDS CLARIFICATION: should ignored tests (see #433/#434 in the FSRS proptest package) run during mutation?] |
| Full-scope run exceeds the CI timeout | Scope is narrowed or sharded; the decision is recorded. [NEEDS CLARIFICATION: is sharding across jobs acceptable?] |
| Equivalent mutant (same observable behaviour) | Recorded as accepted with a reason (FR-008b), not filed as a bug. |
| Tool release changes mutant generation between runs | Counts are compared only within the same tool version; the baseline records the version (FR-007). |
| `audio` feature accidentally enabled | Build fails on headless runner without ALSA; this is a configuration error, caught by FR-003. |

## 7. Success Criteria

| ID | Metric | Target |
|----|--------|--------|
| SC-001 | Scoped run completes within the declared timeout | 100% of scheduled runs |
| SC-002 | Required PR checks affected by the feature | 0 |
| SC-003 | Baseline missed mutants with a disposition (issue or accepted reason) | 100% |
| SC-004 | Result artifact retrievable after each run | yes |
| SC-005 | Gating decision recorded with runtime evidence | yes |
| SC-006 | Run requires no audio device | yes |

## 8. Agent Boundaries

### Always (without asking)
- Keep the mutation run non-gating and out of the required PR checks.
- Read [[constitution]] and [[fsrs-proptest-coverage-gap/README|the FSRS proptest coverage gap package]] before changing scheduler tests.
- Run the mutation tool only on a copy or a scratch worktree, never leaving mutated source in the working tree.
- Follow the issue labeling rules (one `P0`-`P4` priority label plus classification labels) when filing triage issues.

### Ask First
- Adding any dependency to `Cargo.toml` (the tool should not need one).
- Widening scope beyond the four listed files.
- Making the run a required check, or adding it to PR workflows.
- Adding a new GitHub Actions workflow or third-party action (must be SHA-pinned).

### Never
- Weaken or delete an existing test to remove a missed mutant.
- Mark a mutant accepted without a recorded reason.
- Run the tool against the main checkout in a way that leaves mutations applied.
- Reference gitignored paths from this package or any tracked file.

## 9. Risks

| Risk | Impact | Probability | Mitigation |
|------|--------|-------------|------------|
| Runtime too long for CI (each mutant is a rebuild plus test run over about 3,900 lines of target code) | medium | high | Scope to four files, set timeouts (FR-006), keep scheduled-only, consider sharding. |
| Large noisy baseline overwhelms triage | medium | medium | Exclusion config (FR-002), triage by file and severity, accept equivalent mutants with reasons. |
| Equivalent mutants inflate the missed count | low | high | Accepted-survivor list (FR-008, FR-009). |
| Slow or ignored tests distort results | medium | medium | Decide inclusion of ignored tests explicitly. |
| Result treated as a quality score and gamed with weakened scope | low | low | Record scope and tool version with each baseline. |
| Mutation findings duplicate #536 scope | low | medium | Cross-link; triage issues reference #536 where the cause is a vacuous assert. |

## 10. Open Questions

> [!question] Unresolved
> - [NEEDS CLARIFICATION: Final in-scope file list (four files only, or adjacent learning and gamification files).]
> - [NEEDS CLARIFICATION: Runtime budget and per-mutant timeout, to be set from a measured baseline.]
> - [NEEDS CLARIFICATION: Whether ignored tests should run during mutation.]
> - [NEEDS CLARIFICATION: Whether sharding across CI jobs is acceptable if one job is too slow.]
> - [NEEDS CLARIFICATION: Location of the baseline record and accepted-survivor list in tracked files.]
> - [NEEDS CLARIFICATION: Whether contributors want a documented local or diff-scoped mode (FR-011).]
> - [NEEDS CLARIFICATION: Whether any alternative mutation tool should be compared.]
> - [NEEDS CLARIFICATION: Issue #536 is currently closed; confirm whether its vacuous-assert work is complete, so this baseline starts from the post-fix suite.]

## 11. Implementation Status and Baseline (2026-11-12)

| Item | State |
|------|-------|
| `.gitignore` rule `/.cargo/*` with the exception `!/.cargo/mutants.toml`, so the configuration is tracked | built |
| `.cargo/mutants.toml`: the four files of FR-001, `--no-default-features` (FR-003), nextest on `--lib --bins`, noise exclusions (FR-002), 60 s minimum per-mutant test timeout (FR-006) | built; `cargo mutants --list` found 282 mutants (scorer 21, quests 94, performance 143, scheduler 24) before the noise exclusion and finds 278 after it (scheduler 20) |
| Workflow: weekly and manual dispatch, no pull-request trigger (FR-004), `cargo-mutants` 27.1.0 pinned with `--locked`, 240 minute job timeout, result artifacts uploaded (FR-005), exit codes 2 and 3 (missed or timed-out mutants) do not fail the job (FR-012) | built |
| Baseline (FR-007) and triage of missed mutants (FR-008) | **RECORDED 2026-11-12**, see the table below. Triage: the 2 scheduler misses of the raw run were noise (`ReviewItem::eq` and `Scheduler`'s `Debug::fmt`) and are now excluded in `.cargo/mutants.toml`, so no noise is left and all 56 remaining misses are genuine test gaps, ticketed in #633. Effective baseline: 278 mutants, caught 177, missed 56, unviable 45, score 76.0% |
| Gating decision with runtime evidence (FR-010) | **Non-gating confirmed**: the full scoped run takes 41 minutes wall clock at `-j 2` with 0 timeouts, well inside the 240 minute weekly job timeout, which supports the scheduled non-gating design. A pull-request gating threshold is NOT set: it stays pending until the gaps of #633 are covered and a re-run shows the score, so that a threshold is not fixed against a known-weak baseline |

**Effective baseline** (after the noise exclusion; cargo-mutants 27.1.0, `-j 2`, `--no-default-features`, nextest on `--lib --bins`, 278 mutants; DERIVED by subtracting the excluded mutants from the 282-mutant run below, NOT a re-run): caught 177, missed 56, unviable 45; score = caught / (caught + missed) = 177 / 233 = 76.0%.

| File | Caught | Missed | Unviable | Score |
|------|--------|--------|----------|-------|
| `src/game/scorer.rs` | 20 | 0 | 1 | 100% |
| `src/learning/scheduler.rs` | 15 | 0 | 5 | 100% |
| `src/gamification/quests.rs` | 62 | 19 | 13 | 76.5% |
| `src/learning/performance.rs` | 80 | 37 | 26 | 68.4% |

Provenance: the first raw run, on the batch branch at `30376d5` plus the working tree of that moment (41 minutes wall clock, 0 timeouts), had 282 mutants: caught 179, missed 58, unviable 45, score 179 / 237 = 75.5%, with scheduler 17 caught / 2 missed / 5 unviable (89.5%). The exclusions `impl (std::)?(fmt::)?(Display|Debug) for` and `impl PartialEq for ReviewItem` (excluded per impl, so the 2 CAUGHT `ReviewItem::eq` mutants left together with the 2 missed noise mutants) removed 4 mutants and give the effective baseline above.

Disposition of the 56 missed mutants of the effective baseline (FR-008, SC-003): all are genuine test gaps tracked by #633 (the 2 noise misses of the raw run are excluded, not ticketed). The weakest area is FSRS performance tracking (`performance.rs`), then quest generation.

## 12. See Also

- [[constitution]] — project principles
- [[MOC-specs]] — all specifications
- [[fsrs-proptest-coverage-gap/README|FSRS scheduler proptest coverage gap]] — related scheduler test-strength work
- [cargo-mutants documentation](https://mutants.rs)
- [cargo-mutants on crates.io](https://crates.io/crates/cargo-mutants)
- Issue #536 — vacuous test asserts that line coverage cannot detect
