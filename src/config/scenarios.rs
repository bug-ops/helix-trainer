//! Scenario loading and validation
//!
//! This module handles loading TOML scenario files with security validations.

use crate::config::ConfigError;
use crate::constants::{ADVANCED_TIME_LIMIT, BEGINNER_TIME_LIMIT, INTERMEDIATE_TIME_LIMIT};
use crate::helix::Language;
use crate::ids::{CommandId, NormalizedCommand, ScenarioId};
use crate::limits::{
    MAX_COMMAND_SEQUENCE_LENGTH, MAX_FILE_CONTENT_LENGTH, MAX_HINTS, MAX_SCENARIO_FILE_SIZE,
    MAX_SCENARIOS_PER_FILE,
};
use crate::security::{SecurityError, path_validator, sanitizer};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fs;
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};

mod cursor;

pub use cursor::{BoundError, Bounded, CursorField, CursorRange, CursorSpec, Pos};

/// Why a scenario's cursor setup or target was rejected.
///
/// Wrapped in [`ConfigError::Scenario`];
/// the text is for logs only.
///
/// # Examples
///
/// ```
/// use helix_trainer::config::scenarios::ScenarioValidationError;
///
/// let err = ScenarioValidationError::TooManyCursors { max: 100, actual: 101 };
/// assert_eq!(err.to_string(), "too many cursors (max 100, got 101)");
/// ```
#[derive(thiserror::Error, Debug, Clone, PartialEq, Eq)]
pub enum ScenarioValidationError {
    /// None of `cursor_position`, `selection`, `cursors` and `selections` is set.
    #[error("must specify one of cursor_position, selection, cursors, or selections")]
    NoCursorSpecified,

    /// Two or more cursor keys are set; exactly one is allowed.
    #[error("ambiguous cursor: {}; specify exactly one", join_fields(.fields))]
    AmbiguousCursorSpec {
        /// The keys that were set.
        fields: Vec<CursorField>,
    },

    /// `cursors` or `selections` is an empty list.
    #[error("{field} must not be empty")]
    EmptyCursorList {
        /// The empty key.
        field: CursorField,
    },

    /// A row or column is beyond the supported range.
    #[error("cursor position out of range (max {max}, got {actual})")]
    PositionOutOfRange {
        /// Largest accepted coordinate.
        max: usize,
        /// Observed coordinate.
        actual: usize,
    },

    /// More cursors than allowed.
    #[error("too many cursors (max {max}, got {actual})")]
    TooManyCursors {
        /// Maximum allowed count.
        max: usize,
        /// Observed count.
        actual: usize,
    },

    /// More selections than allowed.
    #[error("too many selections (max {max}, got {actual})")]
    TooManySelections {
        /// Maximum allowed count.
        max: usize,
        /// Observed count.
        actual: usize,
    },
}

fn join_fields(fields: &[CursorField]) -> String {
    fields
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(", ")
}

/// Wrapper for scenarios array in TOML file
#[derive(Deserialize, Debug, Clone)]
pub struct ScenariosFile {
    /// Scenarios in file order.
    pub scenarios: Vec<Scenario>,
}

/// How a scenario decides it is finished.
///
/// Set in the TOML as `completion = "after_taught_commands"` under
/// `[scenarios.metadata]`; an unknown value is a scenario load error.
#[derive(Deserialize, Debug, Clone, Copy, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CompletionRule {
    /// Complete as soon as the editor state equals the target.
    #[default]
    TargetState,
    /// Complete when the editor state equals the target AND the last action is
    /// one of the scenario's `commands_taught` AND that action caused the
    /// transition (the target did not already hold), so history navigation
    /// (`u`/`U`) cannot be satisfied by revisiting a state or by a no-op.
    AfterTaughtCommands,
}

/// The just-executed action as seen by [`CompletionRule::is_satisfied`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LastAction<'a> {
    /// The command string as typed (may carry a count prefix).
    pub command: &'a str,
    /// Whether the editor already matched the target before this action ran.
    pub target_held_before: bool,
}

