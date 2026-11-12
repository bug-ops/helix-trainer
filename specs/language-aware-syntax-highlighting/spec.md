---
aliases:
  - Language-Aware Syntax Highlighting
  - Scenario Filetype Field
tags:
  - sdd
  - spec
  - ui
  - scenarios
  - config
  - architecture
  - status/proposed
created: 2026-08-09
status: implemented
related:
  - "[[constitution]]"
  - "[[MOC-specs]]"
---

# Feature: Language-Aware Syntax Highlighting for Scenario Content

> [!info] Amended 2026-11-11 (#544, user decision D-544 = A)
> The language is a CLOSED `Language` enum (`rs`, `md`, `py`) parsed at scenario load. An unknown or empty `language` is a scenario LOAD ERROR, not a plain-text fallback: FR-001, FR-003..FR-006, NFR-002, the data model, the edge cases and SC-003 below are rewritten accordingly. This supersedes the original graceful-degradation design.

> [!info] Metadata
> **Depth**: Lightweight spec only, per this project's SDD scaling guidance
> ("small bug / single function → 3-5 sentences + acceptance criteria") —
> this is a scoped fix touching one struct field and one function body, no
> new module or architecture. A full BRD/SRS/NFR/plan/tasks package would be
> disproportionate.
> **Status**: Implemented — see issue #360.

## 1. Overview

### Problem Statement

`highlight_code_with_multi_cursor` in `src/ui/render/highlight.rs:65` calls
`SYNTAX_SET.find_syntax_by_extension("rs")` unconditionally to highlight
every scenario's `file_content`, regardless of what that content actually
is. The `Scenario`/`Setup` struct in `src/config/scenarios.rs` has no
`language`/`filetype` field at all, so there is no way for a scenario TOML
to declare its content type — and `Scenario` derives
`#[serde(deny_unknown_fields)]`, so a TOML author cannot even add an ad-hoc
field without a schema change.

This has been invisible so far because all 149 currently-shipped scenarios
across the 9 categories under `scenarios/en/` contain exclusively
Rust-flavored content (`fn`, `let`, `struct`, `use`, `println` — verified
via `grep -h "file_content" -A1 scenarios/en/*/*.toml`). But the hardcoding
silently blocks two independently-useful directions that are both currently
being drafted:

1. A markdown/prose-editing scenario track (GitHub issue #152) — markdown
   highlighted as Rust renders headings, links, and emphasis with wrong
   token boundaries.
2. Diversifying existing code-editing categories across multiple
   programming languages (Python, JS/TS, Go, C) so that different
   bracket/quote/indentation/comment-token conventions exercise Helix
   motions differently — any such content would render with incorrect
   (Rust) syntax coloring today.

### Goal

A scenario TOML can optionally declare its content's language/filetype; the
highlighter resolves the syntect syntax definition from that declaration
instead of a hardcoded `"rs"` literal, with existing scenarios continuing
to render identically (defaulting to Rust) without any TOML changes.

### Out of Scope

- Actually authoring any non-Rust scenario content (markdown track,
  multi-language code track) — that is the job of the two dependent specs
  listed in [[#9. See Also|See Also]]
- Adding new syntect syntax definitions beyond what `SyntaxSet::load_defaults_newlines`
  already bundles (covers common languages including Markdown, Python,
  JavaScript, Go, C — sufficient for the markdown/prose track). **Correction
  (verified during implementation review):** the bundled default set does
  *not* include TypeScript (`ts`/`typescript` resolve to neither
  `find_syntax_by_extension` nor `find_syntax_by_token`), unlike the other
  languages listed above. A `language = "ts"` scenario is a scenario
  load error per FR-004 (#544): the closed `Language` enum has no TypeScript
  variant. `specs/multi-language-scenario-content/`
  must account for this — either drop TypeScript from its initial scope or
  explicitly plan to pull in an extra `.sublime-syntax` definition (an
  "Ask First" action per this spec's Agent Boundaries) when it is authored.
- Per-line or embedded/mixed-language highlighting within a single scenario
  (e.g. Markdown with fenced code blocks) — one language per scenario only
- Changing the highlighting theme (`base16-eighties.dark` stays as-is)

## 2. Functional Requirements

Use EARS notation. Prefix with FR-NNN.

| ID | Requirement | Priority |
|----|------------|----------|
| FR-001 | THE SYSTEM SHALL support an optional `language` field on `Setup` (`src/config/scenarios.rs`), deserialized as `Option<Language>` where `Language` is a closed enum in `helix::language` (`Rs`, `Md`, `Py`, serialized as `rs`, `md`, `py`) with `#[serde(default)]`, so existing scenario TOML files that omit it continue to parse without modification | must |
| FR-002 | WHEN a scenario's `Setup.language` is `None` THE SYSTEM SHALL treat it as `"rs"` (current hardcoded behavior), preserving identical rendering for all 149 existing scenarios | must |
| FR-003 | WHEN `highlight_code_with_multi_cursor` is invoked THE SYSTEM SHALL resolve the syntect syntax through `find_syntax_by_extension` with the extension of the scenario's effective `Language` (`rs`, `md` or `py`), instead of the literal `"rs"` | must |
| FR-004 | WHEN the declared `language` value is not one of the `Language` tokens (`rs`, `md`, `py`), including the empty string and a value with different case, THE SYSTEM SHALL reject the scenario at load time through the existing scenario-load error path (`UserError`, naming the scenario and the value), never panic, never fall back to plain text and never silently reuse Rust highlighting. The highlighter has no branch for an unknown language. IF the bundled `SyntaxSet` lacked the syntax of a variant, plain text would be used for that variant; a test asserts that all three variants resolve to a non-plain syntax | must |
| FR-005 | THE SYSTEM SHALL accept exactly the `language` tokens `rs`, `md` and `py` (the file-extension tokens syntect resolves via `find_syntax_by_extension`). A further language (for example `js`, `go`, `c`) requires a new `Language` variant, a rules entry for it ([[helix-core-post-25-07-1-upgrade-readiness/spec]] FR-028) and its syntax; it is added by its own spec, never by free-form strings | must |
| FR-006 | THE SYSTEM SHALL validate `language` by the enum parse itself: control characters, whitespace, oversized values and unknown tokens are all rejected as unknown variants. The free-form validator `validators::validate_language_field` in `src/security.rs` is removed | must |

## 3. Non-Functional Requirements

| ID | Category | Requirement |
|----|----------|-------------|
| NFR-001 | Backward Compatibility | All 149 existing scenario TOML files under `scenarios/en/**/*.toml` MUST continue to load and render with zero visual diff — no `language` field needs to be added to any of them; `cargo nextest run scenario` MUST pass unchanged |
| NFR-002 | Robustness | An unsupported or misspelled `language` value MUST NOT cause a panic or `unwrap()` failure on load or render; it is a clear load error for that scenario naming the value (FR-004), reported through the same path as any other invalid scenario field |
| NFR-003 | Performance | Syntax resolution is a `HashMap`-backed lookup in a `LazyLock`-cached `SyntaxSet` already paid for today; adding a language parameter must not introduce measurable per-frame overhead beyond the existing lookup cost |
| NFR-004 | Maintainability | The change is confined to `Setup`/`highlight_code_with_multi_cursor` and their direct call sites — no new module, no new public API surface beyond the one struct field |

## 4. Data Model

| Entity | Description | Key Attributes |
|--------|-------------|----------------|
| `Setup` (`src/config/scenarios.rs`) | Initial editor state for a scenario, currently `file_content: String` + flattened `CursorSpec` | + `language: Option<Language>` (optional; `None` is treated as `rs`; unknown values are a load error, #544) |

No new entities. `TargetState` is not touched — target content is compared
structurally against the document, never syntax-highlighted, so it does not
need a `language` field.

## 5. Edge Cases and Error Handling

| Scenario | Expected Behavior |
|----------|-------------------|
| Existing scenario TOML with no `language` field | Parses as `None`, highlighter uses `"rs"` — identical to current behavior (FR-002) |
| `language = "md"` and syntect's bundled defaults include Markdown | Highlighted using the Markdown syntax definition |
| `language = "cobol"` (or any token other than `rs`, `md`, `py`) | The scenario fails to load with an error naming the scenario and the value (FR-004); the rest of the scenario set is unaffected |
| `language = ""` (empty string, explicitly set) | Load error as any unknown token (FR-004), not coerced to `"rs"` |
| Two scenarios in the same TOML file with different `language` values | Each resolves independently per its own `Setup.language` — no cross-scenario leakage, since `HighlightLines` is constructed fresh per call inside `highlight_code_with_multi_cursor` |

## 6. Success Criteria

Measurable metrics that prove the feature works:

| ID | Metric | Target |
|----|--------|--------|
| SC-001 | Existing scenario test suite | `cargo nextest run --workspace --all-features --lib --bins` and `cargo nextest run scenario` pass unchanged, 0 regressions |
| SC-002 | Backward-compat rendering | All 149 existing scenarios render with the same highlighted output as before the change (no `language` field added to any of them) |
| SC-003 | Unsupported-language handling | Regression tests assert that an unknown or empty `language` fails the scenario load with an error naming the value, that no panic occurs, and that `rs`, `md` and `py` resolve to non-plain syntaxes |
| SC-004 | Unlocks dependent specs | `specs/writing-markup-scenario-track/` and `specs/multi-language-scenario-content/` can declare `language = "md"` / `language = "py"` (etc.) without further changes to `src/ui/render/highlight.rs` or `src/config/scenarios.rs` |

## 7. Agent Boundaries

### Always (without asking)
- Run `cargo nextest run --workspace --all-features --lib --bins` and
  `cargo nextest run scenario` after touching `src/config/scenarios.rs` or
  `src/ui/render/highlight.rs`
- Keep `Scenario`'s `#[serde(deny_unknown_fields)]` intact — add the new
  field explicitly to `Setup` rather than loosening field validation

### Ask First
- Adding a `Language` variant (needs its rules entry and syntax, FR-005)
- Adding a new dependency to expand syntect's bundled language coverage
  beyond `SyntaxSet::load_defaults_newlines` (e.g. a custom `.sublime-syntax`
  bundle) — the default set is expected to be sufficient for both
  dependent tracks; confirm before pulling in extra assets

### Never
- Add `language` to `TargetState` — target content is never rendered
  through the syntax highlighter
- Reintroduce a fallback for unknown languages (neither reuse Rust
  highlighting nor plain text: unknown is a load error, FR-004) — that reintroduces a silent-wrong-coloring failure mode,
  the same class of bug this spec exists to fix

## 8. Open Questions

- [NEEDS CLARIFICATION: Should `language` also gate any non-rendering
  behavior (e.g. future language-aware scoring/diffing), or is its scope
  strictly limited to `highlight.rs` for now? Assumed strictly rendering-only
  for this spec; dependent specs should re-raise if that assumption breaks.]
- **Resolved**: FR-006 was first implemented as `validators::validate_language_field`
  in `src/security.rs` (permissive character set); #544 removes it because the
  closed `Language` enum parse is the validation.

## 9. See Also

- [[constitution]] — project principles
- [[MOC-specs]] — all specifications
- `specs/writing-markup-scenario-track/` — proposed markdown/prose scenario
  track (issue #152); depends on this spec for correct Markdown rendering
- `specs/multi-language-scenario-content/` — proposed diversification of
  code-editing categories across Python/JS/TS/Go/C; depends on this spec
  for correct per-language rendering
- `src/ui/render/highlight.rs` — implementation target
- `src/config/scenarios.rs` — `Setup` struct, implementation target
