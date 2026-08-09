---
aliases:
  - Register and Command-Line Mode BRD
  - Issue 282 BRD
tags:
  - brd
  - helix-simulator
  - input-system
  - status/implemented
created: 2026-08-09
project: "helix-trainer"
status: implemented
related:
  - "[[spec]]"
  - "[[SRS]]"
  - "[[NFR]]"
  - "[[plan]]"
---

# Named Register and Command-Line Mode Support: Business Requirements Document

> [!abstract]
> Records the business rationale for closing a competitive-parity gap
> (named registers and command-line mode, absent from the simulator) and the
> outcome after implementation. Retroactive: written after commit `1ba668d`
> shipped.

## Executive Summary

A parity scan against [S-Sigdel/vimhjkl](https://github.com/S-Sigdel/vimhjkl)
found helix-trainer had zero simulator support for two real, documented
Helix features: named registers (`"<reg>`) and command-line mode (`:`).
Commit `1ba668d` closed this gap for named registers essentially as
originally scoped, and for command-line mode with a **deliberately narrowed
scope**: only `:goto N` (line-number navigation, alias `:g N`) shipped — not
`:s` substitute, `:g` global-with-subcommand, or ranges, which the original
research spec had targeted as the minimum command-line capability.

## Problem Statement

- **What problem existed?** helix-trainer taught a strict subset of Helix's
  real command surface: no multi-slot register workflow, and no reachable
  command-line surface at all (`EditorMode` only had `NormalMode`/
  `InsertMode`).
- **Who was affected?** Learners practicing intermediate/advanced Helix
  workflows who would encounter `"<reg>` and `:` in real Helix usage but
  never see them drilled in the trainer.
- **Impact of not solving it**: a persistent, named competitive-parity gap
  against a reference project (vimhjkl) that already drills the equivalent
  Vim concepts.

## Decision

> [!important] GO — implemented with explicit scope narrowing
> Named registers were implemented essentially as drafted. Command-line
> mode was implemented at minimum viable scope (`:goto`/`:g N` only) rather
> than the `:s` substitute originally targeted as the feature's minimum bar.
> This narrowing is documented explicitly in `docs/HELIX_KEYBINDINGS.md`
> (a "❌ everything else" disclaimer row), not left implicit.

## What Was Implemented

### Named Registers

- `RegisterFile` (`src/helix/simulator/register_file.rs`) — a
  `HashMap<char, String>` keyed by register name, with a reserved unnamed
  register (`'"'`) as the default slot.
- `HelixSimulator`'s prior single `clipboard: Option<String>` field was
  replaced by `registers: RegisterFile` — a rename/generalization, not an
  additive parallel structure.
- New input-typestate states: `RegisterPending` (after `"`),
  `RegisterOpPending { register: char }` (after `"<reg>`), dispatching to a
  new `src/input/typestate/handlers/register.rs`.
- Scope: `y` (yank), `p`/`P` (paste after/before), and — beyond the original
  draft — `R` (replace-with-yanked) all honor an active register selection.
  Any other key cancels the pending register state (resolves the draft's
  open question about scope-beyond-yank-paste).
- Default/unnamed register behavior is unchanged: plain `y`/`p`/`P`/`R`
  read/write the unnamed slot exactly as before.

### Command-Line Mode

- New input-typestate state `CommandLinePending { buffer: String }`
  (`:` entry), dispatching to `src/input/typestate/handlers/command_line.rs`.
- New `CommandLine` type with a single variant, `Goto(usize)`
  (`src/helix/simulator/command_line.rs`).
- Accepted input: `:` followed by digits (goto line N), or `:g N` /
  `:goto N` as explicit aliases. Enter confirms and moves the cursor;
  Escape cancels with no document mutation; malformed input (non-digit,
  out-of-range) produces `UserError::CommandFailed`, intercepted at
  Enter-time and converted to a cancel rather than propagated as an error —
  explicitly to prevent an app-crashing failure mode found during review.
- `docs/HELIX_KEYBINDINGS.md` was updated with an explicit scope
  disclaimer: only `:goto`/`:g` is implemented; everything else in Helix's
  real command-line surface (`:s`, `:g` global, `:w`, `:sort`, ranges,
  `:normal`) is marked unimplemented.
- `:clear-register` and `:sort` were evaluated and explicitly dropped per
  the commit message as "not verifiable against the snapshot-based scenario
  completion model" — a scoping rationale the original draft did not
  anticipate needing.

> [!warning] Architectural Deviation from the Draft
> `EditorMode` (`src/helix/simulator/mode.rs`) was **not** extended with a
> new `CommandMode` variant, contrary to FR-005/NFR-002 in [[spec]].
> Command-line entry lives entirely in the input-typestate layer
> (`CommandLinePending`), not in the simulator's mode type. See
> [[SRS#FR-005]] and [[plan#Key Design Decisions]] for the practical
> consequence of this deviation.

## Target Users

Unchanged from the original draft: learners practicing intermediate/
advanced Helix workflows; content maintainers adding scenario coverage;
project maintainers evaluating parity against reference training projects.

## Functional Requirements

See [[SRS]] for the full FR-001..010 list with per-requirement verdicts.
Summary: FR-001..004 (registers) satisfied as drafted or better (`R`
added); FR-005 (command-line mode as an `EditorMode` variant) implemented
differently (input-typestate only); FR-006 (`:s` substitute) **not
satisfied — not implemented at all**; FR-007, FR-008, FR-009 satisfied;
FR-010 satisfied via a different mechanism (`normalize_command_id`, not a
`src/learning/` mapping table change).

## Non-Functional Requirements

See [[NFR]] for NFR-001..007, most satisfied; NFR-002 (command-line as an
`EditorMode` variant) not satisfied per the architectural deviation above.

## Scope & Boundaries

### In Scope (of what shipped)
- Named registers: select, yank, paste (after/before), replace
- Command-line mode: goto-line navigation only (`:N`, `:goto N`, `:g N`)
- New scenario categories: `scenarios/en/registers/` and a command-line goto
  scenario under `scenarios/en/movement/`
- `normalize_command_id()` for FSRS/quest command-taught tracking
- Defensive hardening bundled in: Esc now cancels pending register/
  command-line state in arcade mode; unmapped Alt/Ctrl keys no longer fall
  through as bare-char commands (a real security-relevant fix — previously
  e.g. Alt-z could execute as plain `z`)

### Out of Scope (confirmed still absent after this commit)

> [!danger] Explicit Gaps
> - `:s` substitute (search-and-replace) — the original US-002's actual
>   goal — was **not implemented**
> - `:g` as Helix's real global-command-with-subcommand — only a `:g N`
>   goto alias exists, not the real semantics
> - `:w`, `:sort`, line ranges, `:normal`, shell commands — never in scope,
>   still absent
> - A dedicated `EditorMode::CommandMode` variant on the simulator's mode
>   type — command-line state lives only in the input layer
> - Vim-style marks — permanently excluded per issue #104, unaffected by
>   this work

## Constraints & Assumptions

### Technical Constraints
`#![forbid(unsafe_code)]` preserved (verified — no `unsafe` in any file
touched). Multi-byte/non-ASCII register-character input required explicit
hardening (`chars()` vs `len()`/`.nth()`) to avoid panics — a correctness
class the original draft's NFRs did not anticipate.

### Business Constraints
Scope was narrowed during implementation from "at least `:s` substitute" to
"goto-line only," a judgment call made mid-build and documented afterward
rather than re-specified upfront.

### Assumptions

> [!warning] Assumptions
> - The `:s`/`:g`-global gap is assumed to be a legitimate follow-up
>   feature, not an abandoned requirement — `docs/HELIX_KEYBINDINGS.md`'s
>   explicit disclaimer suggests intent to revisit, not permanent rejection
>   (contrast with vim marks, which are permanently rejected)
> - `normalize_command_id()` collapsing all register letters into one
>   FSRS-tracked skill per operation family (`"y`, `"p`, `"P`, `"R`) is
>   assumed to be the correct granularity — the alternative (per-letter
>   tracking) was not chosen and is not currently planned

## Success Criteria

See [[SRS#3.2 Success Criteria]] for the metric-by-metric actual outcome.

## Open Questions

> [!question] Unresolved Items
> - [ ] Should `:s` substitute be scoped as a standalone follow-up feature,
>       given it was the original spec's actual minimum bar and remains
>       fully unimplemented?
> - [ ] Should `EditorMode` gain a real `CommandMode` variant (closing the
>       NFR-002 gap), or is the input-typestate-only approach considered
>       the accepted final architecture?
> - [ ] Should scenario coverage be expanded from 1 scenario/category to the
>       originally targeted ≥3/category spanning difficulty (SC-003)?

## Glossary

| Term | Definition |
|------|-----------|
| `RegisterFile` | `HashMap<char, String>` register storage, `src/helix/simulator/register_file.rs` |
| Unnamed register | The default register slot (`'"'`), used when no explicit `"<reg>` selection is active |
| `CommandLine::Goto(usize)` | The sole implemented command-line command variant, line-number navigation |
| `normalize_command_id` | Canonicalizes raw command strings (e.g. `"ay` → `"y`, `:g 3` → `:goto`) before they reach FSRS/quest tracking |

## See Also

- [[spec]] — original research spec and retroactive resolution summary
- [[SRS]] — functional requirements, marked with implementation verdicts
- [[NFR]] — non-functional requirements, verified against shipped code
- [[plan]] — as-built architecture and documented scope narrowing
- [[tasks]] — retroactive task breakdown
- [[constitution]] — project principles (Section I, input typestate pattern)
- `docs/HELIX_KEYBINDINGS.md` — user-facing scope disclaimer