impl CompletionRule {
    /// The single completion predicate shared by training and arcade.
    ///
    /// `target_reached` is the editor-versus-target comparison and
    /// `last_action` the just-executed action. Commands are compared as review
    /// card ids, so a count prefix is ignored (`2u` matches `u`). An action
    /// that found the target already satisfied (a no-op such as `U` on an empty
    /// redo stack) never completes an [`CompletionRule::AfterTaughtCommands`]
    /// scenario.
    ///
    /// # Examples
    ///
    /// ```
    /// use helix_trainer::config::{CompletionRule, LastAction};
    /// use helix_trainer::ids::NormalizedCommand;
    ///
    /// let taught = [NormalizedCommand::from_raw("u").unwrap()];
    /// let rule = CompletionRule::AfterTaughtCommands;
    /// let action = |command, target_held_before| {
    ///     Some(LastAction { command, target_held_before })
    /// };
    /// assert!(rule.is_satisfied(true, &taught, action("2u", false)));
    /// assert!(!rule.is_satisfied(true, &taught, action("2u", true)));
    /// assert!(!rule.is_satisfied(true, &taught, action("d", false)));
    /// assert!(CompletionRule::TargetState.is_satisfied(true, &taught, action("d", true)));
    /// ```
    #[must_use]
    pub fn is_satisfied(
        self,
        target_reached: bool,
        commands_taught: &[NormalizedCommand],
        last_action: Option<LastAction<'_>>,
    ) -> bool {
        if !target_reached {
            return false;
        }
        match self {
            Self::TargetState => true,
            Self::AfterTaughtCommands => {
                let Some(last_action) = last_action.filter(|a| !a.target_held_before) else {
                    return false;
                };
                let Some(last) = CommandId::from_action(last_action.command) else {
                    return false;
                };
                commands_taught
                    .iter()
                    .filter_map(|taught| CommandId::from_action(taught.as_str()))
                    .any(|taught| taught == last)
            }
        }
    }
}

/// Scenario metadata for filtering, sorting, and quest generation
#[derive(Deserialize, Debug, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct ScenarioMetadata {
    /// Completion rule (defaults to [`CompletionRule::TargetState`])
    #[serde(default)]
    pub completion: CompletionRule,

    /// Scenario category (e.g., movement, editing, clipboard)
    #[serde(default)]
    pub category: Option<ScenarioCategory>,

    /// Difficulty level (beginner, intermediate, advanced)
    #[serde(default)]
    pub difficulty: Option<Difficulty>,

    /// Tags for flexible filtering (e.g., `["delete", "motion", "word"]`)
    #[serde(default)]
    pub tags: Vec<String>,

    /// Commands taught in this scenario (e.g., `["d", "w"]`)
    #[serde(default)]
    pub commands_taught: Vec<NormalizedCommand>,

    /// Prerequisite scenario IDs that should be completed first
    #[serde(default)]
    pub prerequisites: Vec<ScenarioId>,
}

/// Scenario category for organization and filtering
#[derive(Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum ScenarioCategory {
    /// Cursor motions and navigation.
    Movement,
    /// Deleting, changing and inserting text.
    Editing,
    /// Yank, paste and registers used as a clipboard.
    Clipboard,
    /// Search and match navigation.
    Search,
    /// Selection manipulation.
    Selection,
    /// Text object selections (`mi`/`ma`).
    TextObjects,
    /// Less common commands that build on the basics.
    Advanced,
    /// Named register usage.
    Registers,
    /// Writing-oriented tasks such as prose and lists.
    Writing,
    /// Scenarios that span several categories.
    Multi, // Multiple categories
    /// Anything that fits no other category.
    Other,
}

/// Difficulty level for progressive learning
#[derive(Deserialize, Serialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "lowercase")]
pub enum Difficulty {
    /// Introductory scenarios.
    Beginner,
    /// Scenarios that combine several basic commands.
    Intermediate,
    /// Scenarios that need multi-step planning.
    Advanced,
}

impl Difficulty {
    /// All difficulty variants. Keep in sync with the enum — `difficulty_all_is_exhaustive`
    /// below fails to compile if a variant is added here without updating this array.
    pub const ALL: [Difficulty; 3] = [
        Difficulty::Beginner,
        Difficulty::Intermediate,
        Difficulty::Advanced,
    ];

    /// Base per-difficulty time budget, independent of arcade's adaptive level scaling.
    ///
    /// The arcade difficulty controller scales this by its current level; callers
    /// with no controller (Training mode) use it directly as the "speed" reference
    /// budget for achievement thresholds.
    ///
    /// # Examples
    ///
    /// ```
    /// use helix_trainer::config::Difficulty;
    /// use std::time::Duration;
    ///
    /// assert_eq!(Difficulty::Beginner.base_time_limit(), Duration::from_secs(10));
    /// ```
    #[must_use]
    pub fn base_time_limit(self) -> std::time::Duration {
        match self {
            Self::Beginner => BEGINNER_TIME_LIMIT,
            Self::Intermediate => INTERMEDIATE_TIME_LIMIT,
            Self::Advanced => ADVANCED_TIME_LIMIT,
        }
    }
}

