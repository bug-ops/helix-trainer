//! Quest template loading and validation
//!
//! Loads quest definitions from TOML files, following the same
//! security patterns as scenario loading.

use crate::ids::{NormalizedCommand, QuestId, ScenarioId};
use crate::limits::{
    MAX_CUSTOM_XP_REWARD, MAX_LOCALE_LENGTH, MAX_QUEST_TARGET, MAX_QUEST_TEMPLATES_PER_FILE,
    MAX_REQUIRED_CONDITIONS, MAX_SCENARIO_FILE_SIZE, MAX_SPEED_RUN_TIME_SECONDS,
    MAX_VERSION_LENGTH,
};
use crate::security::{SecurityError, UserError, path_validator, sanitizer};

mod error;
pub use error::QuestTemplateError;

/// Longest command name a command-practice quest may target.
const MAX_QUEST_COMMAND_NAME_LENGTH: usize = 10;
use serde::Deserialize;
use std::path::{Path, PathBuf};

/// File wrapper for quest templates
#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct QuestsFile {
    /// File-level metadata.
    pub metadata: QuestsMetadata,
    /// Quest definitions in file order.
    pub quests: Vec<QuestTemplate>,
}

/// File-level metadata
#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct QuestsMetadata {
    /// Schema version of the quest file; validated for length and emptiness.
    #[serde(deserialize_with = "validate_version_field")]
    pub version: String,
    /// Locale the templates are written for, when the file declares one.
    #[serde(default, deserialize_with = "validate_locale_field")]
    pub locale: Option<String>,
}

/// Quest template definition from TOML
#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct QuestTemplate {
    /// Unique identifier, validated as a safe id.
    pub id: QuestId,

    /// Difficulty tier, which selects the default XP reward.
    pub difficulty: QuestDifficulty,

    /// Quest type with its type-specific parameters.
    #[serde(flatten)]
    pub spec: QuestSpec,

    /// Optional override of the difficulty-based XP reward.
    #[serde(default)]
    pub xp: Option<XpConfig>,

    /// Conditions that gate when the quest may be offered.
    #[serde(default)]
    pub conditions: QuestConditions,
}

/// Quest difficulty (matches existing enum)
#[derive(Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum QuestDifficulty {
    /// Short quests suited to new players.
    Easy,
    /// Mid-length quests.
    Medium,
    /// Long or demanding quests.
    Hard,
}

/// Quest type and its type-specific parameters
///
/// Adjacently tagged by the TOML `type` field, with the variant's fields read
/// from the nested `params` table (`#[serde(tag = "type", content = "params")]`).
/// This makes a `type`/`params` shape mismatch a deserialization error instead
/// of a runtime validation concern.
#[derive(Deserialize, Debug, Clone)]
#[serde(tag = "type", content = "params", rename_all = "snake_case")]
pub enum QuestSpec {
    /// Execute one command a number of times.
    CommandPractice {
        /// Command whose executions are counted.
        command: NormalizedCommand,
        /// Number of executions required.
        target: u32,
    },
    /// Complete a number of scenarios.
    ScenarioCompletion {
        /// Number of completions required.
        target: u32,
    },
    /// Finish one specific scenario within a time limit.
    SpeedRun {
        /// Scenario that must be completed.
        scenario_id: ScenarioId,
        /// Time limit in seconds.
        time_limit_seconds: u64,
    },
    /// Spend a total amount of time practicing.
    TimeInvested {
        /// Practice time required, in minutes.
        target_minutes: u32,
    },
    /// Use a number of distinct commands.
    Exploration {
        /// Number of distinct commands required.
        target_commands: u32,
    },
}

/// Optional XP configuration override
#[derive(Deserialize, Debug, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct XpConfig {
    /// Base XP reward replacing the difficulty default.
    #[serde(default)]
    pub base_reward: Option<u32>,
}

/// Quest unlock conditions
///
/// INVARIANT: `min_level` <= `max_level` when both are set, checked by
/// `QuestLoader::validate_quest`.
#[derive(Deserialize, Debug, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct QuestConditions {
    /// Lowest player level at which the quest is offered.
    #[serde(default)]
    pub min_level: Option<u32>,
    /// Highest player level at which the quest is offered.
    #[serde(default)]
    pub max_level: Option<u32>,
    /// Commands the player must already have practiced.
    #[serde(default)]
    pub requires_commands: Vec<NormalizedCommand>,
    /// Scenarios the player must already have completed.
    #[serde(default)]
    pub requires_scenarios: Vec<ScenarioId>,
}

/// Custom deserialization for version field to validate format
fn validate_version_field<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;

    // Version must not be empty and max MAX_VERSION_LENGTH chars
    if s.is_empty() {
        return Err(serde::de::Error::custom("Invalid version: cannot be empty"));
    }
    if s.len() > MAX_VERSION_LENGTH {
        return Err(serde::de::Error::custom(format!(
            "Invalid version: max {MAX_VERSION_LENGTH} characters"
        )));
    }

    Ok(s)
}

