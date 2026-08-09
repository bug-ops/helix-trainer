---
aliases:
  - Multi-Language Scenario Content
  - Non-Rust Scenario Syntax Variety
tags:
  - sdd
  - spec
  - scenarios
  - i18n
  - content
  - status/implemented
created: 2026-08-09
status: implemented
related:
  - "[[constitution]]"
  - "[[MOC-specs]]"
---

# Feature: Multi-Language Scenario Content

> [!info] Metadata
> **Origin**: `rust-researcher` continuous-improvement cycle, 2026-08-09 —
> content-expansion research alongside the writing/markup track proposal.
> **Priority**: P3
> **Depth**: Lightweight spec only, per this project's SDD scaling
> guidance ("feature (1-3 files)" tier) — this is content diversification
> within existing categories, not a new content type or architecture
> change. No BRD/SRS/NFR/plan/tasks package.
> **Status**: Implemented — see issue #362. The pilot shipped exactly as
> scoped: `movement` and `editing` categories, Python only, no
> `ScenarioCategory` changes (SC-004).

## 1. Overview

### Problem Statement

All 149 existing scenarios, across all 9 categories (`basic`, `clipboard`,
`editing`, `macros`, `movement`, `registers`, `repeat`, `search`,
`selection`), use exclusively Rust-flavored syntax in their
`setup.file_content` and `target.file_content` fields — confirmed via
`grep -h "file_content" -A1 scenarios/en/*/*.toml`, which shows only `fn`,
`let`, `struct`, `use`, `println!` style content and zero occurrences of
Python (`def`, indentation-significant blocks), JS/TS, Go, or C-style
content.

This matters because different languages exercise Helix's motions and
text-objects differently:

- **Bracket density** — Rust/C/JS lean heavily on `{}`/`()`/`[]`, which
  drives `mi(`, `ms`, `md` surround-command drills. Python code is
  comparatively bracket-sparse.
- **String quoting style** — single vs. double quotes, Python triple/
  f-strings vs. Rust's plain `"..."` — affects `mi"` / `ma"` text-object
  drills.
- **Indentation semantics** — Python is indentation-significant, unlike
  Rust's brace-delimited blocks. This meaningfully changes how `>`/`<`
  (indent/outdent) and `=` (reindent) commands behave and are trained.
- **Comment tokens** — `//` (Rust/JS/Go) vs. `#` (Python) vs. `/* */`
  block comments — relevant to any future comment-toggle scenarios.

Training exclusively on Rust syntax means a user who writes Python, Go, or
JS/TS day-to-day has drilled Helix motions only against Rust's syntax
shape, not the shape of code they will actually edit. Motion muscle memory
transfers, but the specific keystroke sequence for, e.g., deleting a
Python triple-quoted string or reindenting a Python block after an `if`
insertion is untested by the current library.

> [!note] Competitive Parity
> Researched vim-be-good, vimtutor, OpenVim, Vim Adventures, and VimHero —
> none of them vary the programming language of their code-editing drills
> either. This is a potential differentiator, not catch-up.

### Goal

A pilot set of scenarios in 1-2 categories (recommended: `movement` and
`editing`) has a parallel Python variant validating that non-Rust syntax
content is viable, sets a content-authoring pattern other languages
(Go, JS/TS) can follow later, and does not require any `ScenarioCategory`
enum changes.

### Out of Scope

- Adding new `ScenarioCategory` variants — a Python/Go/JS scenario fits
  existing categories unchanged; this is pure content diversification
- Implementing language-aware syntax highlighting itself — that is the
  prerequisite tracked in [[language-aware-syntax-highlighting/spec|Language-Aware Syntax Highlighting]]
  (being authored separately; this feature is blocked on it landing)
- Full rollout across all 9 categories and all target languages in one
  pass — start with the 1-2 category pilot described here
- Non-programming-language content tracks (markdown/writing prose) — see
  the sibling proposal [[writing-markup-scenario-track/spec|Writing/Markup Scenario Track]]
- Localizing scenario *descriptions/hints* into other spoken languages —
  unrelated to `metadata.locale`; this spec is about the programming
  language of the sample code, not the UI/prose language

