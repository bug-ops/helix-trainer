//! Tests for scenario loading and validation

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