/// Custom deserialization for locale field to validate format
fn validate_locale_field<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let opt: Option<String> = Option::deserialize(deserializer)?;

    if let Some(ref locale) = opt {
        // Locale should be 1-MAX_LOCALE_LENGTH chars (e.g., "en", "ru", "en_US")
        if locale.is_empty() || locale.len() > MAX_LOCALE_LENGTH {
            return Err(serde::de::Error::custom(format!(
                "Invalid locale: must be 1-{MAX_LOCALE_LENGTH} characters"
            )));
        }
        // Only allow alphanumeric and underscore
        if !locale.chars().all(|c| c.is_alphanumeric() || c == '_') {
            return Err(serde::de::Error::custom(
                "Invalid locale: must be alphanumeric with underscores",
            ));
        }
    }

    Ok(opt)
}

/// Quest loader with security validation
///
/// Two loading paths exist:
///
/// - [`load_from_embedded`](Self::load_from_embedded) reads quests compiled
///   into the binary. This is the only path production code takes
///   (`src/gamification/quests.rs`).
/// - [`load_for_locale`](Self::load_for_locale) and [`load`](Self::load) read
///   from the filesystem, restricted to `allowed_base_paths`. They are
///   retained deliberately for a planned "load custom quest packs from disk"
///   feature and are currently exercised only by unit tests — they are not
///   dead code awaiting removal.
pub struct QuestLoader {
    allowed_base_paths: Vec<PathBuf>,
}

impl QuestLoader {
    /// Create a new quest loader with default allowed paths
    pub fn new() -> Self {
        Self {
            allowed_base_paths: vec![
                PathBuf::from("./quests"),
                PathBuf::from("/usr/share/helix-trainer/quests"),
            ],
        }
    }

    /// Create a loader with custom allowed paths for testing
    pub fn with_allowed_paths(paths: Vec<PathBuf>) -> Self {
        Self {
            allowed_base_paths: paths,
        }
    }

    /// Load all quest templates for a given locale
    ///
    /// Tries `<base>/<locale>/daily.toml` under each of `allowed_base_paths`
    /// in order and returns the first that loads successfully.
    ///
    /// # Errors
    /// Returns `UserError` if no candidate path loads or validation fails
    ///
    /// Not reached from production code today — see the [`QuestLoader`] type
    /// docs for why this filesystem path is retained.
    pub fn load_for_locale(&self, locale: &str) -> Result<Vec<QuestTemplate>, UserError> {
        let mut last_err = None;
        for base in &self.allowed_base_paths {
            let path = base.join(locale).join("daily.toml");
            match self.load(&path) {
                Ok(templates) => return Ok(templates),
                Err(e) => last_err = Some(e),
            }
        }
        Err(last_err.unwrap_or(UserError::scenario_load_error()))
    }

    /// Load quest templates from a TOML file
    ///
    /// # Security Validations
    /// - Path must be within allowed directories (prevents path traversal)
    /// - File size must not exceed `MAX_SCENARIO_FILE_SIZE`
    /// - TOML must be valid
    /// - Quest count must not exceed `MAX_QUEST_TEMPLATES_PER_FILE`
    /// - Each quest template is validated for parameter correctness
    ///
    /// # Errors
    /// Returns `UserError` with sanitized message if any validation fails
    ///
    /// Not reached from production code today — see the [`QuestLoader`] type
    /// docs for why this filesystem path is retained.
    pub fn load(&self, path: &Path) -> Result<Vec<QuestTemplate>, UserError> {
        // Validate path to prevent path traversal attacks
        let canonical = path_validator::validate_path(path, &self.allowed_base_paths)
            .map_err(UserError::from)?;

        // Log with sanitized path (doesn't leak full path)
        tracing::info!(
            file = %sanitizer::sanitize_path_for_logging(&canonical),
            "Loading quest template file"
        );

        // Bounded read of a regular file to prevent resource exhaustion
        let content = path_validator::read_capped(&canonical, MAX_SCENARIO_FILE_SIZE)
            .map_err(|e| UserError::from(SecurityError::from(e)))?;

        // Parse, count-check, and validate each quest template
        let quests = super::loader::parse_and_validate::<QuestsFile>(
            &content,
            MAX_QUEST_TEMPLATES_PER_FILE,
            Self::validate_quest,
        )
        .map_err(UserError::from)?;

        tracing::info!(count = quests.len(), "Successfully loaded quest templates");

        Ok(quests)
    }