/// Scenario definition
#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct Scenario {
    /// Unique identifier, validated as a safe id.
    pub id: ScenarioId,

    /// Display name.
    pub name: String,
    /// What the player is asked to accomplish.
    pub description: String,
    /// Initial editor state.
    pub setup: Setup,
    /// Editor state that completes the scenario.
    pub target: TargetState,
    /// Canonical shortest solution.
    pub solution: Solution,

    /// Progressive hints revealed on request.
    #[serde(default)]
    pub hints: Vec<String>,

    /// Parameters of the scoring formula.
    pub scoring: ScoringConfig,

    /// Metadata for filtering, sorting, and quest generation.
    #[serde(default)]
    pub metadata: ScenarioMetadata,
}

impl Scenario {
    /// Whether the scenario is finished, given the editor-versus-target
    /// comparison and the just-executed action.
    ///
    /// Both training and arcade completion go through this, so they cannot
    /// diverge. See [`CompletionRule`].
    #[must_use]
    pub fn is_completed_by(
        &self,
        target_reached: bool,
        last_action: Option<LastAction<'_>>,
    ) -> bool {
        self.metadata.completion.is_satisfied(
            target_reached,
            &self.metadata.commands_taught,
            last_action,
        )
    }
}

/// Effective content language used when a scenario's [`Setup::language`] is `None`.
///
/// Matches the syntax highlighter's original hardcoded behavior (see FR-002 in
/// `specs/language-aware-syntax-highlighting/spec.md`). Shared by
/// `PlayableScenario::language`'s default implementation and its `GameSession`/
/// `ActiveMiniScenario` overrides so the fallback exists in exactly one place.
pub const DEFAULT_LANGUAGE: Language = Language::Rs;

/// Initial editor setup
#[derive(Deserialize, Debug, Clone)]
pub struct Setup {
    /// Initial document text.
    pub file_content: String,

    /// Content's language (`rs`, `md` or `py` in the scenario file); any other value,
    /// including an empty one, is a scenario load error.
    /// `None` (the default for scenarios that omit this field) is treated as
    /// [`DEFAULT_LANGUAGE`], preserving the highlighter's original hardcoded behavior.
    #[serde(default)]
    pub language: Option<Language>,

    /// Cursor and selection placement at the start.
    #[serde(flatten)]
    pub cursor: CursorSpec,
}

/// Target state to achieve
#[derive(Deserialize, Debug, Clone)]
pub struct TargetState {
    /// Document text the player must reach.
    pub file_content: String,

    /// Cursor and selection placement required at the end.
    #[serde(flatten)]
    pub cursor: CursorSpec,
}

/// Optimal solution
#[derive(Deserialize, Debug, Clone)]
pub struct Solution {
    /// Key sequence of the optimal solution, one command per entry.
    pub commands: Vec<String>,
    /// Explanation of why this solution is optimal.
    pub description: String,
}

/// Scoring configuration
#[derive(Deserialize, Debug, Clone)]
pub struct ScoringConfig {
    /// Command count of the optimal solution; the baseline for scoring.
    pub optimal_count: NonZeroUsize,
    /// Points awarded for an optimal run.
    pub max_points: u32,
    /// Extra commands allowed beyond the optimum before the score drops.
    pub tolerance: usize,
}

/// Secure scenario loader with path validation and content verification
///
/// Two loading paths exist:
///
/// - [`load_from_embedded`](Self::load_from_embedded) reads scenarios compiled
///   into the binary. This is the only path production code takes
///   (`src/data_loader.rs`).
/// - [`load`](Self::load), [`load_directory`](Self::load_directory) and
///   [`available_locales`](Self::available_locales) read from the filesystem,
///   restricted to `allowed_base_paths`. They are retained deliberately for a
///   planned "load custom scenario packs from disk" feature and are currently
///   exercised only by unit tests — they are not dead code awaiting removal.
pub struct ScenarioLoader {
    allowed_base_paths: Vec<PathBuf>,
}

impl ScenarioLoader {
    /// Create a new scenario loader with default allowed paths
    ///
    /// Allows loading from both the base scenarios directory and language-specific subdirectories.
    pub fn new() -> Self {
        Self {
            allowed_base_paths: vec![
                PathBuf::from("./scenarios"),
                PathBuf::from("/usr/share/helix-trainer/scenarios"),
            ],
        }
    }

