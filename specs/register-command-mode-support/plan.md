---
aliases:
  - Register and Command-Line Mode Plan
tags:
  - sdd
  - plan
  - helix-simulator
  - input-system
  - status/implemented
created: 2026-08-09
status: implemented
related:
  - "[[spec]]"
  - "[[BRD]]"
  - "[[constitution]]"
---

# Technical Plan: Named Register and Command-Line Mode Support (As-Built)

> [!info] References
> **Spec**: [[spec]]
> This document is retroactive — it describes the architecture actually
> implemented in commit `1ba668d`, including the deliberate deviation from
> the drafted `EditorMode` design and the deliberate scope narrowing of
> command-line mode to goto-only.

## 1. Architecture

### Approach

Both capabilities were built as new states in the existing typestate input
dispatcher (`src/input/typestate/`), following the pattern already
established by `GotoPending`/`ViewPending`/`MatchPending`. Named registers
additionally required a simulator-level data structure change (`clipboard:
Option<String>` → `registers: RegisterFile`); command-line mode did not
touch the simulator's mode type at all — it resolves entirely at the input
layer into a single `CommandLine::Goto(usize)` action applied directly to
the document, without an intervening `EditorMode::CommandMode` state.

### Component Diagram

```mermaid
graph TD
    subgraph "Input Layer (src/input/typestate/)"
        A["\" key"] --> B[RegisterPending]
        B -->|reg char| C["RegisterOpPending{register}"]
        C -->|y/p/P/R| D[HelixSimulator command]
        C -->|other key| E[Cancel -> BaseState]
        F[": key"] --> G["CommandLinePending{buffer}"]
        G -->|digit| G
        G -->|Enter, valid| H[CommandLine::Goto applied]
        G -->|Enter, invalid| E
        G -->|Escape| E
    end
    subgraph "Simulator Layer (src/helix/simulator/)"
        D --> I[RegisterFile: HashMap-char,String-]
        H --> J[Document cursor moved]
    end
    subgraph "EditorMode (unchanged)"
        K[NormalMode] 
        L[InsertMode]
    end
```

### Key Design Decisions

| Decision | Choice | Rationale (reconstructed) | Alternatives Considered (per draft) |
|----------|--------|-----------|------------------------|
| Where command-line state lives | Input-typestate layer only (`CommandLinePending`), not `EditorMode` | Faster to ship, avoids touching the sealed-trait `EditorMode` machinery and its call sites throughout the simulator; the behavior (a distinct, reachable prompt) is achievable without it | A new `EditorMode::CommandMode` sealed-trait variant, as drafted in FR-005/NFR-002 — not taken |
| Command-line surface | `:goto N` / `:g N` alias only | `:s` substitute, `:g` global, `:sort`, `:clear-register` evaluated and explicitly dropped as "not verifiable against the snapshot-based scenario completion model" (from the commit message) — a completion-detection constraint the draft did not anticipate | Full `:s/pattern/replacement/` as originally targeted in FR-006/US-002 — not taken |
| Register storage | Rename+generalize `clipboard: Option<String>` → `registers: RegisterFile` (`HashMap<char, String>`) | Additive without duplicating structure; single source of truth for both unnamed and named slots | A separate parallel register map alongside the untouched `clipboard` field — rejected, would violate FR-004's "no regression" requirement more riskily |
| FSRS command-taught mapping | New `normalize_command_id()` canonicalization function at 3 call sites, rather than a `src/learning/` mapping table change | Keeps the normalization concern in `src/helix/commands.rs` (where the raw command-string format is defined) rather than duplicating knowledge of register/command-line syntax inside `src/learning/` | A `src/learning/`-side lookup table keyed on raw command strings — not taken |

## 2. Project Structure

