---
aliases:
  - Custom Helix Keymap Support
  - use_helix_keymap
tags:
  - sdd
  - spec
  - input-system
  - config
  - status/implemented
created: 2026-08-10
status: implemented
related:
  - "[[constitution]]"
  - "[[MOC-specs]]"
---

# Feature: Custom Helix Keymap Remapping (`use_helix_keymap`)

> [!info] Metadata
> **Resolved by**: commit `999e63e` — "feat(input): support custom Helix
> keymap remapping (#350)", closing issue #163. Follow-up fixes: `463d363`
> (applied-count/fingerprint tracking correctness, #347/#348), `d7003ca`
> (docs: workaround for physically-unreachable bindings on some
> terminals).
> **Status**: Implemented
> **Depth**: Lightweight spec, per this project's SDD scaling guidance —
> retroactively documented because this shipped with no spec anywhere in
> the repository, despite being a substantial, multi-file addition
> (`src/input/keymap/` — `context.rs`, `keys.rs`, `overlay.rs`, `mod.rs`).

## 1. Overview

### Problem Statement

helix-trainer only ever taught the stock Helix keymap. Users who run Helix
with a customized keymap (non-default keyboard layouts, remapped movement
keys) had no way to train against the bindings they actually use day to
day — the trainer's drilled muscle memory would not transfer to their real
editor configuration.

A prerequisite surfaced during investigation: `CommandMetadata.name`
(`src/helix/commands.rs`) held ~35-40 values that did not match real Helix
25.07.1 command identifiers, with no production consumer to catch the
drift. This had to be corrected first (mechanical, behavior-preserving) so
that `CommandMetadata.name` could become the authoritative key a user's
`config.toml` command name resolves against.

### Goal

A user can opt in to training against their own Helix `config.toml`
`[keys.normal]` remaps (including nested `g`/`m`/`z`/`[`/`]` minor-mode
tables) instead of the stock keymap, scoped to gameplay input only, with
review history (FSRS card ids) unaffected by whether the remap is active.

### Out of Scope

- Menu, results, filter screens, and scenario hint prose — these always
  use the stock `j`/`k`/`gg`/`G` navigation, documented in the README,
  since remapping trainer UI chrome would break the canonical command
  identity FSRS review history depends on
- Insert-mode or select-mode remap tables — only `[keys.normal]` (and its
  nested minor-mode tables) is parsed
- Relocating a command across prefixes (e.g. moving a `g`-prefixed command
  to a `z`-prefix) — reported as `UnsupportedRelocation`, not applied
- Key-sequence, `@`-macro, or `:`-typable binding values — reported as
  `UnsupportedBindingForm`, not applied

## 2. User Stories

### US-001: Train against my real keymap
AS A Helix user with a customized keymap
I WANT to opt in to training against the bindings I actually use
SO THAT the muscle memory I build in the trainer transfers directly to my
real editor, instead of teaching me keys I've remapped away from

**Acceptance criteria:**
```
GIVEN AppConfig.use_helix_keymap = true and a valid
  ~/.config/helix/config.toml with a [keys.normal] remap
WHEN  the trainer starts and the player is on the gameplay input path
THEN  physical keys resolve through the parsed remap to canonical Helix
      command keys before reaching command dispatch
```

### US-002: Unsupported bindings never block startup
AS A helix-trainer user with an unusual or partially-unsupported custom
keymap
I WANT the trainer to start normally and tell me what it couldn't apply
SO THAT a malformed or advanced config never locks me out of the app

**Acceptance criteria:**
```
GIVEN a config.toml with some bindings this trainer cannot model
  (unknown command name, key-sequence value, minor-mode relocation, etc.)
WHEN  the trainer starts
THEN  it starts normally, applies every binding it can, and surfaces the
      rest via a quantitative startup notification plus full tracing detail
```

### US-003: Review history survives toggling the keymap
AS A helix-trainer user who enables or disables the custom keymap over time
I WANT my FSRS review history to remain valid either way
SO THAT switching keymaps doesn't reset my learning progress

**Acceptance criteria:**
```
GIVEN a scenario was solved under one keymap mapping
WHEN  the keymap is toggled on/off or changed to a different mapping
THEN  FSRS card ids, quest matching, and scenario solutions (all keyed on
      canonical command strings) are unaffected — only the physical-key
      translation layer changes
```

## 3. Functional Requirements

Use EARS notation. Prefix with FR-NNN.

| ID | Requirement | Priority |
|----|------------|----------|
| FR-001 | WHERE `AppConfig.use_helix_keymap` is `true` (default `false`) THE SYSTEM SHALL load and parse the user's `~/.config/helix/config.toml` `[keys.normal]` section, including nested `g`/`m`/`z`/`[`/`]` minor-mode tables | must |
| FR-002 | WHEN a physical `KeyEvent` arrives at the gameplay input boundary (`handle_gameplay_input`) THE SYSTEM SHALL translate it to canonical Helix key(s) once, via a normalized `PhysicalKey` newtype as the sole lookup key | must |
| FR-003 | WHEN a remap resolves one physical key to a multi-token canonical-key expansion (e.g. a remapped `G` → `"ge"`) THE SYSTEM SHALL apply it atomically (clone-and-commit-or-discard) and only from the `Base` input state — a multi-token expansion looked up from a pending state (count or register prefix) SHALL fall through to the stock key instead of executing | must |
| FR-004 | WHEN loading the config THE SYSTEM SHALL validate one-key-to-many-canonical-key expansions at load time, not at first use | must |
| FR-005 | WHEN a binding cannot be modeled (unknown command name, unparsable key, non-command-name value form, unsupported minor-mode table, or cross-prefix relocation) THE SYSTEM SHALL report it via `KeymapWarningReason` and a startup notification, and SHALL NOT block startup or silently drop it | must |
| FR-006 | WHEN two distinct TOML keys normalize to the same `(context, physical key)` pair THE SYSTEM SHALL detect the collision at insert time (via a `HashMap`-backed `Resolution`) and report it as `KeymapWarningReason::Shadowed`, rather than silently letting one binding shadow the other while still counting both as applied | must |
| FR-007 | WHEN gameplay input is dispatched downstream of the physical-key translation (registry dispatch, FSRS card ids, quest matching, scenario solutions) THE SYSTEM SHALL key everything on canonical command strings, unaffected by whether a custom keymap is active | must |
| FR-008 | WHEN the active keymap mapping changes (enabled, disabled, or a different custom mapping loaded) THE SYSTEM SHALL update `UserProfile.keymap_fingerprint` so a mismatch across a review history is detectable, regardless of whether the *current* keymap is the stock one | must |
| FR-009 | WHERE menu, results, filter screens, or scenario hint prose are rendered THE SYSTEM SHALL always use the stock `j`/`k`/`gg`/`G` navigation, never the custom keymap | must |
| FR-010 | WHEN `AppConfig`/`ConfigData` gains a new field THE SYSTEM SHALL default it via `#[serde(default)]` so an existing `config.json` is merged with defaults rather than discarded wholesale | must |

## 4. Non-Functional Requirements

| ID | Category | Requirement |
|----|----------|-------------|
| NFR-001 | Architectural consistency | New pending-input handling for remap expansion MUST follow the existing typestate pattern in `src/input/typestate/`; no parallel ad-hoc state tracking |
| NFR-002 | Safety | `#![forbid(unsafe_code)]` MUST be preserved; a malformed, missing, or oversized `config.toml` MUST fall back to the stock keymap without blocking startup |
| NFR-003 | Performance | Physical-key lookup on the gameplay hot path MUST NOT allocate for the common single-token case with no active remap (see `CanonicalKeys::is_single_token()`, added in the follow-up perf fix `8e0b6ef`) |
| NFR-004 | Testability | Load-time validation, atomic apply, and every `KeymapWarningReason` variant MUST have dedicated unit coverage |
| NFR-005 | Backward compatibility | Adding `use_helix_keymap` (or any future `AppConfig`/`ConfigData` field) MUST NOT discard an existing `config.json`'s other fields (FR-010) |

## 5. Data Model

| Entity | Description |
|--------|-------------|
| `AppConfig.use_helix_keymap: bool` | Opt-in flag, default `false` |
| `PhysicalKey` (`src/input/keymap/keys.rs`) | Normalized newtype identifying a physical keystroke; sole lookup key for remap resolution |
| `KeymapOverlay` (`src/input/keymap/overlay.rs`) | `HashMap`-backed `(context, PhysicalKey) -> canonical key(s)` resolution built from parsed config entries |
| `KeymapWarningReason` (`src/config/keymap/parse.rs`) | `UnknownCommand`, `UnparsableKey`, `UnsupportedBindingForm`, `UnsupportedMinorMode`, `UnsupportedRelocation`, `Shadowed`, plus a resolve-time replay-mismatch variant — every unsupported binding is one of these, never silently dropped |
| `UserProfile.keymap_fingerprint: Option<u64>` | Detects when review history spans more than one binding mapping, including transitions to/from the stock keymap |

No changes to `Scenario`/`Setup`/`TargetState` — this feature only affects
the physical-key-to-canonical-key translation layer, not scenario content.

## 6. Edge Cases and Error Handling

| Scenario | Expected Behavior |
|----------|-------------------|
| `config.toml` missing or unreadable | Falls back to stock keymap, no startup failure |
| `config.toml` larger than the path-validator size limit | Rejected via the same `validate_file_size` guard used for scenario TOML and profile/config files; falls back to stock |
| Two TOML keys normalize to the same physical key | Detected at insert time, reported `Shadowed`, first-wins is not silently assumed |
| A multi-token expansion is looked up from a pending state (count/register prefix) | Falls through to the stock key rather than executing partway |
| Custom keymap disabled after being active | `keymap_fingerprint` tracking still runs (stock keymap is fingerprinted like any other mapping, per `463d363`), so re-enabling later against a stretch under stock is still detected as a genuine transition |
| A command name in the config doesn't match any `CommandMetadata.name` this trainer implements | Reported as `UnknownCommand`, not applied, does not block other valid bindings |
| Alt-`s` (`split_selection_on_newline`) physically unreachable on a terminal without kitty keyboard protocol support (e.g. macOS Terminal.app, since real Option+s composition was removed for layout-collision reasons — see `cd8751c`) | Documented workaround (`d7003ca`): remap a reachable key to `split_selection_on_newline` via this feature |

## 7. Success Criteria

| ID | Metric | Target |
|----|--------|--------|
| SC-001 | `CommandMetadata.name` matches real Helix 25.07.1 identifiers | Verified by a build-breaking test asserting the name-to-key reverse index is total and injective |
| SC-002 | Regression tests pass for load-time validation, atomic apply, and every warning reason | `src/input/keymap/`, `src/config/keymap/` unit tests green |
| SC-003 | No regression in `cargo nextest run --workspace --all-features --lib --bins` | 100% pass |
| SC-004 | Existing `config.json` files load unchanged after this field is added | Verified by a regression test simulating a pre-`use_helix_keymap` config.json |

## 8. Agent Boundaries

### Always (without asking)
- Run `cargo nextest run --workspace --all-features --lib --bins` after any
  change touching `src/input/keymap/`, `src/config/keymap/`, or
  `handle_gameplay_input`
- Report every unsupported binding via `KeymapWarningReason` — never drop
  one silently

### Ask First
- Extending remap support to insert-mode or select-mode key tables
- Extending remap support to menu/results/filter screens or hint prose
  (violates the gameplay-only scope, FR-009)

### Never
- Key FSRS card ids, quest matching, or scenario solutions on physical
  keys instead of canonical command strings — this is the invariant that
  lets review history survive keymap changes (FR-007)
- Apply a multi-token expansion from a pending input state (FR-003)
- Discard existing `config.json` fields when adding a new `AppConfig` field

## 9. Open Questions

None outstanding — follow-up work items #347, #348 (fixed by `463d363`)
and #349 (perf, fixed by `8e0b6ef`) are resolved. #386 (Alt-s
reachability) has a documented workaround (`d7003ca`); #389 (Alt-c) is
resolved by a separate, unrelated fix — see
[[../register-command-mode-support/spec#11. Post-Release Extensions (v0.6.0)|register-command-mode-support's Post-Release Extensions]].

## 10. See Also

- [[constitution]] — project principles, Section I (typestate pattern)
- [[MOC-specs]] — all specifications
- README.md — "Custom Helix keymap (optional)" user-facing section
- `docs/HELIX_KEYBINDINGS.md` — implemented-command reference
- Issue #163 — original feature request
