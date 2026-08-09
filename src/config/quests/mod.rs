//! Quest template loading and validation
//!
//! Loads quest definitions from TOML files, following the same
//! security patterns as scenario loading.

use crate::security::limits::{
    MAX_CUSTOM_XP_REWARD, MAX_LOCALE_LENGTH, MAX_QUEST_DESCRIPTION_LENGTH, MAX_QUEST_NAME_LENGTH,
    MAX_QUEST_TARGET, MAX_QUEST_TEMPLATES_PER_FILE, MAX_REQUIRED_CONDITIONS,
    MAX_SCENARIO_FILE_SIZE, MAX_SPEED_RUN_TIME_SECONDS, MAX_VERSION_LENGTH,
};
use crate::security::validators::validate_id_field;
use crate::security::{SecurityError, UserError, path_validator, sanitizer};
use serde::Deserialize;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

/// File wrapper for quest templates
#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct QuestsFile {
    pub metadata: QuestsMetadata,
    pub quests: Vec<QuestTemplate>,
}

/// File-level metadata
#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct QuestsMetadata {
    #[serde(deserialize_with = "validate_version_field")]
    pub version: String,
    #[serde(default, deserialize_with = "validate_locale_field")]
    pub locale: Option<String>,
}

/// Quest template definition from TOML
#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct QuestTemplate {
    #[serde(deserialize_with = "validate_id_field")]
    pub id: String,
    pub name: String,
    pub description: String,

    pub difficulty: QuestDifficulty,

    #[serde(flatten)]
    pub spec: QuestSpec,

    #[serde(default)]
    pub xp: Option<XpConfig>,

    #[serde(default)]
    pub conditions: QuestConditions,
}

/// Quest difficulty (matches existing enum)
#[derive(Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum QuestDifficulty {
    Easy,
    Medium,
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
    CommandPractice {
        command: String,
        target: u32,
    },
    ScenarioCompletion {
        target: u32,
    },
    SpeedRun {
        scenario_id: String,
        time_limit_seconds: u64,
    },
    TimeInvested {
        target_minutes: u32,
    },
    Exploration {
        target_commands: u32,
    },
}

/// Optional XP configuration override
#[derive(Deserialize, Debug, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct XpConfig {
    #[serde(default)]
    pub base_reward: Option<u32>,
}

