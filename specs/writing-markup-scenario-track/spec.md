---
aliases:
  - Writing / Markup Scenario Track
  - Markdown & Typst Prose Editing Scenarios
tags:
  - sdd
  - spec
  - scenarios
  - ui
  - config
  - status/implemented
created: 2026-08-09
status: implemented
related:
  - "[[constitution]]"
  - "[[MOC-specs]]"
---

# Feature: Writing / Markup Scenario Track

> [!info] Metadata
> **Source**: GitHub issue #152 ("markdown/typst related scenarios"),
> filed by @fabiosirna, labeled `enhancement`, `P4`, `scenarios`
> (repo-assigned priority). This spec's own triage priority is **P3** —
> see rationale below.
> **Status**: Implemented — see issue #361.
> **Depth**: Lightweight spec only, per this project's SDD scaling
> guidance. No dedicated new engine capability is required (see FR-001
> rationale) — this is primarily new scenario content plus one closed-enum
> extension, so a full BRD/SRS/NFR/plan/tasks package would be
> disproportionate at this stage. Revisit depth if scope grows once
> `[NEEDS CLARIFICATION]` items below are resolved.

## 1. Overview

### Problem Statement

helix-trainer's scenario library (`scenarios/en/{basic,clipboard,editing,
macros,movement,registers,repeat,search,selection}/`) is built almost
entirely around code-editing motions and commands. Issue #152 observes that
users who write prose or technical documentation in Helix (Markdown, Typst)
exercise a distinct pattern set that today's scenarios don't drill:
surround-based emphasis (bold/italic), link construction, list
indent/outdent, heading-level changes, blockquote prefixing, and code-fence
wrapping. None of the current categories target this workflow, so a writer
learning Helix has no dedicated on-ramp — they'd have to infer prose-editing
technique from code-focused drills.

Investigation confirmed this is cheaper to deliver than the issue title
("new module") suggests:

1. The core mechanic issue #152 leans on hardest — `ms`/`md`/`mr`
   (add/delete/replace surround) — is **already fully implemented**:
   `CMD_SURROUND_ADD_PREFIX` in `src/helix/commands.rs`, the `ms` chord
   resolution in `src/helix/registry/keytrie.rs`, `surround_selection` /
   `delete_surround` / `replace_surround` in
   `src/helix/simulator/commands/editing.rs`, and full typestate coverage
   in `src/input/typestate/`. However,
   `scenarios/en/editing/surround.toml` carries a **stale** header comment
   ("Note: ms (add surround) is not yet implemented") — the file currently
   contains only `md`/`mr` scenarios; no `ms` scenarios exist yet, but
   nothing blocks authoring them today. See FR-006.
2. `HelixSimulator` has no tree-sitter or filetype-aware textobject logic
   (confirmed: no `tree_sitter` / `Syntax::` / `Loader::` usage anywhere
   under `src/`). It purely replays real Helix keystrokes against a
   `helix-core` buffer. A "markdown scenario" is therefore not a new
   simulator capability — it is ordinary `setup`/`target`/`solution` TOML
   content, same as any other scenario, provided the syntax-highlighting
   rendering issue below is addressed first.
3. `ScenarioCategory` (`src/config/scenarios.rs`) is a closed Rust enum —
   `Movement | Editing | Clipboard | Search | Selection | TextObjects |
   Advanced | Registers | Multi | Other` — not a free-form string sourced
   from TOML. Introducing a "Writing" category is a source-code change (new
   variant + updating every exhaustive `match` over it across
   filtering/UI/quest-generation code), not just adding a new
   `scenarios/en/writing/` directory.
4. **Prerequisite**: `src/ui/render/highlight.rs` hardcodes
   `find_syntax_by_extension("rs")` and `Setup` has no `language` field —
   all scenario content renders with Rust syntax highlighting regardless of
   actual content. Markdown/Typst scenario text would render with
   incorrect coloring until this is fixed. Tracked separately in
   [[../language-aware-syntax-highlighting/spec|Language-Aware Syntax
   Highlighting]] (parallel/companion spec). This feature depends on that
   one landing first — see FR-007.
5. **Competitive landscape**: vim-be-good, vimtutor, OpenVim, Vim
   Adventures, and VimHero were checked — none ship a dedicated
   markdown/prose-editing drill track. This would be a genuine
   differentiator for helix-trainer, not parity catch-up, which supports
   prioritizing it above a typical "someday" (P4) request despite its
   modest scope.

### Goal

A learner can select a "Writing" category filter and complete a coherent
set of scenarios that drill the concrete prose-editing use cases from issue
#152 — surround-based emphasis, link construction, list indent/outdent,
heading-level change, blockquote prefixing, and code-fence wrapping —
rendered with correct Markdown (and, if in scope, Typst) syntax
highlighting.

### Out of Scope

- Any new `HelixSimulator` capability, tree-sitter integration, or
  filetype-aware textobjects — this feature is scenario content plus a
  category enum addition, not an engine change
- "Structural paragraph/list movement" (issue #152's Obsidian
  Alt+Up/Alt+Down analogy) — Helix has no single built-in keybinding
  equivalent to move a paragraph/list-item as a unit; see
  `[NEEDS CLARIFICATION]` FR-003 below on whether to scope this out
  entirely or approximate it with select+delete+paste
- Converting an unordered list to a numbered list (mentioned in issue
  #152's "List management" bullet) — no direct Helix command performs this
  transformation; likely out of scope, see FR-004
- The [[../language-aware-syntax-highlighting/spec|syntax-highlighting
  prerequisite]] itself — that is a separate spec and separate PR
- Any change to `ScenarioCategory`'s existing variants or the surround
  command implementation

## 2. User Stories

### US-001: Practicing text emphasis in prose

AS A technical writer using Helix for Markdown documentation
I WANT dedicated drills for wrapping a word or selection in `**bold**` or
`_italic_` using `ms`
SO THAT I build the same muscle memory for emphasis that I already have for
code-editing motions

**Acceptance criteria:**
```
GIVEN a scenario in the Writing category with setup text containing an
  unformatted word and a target with that word wrapped in ** **