```
src/helix/
├── simulator/
│   ├── register_file.rs        # NEW — RegisterFile, UNNAMED_REGISTER
│   ├── command_line.rs         # NEW — CommandLine::Goto(usize), parse()
│   ├── mode.rs                 # UNCHANGED — still NormalMode/InsertMode only
│   ├── mod.rs                  # HelixSimulator.clipboard -> .registers: RegisterFile
│   └── commands/
│       ├── clipboard.rs        # yank_to_register/paste now register-aware
│       └── mod.rs               # dispatch tests for register ops
└── commands.rs                 # normalize_command_id(), CMD_SELECT_REGISTER, CMD_COMMAND_LINE

src/input/typestate/
├── input_state.rs              # + RegisterPending, RegisterOpPending{register}, CommandLinePending{buffer}
├── state_types.rs              # + marker structs, HandlerState/Sealed impls
├── state_machine.rs            # + RegisterPreview enum (UI feedback)
├── handlers/
│   ├── base.rs                 # " -> RegisterPending; existing Esc/Alt/Ctrl hardening
│   ├── register.rs             # NEW — RegisterOpPending dispatch, y/p/P/R handling
│   └── command_line.rs         # NEW — CommandLinePending dispatch, parse/execute/cancel

src/security.rs                 # + limits::MAX_COMMAND_LINE_LEN = 256

src/config/scenarios.rs         # + ScenarioCategory::Registers

scenarios/en/
├── registers/
│   └── named-registers.toml    # NEW — named_register_yank_paste_001
└── movement/
    └── command-line-goto.toml  # NEW — command_line_goto_001

docs/HELIX_KEYBINDINGS.md       # updated with explicit scope disclaimer
```

## 3. Data Model

```rust
// src/helix/simulator/register_file.rs (as-built, illustrative)
pub const UNNAMED_REGISTER: char = '"';

pub struct RegisterFile {
    registers: std::collections::HashMap<char, String>,
}

impl RegisterFile {
    pub fn write(&mut self, register: Option<char>, content: String);
    pub fn read(&self, register: Option<char>) -> Option<&str>;
}

// src/helix/simulator/command_line.rs (as-built, illustrative)
pub enum CommandLine {
    Goto(usize),
}

impl CommandLine {
    pub fn parse(input: &str) -> Result<Self, UserError>;
}
```

```rust
// src/input/typestate/input_state.rs (as-built, illustrative)
pub enum InputState {
    // ...existing variants (GotoPending, ViewPending, MatchPending, ...)
    RegisterPending,
    RegisterOpPending { register: char },
    CommandLinePending { buffer: String },
}
```

### Migrations

None — in-memory simulator state only; `RegisterFile` is not persisted to
`profile.json` or `config.json`.

## 4. API Design

Not applicable — no HTTP/external API. Internal command dispatch:
`HelixSimulator::execute_command` gained no new signature; register/
command-line operations resolve to existing clipboard/cursor-movement
primitives underneath.

## 5. Integration Points

| System | Direction | Notes |
|--------|-----------|-------|
| `src/learning/` (FSRS) | inbound | Consumes `normalize_command_id()`-canonicalized command strings for command-taught tracking; no changes to `src/learning/` itself |
| `src/ui/state/handlers/quests.rs` | inbound | Same normalization call site, for daily-quest command tracking |
| `tests/scenario_validation.rs` | inbound | Validates new scenario TOML via the unchanged existing harness |
| `docs/HELIX_KEYBINDINGS.md` | outbound (docs) | Updated to disclose the goto-only scope to users/contributors |

## 6. Security

- Authentication/Authorization: not applicable (single-user local TUI).
- Input validation: `CommandLine::parse` rejects non-digit and
  out-of-range input via `UserError::CommandFailed`; register-name
  characters validated as part of the `RegisterOpPending` transition.
- New bound: `security::limits::MAX_COMMAND_LINE_LEN = 256` caps
  command-line buffer growth.
- Sensitive data: not applicable — registers hold only in-session buffer
  text, never persisted or exposed externally.
- Hardening found and fixed alongside (not originally scoped): multi-byte
  register-char panics; Alt/Ctrl modifier keys falling through as bare-char
  commands.

## 7. Testing Strategy