    /// Detect available locales by scanning the allowed scenario directories
    ///
    /// Returns the union of locale codes (e.g. `["en", "ru"]`) found as
    /// two-letter lowercase subdirectories across `allowed_base_paths`,
    /// deduplicated and sorted. Falls back to `["en"]` when nothing is found.
    ///
    /// Not reached from production code today — see the [`ScenarioLoader`] type
    /// docs for why this filesystem path is retained.
    pub fn available_locales(&self) -> Vec<String> {
        // Enumerating the configured roots themselves, not caller-supplied
        // input, so there is no traversal surface for path_validator to guard.
        let mut locales: BTreeSet<String> = BTreeSet::new();

        for base in &self.allowed_base_paths {
            let Ok(entries) = fs::read_dir(base) else {
                continue;
            };
            for entry in entries.flatten() {
                let Ok(file_type) = entry.file_type() else {
                    continue;
                };
                if !file_type.is_dir() {
                    continue;
                }
                if let Some(name) = entry.file_name().to_str()
                    && name.len() == 2
                    && name.chars().all(|c| c.is_ascii_lowercase())
                {
                    locales.insert(name.to_string());
                }
            }
        }

        if locales.is_empty() {
            tracing::warn!("No scenario locales found; falling back to English");
            return vec!["en".to_string()];
        }

        locales.into_iter().collect()
    }

    /// Create a loader with custom allowed paths for testing
    pub fn with_allowed_paths(paths: Vec<PathBuf>) -> Self {
        Self {
            allowed_base_paths: paths,
        }
    }

    /// Load scenarios from a directory, scanning recursively for all .toml files
    ///
    /// # Security Validations
    /// - Directory must be within allowed directories
    /// - Each file is validated using the same security checks as `load()`
    /// - Total scenarios across all files must not exceed reasonable limits
    ///
    /// # Errors
    /// Returns [`ConfigError`] if any file fails to load or validation fails
    ///
    /// Not reached from production code today — see the [`ScenarioLoader`] type
    /// docs for why this filesystem path is retained.
    pub fn load_directory(&self, dir_path: &Path) -> Result<Vec<Scenario>, ConfigError> {
        // Validate directory path
        let canonical = path_validator::validate_path(dir_path, &self.allowed_base_paths)?;

        if !canonical.is_dir() {
            tracing::error!("Path is not a directory");
            return Err(SecurityError::InvalidPath.into());
        }

        tracing::info!(
            dir = %sanitizer::sanitize_path_for_logging(&canonical),
            "Loading scenarios from directory"
        );

        // Recursively walk directory and collect all .toml files
        let (all_scenarios, file_count) = self.visit_toml_files(&canonical)?;

        if all_scenarios.is_empty() {
            tracing::warn!("No scenario files found in directory");
            return Err(ConfigError::NotFound);
        }

        tracing::info!(
            scenario_count = all_scenarios.len(),
            file_count = file_count,
            "Successfully loaded scenarios from directory"
        );

        Ok(all_scenarios)
    }

    /// Recursively visit all .toml files in a directory
    ///
    /// Returns tuple of (scenarios, `file_count`)
    ///
    /// Not reached from production code today — see the [`ScenarioLoader`] type
    /// docs for why this filesystem path is retained.
    fn visit_toml_files(&self, dir: &Path) -> Result<(Vec<Scenario>, usize), ConfigError> {
        let entries = fs::read_dir(dir).map_err(ConfigError::Io)?;

        let mut scenarios = Vec::new();
        let mut file_count: usize = 0;

        for entry in entries {
            let entry = entry.map_err(ConfigError::Io)?;

            let path = entry.path();

            if path.is_dir() {
                // Recursively visit subdirectories
                let (sub_scenarios, sub_count) = self.visit_toml_files(&path)?;
                scenarios.extend(sub_scenarios);
                file_count = file_count.saturating_add(sub_count);
            } else if path.extension().and_then(|s| s.to_str()) == Some("toml") {
                // Load scenarios from this file
                let file_scenarios = self.load(&path).inspect_err(|e| {
                    tracing::error!(
                        file = %sanitizer::sanitize_path_for_logging(&path),
                        "Failed to load scenario file: {:?}",
                        e
                    );
                })?;

                file_count = file_count.saturating_add(1);
                scenarios.extend(file_scenarios);
            }
        }

        Ok((scenarios, file_count))
    }