WHEN the learner selects the word and executes ms*
THEN the scorer reports success against the target buffer state
```

### US-002: Constructing a Markdown link

AS A documentation author
I WANT a scenario that drills select-word → wrap in `[ ]` → move to end →
append `(url)`
SO THAT I can build links fluently without looking up the keystroke sequence

**Acceptance criteria:**
```
GIVEN a scenario with setup text containing a bare word and a target with
  that word rendered as a Markdown link to a fixed placeholder URL
WHEN the learner performs the documented solution keystrokes
THEN the resulting buffer matches the target exactly
```

### US-003: Adjusting list and heading structure

AS A writer restructuring a Markdown outline
I WANT drills for indenting/outdenting list items and bumping a heading
between `##` and `###`
SO THAT these become fast, deliberate edits rather than manual retyping

**Acceptance criteria:**
```
GIVEN a scenario with a list item or heading line at one level
WHEN the learner applies the documented indent/outdent or heading-edit
  solution
THEN the resulting line matches the target level exactly
```

### US-004: Quoting and fencing a block

AS A writer citing a source or including a code sample in a Markdown/Typst
document
I WANT drills for prefixing a paragraph with `> ` (blockquote) and wrapping
a block with triple backticks (code fence)
SO THAT these structural edits become as fluent as `ms`-based surround
edits already are for inline emphasis

**Acceptance criteria:**
```
GIVEN a scenario with a plain paragraph and a target with `> ` prefixed to
  each line (blockquote) or the block wrapped in fenced code delimiters
WHEN the learner applies the documented solution
THEN the resulting buffer matches the target exactly
```

## 3. Functional Requirements

Use EARS notation. Prefix with FR-NNN.

