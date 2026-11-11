---
aliases:
  - Project Principles
  - helix-trainer Constitution
tags:
  - sdd
  - constitution
created: 2026-08-09
status: permanent
---

# Project Constitution

> [!important]
> Non-negotiable principles governing ALL development in helix-trainer.
> Every specification, plan, and task MUST comply with this document.
> Derived from the project's existing conventions (architecture guide,
> commit/issue rules, branching rules, continuous-improvement notes)
> rather than invented from scratch — this document formalizes what the project already practices.
> Update this file only through explicit team decision.

## I. Architecture

- Single-binary TUI: **ratatui** + **crossterm** rendering, **tokio** async
  runtime (2 worker threads). No client/server split, no HTTP API.
- State management follows the **Elm Architecture** (`src/ui/state/`):
  `AppState` is the single source of truth; `TypedScreen` is a type-safe
  enum of all screens; `Message` is an exhaustive enum of all events; `update()`
  is a pure function — all state mutations flow through it.
- The Helix editor is simulated, never faked: all in-editor command
  execution routes through `HelixSimulator`/`AnyModeSimulator`
  (`src/helix/`), which wraps real `helix-core` (upstream tag 25.07.1).
  Bespoke coordinate/string-diff shortcuts that bypass the simulator are
  forbidden — this is the single most load-bearing invariant in the
  codebase and is referenced by every feature spec that touches input.
- Multi-key input sequences (counts, `g`/`z`/`m` leader keys, registers,
  command-line mode) are modeled with the **typestate pattern**
  (`src/input/typestate/`) — zero-sized marker structs plus
  `InputStateMachine::process_key` dispatch. New pending-input states MUST
  follow this pattern; no parallel/ad-hoc state-tracking mechanism is
  permitted.
- Background I/O (scenario loading, profile persistence) is spawned via
  `data_loader::spawn_data_loaders` and communicates with `AppState`
  exclusively over `mpsc::channel` messages — never by mutating shared
  state from a background task directly.
- Rendering (`src/ui/render/`) is pure: no side effects, no state
  mutation, computed entirely from `AppState`.

## II. Technology Stack

- Language: Rust, MSRV per `Cargo.toml` `rust-version` (1.98 at time of
  writing).
- TUI: `ratatui` + `crossterm`.
- Async runtime: `tokio` (2 worker threads).
- Spaced repetition: `fsrs` crate, wrapped by `src/learning/`.
- i18n: `rust-i18n` with compile-time codegen (`locales/`), invoked via
  `rust_i18n::i18n!("locales", fallback = "en")` in `lib.rs`.
- Optional audio: `rodio`, feature-gated behind `audio` (requires
  `libasound2-dev` on Linux).
