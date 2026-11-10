---
aliases:
  - Duplicate Scenario IDs
  - Fix Duplicate Scenario IDs Across TOML Files
tags:
  - sdd
  - spec
  - bug-fix
  - scenarios
  - learning
  - status/implemented
created: 2026-08-09
status: implemented
related:
  - "[[constitution]]"
  - "[[../end-game-summary-screen/spec]]"
  - "[[MOC-specs]]"
---

# Feature: Fix Duplicate Scenario IDs Across TOML Files

> [!info] Metadata
> **Source**: rust-agents CI cycle finding (priority P2, bug)
> **Resolved by**: GitHub issue #363, merged via PR #372 (`7f34268`)
> **Status**: Implemented
> **Depth**: Lightweight spec, per this project's SDD scaling guidance for a single scoped bug fix. Migrated from an earlier untracked draft into the tracked `specs/` tree and reconciled against the shipped code; the original problem analysis is preserved below, and "Resolution (as shipped)" records what actually landed.

> [!note] Reading guide
> Sections 1-8 preserve the original pre-fix analysis (including line numbers and "currently" wording that describe the code before the fix). Section 9 records the shipped resolution and is authoritative for current behavior.

## 1. Overview

### Problem Statement

`ScenarioCollection` loads every scenario TOML file under `scenarios/en/` into a
single `Vec<Scenario>` (`src/config/scenarios.rs:21`). Two `id` values are
duplicated across genuinely different scenario definitions:

1. `id = "delete_line_001"` — appears in `scenarios/en/basic/delete.toml`
   (name: "Delete current line", uses `x` then `d`) and in
   `scenarios/en/editing/delete-selection.toml` (name: "Delete entire line",
   uses `xd`).
2. `id = "join_lines_001"` — appears in `scenarios/en/editing/join.toml`
   (name: "Join two lines", `J` adds a space between joined lines) and in
   `scenarios/en/editing/advanced-editing.toml` (name: "Join two lines", `J`
   described as concatenating directly with no space — contradictory
   documented behavior for the same id).

Both scenarios in each pair load successfully and appear as independently
playable, distinct menu entries in Training mode. However, per-scenario
completion/progress tracking in `ScenarioHistory::completions`
(`src/learning/scenario_history.rs:284-285, 355`) is keyed by this same `id`
string in a map, so a map can only hold one record per key. Playing *either*
scenario in a duplicate-id pair writes to the same `ScenarioCompletion`
record, silently marking the other (unplayed) scenario as completed too.

This was confirmed live: playing only the `basic/delete.toml` version of
"Delete current line" to completion produced exactly one
`scenario_history.completions["delete_line_001"]` record in `profile.json`,
which is now indistinguishable from a play of "Delete entire line"
(`editing/delete-selection.toml`).

Root cause: the scenario TOML authoring process has no automated guard
against copy-pasting a scenario file and forgetting to change its `id`. The
load-bearing scenario validation test,
`test_all_scenarios_load_successfully` (`tests/scenario_validation.rs:20-60`),
validates that each scenario individually constructs a `GameSession` but
never checks `id` uniqueness across the full collection.

### Goal

Every scenario `id` in the embedded collection (`scenarios/en/**/*.toml`) is
unique, both duplicate pairs are resolved by assigning a new distinct `id` to
one scenario in each pair (rather than deleting either scenario, since both
are legitimate, differently-authored training exercises), and
`cargo nextest run scenario` fails loudly if a future duplicate `id` is ever
introduced.

### Out of Scope

- Retroactively repairing already-persisted `profile.json` completion
  history that was corrupted by the aliasing bug before this fix ships (no
  migration is defined; existing users may see a temporary reset of mastery
  state for the affected scenarios after upgrading — see Open Questions).
- Broader validation of scenario *content* correctness (e.g., whether
  `solution.commands` actually transforms `setup` into `target`) — out of
  scope for this bug fix; only `id` uniqueness is addressed.
- Deduplicating/removing either scenario in a colliding pair. Both
  `delete_line_001`/`delete_selection`-style and `join_lines_001` pairs
  represent intentionally distinct exercises (different name, description,
  and — for the join pair — different documented behavior) and both are
  kept, just under distinct ids.
- Resolving the join-pair's underlying behavioral contradiction (whether `J`
  inserts a space or not) beyond what's needed to keep both scenarios loadable
  with correct, non-conflicting `target`/`solution` data — see Open Questions.

## 2. User Stories

### US-001: Trainer maintainer authors a new scenario without ever creating a hidden id collision
AS A scenario content author
I WANT the test suite to reject a scenario TOML file whose `id` collides with
an existing one
SO THAT copy-paste authoring mistakes are caught in CI before they corrupt
any player's progress tracking