## 2. User Stories

### US-001: Python developer drills Python-shaped code

AS A helix-trainer user whose daily work is in Python
I WANT movement and editing drills that operate on Python syntax
(indentation-significant blocks, `def`/`class`, single/double-quoted
strings, `#` comments)
SO THAT the motions I practice map directly onto the code shapes I edit
every day, including indent/outdent behavior that Rust's brace-delimited
syntax never exercises

**Acceptance criteria:**
```
GIVEN the scenario library includes a Python variant of a movement or
  editing scenario
WHEN  the user selects that scenario from the scenario browser
THEN  the scenario loads with Python source content, a correct target
      state expressed in Python syntax, and a solution key sequence that
      achieves that target using the same Helix commands taught by the
      Rust original
```

### US-002: Content author adds a new-language variant without touching the enum

AS A content author extending the scenario library to a new language
I WANT to author a scenario TOML file using existing `ScenarioCategory`
values
SO THAT I do not need a code change to `src/config/scenarios.rs` to add
language variety — only new TOML content and (once the prerequisite
lands) a `language` field on `Setup`

**Acceptance criteria:**
```
GIVEN a new scenario TOML file with Python file_content is placed under
  scenarios/en/movement/ using an existing category value (e.g. "movement")
WHEN  the scenario loader parses the file
THEN  it loads successfully with no changes required to ScenarioCategory
      or any other enum in src/config/scenarios.rs
```

## 3. Functional Requirements

| ID | Requirement | Priority |
|----|------------|----------|
| FR-001 | THE SYSTEM SHALL support scenario content in languages other than Rust using the existing `ScenarioCategory` enum unchanged — no new variants added for this feature | must |
| FR-002 | WHEN [[language-aware-syntax-highlighting/spec\|the language-aware syntax highlighting prerequisite]] lands (a `language` field on `Setup`/`TargetState` and per-language syntax lookup replacing the hardcoded `find_syntax_by_extension("rs")` in `src/ui/render/highlight.rs`) THE SYSTEM SHALL allow non-Rust scenario TOML files to declare their language and render with correct syntax highlighting | must |
| FR-003 | THE SYSTEM SHALL author a pilot set of Python-variant scenarios covering 1-2 existing categories (recommended: `movement`, `editing`), each a parallel of an existing Rust scenario testing the same command(s) against Python-shaped content | must |
| FR-004 | THE SYSTEM SHALL prioritize Python pilot scenarios that specifically exercise indentation-significant behavior (`>`, `<`, `=` commands against `if`/`for`/`def` blocks) since this is the syntax dimension with no Rust equivalent | should |
| FR-005 | WHEN no `language` field is present on a scenario's `Setup` THE SYSTEM SHALL default to Rust syntax highlighting, preserving current behavior for all 149 existing scenarios | must |
| FR-006 | THE SYSTEM SHALL NOT modify or renumber any existing Rust scenario's `id` when adding parallel-language variants — new scenarios get new, distinct IDs | must |

## 4. Non-Functional Requirements

| ID | Category | Requirement |
|----|----------|-------------|
| NFR-001 | Compatibility | All 149 existing Rust-only scenarios remain the default and are byte-for-byte unchanged by this feature — no `language` field means Rust, exactly as today |
| NFR-002 | Testability | New Python-variant scenario TOML files must pass `cargo nextest run scenario` (structural/schema validation) with no changes to the validator required beyond what the prerequisite spec already introduces |
| NFR-003 | Dependency | This feature cannot ship visually-correct syntax highlighting until [[language-aware-syntax-highlighting/spec\|language-aware-syntax-highlighting]] lands; scenario TOML content may be authored ahead of that landing but should not be exposed to users (e.g. behind a feature flag or unmerged branch) until highlighting is correct, to avoid a regression where Python code is mis-highlighted as Rust |
| NFR-004 | Scope discipline | The pilot (FR-003) is capped at 1-2 categories and does not block on or require full 9-category, multi-language coverage before being considered a successful validation |

## 5. Data Model

