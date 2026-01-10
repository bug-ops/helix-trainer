//! Scenario loading and validation
//!
//! This module handles loading TOML scenario files with security validations.

use crate::security::limits::*;
use crate::security::{SecurityError, UserError, path_validator, sanitizer};
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};

/// Wrapper for scenarios array in TOML file
#[derive(Deserialize, Debug, Clone)]
pub struct ScenariosFile {
    pub scenarios: Vec<Scenario>,
}

/// Scenario metadata for filtering, sorting, and quest generation
#[derive(Deserialize, Debug, Clone, Default)]
pub struct ScenarioMetadata {
    /// Scenario category (e.g., movement, editing, clipboard)
    #[serde(default)]
    pub category: Option<ScenarioCategory>,

    /// Difficulty level (beginner, intermediate, advanced)
    #[serde(default)]
    pub difficulty: Option<Difficulty>,

    /// Tags for flexible filtering (e.g., ["delete", "motion", "word"])
    #[serde(default)]
    pub tags: Vec<String>,

    /// Commands taught in this scenario (e.g., ["d", "w"])
    #[serde(default)]
    pub commands_taught: Vec<String>,

    /// Prerequisite scenario IDs that should be completed first
    #[serde(default)]
    pub prerequisites: Vec<String>,

    /// Estimated time to complete scenario in seconds
    #[serde(default)]
    pub estimated_time_seconds: Option<u32>,

    /// Locale/language code (e.g., "en", "ru")
    #[serde(default)]
    pub locale: Option<String>,
}

/// Scenario category for organization and filtering
#[derive(Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum ScenarioCategory {
    Movement,
    Editing,
    Clipboard,
    Search,
    Selection,
    TextObjects,
    Advanced,
    Multi, // Multiple categories
    Other,
}

/// Difficulty level for progressive learning
#[derive(Deserialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "lowercase")]
pub enum Difficulty {
    Beginner,
    Intermediate,
    Advanced,
}

/// Scenario definition
#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct Scenario {
    #[serde(deserialize_with = "validate_id_field")]
    pub id: String,

    pub name: String,
    pub description: String,
    pub setup: Setup,
    pub target: TargetState,
    pub solution: Solution,

    #[serde(default)]
    pub alternatives: Vec<AlternativeSolution>,

    #[serde(default)]
    pub hints: Vec<String>,

    pub scoring: ScoringConfig,

    /// Metadata for filtering, sorting, and quest generation (optional for backward compatibility)
    #[serde(default)]
    pub metadata: Option<ScenarioMetadata>,
}

/// Initial editor setup
#[derive(Deserialize, Debug, Clone)]
pub struct Setup {
    pub file_content: String,
    pub cursor_position: (usize, usize),
    /// Optional selection range: [start_line, start_col, end_line, end_col]
    #[serde(default)]
    pub selection: Option<[usize; 4]>,
}

/// Target state to achieve
#[derive(Deserialize, Debug, Clone)]
pub struct TargetState {
    pub file_content: String,
    pub cursor_position: (usize, usize),
    /// Optional selection range: [start_line, start_col, end_line, end_col]
    #[serde(default)]
    pub selection: Option<[usize; 4]>,
}

/// Optimal solution
#[derive(Deserialize, Debug, Clone)]
pub struct Solution {
    pub commands: Vec<String>,
    pub description: String,
}

/// Alternative solution
#[derive(Deserialize, Debug, Clone)]
pub struct AlternativeSolution {
    pub commands: Vec<String>,
    pub points_multiplier: f32,
    pub description: String,
}

/// Scoring configuration
#[derive(Deserialize, Debug, Clone)]
pub struct ScoringConfig {
    pub optimal_count: usize,
    pub max_points: u32,
    pub tolerance: usize,
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

/// Secure scenario loader with path validation and content verification
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