    /// Load scenarios from a TOML file with comprehensive security validations
    ///
    /// # Security Validations
    /// - Path must be within allowed directories (prevents path traversal)
    /// - File size must not exceed `MAX_SCENARIO_FILE_SIZE`
    /// - TOML must be valid
    /// - Scenario count must not exceed `MAX_SCENARIOS_PER_FILE`
    /// - Each scenario is validated for content size, cursor positions, etc.
    ///
    /// # Errors
    /// Returns [`ConfigError`] if any validation fails
    ///
    /// Not reached from production code today — see the [`ScenarioLoader`] type
    /// docs for why this filesystem path is retained.
    pub fn load(&self, path: &Path) -> Result<Vec<Scenario>, ConfigError> {
        // Validate path to prevent path traversal attacks
        let canonical = path_validator::validate_path(path, &self.allowed_base_paths)?;

        // Log with sanitized path (doesn't leak full path)
        tracing::info!(
            file = %sanitizer::sanitize_path_for_logging(&canonical),
            "Loading scenario file"
        );

        // Bounded read of a regular file to prevent resource exhaustion
        let content = path_validator::read_capped(&canonical, MAX_SCENARIO_FILE_SIZE)
            .map_err(SecurityError::from)?;

        // Parse, count-check, and validate each scenario
        let scenarios = super::loader::parse_and_validate::<ScenariosFile>(
            &content,
            MAX_SCENARIOS_PER_FILE,
            Self::validate_scenario,
        )?;

        tracing::info!(count = scenarios.len(), "Successfully loaded scenarios");

        Ok(scenarios)
    }

    /// Load scenarios from embedded TOML strings
    ///
    /// This method loads scenarios from compile-time embedded content,
    /// eliminating the need for filesystem access. It applies the same
    /// validation as filesystem loading.
    ///
    /// # Arguments
    ///
    /// * `locale` - Locale code (e.g., "en")
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError`] if no embedded data exists for the locale or
    /// if validation fails.
    ///
    /// # Examples
    ///
    /// ```
    /// use helix_trainer::config::ScenarioLoader;
    ///
    /// let loader = ScenarioLoader::new();
    /// let scenarios = loader.load_from_embedded("en")?;
    /// assert!(!scenarios.is_empty());
    /// # Ok::<(), helix_trainer::error::UserError>(())
    /// ```
    pub fn load_from_embedded(&self, locale: &str) -> Result<Vec<Scenario>, ConfigError> {
        let embedded_contents = embedded::get_embedded_scenarios(locale);

        if embedded_contents.is_empty() {
            tracing::warn!(locale = locale, "No embedded scenarios for locale");
            return Err(ConfigError::NotFound);
        }

        tracing::info!(
            locale = locale,
            file_count = embedded_contents.len(),
            "Loading scenarios from embedded data"
        );

        let mut all_scenarios = Vec::new();

        for (index, content) in embedded_contents.iter().enumerate() {
            // Parse, count-check, and validate each scenario
            let scenarios = super::loader::parse_and_validate::<ScenariosFile>(
                content,
                MAX_SCENARIOS_PER_FILE,
                Self::validate_scenario,
            )
            .inspect_err(|e| {
                tracing::error!(
                    locale = locale,
                    index = index,
                    "Failed to load embedded scenario file: {:?}",
                    e
                );
            })?;

            all_scenarios.extend(scenarios);
        }

        tracing::info!(
            locale = locale,
            scenario_count = all_scenarios.len(),
            "Successfully loaded scenarios from embedded data"
        );

        Ok(all_scenarios)
    }

    /// Validate a single scenario for security and correctness
    /// Validate content size is within limits
    fn validate_content_size(content: &str) -> Result<(), SecurityError> {
        (content.len() <= MAX_FILE_CONTENT_LENGTH).ok_or(SecurityError::ContentTooLarge {
            max: MAX_FILE_CONTENT_LENGTH,
            actual: content.len(),
        })
    }

    /// Validate command sequence length
    fn validate_command_sequence(commands: &[String]) -> Result<(), SecurityError> {
        (commands.len() <= MAX_COMMAND_SEQUENCE_LENGTH).ok_or(
            SecurityError::CommandSequenceTooLong {
                max: MAX_COMMAND_SEQUENCE_LENGTH,
            },
        )
    }

    fn validate_scenario(scenario: &Scenario) -> Result<(), ConfigError> {
        // Validate content sizes
        Self::validate_content_size(&scenario.setup.file_content)?;
        Self::validate_content_size(&scenario.target.file_content)?;

        // Validate collection sizes
        if scenario.hints.len() > MAX_HINTS {
            return Err(SecurityError::TooManyHints { max: MAX_HINTS }.into());
        }

        // Validate command sequences
        Self::validate_command_sequence(&scenario.solution.commands)?;

        Ok(())
    }
}

impl Default for ScenarioLoader {
    fn default() -> Self {
        Self::new()
    }
}

/// Embedded scenario assets compiled into the binary
pub mod embedded;

#[cfg(test)]
mod tests;