/// Quest unlock conditions
///
/// INVARIANT: min_level <= max_level when both are set, checked by
/// `QuestLoader::validate_quest`.
#[derive(Deserialize, Debug, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct QuestConditions {
    #[serde(default)]
    pub min_level: Option<u32>,
    #[serde(default)]
    pub max_level: Option<u32>,
    #[serde(default)]
    pub requires_commands: Vec<String>,
    #[serde(default)]
    pub requires_scenarios: Vec<String>,
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
            "Invalid version: max {} characters",
            MAX_VERSION_LENGTH
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
                "Invalid locale: must be 1-{} characters",
                MAX_LOCALE_LENGTH
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
    /// # Errors
    /// Returns UserError if file cannot be loaded or validation fails
    pub fn load_for_locale(&self, locale: &str) -> Result<Vec<QuestTemplate>, UserError> {
        let path = PathBuf::from("./quests").join(locale).join("daily.toml");
        self.load(&path)
    }

    /// Load quest templates from a TOML file
    ///
    /// # Security Validations
    /// - Path must be within allowed directories (prevents path traversal)
    /// - File size must not exceed MAX_SCENARIO_FILE_SIZE
    /// - TOML must be valid
    /// - Quest count must not exceed MAX_QUEST_TEMPLATES_PER_FILE
    /// - Each quest template is validated for parameter correctness
    ///
    /// # Errors
    /// Returns UserError with sanitized message if any validation fails
    pub fn load(&self, path: &Path) -> Result<Vec<QuestTemplate>, UserError> {
        // Validate path to prevent path traversal attacks
        let canonical = path_validator::validate_path(path, &self.allowed_base_paths)
            .map_err(UserError::from)?;

        // Validate file size to prevent resource exhaustion
        path_validator::validate_file_size(&canonical, MAX_SCENARIO_FILE_SIZE)
            .map_err(UserError::from)?;

        // Log with sanitized path (doesn't leak full path)
        tracing::info!(
            file = %sanitizer::sanitize_path_for_logging(&canonical),
            "Loading quest template file"
        );

        // Read file content
        let content = fs::read_to_string(&canonical).map_err(|e| {
            tracing::error!("Failed to read quest template file: {}", e);
            UserError::ScenarioLoadError
        })?;

        // Parse, count-check, and validate each quest template
        let quests = super::loader::parse_and_validate::<QuestsFile>(
            &content,
            MAX_QUEST_TEMPLATES_PER_FILE,
            |q| self.validate_quest(q),
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
    /// Returns UserError if no embedded data exists for the locale or
    /// if validation fails.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let loader = QuestLoader::new();
    /// let templates = loader.load_from_embedded("en")?;
    /// assert!(!templates.is_empty());
    /// ```
    pub fn load_from_embedded(&self, locale: &str) -> Result<Vec<QuestTemplate>, UserError> {
        let content = embedded::get_embedded_quests(locale).ok_or_else(|| {
            tracing::warn!(locale = locale, "No embedded quests for locale");
            UserError::ScenarioLoadError
        })?;

        tracing::info!(locale = locale, "Loading quests from embedded data");

        // Parse, count-check, and validate each quest template
        let quests = super::loader::parse_and_validate::<QuestsFile>(
            content,
            MAX_QUEST_TEMPLATES_PER_FILE,
            |q| self.validate_quest(q),
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
    fn validate_quest(&self, quest: &QuestTemplate) -> Result<(), SecurityError> {
        // Validate string lengths
        if quest.name.len() > MAX_QUEST_NAME_LENGTH {
            return Err(SecurityError::InvalidInput("Quest name too long".into()));
        }
        if quest.description.len() > MAX_QUEST_DESCRIPTION_LENGTH {
            return Err(SecurityError::InvalidInput(
                "Quest description too long".into(),
            ));
        }

        // Validate custom XP reward
        if let Some(xp_config) = &quest.xp
            && let Some(reward) = xp_config.base_reward
            && reward > MAX_CUSTOM_XP_REWARD
        {
            return Err(SecurityError::InvalidInput(
                "Custom XP reward exceeds maximum".into(),
            ));
        }

        // Validate conditions
        if quest.conditions.requires_commands.len() > MAX_REQUIRED_CONDITIONS {
            return Err(SecurityError::InvalidInput(
                "Too many required commands".into(),
            ));
        }
        if quest.conditions.requires_scenarios.len() > MAX_REQUIRED_CONDITIONS {
            return Err(SecurityError::InvalidInput(
                "Too many required scenarios".into(),
            ));
        }

        // Validate min_level <= max_level consistency
        if let (Some(min), Some(max)) = (quest.conditions.min_level, quest.conditions.max_level)
            && min > max
        {
            return Err(SecurityError::InvalidInput(
                "min_level cannot be greater than max_level".into(),
            ));
        }

        // Validate spec parameter bounds
        self.validate_spec(quest)?;

        Ok(())
    }

    /// Validate quest spec parameters are within bounds
    fn validate_spec(&self, quest: &QuestTemplate) -> Result<(), SecurityError> {
        match &quest.spec {
            QuestSpec::CommandPractice { command, target } => {
                if command.is_empty() || command.len() > 10 {
                    return Err(SecurityError::InvalidInput("Invalid command name".into()));
                }
                if *target == 0 || *target > MAX_QUEST_TARGET {
                    return Err(SecurityError::InvalidInput(
                        "Invalid command practice target".into(),
                    ));
                }
            }
            QuestSpec::ScenarioCompletion { target } => {
                if *target == 0 || *target > MAX_QUEST_TARGET {
                    return Err(SecurityError::InvalidInput(
                        "Invalid scenario completion target".into(),
                    ));
                }
            }
            QuestSpec::SpeedRun {
                scenario_id,
                time_limit_seconds,
            } => {
                if scenario_id.is_empty() || scenario_id.len() > 64 {
                    return Err(SecurityError::InvalidInput("Invalid scenario ID".into()));
                }
                if *time_limit_seconds == 0 || *time_limit_seconds > MAX_SPEED_RUN_TIME_SECONDS {
                    return Err(SecurityError::InvalidInput(
                        "Invalid speed run time limit".into(),
                    ));
                }
            }
            QuestSpec::TimeInvested { target_minutes } => {
                if *target_minutes == 0 || *target_minutes > MAX_QUEST_TARGET {
                    return Err(SecurityError::InvalidInput(
                        "Invalid time invested target".into(),
                    ));
                }
            }
            QuestSpec::Exploration { target_commands } => {
                if *target_commands == 0 || *target_commands > MAX_QUEST_TARGET {
                    return Err(SecurityError::InvalidInput(
                        "Invalid exploration target".into(),
                    ));
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

impl QuestTemplate {
    /// Convert template to runtime quest instance
    ///
    /// # Examples
    /// ```ignore
    /// let template = QuestTemplate { /* ... */ };
    /// let quest = template.to_quest();
    /// ```
    pub fn to_quest(&self) -> crate::gamification::Quest {
        use crate::gamification::Quest;

        let quest_type = self.spec_to_quest_type();
        let difficulty = self.difficulty_to_runtime();

        Quest::new(
            self.id.clone(),
            quest_type,
            self.description.clone(),
            difficulty,
        )
    }

    /// Convert template spec to runtime QuestType
    fn spec_to_quest_type(&self) -> crate::gamification::QuestType {
        use crate::gamification::QuestType;

        match &self.spec {
            QuestSpec::CommandPractice { command, target } => QuestType::CommandPractice {
                command: command.clone(),
                target: *target,
                current: 0,
            },
            QuestSpec::ScenarioCompletion { target } => QuestType::ScenarioCompletion {
                target: *target,
                current: 0,
            },
            QuestSpec::SpeedRun {
                scenario_id,
                time_limit_seconds,
            } => QuestType::SpeedRun {
                scenario_id: scenario_id.clone(),
                time_limit: std::time::Duration::from_secs(*time_limit_seconds),
            },
            QuestSpec::TimeInvested { target_minutes } => QuestType::TimeInvested {
                target_minutes: *target_minutes,
                current_minutes: 0,
            },
            QuestSpec::Exploration { target_commands } => QuestType::Exploration {
                target_commands: *target_commands,
                commands_used: HashSet::new(),
            },
        }
    }

    /// Convert template difficulty to runtime difficulty
    fn difficulty_to_runtime(&self) -> crate::gamification::QuestDifficulty {
        use crate::gamification::QuestDifficulty as RuntimeDifficulty;

        match self.difficulty {
            QuestDifficulty::Easy => RuntimeDifficulty::Easy,
            QuestDifficulty::Medium => RuntimeDifficulty::Medium,
            QuestDifficulty::Hard => RuntimeDifficulty::Hard,
        }
    }
}

/// Embedded quest assets compiled into the binary
pub mod embedded;

#[cfg(test)]
mod tests;
