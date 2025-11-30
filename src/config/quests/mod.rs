//! Quest template loading and validation
//!
//! Loads quest definitions from TOML files, following the same
//! security patterns as scenario loading.

use crate::security::limits::*;
use crate::security::{SecurityError, UserError, path_validator, sanitizer};
use serde::Deserialize;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

/// File wrapper for quest templates
#[derive(Deserialize, Debug, Clone)]
pub struct QuestsFile {
    pub metadata: QuestsMetadata,
    pub quests: Vec<QuestTemplate>,
}

/// File-level metadata
#[derive(Deserialize, Debug, Clone)]
pub struct QuestsMetadata {
    pub version: String,
    #[serde(default)]
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

    #[serde(rename = "type")]
    pub quest_type: QuestTypeTag,
    pub difficulty: QuestDifficulty,
    pub params: QuestParams,

    #[serde(default)]
    pub xp: Option<XpConfig>,

    #[serde(default)]
    pub conditions: QuestConditions,
}

/// Quest type discriminator (for TOML deserialization)
#[derive(Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum QuestTypeTag {
    CommandPractice,
    ScenarioCompletion,
    SpeedRun,
    TimeInvested,
    Exploration,
}

/// Quest difficulty (matches existing enum)
#[derive(Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum QuestDifficulty {
    Easy,
    Medium,
    Hard,
}

/// Quest-specific parameters (untagged enum for flexible deserialization)
#[derive(Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum QuestParams {
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
pub struct XpConfig {
    #[serde(default)]
    pub base_reward: Option<u32>,
}

/// Quest unlock conditions
#[derive(Deserialize, Debug, Clone, Default)]
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

/// Custom deserialization for ID field to validate format
fn validate_id_field<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;

    // Validate ID format: alphanumeric with underscores, max 64 chars
    if s.len() > 64 || !s.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return Err(serde::de::Error::custom(
            "Invalid ID: must be alphanumeric with underscores, max 64 chars",
        ));
    }

    Ok(s)
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

        // Parse TOML with proper error handling
        let quests_file: QuestsFile = toml::from_str(&content)
            .map_err(|e| UserError::from(SecurityError::InvalidToml(e.to_string())))?;

        let quests = quests_file.quests;

        // Validate quest count
        if quests.len() > MAX_QUEST_TEMPLATES_PER_FILE {
            return Err(UserError::from(SecurityError::TooManyScenarios {
                max: MAX_QUEST_TEMPLATES_PER_FILE,
                actual: quests.len(),
            }));
        }

        // Validate each quest template
        for quest in &quests {
            self.validate_quest(quest).map_err(UserError::from)?;
        }

        tracing::info!(count = quests.len(), "Successfully loaded quest templates");

        Ok(quests)
    }

    /// Validate a single quest template for security and correctness
    fn validate_quest(&self, quest: &QuestTemplate) -> Result<(), SecurityError> {
        // Validate string lengths
        if quest.name.len() > 100 {
            return Err(SecurityError::InvalidInput("Quest name too long".into()));
        }
        if quest.description.len() > 500 {
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
        if quest.conditions.requires_commands.len() > 20 {
            return Err(SecurityError::InvalidInput(
                "Too many required commands".into(),
            ));
        }
        if quest.conditions.requires_scenarios.len() > 20 {
            return Err(SecurityError::InvalidInput(
                "Too many required scenarios".into(),
            ));
        }

        // Validate that params match quest type
        self.validate_params_match_type(quest)?;

        Ok(())
    }

    /// Validate that quest parameters match the quest type
    fn validate_params_match_type(&self, quest: &QuestTemplate) -> Result<(), SecurityError> {
        match (&quest.quest_type, &quest.params) {
            (QuestTypeTag::CommandPractice, QuestParams::CommandPractice { command, target }) => {
                if command.is_empty() || command.len() > 10 {
                    return Err(SecurityError::InvalidInput("Invalid command name".into()));
                }
                if *target == 0 || *target > MAX_QUEST_TARGET {
                    return Err(SecurityError::InvalidInput(
                        "Invalid command practice target".into(),
                    ));
                }
            }
            (QuestTypeTag::ScenarioCompletion, QuestParams::ScenarioCompletion { target }) => {
                if *target == 0 || *target > MAX_QUEST_TARGET {
                    return Err(SecurityError::InvalidInput(
                        "Invalid scenario completion target".into(),
                    ));
                }
            }
            (
                QuestTypeTag::SpeedRun,
                QuestParams::SpeedRun {
                    scenario_id,
                    time_limit_seconds,
                },
            ) => {
                if scenario_id.is_empty() || scenario_id.len() > 64 {
                    return Err(SecurityError::InvalidInput("Invalid scenario ID".into()));
                }
                if *time_limit_seconds == 0 || *time_limit_seconds > MAX_SPEED_RUN_TIME_SECONDS {
                    return Err(SecurityError::InvalidInput(
                        "Invalid speed run time limit".into(),
                    ));
                }
            }
            (QuestTypeTag::TimeInvested, QuestParams::TimeInvested { target_minutes }) => {
                if *target_minutes == 0 || *target_minutes > MAX_QUEST_TARGET {
                    return Err(SecurityError::InvalidInput(
                        "Invalid time invested target".into(),
                    ));
                }
            }
            (QuestTypeTag::Exploration, QuestParams::Exploration { target_commands }) => {
                if *target_commands == 0 || *target_commands > MAX_QUEST_TARGET {
                    return Err(SecurityError::InvalidInput(
                        "Invalid exploration target".into(),
                    ));
                }
            }
            _ => {
                return Err(SecurityError::InvalidInput(
                    "Quest type does not match parameters".into(),
                ));
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

        let quest_type = self.params_to_quest_type();
        let difficulty = self.difficulty_to_runtime();

        Quest::new(
            self.id.clone(),
            quest_type,
            self.description.clone(),
            difficulty,
        )
    }

    /// Convert template parameters to runtime QuestType
    fn params_to_quest_type(&self) -> crate::gamification::QuestType {
        use crate::gamification::QuestType;

        match &self.params {
            QuestParams::CommandPractice { command, target } => QuestType::CommandPractice {
                command: command.clone(),
                target: *target,
                current: 0,
            },
            QuestParams::ScenarioCompletion { target } => QuestType::ScenarioCompletion {
                target: *target,
                current: 0,
            },
            QuestParams::SpeedRun {
                scenario_id,
                time_limit_seconds,
            } => QuestType::SpeedRun {
                scenario_id: scenario_id.clone(),
                time_limit: std::time::Duration::from_secs(*time_limit_seconds),
            },
            QuestParams::TimeInvested { target_minutes } => QuestType::TimeInvested {
                target_minutes: *target_minutes,
                current_minutes: 0,
            },
            QuestParams::Exploration { target_commands } => QuestType::Exploration {
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

#[cfg(test)]
mod tests;
