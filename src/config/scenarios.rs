//! Scenario loading and validation
//!
//! This module handles loading TOML scenario files with security validations.

use crate::security::limits::*;
use crate::security::{path_validator, sanitizer, SecurityError, UserError};
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};

/// Wrapper for scenarios array in TOML file
#[derive(Deserialize, Debug, Clone)]
pub struct ScenariosFile {
    pub scenarios: Vec<Scenario>,
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
}

/// Initial editor setup
#[derive(Deserialize, Debug, Clone)]
pub struct Setup {
    pub file_content: String,
    pub cursor_position: (usize, usize),
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
    pub fn new() -> Self {
        Self {
            allowed_base_paths: vec![
                PathBuf::from("./scenarios"),
                PathBuf::from("/usr/share/helix-trainer/scenarios"),
            ],
        }
    }

    /// Create a loader with custom allowed paths for testing
    pub fn with_allowed_paths(paths: Vec<PathBuf>) -> Self {
        Self {
            allowed_base_paths: paths,
        }
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

    /// Validate a single scenario for security and correctness
    fn validate_scenario(&self, scenario: &Scenario) -> Result<(), SecurityError> {
        // Validate setup content size
        if scenario.setup.file_content.len() > MAX_FILE_CONTENT_LENGTH {
            return Err(SecurityError::ContentTooLarge {
                max: MAX_FILE_CONTENT_LENGTH,
                actual: scenario.setup.file_content.len(),
            });
        }

        // Validate target content size
        if scenario.target.file_content.len() > MAX_FILE_CONTENT_LENGTH {
            return Err(SecurityError::ContentTooLarge {
                max: MAX_FILE_CONTENT_LENGTH,
                actual: scenario.target.file_content.len(),
            });
        }

        // Validate cursor positions
        self.validate_cursor_position(scenario.setup.cursor_position)?;
        self.validate_cursor_position(scenario.target.cursor_position)?;

        // Validate hints count
        if scenario.hints.len() > MAX_HINTS {
            return Err(SecurityError::TooManyHints { max: MAX_HINTS });
        }

        // Validate alternatives count
        if scenario.alternatives.len() > MAX_ALTERNATIVES {
            return Err(SecurityError::TooManyAlternatives {
                max: MAX_ALTERNATIVES,
            });
        }

        // Validate scoring configuration
        if scenario.scoring.optimal_count == 0 {
            return Err(SecurityError::InvalidScoringConfig);
        }

        // Validate command sequences within bounds
        if scenario.solution.commands.len() > MAX_COMMAND_SEQUENCE_LENGTH {
            return Err(SecurityError::CommandSequenceTooLong {
                max: MAX_COMMAND_SEQUENCE_LENGTH,
            });
        }

        // Validate alternative command sequences
        for alt in &scenario.alternatives {
            if alt.commands.len() > MAX_COMMAND_SEQUENCE_LENGTH {
                return Err(SecurityError::CommandSequenceTooLong {
                    max: MAX_COMMAND_SEQUENCE_LENGTH,
                });
            }
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn create_test_scenario_toml() -> String {
        r#"
[[scenarios]]
id = "test_001"
name = "Test Scenario"
description = "A test scenario"

[scenarios.setup]
file_content = "Hello, World!"
cursor_position = [0, 0]

[scenarios.target]
file_content = "Hello, Rust!"
cursor_position = [0, 7]

[scenarios.solution]
commands = ["w", "cw", "Rust", "Esc"]
description = "Change 'World' to 'Rust'"

[scenarios.scoring]
optimal_count = 4
max_points = 100
tolerance = 1
        "#
        .to_string()
    }

    #[test]
    fn test_valid_scenario_loading() {
        let toml = create_test_scenario_toml();

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(toml.as_bytes()).unwrap();
        temp_file.flush().unwrap();
        let temp_path = temp_file.path();

        let parent_dir = temp_path.parent().unwrap().canonicalize().unwrap();
        let loader = ScenarioLoader::with_allowed_paths(vec![parent_dir]);

        let result = loader.load(temp_path);
        assert!(
            result.is_ok(),
            "Failed to load scenario: {:?}",
            result.err()
        );

        let scenarios = result.unwrap();
        assert_eq!(scenarios.len(), 1);
        assert_eq!(scenarios[0].id, "test_001");
    }

    #[test]
    fn test_invalid_id_rejection() {
        let toml = r#"
[[scenarios]]
id = "test-with-dashes!"
name = "Test"
description = "Test"

[scenarios.setup]
file_content = "test"
cursor_position = [0, 0]

[scenarios.target]
file_content = "test"
cursor_position = [0, 0]

[scenarios.solution]
commands = ["test"]
description = "test"

[scenarios.scoring]
optimal_count = 1
max_points = 100
tolerance = 0
        "#;

        let result: Result<ScenariosFile, _> = toml::from_str(toml);
        assert!(result.is_err());
    }

    #[test]
    fn test_oversized_content_rejection() {
        let huge_content = "A".repeat(200_000);
        let toml = format!(
            r#"
[[scenarios]]
id = "test_001"
name = "Test"
description = "Test"

[scenarios.setup]
file_content = "{}"
cursor_position = [0, 0]

[scenarios.target]
file_content = "target"
cursor_position = [0, 0]

[scenarios.solution]
commands = ["test"]
description = "test"

[scenarios.scoring]
optimal_count = 1
max_points = 100
tolerance = 0
        "#,
            huge_content
        );

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(toml.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        let parent_dir = temp_file.path().parent().unwrap().canonicalize().unwrap();
        let loader = ScenarioLoader::with_allowed_paths(vec![parent_dir]);

        let result = loader.load(temp_file.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_multiple_scenarios() {
        let toml = r#"
[[scenarios]]
id = "test_001"
name = "Test 1"
description = "Test 1"

[scenarios.setup]
file_content = "test1"
cursor_position = [0, 0]

[scenarios.target]
file_content = "test1"
cursor_position = [0, 0]

[scenarios.solution]
commands = ["test1"]
description = "test1"

[scenarios.scoring]
optimal_count = 1
max_points = 100
tolerance = 0

[[scenarios]]
id = "test_002"
name = "Test 2"
description = "Test 2"

[scenarios.setup]
file_content = "test2"
cursor_position = [0, 0]

[scenarios.target]
file_content = "test2"
cursor_position = [0, 0]

[scenarios.solution]
commands = ["test2"]
description = "test2"

[scenarios.scoring]
optimal_count = 1
max_points = 100
tolerance = 0
        "#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(toml.as_bytes()).unwrap();
        temp_file.flush().unwrap();
        let temp_path = temp_file.path();

        let parent_dir = temp_path.parent().unwrap().canonicalize().unwrap();
        let loader = ScenarioLoader::with_allowed_paths(vec![parent_dir]);

        let result = loader.load(temp_path);
        assert!(
            result.is_ok(),
            "Failed to load scenarios: {:?}",
            result.err()
        );

        let scenarios = result.unwrap();
        assert_eq!(scenarios.len(), 2);
        assert_eq!(scenarios[0].id, "test_001");
        assert_eq!(scenarios[1].id, "test_002");
    }

    #[test]
    fn test_invalid_cursor_position() {
        let toml = r#"
[[scenarios]]
id = "test_001"
name = "Test"
description = "Test"

[scenarios.setup]
file_content = "test"
cursor_position = [50000, 0]

[scenarios.target]
file_content = "test"
cursor_position = [0, 0]

[scenarios.solution]
commands = ["test"]
description = "test"

[scenarios.scoring]
optimal_count = 1
max_points = 100
tolerance = 0
        "#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(toml.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        let parent_dir = temp_file.path().parent().unwrap().canonicalize().unwrap();
        let loader = ScenarioLoader::with_allowed_paths(vec![parent_dir]);

        let result = loader.load(temp_file.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_zero_optimal_count_rejection() {
        let toml = r#"
[[scenarios]]
id = "test_001"
name = "Test"
description = "Test"

[scenarios.setup]
file_content = "test"
cursor_position = [0, 0]

[scenarios.target]
file_content = "test"
cursor_position = [0, 0]

[scenarios.solution]
commands = ["test"]
description = "test"

[scenarios.scoring]
optimal_count = 0
max_points = 100
tolerance = 0
        "#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(toml.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        let parent_dir = temp_file.path().parent().unwrap().canonicalize().unwrap();
        let loader = ScenarioLoader::with_allowed_paths(vec![parent_dir]);

        let result = loader.load(temp_file.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_too_many_alternatives() {
        let mut toml = r#"
[[scenarios]]
id = "test_001"
name = "Test"
description = "Test"

[scenarios.setup]
file_content = "test"
cursor_position = [0, 0]

[scenarios.target]
file_content = "test"
cursor_position = [0, 0]

[scenarios.solution]
commands = ["test"]
description = "test"

[scenarios.scoring]
optimal_count = 1
max_points = 100
tolerance = 0
"#
        .to_string();

        // Add 21 alternatives (MAX_ALTERNATIVES is 20)
        for i in 0..21 {
            toml.push_str(&format!(
                "\n[[scenarios.alternatives]]\ncommands = [\"alt{}\"]\npoints_multiplier = 1.0\ndescription = \"Alternative {}\"",
                i, i
            ));
        }

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(toml.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        let parent_dir = temp_file.path().parent().unwrap().canonicalize().unwrap();
        let loader = ScenarioLoader::with_allowed_paths(vec![parent_dir]);

        let result = loader.load(temp_file.path());
        assert!(result.is_err(), "Should reject too many alternatives");
    }

    #[test]
    fn test_command_sequence_too_long() {
        let commands = vec!["cmd".to_string(); MAX_COMMAND_SEQUENCE_LENGTH + 1];
        let commands_str = commands
            .iter()
            .map(|c| format!("\"{}\"", c))
            .collect::<Vec<_>>()
            .join(", ");

        let toml = format!(
            r#"
[[scenarios]]
id = "test_001"
name = "Test"
description = "Test"

[scenarios.setup]
file_content = "test"
cursor_position = [0, 0]

[scenarios.target]
file_content = "test"
cursor_position = [0, 0]

[scenarios.solution]
commands = [{}]
description = "test"

[scenarios.scoring]
optimal_count = 1
max_points = 100
tolerance = 0
        "#,
            commands_str
        );

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(toml.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        let parent_dir = temp_file.path().parent().unwrap().canonicalize().unwrap();
        let loader = ScenarioLoader::with_allowed_paths(vec![parent_dir]);

        let result = loader.load(temp_file.path());
        assert!(
            result.is_err(),
            "Should reject command sequence that is too long"
        );
    }

    #[test]
    fn test_alternative_command_sequence_too_long() {
        let commands_str = (0..=MAX_COMMAND_SEQUENCE_LENGTH)
            .map(|i| format!("\"cmd{}\"", i))
            .collect::<Vec<_>>()
            .join(", ");

        let toml = format!(
            r#"
[[scenarios]]
id = "test_001"
name = "Test"
description = "Test"

[scenarios.setup]
file_content = "test"
cursor_position = [0, 0]

[scenarios.target]
file_content = "test"
cursor_position = [0, 0]

[scenarios.solution]
commands = ["test"]
description = "test"

[[scenarios.alternatives]]
commands = [{}]
points_multiplier = 1.0
description = "Alternative"

[scenarios.scoring]
optimal_count = 1
max_points = 100
tolerance = 0
        "#,
            commands_str
        );

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(toml.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        let parent_dir = temp_file.path().parent().unwrap().canonicalize().unwrap();
        let loader = ScenarioLoader::with_allowed_paths(vec![parent_dir]);

        let result = loader.load(temp_file.path());
        assert!(
            result.is_err(),
            "Should reject alternative command sequence that is too long"
        );
    }

    #[test]
    fn test_scenario_with_hints_and_alternatives() {
        let toml = r#"
[[scenarios]]
id = "comprehensive_test_001"
name = "Comprehensive Test"
description = "A test with hints and alternatives"
hints = ["First hint", "Second hint"]

[scenarios.setup]
file_content = """Line 1
Line 2
Line 3"""
cursor_position = [1, 0]

[scenarios.target]
file_content = """Line 1
Line 3"""
cursor_position = [1, 0]

[scenarios.solution]
commands = ["d", "d"]
description = "Delete line 2"

[[scenarios.alternatives]]
commands = ["j", "d", "d"]
points_multiplier = 0.9
description = "Move down then delete"

[[scenarios.alternatives]]
commands = ["ctrl-k"]
points_multiplier = 0.95
description = "Using Ctrl+K shortcut"

[scenarios.scoring]
optimal_count = 2
max_points = 100
tolerance = 1
        "#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(toml.as_bytes()).unwrap();
        temp_file.flush().unwrap();
        let temp_path = temp_file.path();

        let parent_dir = temp_path.parent().unwrap().canonicalize().unwrap();
        let loader = ScenarioLoader::with_allowed_paths(vec![parent_dir]);

        let result = loader.load(temp_path);
        assert!(
            result.is_ok(),
            "Failed to load comprehensive scenario: {:?}",
            result.err()
        );

        let scenarios = result.unwrap();
        assert_eq!(scenarios.len(), 1);

        let scenario = &scenarios[0];
        assert_eq!(scenario.id, "comprehensive_test_001");
        assert_eq!(scenario.alternatives.len(), 2);
        assert_eq!(scenario.hints.len(), 2);
        assert_eq!(scenario.solution.commands.len(), 2);
    }

    #[test]
    fn test_default_loader() {
        let loader = ScenarioLoader::default();
        assert!(!loader.allowed_base_paths.is_empty());
    }

    #[test]
    fn test_path_traversal_attack_rejected() {
        let toml = create_test_scenario_toml();
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(toml.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        let temp_dir = temp_file.path().parent().unwrap();

        // Create a loader that only allows a specific subdirectory
        let allowed_dir = temp_dir.join("scenarios");
        let loader = ScenarioLoader::with_allowed_paths(vec![allowed_dir]);

        // Try to load from parent directory
        let result = loader.load(temp_file.path());
        assert!(
            result.is_err(),
            "Should reject path outside allowed directories"
        );
    }
}