| Level | Framework | What Is Tested | Coverage |
|-------|-----------|-----------------|----------|
| Unit | `#[test]` | `RegisterFile` read/write/unnamed-default (5), `CommandLine::parse`/execute/sanitization (13), `keytrie` multi-byte regressions (6), register handler dispatch (6), command-line handler dispatch (12), clipboard dispatch (6) | `src/helix/simulator/`, `src/input/typestate/` |
| Integration | `#[test]` in `tests/ui_multi_key_commands.rs` | End-to-end register/goto key sequences through the full input stack (+159 lines) | Cross-module |
| Scenario | `cargo nextest run scenario` | `named-registers.toml`, `command-line-goto.toml` complete via their declared `solution` | Content-level |
| Regression | `#[test]` | FSRS-normalization regression in `minigame/session.rs`; arcade Esc-routing (3); state-machine boundary-cancel (2) | Cross-cutting |
| **Gap** | — | No property-based (`proptest`) coverage was added for `CommandLine::parse` or register-name validation, despite both being pure, deterministic parsing functions well-suited to it | `src/helix/simulator/command_line.rs`, `register_file.rs` |

## 8. Performance Considerations

No measurable impact — synchronous, in-process state-machine dispatch on
keypress, same cost class as every other typestate transition already in
the input pipeline.

## 9. Rollout Plan

Already shipped as part of commit `1ba668d` (2026-08-09), merged via PR
#329, closing issue #282. No feature flag — immediately active for all
users on merge. `docs/HELIX_KEYBINDINGS.md`'s scope disclaimer serves as
the user-facing rollout communication for the goto-only narrowing.

## 10. Constitution Compliance

| Principle | Status | Notes |
|-----------|--------|-------|
| I. Architecture — input routed through `HelixSimulator`/`AnyModeSimulator`, typestate pattern for pending states | Compliant for registers; partially compliant for command-line (typestate used, but `EditorMode` itself not extended) | See [[NFR#NFR-002]] |
| IV. Code Style — `#![forbid(unsafe_code)]` preserved | Compliant | Verified via grep |
| IV. Code Style — doc comments on new `pub` items | Compliant | rustdoc CI gate would have blocked merge otherwise |
| V. Security — input validation via `UserError`/`SecurityError` | Compliant | `CommandLine::parse`, register-name validation |
| VII. Simplicity — extend existing subsystem rather than introduce a parallel one | Compliant | `RegisterFile` replaces/generalizes `clipboard`, not duplicates it |

## Documented Scope Narrowing

`docs/HELIX_KEYBINDINGS.md` was updated with an explicit disclaimer row
("❌ everything else") stating only `:goto`/`:g` is implemented in the
command-line surface. This is the project's own record of the FR-006/
SC-002 gap, written at implementation time rather than discovered
retroactively by this SDD pass.

## 11. Risks and Mitigations

| Risk | Impact | Probability | Mitigation |
|------|--------|-------------|------------|
| A future contributor assumes `:s` substitute exists because "command-line mode" shipped | Wasted investigation time, or a duplicate/conflicting implementation attempt | Medium | `docs/HELIX_KEYBINDINGS.md` disclaimer + this record's explicit FR-006 "not implemented" verdict |
| `EditorMode` never gaining a real `CommandMode` variant becomes permanent architectural drift | Command-line handling diverges further from the simulator's own mode model as more commands are added later | Low-Medium | [[BRD#Open Questions]] flags this for a maintainer decision before any `:s`/`:g` follow-up is scoped |
| No proptest coverage for `CommandLine::parse` despite it being pure/deterministic | Parsing edge cases (malformed digit sequences, boundary line numbers) undetected by example-based tests alone | Low | Flagged in Testing Strategy gap above; follow-up candidate, not scheduled |

## See Also

- [[spec]] — original research spec
- [[BRD]] — business rationale and what shipped
- [[SRS]] — FR-by-FR verdict
- [[tasks]] — retroactive task breakdown
- [[constitution]] — project principles, Section I (Architecture)