- Config/scenario data: TOML (`scenarios/en/<category>/*.toml`), user
  profile/config: JSON in the OS config directory (`dirs::config_dir()/helix-trainer/`;
  macOS uses `~/.config/helix-trainer`). There is no current-directory fallback: when no
  config directory can be resolved the application refuses to start (#457).
- YAML tooling (when needed): `fast-yaml` (`fy` CLI) exclusively — never
  hand-edit YAML without validating via `fy` afterward.

## III. Testing (NON-NEGOTIABLE)

- Framework: `cargo nextest` (`cargo nextest run --workspace
  --all-features --lib --bins`), plus `cargo test --doc` for doc-tests.
- Every shipped scenario TOML file must pass `cargo nextest run scenario`
  (validates via `tests/scenario_validation.rs` that the declared
  `solution.commands` actually completes the scenario through
  `HelixSimulator`).
- **Property-based testing** (`proptest`, declared in `[dev-dependencies]`)
  is the required tool for deterministic-given-fixed-inputs logic — FSRS
  scheduling invariants, difficulty-controller selection determinism. Any
  declared-but-unused test dependency is a defect, not a neutral state (see
  `specs/fsrs-proptest-coverage-gap/`): if `proptest` is
  declared, it must be exercised somewhere with a traceable invariant, or
  removed together with any documentation claiming otherwise.
- Property tests must be deterministic and reproducible: never read live
  wall-clock time (`Utc::now()`) as a generator input or baseline inside a
  proptest body — inject a fixed/fake clock so shrunk failures reproduce.
- Colocate unit tests (including property tests) in `#[cfg(test)]` modules
  within the module under test, unless the module grows large enough to
  warrant a dedicated `_tests.rs`/`_proptest.rs` submodule.
- Prefer regression tests with real dependencies (real `HelixSimulator`
  execution, real `fsrs` state transitions) over mocks — this project has
  no service boundaries to mock across.
- Audio (`--features audio`) is compile-checked (`cargo check --features
  audio`) but not executed in CI (no sound device); Windows-specific
  `KeyEventKind::Release` filtering is a known regression class
  (`fix/182`) and must stay covered.

## IV. Code Style

- Follow existing patterns in the codebase before introducing new ones —
  check `src/minigame/`, `src/game/`, `src/input/typestate/` for the
  established idiom (typestate, sealed traits, exhaustive `match`) before
  adding a parallel mechanism.
- `[lints.rust] unsafe_code = "forbid"` in `Cargo.toml` (replacing the former
  `#![forbid(unsafe_code)]` crate attributes in `main.rs` and `lib.rs`) MUST
  remain in force project-wide — no exceptions, no `#[allow(unsafe_code)]`
  overrides.
- Clippy pedantic lints are enabled project-wide (#532). Kept allowed in `Cargo.toml`, each with a one-line reason: `module_name_repetitions` and `must_use_candidate` (user decision), and, as a deferred residual, `assert_is_empty`, `match_same_arms`, `similar_names` and `too_many_lines`.
- Every `pub` type, trait, function, and method must have a `///` doc
  comment explaining *what* and *why*, not merely restating the name;
  non-trivial public APIs need a runnable `# Examples` doc-test.
  Feature-gated items get `#[cfg_attr(docsrs, doc(cfg(...)))]`.
- Do not add redundant comments — only explain cyclomatically or
  cognitively complex blocks.
- Follow DRY: search for existing implementations (Grep/Glob) and reuse
  established patterns before writing new functionality.
- For MVP-stage work: implement only the minimum necessary functionality,
  no syntactic sugar, no premature abstraction or optimization. Before
  v1.0.0, backward compatibility is not a constraint — prefer clean code
  over deprecation shims; document breaking changes in `CHANGELOG.md`.
- Non-exhaustive `match`/`matches!` wildcards (`_ => ...`) over an enum
  that models a closed, growable set (e.g. `MiniGameMode`) are a defect
  class this project has hit before (see
  `specs/arcade-game-mode-variety/appendix-spinoffs.md`, Spin-off B) — new
  code should match exhaustively over enum variants so adding a variant is
  a compile error at every call site, not a silent runtime fallthrough.

## V. Security

- All user-facing input validation and error types route through
  `src/security.rs` (`UserError`, `SecurityError`) — never panic on
  malformed user input (invalid register names, malformed command-line
  expressions, oversized input) and never silently no-op it either.
- A recoverable `UserError` (including a message that arrives for a screen other than the current one, a failed background save, or a rejected simulator command) MUST NOT terminate the TUI session. The event loop reports it (user-visible notification where the user can act on it, `tracing` otherwise; a stale-screen message is a silent debug-level no-op) and keeps running. Only unrecoverable terminal I/O failure ends the session: an input-stream error is reported and the loop exits only after 5 consecutive errors, never by spinning. A `WrongScreen` error is ignored silently; every other `UserError` is shown as a notification. A single `apply_update` boundary in `src/event_loop.rs` owns this policy; `?` on `update()`/`handle_data_message()` inside the loop is forbidden.
- Never commit secrets, API keys, or credentials. This project has no
  external services/API keys in scope today; if any are ever introduced,
  they must go through the vault mechanism described in the user's global
  tooling conventions, never `.env` files or shell profiles.
- Bound untrusted input length explicitly (e.g. `security::limits::
  MAX_COMMAND_LINE_LEN`) rather than relying on incidental limits.

## VI. Performance

- The animation/tick interval used by `event_loop::run_async_event_loop`
  is the baseline for "perceptible lag" — any new per-frame or per-tick
  logic must not introduce lag perceptible against that baseline.
- No real-time, per-frame game loop exists today; `event_loop.rs` ticks
  are predicate-polling (timeout checks, transition auto-advance), not a
  dedicated session clock. Introducing one is an architectural decision
  requiring explicit design, not an incidental addition.

## VII. Simplicity

- Prefer extending an existing enum variant / existing subsystem (e.g. a
  new `MiniGameMode` variant, a new typestate pending-state) over
  introducing a parallel top-level type that duplicates session/timer/
  scoring/state-machine machinery already provided by `GameSession`/
  `MiniGameSession`.
- New dependencies require justification and must pass `cargo deny check`
  with no new advisories.
- Scope narrowing during implementation is acceptable and expected when
  the full ambition of a draft spec proves disproportionate to its
  priority/evidence (e.g. command-line mode shipped as `:goto`/`:g` only,
  not full `:s` substitute) — retroactive specs must document the actual
  shipped scope, not silently imply the original scope was fully met.

## VIII. Git Workflow

- Commit messages: [Conventional Commits 1.0.0](https://www.conventionalcommits.org/en/v1.0.0/#specification)
  per the project's commit rules — `type(scope): imperative,
  present-tense description`, no period, no AI/co-author mentions, no
  emoji.
- Branch naming: `feat/{issue}-{slug}`, `fix/{issue}-{slug}`,
  `hotfix/{issue}-{slug}`, `chore/{slug}`; omit the `{issue}-` segment
  when no issue exists.
- All changes land via PR into `main`; sync with `main` via `git fetch
  origin main && git rebase origin/main` before opening a PR.
- Before every commit/push/PR: `cargo +nightly fmt --check`, `cargo
  clippy --all-targets --all-features --workspace -- -D warnings`, `cargo
  nextest run --workspace --all-features --lib --bins`, and the rustdoc
  gate (`RUSTDOCFLAGS="--deny rustdoc::broken_intra_doc_links" cargo doc
  --no-deps`). If new/modified scenario TOML files are included, also run
  `cargo nextest run scenario`.
- Every GitHub issue carries exactly one `P0`-`P4` priority label plus up
  to 4 classification labels (type and subsystem, e.g. `bug`, `ui`); a
  missing classification label is created, never replaced by an unrelated
  one. Priority meaning:

  | Label | Meaning |
  |-------|---------|
  | `P0` | Broken core gameplay, data loss, crash on startup |
  | `P1` | Incorrect scoring, unbeatable scenario, broken FSRS persistence |
  | `P2` | Suboptimal UX, minor display issue |
  | `P3` | Cosmetic, edge case unlikely in practice |
  | `P4` | Nice-to-have, speculative, or process/coverage gap |

- Spec-first workflow: specs are created and updated only in root
  `specs/` by the sdd agent, and issues for new or changed behavior link
  to the relevant spec package first.
- `CHANGELOG.md` `[Unreleased]` section is updated at the end of every
  implementation phase/PR.

## See Also

- [[MOC-specs]] — all specifications
- `CONTRIBUTING.md` — Conventional Commits and the pre-commit check suite