    /// Load quest templates from embedded TOML content
    ///
    /// This method loads quests from compile-time embedded content,
    /// eliminating the need for filesystem access. It applies the same
    /// validation as filesystem loading.
    ///
    /// # Arguments
    ///
    /// * `locale` - Locale code (e.g., "en")
    ///
    /// # Errors
    ///
    /// Returns `UserError` if no embedded data exists for the locale or
    /// if validation fails.
    ///
    /// # Examples
    ///
    /// ```
    /// use helix_trainer::config::quests::QuestLoader;
    ///
    /// let loader = QuestLoader::new();
    /// let templates = loader.load_from_embedded("en")?;
    /// assert!(!templates.is_empty());
    /// # Ok::<(), helix_trainer::security::UserError>(())
    /// ```
    pub fn load_from_embedded(&self, locale: &str) -> Result<Vec<QuestTemplate>, UserError> {
        let content = embedded::get_embedded_quests(locale).ok_or_else(|| {
            tracing::warn!(locale = locale, "No embedded quests for locale");
            UserError::scenario_load_error()
        })?;

        tracing::info!(locale = locale, "Loading quests from embedded data");

        // Parse, count-check, and validate each quest template
        let quests = super::loader::parse_and_validate::<QuestsFile>(
            content,
            MAX_QUEST_TEMPLATES_PER_FILE,
            Self::validate_quest,
        )
        .map_err(|e| {
            tracing::error!(
                locale = locale,
                "Failed to load embedded quest file: {:?}",
                e
            );
            UserError::from(e)
        })?;

        tracing::info!(
            locale = locale,
            count = quests.len(),
            "Successfully loaded quest templates from embedded data"
        );

        Ok(quests)
    }

    /// Validate a single quest template for security and correctness
    fn validate_quest(quest: &QuestTemplate) -> Result<(), SecurityError> {
        Ok(Self::check_quest(quest)?)
    }

    fn check_quest(quest: &QuestTemplate) -> Result<(), QuestTemplateError> {
        if let Some(xp_config) = &quest.xp
            && let Some(reward) = xp_config.base_reward
            && reward > MAX_CUSTOM_XP_REWARD
        {
            return Err(QuestTemplateError::XpRewardTooHigh {
                max: MAX_CUSTOM_XP_REWARD,
                actual: reward,
            });
        }

        let commands = quest.conditions.requires_commands.len();
        if commands > MAX_REQUIRED_CONDITIONS {
            return Err(QuestTemplateError::TooManyRequiredCommands {
                max: MAX_REQUIRED_CONDITIONS,
                actual: commands,
            });
        }
        let scenarios = quest.conditions.requires_scenarios.len();
        if scenarios > MAX_REQUIRED_CONDITIONS {
            return Err(QuestTemplateError::TooManyRequiredScenarios {
                max: MAX_REQUIRED_CONDITIONS,
                actual: scenarios,
            });
        }

        if let (Some(min), Some(max)) = (quest.conditions.min_level, quest.conditions.max_level)
            && min > max
        {
            return Err(QuestTemplateError::LevelRangeInverted { min, max });
        }

        Self::check_spec(&quest.spec)
    }

    /// Validate quest spec parameters are within bounds
    fn check_spec(spec: &QuestSpec) -> Result<(), QuestTemplateError> {
        match spec {
            QuestSpec::CommandPractice { command, target } => {
                if command.as_str().len() > MAX_QUEST_COMMAND_NAME_LENGTH {
                    return Err(QuestTemplateError::CommandNameTooLong {
                        max: MAX_QUEST_COMMAND_NAME_LENGTH,
                        actual: command.as_str().len(),
                    });
                }
                if !(1..=MAX_QUEST_TARGET).contains(target) {
                    return Err(QuestTemplateError::InvalidCommandPracticeTarget {
                        max: MAX_QUEST_TARGET,
                        actual: *target,
                    });
                }
            }
            QuestSpec::ScenarioCompletion { target } => {
                if !(1..=MAX_QUEST_TARGET).contains(target) {
                    return Err(QuestTemplateError::InvalidScenarioCompletionTarget {
                        max: MAX_QUEST_TARGET,
                        actual: *target,
                    });
                }
            }
            QuestSpec::SpeedRun {
                time_limit_seconds, ..
            } => {
                if !(1..=MAX_SPEED_RUN_TIME_SECONDS).contains(time_limit_seconds) {
                    return Err(QuestTemplateError::InvalidSpeedRunTimeLimit {
                        max: MAX_SPEED_RUN_TIME_SECONDS,
                        actual: *time_limit_seconds,
                    });
                }
            }
            QuestSpec::TimeInvested { target_minutes } => {
                if !(1..=MAX_QUEST_TARGET).contains(target_minutes) {
                    return Err(QuestTemplateError::InvalidTimeInvestedTarget {
                        max: MAX_QUEST_TARGET,
                        actual: *target_minutes,
                    });
                }
            }
            QuestSpec::Exploration { target_commands } => {
                if !(1..=MAX_QUEST_TARGET).contains(target_commands) {
                    return Err(QuestTemplateError::InvalidExplorationTarget {
                        max: MAX_QUEST_TARGET,
                        actual: *target_commands,
                    });
                }
            }
        }

        Ok(())
    }
}

impl Default for QuestLoader {
    fn default() -> Self {
        Self::new()
    }
}

/// Embedded quest assets compiled into the binary
pub mod embedded;

#[cfg(test)]
mod tests;