| ID | Requirement | Priority |
|----|------------|----------|
| FR-001 | WHEN this feature is implemented THE SYSTEM SHALL NOT require any new `HelixSimulator`, tree-sitter, or filetype-aware textobject capability — all scenarios SHALL be expressible with existing `setup`/`target`/`solution` TOML fields and existing simulator commands | must |
| FR-002 | WHEN a scenario is authored under the Writing track THE SYSTEM SHALL classify it via a new `ScenarioCategory::Writing` enum variant, and every existing exhaustive `match` over `ScenarioCategory` (filtering UI, quest generation, and any other call site) SHALL be updated to handle it | must |
| FR-003 | WHEN authoring scenarios for issue #152's "Structural movement" use case (move a paragraph/list item, analogous to Obsidian's Alt+Up/Down) THE SYSTEM SHALL either omit this use case from the initial scope or implement it via an approximation using existing select/delete/paste commands — **Decision: scoped out entirely for this pilot.** No move-paragraph/list-item scenario was authored; Helix has no single built-in "move block" keybinding, and an artificial select+delete+paste workaround was judged not worth presenting as canonical technique. Revisit as a future dedicated-command request if there's demand. | must |
| FR-004 | WHEN authoring scenarios for issue #152's "convert a list to a numbered list" use case THE SYSTEM SHALL treat it as out of scope, since no direct Helix command performs this transformation — **Decision: exclusion confirmed.** No worthwhile approximate drill was identified; not implemented in this pilot. | should |
| FR-005 | WHEN authoring Writing-track scenarios THE SYSTEM SHALL cover, at minimum, the following concrete use cases from issue #152: (a) bold/italic emphasis via `ms`, (b) Markdown link construction (select word, wrap `[ ]`, append `(url)`), (c) list item indent/outdent (`>`/`<`), (d) heading level change (e.g., `##` ↔ `###`), (e) blockquote prefixing (`> `), (f) code-fence wrapping (triple backtick) | must |
| FR-006 | WHEN this feature is implemented THE SYSTEM SHALL correct the stale header comment in `scenarios/en/editing/surround.toml` ("Note: ms (add surround) is not yet implemented") as a drive-by fix, since `ms` has been fully implemented since the surround feature landed | must |
| FR-007 | WHEN Writing-track scenarios are authored with Markdown or Typst content THE SYSTEM SHALL render them with correct language-specific syntax highlighting, which depends on [[../language-aware-syntax-highlighting/spec|Language-Aware Syntax Highlighting]] landing first — **Decision: Markdown-only for v1.** All six Writing-track scenarios set `setup.language = "md"`; Typst is deferred until a dedicated need arises. | must |
| FR-008 | WHEN a Writing-track scenario TOML file is added to the repository THE SYSTEM SHALL pass `cargo nextest run scenario` schema validation, identical to every other scenario category | must |

## 4. Non-Functional Requirements