    /// Detect available locales by scanning the scenarios directory
    ///
    /// Returns a list of locale codes (e.g., ["en", "ru"]) found as subdirectories
    /// in the scenarios directory.
    pub fn available_locales() -> Vec<String> {
        let scenarios_path = Path::new("./scenarios");

        if !scenarios_path.exists() || !scenarios_path.is_dir() {
            tracing::warn!("Scenarios directory not found");
            return vec!["en".to_string()]; // Fallback to English
        }

        let mut locales = Vec::new();

        if let Ok(entries) = fs::read_dir(scenarios_path) {
            for entry in entries.flatten() {
                if let Ok(file_type) = entry.file_type()
                    && file_type.is_dir()
                    && let Some(name) = entry.file_name().to_str()
                {
                    // Validate locale code: 2-letter ISO code
                    if name.len() == 2 && name.chars().all(|c| c.is_ascii_lowercase()) {
                        locales.push(name.to_string());
                    }
                }
            }
        }

        // Always ensure English is available as fallback
        if locales.is_empty() {
            locales.push("en".to_string());
        }

        locales.sort();
        locales
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
    /// Returns UserError if any file fails to load or validation fails
    pub fn load_directory(&self, dir_path: &Path) -> Result<Vec<Scenario>, UserError> {
        // Validate directory path
        let canonical = path_validator::validate_path(dir_path, &self.allowed_base_paths)
            .map_err(UserError::from)?;

        if !canonical.is_dir() {
            tracing::error!("Path is not a directory");
            return Err(UserError::from(SecurityError::InvalidPath));
        }

        tracing::info!(
            dir = %sanitizer::sanitize_path_for_logging(&canonical),
            "Loading scenarios from directory"
        );

        // Recursively walk directory and collect all .toml files
        let (all_scenarios, file_count) = self.visit_toml_files(&canonical)?;

        if all_scenarios.is_empty() {
            tracing::warn!("No scenario files found in directory");
            return Err(UserError::ScenarioLoadError);
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
    /// Returns tuple of (scenarios, file_count)
    fn visit_toml_files(&self, dir: &Path) -> Result<(Vec<Scenario>, usize), UserError> {
        let entries = fs::read_dir(dir).map_err(|e| {
            tracing::error!("Failed to read directory: {}", e);
            UserError::ScenarioLoadError
        })?;

        let mut scenarios = Vec::new();
        let mut file_count = 0;

        for entry in entries {
            let entry = entry.map_err(|e| {
                tracing::error!("Failed to read directory entry: {}", e);
                UserError::ScenarioLoadError
            })?;

            let path = entry.path();

            if path.is_dir() {
                // Recursively visit subdirectories
                let (sub_scenarios, sub_count) = self.visit_toml_files(&path)?;
                scenarios.extend(sub_scenarios);
                file_count += sub_count;
            } else if path.extension().and_then(|s| s.to_str()) == Some("toml") {
                // Load scenarios from this file
                let file_scenarios = self.load(&path).map_err(|e| {
                    tracing::error!(
                        file = %sanitizer::sanitize_path_for_logging(&path),
                        "Failed to load scenario file: {:?}",
                        e
                    );
                    e
                })?;

                file_count += 1;
                scenarios.extend(file_scenarios);
            }
        }

        Ok((scenarios, file_count))
    }

    /// Load scenarios from a TOML file with comprehensive security validations
    ///
    /// # Security Validations
    /// - Path must be within allowed directories (prevents path traversal)
    /// - File size must not exceed MAX_SCENARIO_FILE_SIZE
    /// - TOML must be valid
    /// - Scenario count must not exceed MAX_SCENARIOS_PER_FILE
    /// - Each scenario is validated for content size, cursor positions, etc.
    ///
    /// # Errors
    /// Returns UserError with sanitized message if any validation fails
    pub fn load(&self, path: &Path) -> Result<Vec<Scenario>, UserError> {
        // Validate path to prevent path traversal attacks
        let canonical = path_validator::validate_path(path, &self.allowed_base_paths)
            .map_err(UserError::from)?;

        // Validate file size to prevent resource exhaustion
        path_validator::validate_file_size(&canonical, MAX_SCENARIO_FILE_SIZE)
            .map_err(UserError::from)?;

        // Log with sanitized path (doesn't leak full path)
        tracing::info!(
            file = %sanitizer::sanitize_path_for_logging(&canonical),
            "Loading scenario file"
        );

        // Read file content
        let content = fs::read_to_string(&canonical).map_err(|e| {
            tracing::error!("Failed to read scenario file: {}", e);
            UserError::ScenarioLoadError
        })?;

        // Parse TOML with proper error handling
        let scenarios_file: ScenariosFile = toml::from_str(&content)
            .map_err(|e| UserError::from(SecurityError::InvalidToml(e.to_string())))?;

        let scenarios = scenarios_file.scenarios;

        // Validate scenario count
        if scenarios.len() > MAX_SCENARIOS_PER_FILE {
            return Err(UserError::from(SecurityError::TooManyScenarios {
                max: MAX_SCENARIOS_PER_FILE,
                actual: scenarios.len(),
            }));
        }

        // Validate each scenario
        for scenario in &scenarios {
            self.validate_scenario(scenario).map_err(UserError::from)?;
        }

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
    /// Returns UserError if no embedded data exists for the locale or
    /// if validation fails.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let loader = ScenarioLoader::new();
    /// let scenarios = loader.load_from_embedded("en")?;
    /// assert!(!scenarios.is_empty());
    /// ```
    pub fn load_from_embedded(&self, locale: &str) -> Result<Vec<Scenario>, UserError> {
        let embedded_contents = embedded::get_embedded_scenarios(locale);

        if embedded_contents.is_empty() {
            tracing::warn!(locale = locale, "No embedded scenarios for locale");
            return Err(UserError::ScenarioLoadError);
        }

        tracing::info!(
            locale = locale,
            file_count = embedded_contents.len(),
            "Loading scenarios from embedded data"
        );

        let mut all_scenarios = Vec::new();

        for (index, content) in embedded_contents.iter().enumerate() {
            // Parse TOML content
            let scenarios_file: ScenariosFile = toml::from_str(content).map_err(|e| {
                tracing::error!(
                    locale = locale,
                    index = index,
                    "Failed to parse embedded scenario TOML: {}",
                    e
                );
                UserError::from(SecurityError::InvalidToml(e.to_string()))
            })?;

            let scenarios = scenarios_file.scenarios;

            // Validate scenario count per file
            if scenarios.len() > MAX_SCENARIOS_PER_FILE {
                return Err(UserError::from(SecurityError::TooManyScenarios {
                    max: MAX_SCENARIOS_PER_FILE,
                    actual: scenarios.len(),
                }));
            }

            // Validate each scenario
            for scenario in &scenarios {
                self.validate_scenario(scenario).map_err(UserError::from)?;
            }

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
    fn validate_content_size(&self, content: &str) -> Result<(), SecurityError> {
        if content.len() > MAX_FILE_CONTENT_LENGTH {
            Err(SecurityError::ContentTooLarge {
                max: MAX_FILE_CONTENT_LENGTH,
                actual: content.len(),
            })
        } else {
            Ok(())
        }
    }

    /// Validate command sequence length
    fn validate_command_sequence(&self, commands: &[String]) -> Result<(), SecurityError> {
        if commands.len() > MAX_COMMAND_SEQUENCE_LENGTH {
            Err(SecurityError::CommandSequenceTooLong {
                max: MAX_COMMAND_SEQUENCE_LENGTH,
            })
        } else {
            Ok(())
        }
    }

    fn validate_scenario(&self, scenario: &Scenario) -> Result<(), SecurityError> {
        // Validate content sizes
        self.validate_content_size(&scenario.setup.file_content)?;
        self.validate_content_size(&scenario.target.file_content)?;

        // Validate cursor positions
        self.validate_cursor_position(scenario.setup.cursor_position)?;
        self.validate_cursor_position(scenario.target.cursor_position)?;

        // Validate collection sizes
        if scenario.hints.len() > MAX_HINTS {
            return Err(SecurityError::TooManyHints { max: MAX_HINTS });
        }

        if scenario.alternatives.len() > MAX_ALTERNATIVES {
            return Err(SecurityError::TooManyAlternatives {
                max: MAX_ALTERNATIVES,
            });
        }

        // Validate scoring configuration
        if scenario.scoring.optimal_count == 0 {
            return Err(SecurityError::InvalidScoringConfig);
        }

        // Validate command sequences
        self.validate_command_sequence(&scenario.solution.commands)?;
        for alt in &scenario.alternatives {
            self.validate_command_sequence(&alt.commands)?;
        }

        Ok(())
    }

    /// Validate cursor position bounds
    fn validate_cursor_position(&self, pos: (usize, usize)) -> Result<(), SecurityError> {
        const MAX_POSITION: usize = 10000;
        if pos.0 > MAX_POSITION || pos.1 > MAX_POSITION {
            return Err(SecurityError::InvalidCursorPosition);
        }
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
