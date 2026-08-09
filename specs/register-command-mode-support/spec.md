---
aliases:
  - Named Registers and Command-Line Mode
  - Register + Goto-Mode Support
tags:
  - sdd
  - spec
  - helix-simulator
  - input-system
  - status/implemented
created: 2026-08-08
status: implemented
related:
  - "[[constitution]]"
  - "[[BRD]]"
  - "[[SRS]]"
  - "[[NFR]]"
  - "[[plan]]"
  - "[[tasks]]"
  - "[[MOC-specs]]"
---

# Feature: Named Register and Command-Line (`:`) Mode Support

> [!info] Metadata
> **Author**: rust-agents:sdd (research spec, from competitive-parity finding)
> **Resolved by**: commit `1ba668d` — "feat(helix-core): add named registers
> and command-line goto mode (#329)", closing issues #282 (implementation)
> **Status**: Implemented, narrower scope than drafted — see [[SRS]] for the
> FR-by-FR verdict. This document is retroactive: preserved below as the
> original research spec, with a Resolution section appended describing
> what actually shipped.

## 1. Overview

### Problem Statement

A competitive-parity research scan compared helix-trainer against reference
Vim/Helix training projects, including
[S-Sigdel/vimhjkl](https://github.com/S-Sigdel/vimhjkl) (524 stars, actively
maintained, terminal-based spaced-repetition Vim trainer, 66 skills / 230
challenges). vimhjkl's curriculum explicitly drills "registers, marks,
macros, `:g` / `:normal` / ranges, regex and substitution" as core skill
categories.

Cross-checking against helix-trainer's own simulator (`src/helix/`) found
two real, documented [Helix keymap](https://docs.helix-editor.com/keymap.html)
features with zero simulator support and zero scenario coverage:

1. **Named registers** — `"` `<reg>` = `select_register` (select a register
   to yank to or paste from). `HelixSimulator<M>` had only a single
   `clipboard: Option<String>` field, no register map keyed by name.
2. **Command-line / ex mode (`:`)** — documented under *Minor modes*,
   supporting `:s` substitute, `:g` global, ranges, `:w`, `:sort`, etc.
   `EditorMode` defined only `NormalMode`/`InsertMode` — no command-line
   mode existed at all.

**Why it matters**: named registers (multi-slot yank/paste) and command-line
commands are among the most commonly used Helix command categories in daily
editing. Their total absence meant helix-trainer taught a strict subset of
Helix's real command surface compared to reference projects drilling the
equivalent Vim concepts.

### Goal

Define named-register support (yank to / paste from a register selected via
`"<reg>`) and a scoped command-line mode as first-class additions to
`HelixSimulator` and the typestate input system, with at least one new
scenario category demonstrating each.

### Out of Scope

> [!danger] Explicit Exclusion — Vim-Style Marks
> Vim-style marks (backtick/apostrophe jump-to-mark) are **explicitly and
> permanently out of scope**. helix-trainer already investigated and
> rejected this in closed issue #104: "Original plan had 'marks (m, '')'
> which is a VIM feature, NOT Helix! In Helix, `m` enters Match Mode for
> brackets/surround/textobjects." This decision is correct and final.

Additional exclusions in the original draft (some became permanent, not
just deferred — see Resolution below):

- Full command-line surface (`:g` global, `:w` write, `:sort`, arbitrary
  line ranges, shell commands, `:normal`)
- Register content inspection UI
- Macro recording/playback (tracked separately under issue #198)
- Scroll commands, view mode extensions, selection-regex (tracked under
  issue #198)

## 2. User Stories

### US-001: Yank and paste using a named register
AS A helix-trainer user practicing intermediate/advanced Helix workflows
I WANT to select a named register with `"<reg>` before a yank or paste
operation
SO THAT I can hold multiple pieces of text simultaneously and learn Helix's
real multi-register workflow.

**Status**: satisfied as drafted, plus `R` (replace-with-yanked) additionally
scoped to registers. See [[SRS#FR-001]]..[[SRS#FR-004]].

### US-002: Substitute text via command-line mode
AS A helix-trainer user practicing intermediate/advanced Helix workflows
I WANT to enter command-line mode with `:` and run a substitute command
SO THAT I can learn Helix's search-and-replace workflow.

**Status**: **not satisfied as drafted.** No `:s/pattern/replacement/`
substitute was implemented. What shipped instead is `:goto N` / `:g N`
(line-number navigation) — see [[BRD#What Was Implemented]] and
[[SRS#FR-006]].

### US-003: Scenario coverage for both new capabilities
AS A helix-trainer content maintainer
I WANT dedicated scenario categories exercising named registers and
command-line mode
SO THAT the FSRS scheduler can teach and drill these commands.

**Status**: satisfied at minimum viable coverage (1 scenario per category,
not the ≥3-per-category-spanning-difficulty originally targeted by SC-003).

## 3. Functional Requirements

Use EARS notation. Prefix with FR-NNN. See [[SRS]] for the actual verdict of
each.

| ID | Requirement | Priority |
|----|------------|----------|
| FR-001 | WHEN the user presses `"` followed by a register name character in Normal mode THE SYSTEM SHALL enter a register-selection pending state analogous to `GotoPending`/`ViewPending`/`MatchPending` | must |
| FR-002 | WHEN a register has been selected via `"<reg>` and the user subsequently presses `y` THE SYSTEM SHALL yank the current selection into that named register | must |
| FR-003 | WHEN a register has been selected via `"<reg>` and the user subsequently presses `p` or `P` THE SYSTEM SHALL paste the content of that named register after/before the cursor respectively | must |
| FR-004 | WHEN no register is explicitly selected THE SYSTEM SHALL preserve existing default-clipboard yank/paste behavior unchanged | must |
| FR-005 | WHEN the user presses `:` in Normal mode THE SYSTEM SHALL enter a new command-line/prompt mode distinct from `NormalMode` and `InsertMode` | must |
| FR-006 | WHEN the system is in command-line mode and the user types a valid `:s/pattern/replacement/` expression and confirms (Enter) THE SYSTEM SHALL apply the substitution and return to Normal mode | must |
| FR-007 | WHEN the system is in command-line mode and the user presses Escape THE SYSTEM SHALL discard the pending input and return to Normal mode without modifying the document | must |
| FR-008 | WHEN the system is in command-line mode and the user types an unrecognized or malformed command THE SYSTEM SHALL surface a `UserError` rather than panicking or silently no-op-ing | must |
| FR-009 | WHERE new scenario categories are added THE SYSTEM SHALL validate them via `cargo nextest run scenario` with no changes to the validation harness itself | should |
| FR-010 | WHEN a scenario's `metadata.commands_taught` references a register-select or command-line command THE SYSTEM SHALL recognize it as a valid taught command for FSRS tracking | should |

## 4. Non-Functional Requirements

| ID | Category | Requirement |
|----|----------|-------------|
| NFR-001 | Architectural consistency | New register-selection and command-line states MUST use the existing typestate pattern in `src/input/typestate/` |
| NFR-002 | Architectural consistency | Command-line mode MUST be added to `EditorMode` (`src/helix/simulator/mode.rs`) following the sealed-trait typestate pattern |
| NFR-003 | Scope discipline | Vim-style marks MUST NOT be introduced under any name — see issue #104 |
| NFR-004 | Safety | `#![forbid(unsafe_code)]` MUST be preserved |
| NFR-005 | Testability | Both capabilities MUST be covered by scenario TOML fixtures plus unit tests |
| NFR-006 | Error handling | All new user-facing failure modes MUST route through `UserError`/`SecurityError` |
| NFR-007 | Documentation | All new `pub` types/functions MUST carry `///` doc comments |

## 5. Data Model

Described at the WHAT level in the original draft; see [[plan]] for the
concrete as-built types.

| Entity | Description |
|--------|-------------|
| Named Register | A named storage slot for yanked/deleted text, addressable via `"<reg>` |
| Register Selection (pending input state) | Transient typestate marker capturing that the next operation targets a specific register |
| Command-Line Session | Transient state representing an open `:` prompt |
| Substitute Command | A parsed `:s/pattern/replacement/[flags]` expression — **not implemented, see Resolution** |

## 6. Edge Cases and Error Handling

| Scenario | Expected Behavior | Actual (as shipped) |
|----------|-------------------|----------------------|
| User presses `"` then an invalid register-name character | `UserError`, return to `BaseState` | Matches — see [[SRS#FR-001]] |
| User presses `"<reg>` then a key that is neither `y`, `p`, nor `P` | `[NEEDS CLARIFICATION]` in draft | Resolved: also allows `R`; anything else cancels the pending register state |
| User enters command-line mode via `:` then presses `:` again mid-input | Treated as literal input unless a recognized control key | Not applicable — command-line only accepts digits for `:goto`; non-digit input is rejected as malformed |
| `:s/pattern/replacement/` with invalid regex | `UserError` describing regex failure | Not applicable — `:s` was never implemented |
| `:s` invoked with no active selection/range | `[NEEDS CLARIFICATION]` in draft | Moot — `:s` was never implemented |
| Named register read via `"<reg>p` before anything yanked to it | No-op, no panic, mirrors default clipboard | Matches |
| Scenario TOML references an unrecognized register/command-line command | `cargo nextest run scenario` MUST fail loudly | Matches — validation harness unchanged, still fails loudly on unmapped commands |

## 7. Success Criteria

| ID | Metric | Target | Actual |
|----|--------|--------|--------|
| SC-001 | Named-register yank/paste round-trip correctness | 100% pass, no regression | Met |
| SC-002 | Command-line `:s` substitute correctness | 100% pass for `:s/pattern/replacement/` | **Not met — `:s` never implemented; `:goto`/`:g` implemented instead, with its own passing tests** |
| SC-003 | ≥1 scenario category per capability, ≥3 scenarios/category spanning difficulty | ≥3 per category | **Partially met — 1 category + 1 scenario per capability, single difficulty level** |
| SC-004 | No architectural drift — new states follow the typestate pattern | Code review confirms | Met for register states; **not met for command-line, which lives in input-typestate only, not `EditorMode`** — see [[SRS#FR-005]] |
| SC-005 | No scope creep into vim marks | Code review confirms | Met |

## 8. Agent Boundaries

### Always (without asking)
- Run `cargo nextest run --workspace --all-features --lib --bins` and
  `cargo nextest run scenario` after any implementation change here
- Follow the existing typestate pattern in `src/input/typestate/`
- Add doc comments (`///`) to all new `pub` items

### Ask First
- Implementing `:s` substitute or any command-line command beyond
  `:goto`/`:g` (see [[BRD#What Was Implemented]] — this is now a genuinely
  new, unscoped follow-up, not part of what shipped)
- Any change to the existing default/unnamed clipboard behavior

### Never
- Introduce vim-style marks under any name — permanently rejected per issue #104
- Silently swallow malformed command-line input or invalid register names

## 9. Open Questions

All five `[NEEDS CLARIFICATION]` items from the original draft were
implicitly resolved by the implementation — see [[SRS#5.1 Traceability Matrix]]
for how. One new question is introduced by the resolution:

- [NEEDS CLARIFICATION: should `:s` substitute (and `:g` global-with-subcommand,
  as distinct from the `:g N` goto alias that shipped) be scoped as its own
  follow-up feature, given the original US-002/FR-006 were not satisfied?
  `docs/HELIX_KEYBINDINGS.md` already documents this narrowing explicitly to
  users — see [[plan#Documented Scope Narrowing]].]

## 10. Resolution (Retroactive)

Implemented by commit `1ba668d` (2026-08-09), closing issue #282. Named
registers shipped essentially as drafted (FR-001..004), plus `R` support.
Command-line mode shipped **narrower than drafted**: only `:goto N` (line
navigation, alias `:g N`) exists — no `:s` substitute, no `:g` global
command, no ranges. This was an explicit, documented scoping decision (see
`docs/HELIX_KEYBINDINGS.md`'s scope disclaimer), not an oversight. Full
verdict in [[SRS]].

## 11. See Also

- [[constitution]] — project principles
- [[BRD]] — business rationale and what actually shipped
- [[SRS]] — FR-by-FR verdict
- [[NFR]] — non-functional requirement verdicts
- [[plan]] — as-built architecture
- [[tasks]] — retroactive task breakdown
- [[MOC-specs]] — all specifications
- [Helix Keymap Reference](https://docs.helix-editor.com/keymap.html)
- [Helix Command-Line Reference](https://docs.helix-editor.com/command-line.html)
- [S-Sigdel/vimhjkl](https://github.com/S-Sigdel/vimhjkl) — reference project
- Issue #198 — Helix command content-coverage gap (macros, scroll, view mode, selection-regex — distinct, unaddressed)
- Issue #104 — vim-style marks explicitly rejected (authoritative prior art)
