//! Security configuration constants
//!
//! Size, count and length limits that keep untrusted input (scenario and
//! config files, typed text, ids) bounded. A dependency-free leaf module.

/// Maximum size of a scenario TOML file (10 MB)
pub(crate) const MAX_SCENARIO_FILE_SIZE: u64 = 10 * 1024 * 1024;

/// Maximum number of scenarios per file
pub(crate) const MAX_SCENARIOS_PER_FILE: usize = 100;

/// Maximum length of scenario file content
pub(crate) const MAX_FILE_CONTENT_LENGTH: usize = 100_000;

/// Maximum number of hints per scenario
pub(crate) const MAX_HINTS: usize = 10;

/// Maximum command sequence length
pub(crate) const MAX_COMMAND_SEQUENCE_LENGTH: usize = 100;

/// Maximum number of commands stored in a single `q`/`Q` macro
///
/// Recording stops accepting further commands once this cap is hit,
/// keeping what was already captured rather than failing outright.
pub(crate) const MAX_MACRO_LENGTH: usize = 100;

/// Maximum length of a single-line prompt buffer
///
/// Shared by every prompt that accumulates text one keystroke at a time
/// before executing atomically: the `:`-prefixed command-line buffer and
/// the `s`/`S` regex-selection prompt. `InputState` is cloned on every
/// keystroke transition, so this bounds the per-keystroke clone cost
/// (O(n²) in the worst case) as well as the rendered prompt line length.
pub(crate) const MAX_COMMAND_LINE_LEN: usize = 256;

/// Maximum number of selection ranges kept from a single `s`/`S` regex
/// match pass
///
/// `helix_core::selection::{select_on_matches, split_on_matches}` loop
/// `find_iter` unbounded and accept no cap parameter, so this bounds the
/// *retained* selection post-hoc rather than peak allocation during the
/// scan. Peak allocation is bounded by document size (not
/// attacker-controlled: documents are scenario-authored and tiny), and
/// `regex-cursor` is finite-automaton based, so catastrophic backtracking
/// is not a concern here.
pub(crate) const MAX_REGEX_SELECTION_MATCHES: usize = 10_000;

/// Maximum number of quest templates per file
pub(crate) const MAX_QUEST_TEMPLATES_PER_FILE: usize = 100;

/// Maximum target value for quest objectives
pub(crate) const MAX_QUEST_TARGET: u32 = 100;

/// Maximum speed run time limit in seconds (1 hour)
pub(crate) const MAX_SPEED_RUN_TIME_SECONDS: u64 = 3600;

/// Maximum custom XP reward
pub(crate) const MAX_CUSTOM_XP_REWARD: u32 = 1000;

/// Maximum quest name length
pub(crate) const MAX_QUEST_NAME_LENGTH: usize = 100;

/// Maximum quest description length
pub(crate) const MAX_QUEST_DESCRIPTION_LENGTH: usize = 500;

/// Maximum version string length
pub(crate) const MAX_VERSION_LENGTH: usize = 20;

/// Maximum locale string length
pub(crate) const MAX_LOCALE_LENGTH: usize = 10;

/// Maximum length of a scenario `Setup.language` token (file-extension-style,
/// e.g. "rs", "md", "py" — longest bundled syntect extensions are well under this)
pub(crate) const MAX_LANGUAGE_LENGTH: usize = 20;

/// Maximum required conditions count
pub(crate) const MAX_REQUIRED_CONDITIONS: usize = 20;

/// Maximum number of cursors per scenario
pub(crate) const MAX_CURSORS_PER_SCENARIO: usize = 100;

/// Maximum number of selections per scenario
pub(crate) const MAX_SELECTIONS_PER_SCENARIO: usize = 100;

/// Maximum length of a scenario or quest ID
pub(crate) const MAX_ID_LENGTH: usize = 64;

/// Maximum size of the user's Helix `config.toml` we'll read for
/// keymap remapping (1 MB — real configs are a few KB at most).
pub(crate) const MAX_KEYMAP_FILE_SIZE: u64 = 1024 * 1024;

/// Maximum number of keymap bindings (top-level plus minor-mode
/// entries combined) accepted from a single Helix config file.
pub(crate) const MAX_KEYMAP_BINDINGS: usize = 1000;

/// Maximum size of `profile.json` (1 MB). At the current curriculum
/// size (148 scenarios, 76 distinct commands tracked by FSRS), a
/// saturated profile — every scenario completed, every command with a
/// full review history — serializes to roughly 70 KB, so this cap
/// leaves about 14x headroom for curriculum growth before it needs
/// revisiting.
pub(crate) const MAX_PROFILE_FILE_SIZE: u64 = 1024 * 1024;

/// Maximum size of the `profile.json.lock` file: a decimal process id.
pub(crate) const MAX_LOCK_FILE_SIZE: u64 = 64;

/// Maximum size of `config.json` (1 MB). `AppConfig` today is a
/// handful of booleans (well under 1 KB serialized), so this leaves
/// generous headroom for new settings fields.
pub(crate) const MAX_CONFIG_FILE_SIZE: u64 = 1024 * 1024;