No new persistent entities. Reuses the existing `Scenario` /
`ScenarioMetadata` / `Setup` / `TargetState` structures in
`src/config/scenarios.rs`. The only structural addition this feature
depends on — a `language` field on `Setup`/`TargetState` — is owned by
the prerequisite spec, not this one.

| Entity | Description | Relevant Fields (existing, unchanged) |
|--------|-------------|----------------|
| `Scenario` | A single trainable drill | `id`, `setup`, `target`, `solution`, `metadata` |
| `ScenarioMetadata` | Filtering/quest metadata | `category` (existing `ScenarioCategory` enum, unchanged), `locale` (spoken-language of prose, unrelated to code language) |
| `Setup` / `TargetState` | Initial/goal editor content | `file_content` (will hold Python source for pilot scenarios) |

## 6. Edge Cases and Error Handling

| Scenario | Expected Behavior |
|----------|-------------------|
| A Python-variant scenario is authored before the highlighting prerequisite lands | Content is valid TOML and passes `cargo nextest run scenario`, but is not surfaced to end users until highlighting support ships (per NFR-003) |
| A scenario's Python solution requires a Helix command with no direct Rust-scenario equivalent (e.g. reindent after `def`) | Author it as a new scenario testing that command explicitly rather than forcing a 1:1 mirror of an existing Rust scenario |
| Difficulty/category filters are applied by the user in the scenario browser | Python-variant scenarios appear identically alongside Rust scenarios in the same category/difficulty buckets — no separate "language" filter is introduced by this feature |

## 7. Success Criteria

| ID | Metric | Target |
|----|--------|--------|
| SC-001 | Pilot Python scenarios pass validation | `cargo nextest run scenario` green for all new scenario files |
| SC-002 | Existing scenario suite unaffected | `cargo nextest run --workspace --all-features --lib --bins` shows zero regressions in existing 149 scenarios |
| SC-003 | Pilot scope | At least one Python scenario each in `movement` and `editing` categories, with at least one specifically exercising indent/outdent (`>`/`<`/`=`) semantics |
| SC-004 | No enum drift | `ScenarioCategory` in `src/config/scenarios.rs` has zero new variants after this feature ships |

## 8. Agent Boundaries

### Always (without asking)
- Run `cargo nextest run scenario` after adding or modifying any scenario TOML file
- Reuse existing `ScenarioCategory` values; never introduce a new category for language variety alone

### Ask First
- Merging/exposing Python-variant scenario content to users before confirming [[language-aware-syntax-highlighting/spec|language-aware-syntax-highlighting]] has landed and been verified against non-Rust content
- Adding a `language` field to `Setup`/`TargetState` outside the prerequisite spec's implementation (avoid duplicating that work here)

### Never
- Modify the `id` or content of an existing Rust scenario to "convert" it — always add a new, separate scenario file for the language variant
- Add a new `ScenarioCategory` variant to accommodate a new language — categories are motion/action-based, not language-based

## 9. Open Questions

- [NEEDS CLARIFICATION: Which languages beyond Python are in scope for a second wave — Go and JS/TS were named as candidates but not prioritized against each other]
- [NEEDS CLARIFICATION: Should the scenario browser eventually gain a language filter/badge, or should language variety stay invisible to the category/difficulty UI as proposed here]

## 10. See Also

- [[constitution]] — project principles
- [[MOC-specs]] — all specifications
- [[language-aware-syntax-highlighting/spec|Language-Aware Syntax Highlighting]] — **prerequisite**: this feature's syntax-correct rendering is blocked on that spec landing (`language` field on `Setup`, replacing the hardcoded `find_syntax_by_extension("rs")` in `src/ui/render/highlight.rs`). Being authored in parallel by the same research cycle; link target may not exist yet at time of writing.
- [[writing-markup-scenario-track/spec|Writing/Markup Scenario Track]] — sibling content-expansion proposal from the same research cycle, sharing the same highlighting prerequisite but targeting prose/markdown content rather than additional programming languages
- `src/config/scenarios.rs` — `ScenarioCategory`, `Scenario`, `Setup` definitions referenced throughout this spec
- `scenarios/en/` — existing 149 Rust-only scenario TOML files across 9 categories
