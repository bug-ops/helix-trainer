---
aliases:
  - Backtick Misparsed as Alt-Backtick
tags:
  - sdd
  - spec
  - bug-fix
  - input
  - status/implemented
created: 2026-08-09
status: implemented
related:
  - "[[constitution]]"
  - "[[../writing-markup-scenario-track/spec]]"
  - "[[MOC-specs]]"
---

# Feature: Bare Backtick Keypress Misparsed as Alt-Backtick

> [!info] Metadata
> **Source**: CI cycle (rust-live-tester finding), priority P2
> **Resolved by**: GitHub issue #377, merged via PR #380 (`45bbd1f`)
> **Status**: Implemented
> **Depth**: Lightweight spec, per this project's SDD scaling guidance for a single scoped bug fix. Migrated from an earlier untracked draft into the tracked `specs/` tree and reconciled against the shipped code; the original problem analysis is preserved below, and "Resolution (as shipped)" records what actually landed.

> [!note] Reading guide
> Sections 1-8 preserve the original pre-fix analysis (including line numbers and "currently" wording that describe the code before the fix). Section 9 records the shipped resolution and is authoritative for current behavior.

## 1. Overview

### Problem Statement

A single literal backtick keystroke (byte `0x60`, no modifier held) is consistently
parsed by the application as `Alt-`` ` `` instead of a plain `` ` ``. Both the `keys`
(canonical) and `typed` fields logged for the resulting `Message::ExecuteCommand`
show `"Alt-`"`, which means the phantom `Alt` modifier is present at or before the
app's own key-canonicalization layer — it is not merely a downstream keymap-mapping
quirk.

The Helix simulator binds a bare backtick to `switch_case`'s alternative form
(`pub const CMD_SWITCH_CASE_ALT: &str = "`";`, `src/helix/commands.rs:45`), and a
genuine `Alt-`` ` `` chord is separately bound to `switch_to_uppercase`
(`pub const CMD_SWITCH_TO_UPPERCASE: &str = "Alt-`";`, `src/helix/commands.rs:46`,
mapped in `src/input/typestate/key_mapping.rs:287`). Because a plain backtick keypress
is being reported as if `Alt` were held, it never reaches the `CMD_SWITCH_CASE_ALT`
binding (`src/input/typestate/key_mapping.rs:331`) — it is silently swallowed instead
(no command bound to that canonical key in the context where it lands), blocking
completion of any scenario whose documented solution requires typing a literal
backtick, whether as a Normal-mode command or as a literal character during Insert
mode.

Two scenarios in the training library are affected:

- `scenarios/en/editing/advanced-editing.toml`, scenario `switch_case_alt_001`
  ("Switch case with backtick") — solution is a single Normal-mode command,
  `commands = ["`"]`.
- `scenarios/en/writing/code-fence.toml`, scenario `writing_code_fence_001`
  ("Wrap a line in a code fence") — solution types a literal backtick three times per
  fence line while in Insert mode, `commands = ["O", "`", "`", "`", "Escape", "j", "o",
  "`", "`", "`", "Escape"]`.

### Goal

A bare backtick keystroke (no modifier held) is parsed and dispatched as canonical key
`` ` `` with no `Alt` modifier, in both Normal mode (triggering `CMD_SWITCH_CASE_ALT`)
and Insert mode (inserting a literal `` ` `` character), so that both
`switch_case_alt_001` and `writing_code_fence_001` can be completed with their
documented solutions, while a genuine `Alt-`` ` `` chord continues to trigger
`CMD_SWITCH_TO_UPPERCASE` without regression.

### Out of Scope

- Auditing every other punctuation key for the same class of misparse. Live testing
  already ruled this out for `~`, `!`, and `^` (all parsed correctly via the identical
  `tmux send-keys -l` method); backtick is the only character observed to trigger this.
  A broader audit is not requested here but may be filed as a separate follow-up if
  this fix reveals a systemic pattern.
- Redesigning the tmux-based scripted-TUI-testing playbook
  (a local, untracked testing playbook). If the root cause turns out to be specific
  to tmux/terminal emulation rather than the app, playbook updates are a possible
  follow-up but not part of this fix.
- Adding Kitty keyboard protocol / enhanced keyboard support as a general feature,
  unless it turns out to be the minimal correct fix for this specific defect.
- Verifying behavior on a real physical keyboard/terminal outside tmux (see
  Open Questions — flagged for follow-up validation, not blocking this fix).

## 2. User Stories

### US-001: Complete the backtick case-toggle scenario
AS A trainee working through the Editing track
I WANT pressing `` ` `` to toggle the case of the character/selection under the cursor
SO THAT I can complete the "Switch case with backtick" scenario as documented

**Acceptance criteria:**
```
GIVEN the Training-mode scenario "switch_case_alt_001" is active with cursor on 'M' in "const MAX = 100;"
WHEN the trainee presses a single, unmodified backtick key
THEN the canonical key dispatched is "`" (no Alt modifier)
AND the document updates to "const mAX = 100;" (switch_case_alt applied)
AND the scenario is scored as completed with optimal_count = 1
```

### US-002: Type literal backticks to build a Markdown fence
AS A trainee working through the Writing track
I WANT to type three literal backtick characters while in Insert mode
SO THAT I can wrap a code sample in a Markdown triple-backtick fence

**Acceptance criteria:**
```
GIVEN the Training-mode scenario "writing_code_fence_001" is active with the cursor on the first line of a markdown file
WHEN the trainee executes "O", then three unmodified backtick keypresses, then "Escape", "j", "o", three more unmodified backtick keypresses, then "Escape"
THEN each backtick keypress inserts a literal "`" character (no Alt modifier, no dropped keystroke)
AND the resulting document matches the scenario target (fence lines "```" above and below the code sample)
AND the scenario is scored as completed within tolerance = 1
```

### US-003: Genuine Alt-backtick chord still works
AS A trainee or developer using the uppercase-conversion command
I WANT an actual Alt+backtick key chord to keep triggering "switch to uppercase"
SO THAT fixing the bare-backtick misparse does not silently remove the existing Alt-` binding

**Acceptance criteria:**
```
GIVEN a scenario or interactive session where CMD_SWITCH_TO_UPPERCASE is bound to "Alt-`"
WHEN the user presses backtick while genuinely holding Alt (as opposed to a bare backtick keystroke)
THEN the canonical key dispatched is "Alt-`"
AND switch_to_uppercase executes as before
```

## 3. Functional Requirements

Use EARS notation. Prefix with FR-NNN.

| ID | Requirement | Priority |
|----|------------|----------|
| FR-001 | WHEN the user presses a bare backtick key with no modifier held THE SYSTEM SHALL canonicalize it to key `` ` `` with no `Alt` modifier | must |
| FR-002 | WHEN canonical key `` ` `` is dispatched in Normal or Select mode THE SYSTEM SHALL execute `CMD_SWITCH_CASE_ALT` | must |
| FR-003 | WHEN canonical key `` ` `` is dispatched in Insert mode THE SYSTEM SHALL insert a literal `` ` `` character at the cursor | must |
| FR-004 | WHEN the user presses backtick while genuinely holding the `Alt` modifier THE SYSTEM SHALL continue to canonicalize it as `Alt-`` ` `` and execute `CMD_SWITCH_TO_UPPERCASE` | must |
| FR-005 | WHERE the investigation determines the misparse originates in the crossterm/terminal event-parsing layer rather than app logic (per the finding's evidence — no app-level backtick-specific handling was found outside tests), THE fix SHALL be applied at the layer where the phantom modifier is actually introduced, not papered over downstream | must |
| FR-006 | WHEN a regression test suite is run THE SYSTEM SHALL include a test asserting that a bare `KeyEvent { code: Char('`'), modifiers: NONE }` canonicalizes to `` ` `` (not `Alt-`` ` ``) | must |
| FR-007 | WHEN `cargo nextest run scenario` is run THE SYSTEM SHALL keep validating `switch_case_alt_001` and `writing_code_fence_001` against their documented solutions | must |

## 4. Non-Functional Requirements

| ID | Category | Requirement |
|----|----------|-------------|
| NFR-001 | Compatibility | Fix must not regress the existing `Alt-`` ` ``  → `CMD_SWITCH_TO_UPPERCASE` binding or its existing test coverage (`src/input/typestate/tests/key_mapping_tests.rs:856`, `src/helix/simulator/tests/editing_tests.rs:176`) |
| NFR-002 | Testability | Root cause and fix must be verifiable both via `cargo nextest run --workspace --all-features --lib --bins` and via the project's documented tmux-based scripted-TUI-testing method, since that is the standard live-testing pipeline for this repo |
| NFR-003 | Portability | The fix must not assume a specific terminal emulator or multiplexer (e.g., must not special-case "running inside tmux"); it should correct backtick parsing generally, since the underlying cause may sit in the terminal input layer used regardless of tmux |

## 5. Data Model

Not applicable — this is a key-input parsing defect with no persistent entities.
For reference, the relevant in-memory constructs are:

| Entity | Description | Key Attributes |
|--------|-------------|----------------|
| `KeyEvent` (crossterm) | Raw terminal key event received by the event loop | `code: KeyCode`, `modifiers: KeyModifiers` |
| `CanonicalKeys` | App's normalized representation of a keystroke used for keymap lookup and debug logging | canonical string form, e.g. `` "`" ``, `"Alt-`"`, `"~"` |
| Keymap entry | Binding from a canonical key (in a given mode) to a Helix command constant | e.g. `CMD_SWITCH_CASE_ALT = "`"`, `CMD_SWITCH_TO_UPPERCASE = "Alt-`"` |

## 6. Edge Cases and Error Handling

| Scenario | Expected Behavior |
|----------|-------------------|
| Bare backtick pressed in Normal mode | Canonicalized to `` ` ``; executes `switch_case` (alternative form) |
| Bare backtick pressed in Insert mode | Canonicalized to `` ` ``; inserted as a literal character, not interpreted as a command |
| Genuine Alt+backtick chord pressed (real Alt held) | Canonicalized to `` Alt-` ``; executes `switch_to_uppercase` — must not regress |
| Backtick pressed as part of a multi-key sequence (e.g., after a `g`/`z`/`m` leader, or with a count prefix active) | Backtick is consumed by the pending leader/count context per existing typestate rules, unaffected by this fix; only the base single-key canonicalization changes |
| Backtick sent via `tmux send-keys -l` (literal mode) | Canonicalized to `` ` `` — this is the primary reproduction path documented in the finding and must pass after the fix |
| Backtick sent via raw byte injection bypassing `tmux send-keys` (`tmux load-buffer` + `paste-buffer`) | Canonicalized to `` ` `` — rules out a `send-keys`-specific quoting artifact; must also pass after the fix |
| Backtick pressed on a real physical keyboard/terminal outside tmux | Unconfirmed whether the bug reproduces here; see Open Questions |
| Other punctuation keys (`~`, `!`, `^`) | Already confirmed unaffected (no phantom modifier) — used as the regression baseline that this fix must not disturb |

## 7. Success Criteria

Measurable metrics that prove the feature works:

| ID | Metric | Target |
|----|--------|--------|
| SC-001 | `switch_case_alt_001` scenario completable via a single unmodified backtick keystroke in an automated/scripted regression check | Pass |
| SC-002 | `writing_code_fence_001` scenario completable via its documented solution sequence, including six literal backtick keystrokes in Insert mode | Pass |
| SC-003 | Existing test coverage for the genuine `Alt-`` ` `` → `switch_to_uppercase` binding continues to pass with zero regressions | 100% pass |
| SC-004 | New regression test explicitly covering `KeyEvent(Char('`'), NONE)` → canonical `` ` `` (not `Alt-`` ` ``) added and passing | Present and passing |
| SC-005 | `cargo nextest run scenario` passes for both affected scenario files | Pass |

## 8. Agent Boundaries

### Always (without asking)
- Reproduce the bug first (per repo triage protocol) before implementing a fix — ideally re-confirm via the tmux method documented in the project's local testing playbook
- Add/extend unit tests in `src/input/typestate/tests/key_mapping_tests.rs` (or the layer where the actual fix lands) covering bare backtick vs genuine Alt-backtick
- Run `cargo +nightly fmt --check`, `cargo clippy --all-targets --all-features --workspace -- -D warnings`, `cargo nextest run --workspace --all-features --lib --bins`, and `cargo nextest run scenario` before considering the fix complete
- Follow existing patterns in `src/input/typestate/key_mapping.rs` and the crossterm event-handling code in `src/input/handlers.rs`
- Update `CHANGELOG.md` under `[Unreleased]` once the fix lands

### Ask First
- Enabling the Kitty keyboard protocol or any crossterm terminal-capability negotiation change, if that turns out to be necessary — this affects behavior across all keys, not just backtick, and needs explicit sign-off
- Any change to the local tmux testing playbook or the tmux scripted-testing workflow itself
- Any change that would alter canonicalization behavior for keys other than backtick

### Never
- Reinterpret genuine `Alt-`` ` `` chords as plain backtick (this would silently remove the `switch_to_uppercase` binding)
- Special-case "when running inside tmux" as the fix location, without first confirming (per Open Questions) whether the bug is tmux-specific or general
- Modify scenario TOML solutions to work around the bug instead of fixing the root cause (e.g., do not change `switch_case_alt_001` or `writing_code_fence_001` to avoid using backtick)

## 9. Resolution (as shipped)

Merged in PR #380 (issue #377, commit `45bbd1f`); recorded in `CHANGELOG.md`.

- **Root cause**: app-level, not crossterm, tmux or terminal. `map_macos_composed_char` (`src/input/typestate/key_mapping.rs`), which rewrites macOS Option-key composition artifacts (e.g. `ç` -> Alt-c) into `(base char, ALT)`, had an arm that unconditionally rewrote every plain `` ` `` to Alt-`` ` ``. Unlike its other entries, backtick is also a directly-typeable ASCII key, so real unmodified keystrokes were rewritten. The function is used by both the live per-keystroke path and the custom-keymap overlay path (`src/input/keymap/keys.rs`, `src/config/keymap/parse.rs`).
- **Fix** (FR-001/FR-005): the backtick arm was removed, at the layer where the phantom modifier was introduced. A bare `` ` `` now resolves to `CMD_SWITCH_CASE_ALT` (FR-002) and a real Alt-`` ` `` still resolves to `CMD_SWITCH_TO_UPPERCASE` (FR-004).
- **Tests** (FR-006, NFR-001): `test_map_key_plain_backtick_resolves_to_switch_case_alt`, `test_map_key_alt_backtick_resolves_to_switch_to_uppercase`, `test_normalize_key_event_does_not_touch_plain_backtick` in `src/input/typestate/tests/key_mapping_tests.rs`. `switch_case_alt_001` and `writing_code_fence_001` stay covered by `cargo nextest run scenario` (FR-007).
- **Accepted trade-off (unverified)**: if a terminal ever delivers a bare backtick for a released Option-`` ` `` dead key, it now reads as plain `` ` `` (`switch_case_alt`) instead of the former `switch_to_uppercase` guess. The same class of collision was later fixed for `ç`/`ß` (#381) and tracked for other table entries (#382, #386).

## 10. Resolved Questions

- **Does it reproduce on a real terminal outside tmux?** Not verified. The root cause lives in app code that runs for any terminal, so the fix does not depend on the answer, but no physical-keyboard check is recorded.
- **Which layer?** The app's own `map_macos_composed_char` table, not crossterm, tmux `extended-keys`, or macOS Terminal byte sequences.
- **Mitigation if outside repo control?** Not applicable; the cause was inside the repository.

## 11. See Also

- [[MOC-specs]] — all specifications
- `src/helix/commands.rs` — `CMD_SWITCH_CASE_ALT`, `CMD_SWITCH_TO_UPPERCASE` constants
- `src/input/typestate/key_mapping.rs` — `map_macos_composed_char` (root cause) and the canonical-key-to-command mapping
- `scenarios/en/editing/advanced-editing.toml` — `switch_case_alt_001` scenario
- `scenarios/en/writing/code-fence.toml` — `writing_code_fence_001` scenario
- The project's local tmux-based scripted-TUI-testing playbook (not tracked) — method used to reproduce this bug