| ID | Category | Requirement |
|----|----------|-------------|
| NFR-001 | Compatibility | Adding `ScenarioCategory::Writing` SHALL NOT change the serialized representation or behavior of any existing category value (backward-compatible enum extension, `#[serde(rename_all = "lowercase")]` convention preserved) |
| NFR-002 | Testability | All new scenario TOML files SHALL pass `cargo nextest run scenario` (schema/solution validation) as a hard gate before merge |
| NFR-003 | Consistency | New scenarios SHALL follow the existing TOML schema and file organization convention (`scenarios/en/<category>/*.toml`) used by all other categories, with no new fields introduced without updating the shared scenario schema/parser |
| NFR-004 | Maintainability | Filtering UI and quest-generation code touched to handle the new enum variant SHALL be updated exhaustively (no `_ => ...` catch-all introduced solely to sidestep the compiler's exhaustiveness check) |

## 5. Data Model

| Entity | Description | Key Attributes |
|--------|-------------|-----------------|
| `ScenarioCategory::Writing` | New enum variant identifying prose/markup-editing scenarios | Serialized as `writing` per existing `rename_all = "lowercase"` convention |
| Writing-track `Scenario` (TOML) | Ordinary `Scenario` entity (no new fields), content targeting Markdown/Typst prose-editing patterns | `setup` (initial buffer + cursor, Markdown/Typst content), `target` (goal buffer state), `solution` (canonical keystrokes), `metadata.category = Writing`, `metadata.commands_taught`, `metadata.tags` |

No new persistent entities, no changes to `Profile`, FSRS card state, or
any other data model outside the `ScenarioCategory` enum itself.

## 6. Edge Cases and Error Handling

| Scenario | Expected Behavior |
|----------|-------------------|
| Learner selects "Writing" category filter before any Writing scenarios exist | Category is a valid, empty result set — same behavior as any other category with zero matching scenarios today; no special-case handling required |
| A Writing scenario's `setup` content contains Typst-specific syntax before Typst highlighting support lands | Scenario SHOULD be gated behind confirming Typst support in the prerequisite spec, or restricted to Markdown-only content for v1 (see FR-007 clarification) |
| Existing code that exhaustively matches `ScenarioCategory` is missed during the enum addition | `cargo build` fails to compile (Rust's exhaustiveness checking on non-`_` matches) — this is a compile-time safety net, not a runtime edge case, but should be called out during implementation review |
| A Writing scenario's `target` requires the "move paragraph" use case that has no direct Helix equivalent (FR-003) | Scenario SHOULD NOT be authored until FR-003's `[NEEDS CLARIFICATION]` is resolved — do not ship a scenario whose "solution" is an artificial workaround presented as canonical technique without flagging it as such in `metadata.tags` |

## 7. Success Criteria

| ID | Metric | Target |
|----|--------|--------|
| SC-001 | `ScenarioCategory::Writing` compiles and every existing exhaustive match site handles it | 100% (verified by `cargo build` + `cargo clippy --all-targets --all-features --workspace -- -D warnings`) |
| SC-002 | New Writing-track scenario TOML files pass schema validation | 100% via `cargo nextest run scenario` |
| SC-003 | Concrete use cases from issue #152 covered by at least one scenario each (excluding items explicitly scoped out per FR-003/FR-004) | 6/6 in-scope use cases (FR-005 a–f) |
| SC-004 | Stale `surround.toml` comment corrected | Done (FR-006) |

## 8. Agent Boundaries

### Always (without asking)
- Run `cargo nextest run scenario` after adding or modifying any Writing
  scenario TOML file
- Run the full check suite (`cargo +nightly fmt --check`, `cargo clippy
  --all-targets --all-features --workspace -- -D warnings`, `cargo nextest
  run --workspace --all-features --lib --bins`) after adding the
  `ScenarioCategory::Writing` enum variant
- Correct the stale `scenarios/en/editing/surround.toml` header comment as
  part of this work (FR-006)

### Ask First
- Authoring any scenario that approximates the "move paragraph/list item"
  use case (FR-003) before its `[NEEDS CLARIFICATION]` is resolved
- Adding Typst-specific scenario content before confirming Typst support in
  [[../language-aware-syntax-highlighting/spec|the highlighting
  prerequisite]] (FR-007)

### Never
- Add a new `HelixSimulator` capability, tree-sitter dependency, or
  filetype-aware textobject logic to satisfy this feature — if a use case
  appears to require one, flag it and scope it out rather than expanding
  the simulator (violates FR-001 / Out of Scope)
- Introduce a `_ => ...` catch-all in an exhaustive `ScenarioCategory`
  match purely to avoid updating call sites for the new variant

## 9. Open Questions

- **Resolved (FR-003)**: "structural paragraph/list movement" is scoped out entirely for this pilot — no scenario was authored for it.
- **Resolved (FR-004)**: "convert list to numbered list" is excluded — no worthwhile approximate drill was identified.
- **Resolved (FR-007)**: v1 ships Markdown-only; Typst is deferred.
- Should Writing-track scenarios support a language other than English content-wise (this repo's existing scenarios live under `scenarios/en/`), or is English-only prose content acceptable for v1 given the trainer's existing i18n scope is UI strings, not scenario content?

## 10. See Also

- [[constitution]] — project principles
- [[MOC-specs]] — all specifications
- [[../language-aware-syntax-highlighting/spec|Language-Aware Syntax Highlighting]] — prerequisite spec; Writing-track scenarios depend on this landing first for correct Markdown/Typst rendering
- GitHub issue [#152](https://github.com/bug-ops/helix-trainer/issues/152) — "markdown/typst related scenarios" (original feature request this spec evaluates)
- `scenarios/en/editing/surround.toml` — contains the stale `ms`-not-implemented comment corrected by FR-006
- `src/config/scenarios.rs` — `ScenarioCategory` enum definition (FR-002 target)
- `src/ui/render/highlight.rs` — hardcoded Rust syntax highlighting, addressed by the prerequisite spec