**Acceptance criteria:**
```
GIVEN a new scenario TOML file is added under scenarios/en/ with an id
  that already exists in another TOML file in the collection
WHEN cargo nextest run scenario is executed
THEN the test suite fails with an error identifying both colliding scenario
  ids and the files/names involved
```

### US-002: Player's completion history accurately reflects which scenario was played
AS A helix-trainer user
I WANT each distinct scenario I complete to be tracked under its own
completion record
SO THAT my per-scenario mastery, attempt count, and best score reflect only
scenarios I actually played, and curriculum-completion / end-game summary
statistics are accurate

**Acceptance criteria:**
```
GIVEN the "Delete current line" (basic/delete.toml) and "Delete entire line"
  (editing/delete-selection.toml) scenarios now have distinct ids
WHEN a player completes only "Delete current line"
THEN ScenarioHistory::completions contains a record for "Delete current
  line"'s id only, and "Delete entire line" has no completion record
```

```
GIVEN the "Join two lines" (editing/join.toml) and "Join two lines"
  (editing/advanced-editing.toml) scenarios now have distinct ids
WHEN a player completes only one of the two
THEN is_curriculum_complete() and the end-game summary's per-category
  mastery tally do not count the other, unplayed scenario as completed
```

## 3. Functional Requirements

| ID | Requirement | Priority |
|----|------------|----------|
| FR-001 | THE SYSTEM SHALL give the two scenarios that shared `id = "delete_line_001"` distinct ids. As shipped: `scenarios/en/basic/delete.toml` was renamed to `delete_current_line_001`; `scenarios/en/editing/delete-selection.toml` keeps `delete_line_001` (the id `quests/en/daily.toml`'s `speed_delete_hard` quest already targeted) | must |
| FR-002 | THE SYSTEM SHALL give the two scenarios that shared `id = "join_lines_001"` distinct ids. As shipped: `scenarios/en/editing/join.toml` was renamed to `join_lines_basic_001`; `scenarios/en/editing/advanced-editing.toml` keeps `join_lines_001` (its numbered sequence continues with `join_lines_002`) | must |
| FR-003 | WHEN `test_all_scenarios_load_successfully` (or a new dedicated test) runs in `tests/scenario_validation.rs` THE SYSTEM SHALL verify that every loaded scenario's `id` is unique across the entire embedded "en" collection | must |
| FR-004 | IF a duplicate `id` is detected during the uniqueness check THEN THE SYSTEM SHALL fail the test with a message listing every colliding `id` and the `name` of each scenario that shares it | must |
| FR-005 | WHEN either renamed scenario id is changed THE SYSTEM SHALL verify no other source in the repository (tests, docs, other scenario `metadata.tags`/cross-references, if any) hardcodes the old id string | should |
| FR-006 | THE SYSTEM SHALL preserve all other fields (`name`, `description`, `hints`, `setup`, `target`, `solution`, `metadata`) of both renamed scenarios unchanged — only the `id` value changes | must |

## 4. Non-Functional Requirements

| ID | Category | Requirement |
|----|----------|-------------|
| NFR-001 | Reliability | The new uniqueness check must run as part of `cargo nextest run scenario`, the existing CI-gating command, with no new test invocation required |
| NFR-002 | Maintainability | The uniqueness check must scale to any number of scenario TOML files without per-file updates (i.e., it inspects the loaded collection, not a hardcoded file list) |
| NFR-003 | Performance | The uniqueness check adds negligible overhead to the existing ~32-scenario validation run (O(n) or O(n log n) over the collection) |

## 5. Data Model

| Entity | Description | Key Attributes |
|--------|-------------|----------------|
| `Scenario` | A single training exercise loaded from a TOML file (`src/config/scenarios.rs`) | `id: String` (must be globally unique within the embedded "en" collection), `name`, `description`, `setup`, `target`, `solution`, `metadata` |
| `ScenarioCollection` | In-memory collection of all loaded scenarios (`Vec<Scenario>`) | `scenarios: Vec<Scenario>` |
| `ScenarioCompletion` | Per-scenario progress record keyed by scenario id (`src/learning/scenario_history.rs`) | `attempts`, `best_score`, `mastery_level`, `attempts_today`, `first_attempt`, `last_attempt` — looked up/stored via `scenario_id: &str` |

## 6. Edge Cases and Error Handling

| Scenario | Expected Behavior |
|----------|-------------------|
| A future PR adds a new scenario TOML file whose `id` collides with an existing one | `cargo nextest run scenario` fails with a clear message naming both colliding scenarios (FR-003/FR-004) — build/PR checks block the merge |
| Two scenarios have the same `name` but different `id` (not a bug, just a naming coincidence) | Not flagged — uniqueness is enforced on `id` only, not `name` |
| An existing player already has `profile.json` history keyed by `delete_line_001` or `join_lines_001` from before this fix | Their history stays associated with whichever scenario kept the original id (`editing/delete-selection.toml` and `editing/advanced-editing.toml`, per FR-001/FR-002); the renamed sibling (`basic/delete.toml`, `editing/join.toml`) starts with a fresh, empty completion record. A player who only completed the renamed-away scenario will see the surviving sibling falsely shown as completed. No migration is provided; the consequence is documented in `CHANGELOG.md` (#363) |
| Renaming an id accidentally introduces a *new* collision with an unrelated existing id (e.g. picking `delete_line_002` when it's already taken) | Caught immediately by the same uniqueness check (FR-003), so the fix is self-verifying |

## 7. Success Criteria

| ID | Metric | Target |
|----|--------|--------|
| SC-001 | Duplicate scenario ids in `scenarios/en/**/*.toml` | 0 (was 2 pairs / 4 scenarios; 0 after the fix) |
| SC-002 | `cargo nextest run scenario` fails when a duplicate id is present | Enforced by `test_all_scenario_ids_are_unique` in `tests/scenario_validation.rs` |
| SC-003 | `is_curriculum_complete()` and end-game summary category tallies count exactly the number of distinct, independently loaded scenarios | Every loaded scenario has a unique id |

## 8. Agent Boundaries

### Always (without asking)
- Rename the colliding `id` fields in `scenarios/en/editing/delete-selection.toml` and `scenarios/en/editing/advanced-editing.toml`
- Add/extend the uniqueness check in `tests/scenario_validation.rs`
- Run `cargo nextest run scenario` (and the full `cargo nextest run --workspace --all-features --lib --bins`) after the change
- Update `CHANGELOG.md` under `[Unreleased]`

### Ask First
- Changing which scenario in each pair keeps the original id vs. gets renamed (as shipped: the `basic/delete.toml` and `editing/join.toml` scenarios were renamed, see FR-001/FR-002)
- Any change to `scenarios.json`/schema validation tooling beyond the Rust test suite
- Resolving the join-pair's contradictory space/no-space `J` behavior description, if that is judged in-scope

### Never
- Delete either scenario in a colliding pair — both are legitimate distinct exercises
- Silently migrate or merge existing `profile.json` completion history as part of this fix (no migration is specified — see Open Questions)
- Modify scenario `setup`/`target`/`solution` semantics while renaming ids

## 9. Resolution (as shipped)

Merged in PR #372 (issue #363, commit `7f34268`); recorded in `CHANGELOG.md`.

- **Ids**: `delete_line_001` -> `delete_current_line_001` in `scenarios/en/basic/delete.toml`; `join_lines_001` -> `join_lines_basic_001` in `scenarios/en/editing/join.toml`. The original ids stay with the other scenario of each pair. This reverses the draft's default, which proposed renaming the `editing/` variants. After the rename, `quests/en/daily.toml`'s `speed_delete_hard` quest (`scenario_id = "delete_line_001"`) resolves to the surviving `delete-selection.toml` scenario.
- **Guard**: `test_all_scenario_ids_are_unique` in `tests/scenario_validation.rs` fails with the offending ids and scenario names (FR-003/FR-004). It is a dedicated test, so it runs under `cargo nextest run scenario`.
- **Extra guard**: `test_quest_scenario_references_exist` cross-checks every quest's `scenario_id` and `requires_scenarios` against the loaded collection, catching a dangling reference after any future rename.
- **Content**: only `id` values changed (FR-006); the `J` space/no-space behavioral contradiction between the two join scenarios was left out of scope.
- **Profile migration**: none, per the Out of Scope list above.

## 10. Resolved Questions

- **New id values?** `delete_current_line_001` and `join_lines_basic_001` (any unique string is acceptable; the uniqueness test is the contract).
- **Migrate `profile.json`?** No. Accepted one-time re-attribution, documented in `CHANGELOG.md`.
- **Reconcile the join pair's `J` behavior?** Out of scope, not addressed.
- **Dedicated test or folded in?** Dedicated `test_all_scenario_ids_are_unique`.

## 11. See Also

- [[MOC-specs]] — all specifications
- `tests/scenario_validation.rs` — load-bearing scenario validation suite (`cargo nextest run scenario`)
- `src/config/scenarios.rs` — `ScenarioCollection` / `Scenario` definitions
- `src/learning/scenario_history.rs` — `ScenarioHistory::completions` map keyed by scenario id
- `src/config/scenario_collection/mod.rs` — `is_curriculum_complete()`
- `src/ui/state/handlers/scenario.rs` — `build_end_game_summary`
