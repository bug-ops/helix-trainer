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

[scenarios.metadata]
category = "editing"
difficulty = "beginner"
tags = ["test"]
commands_taught = ["w", "cw"]
estimated_time_seconds = 10
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

#[test]
fn test_repeat_insert_scenario_loads_correctly() {
    use crate::game::GameSession;
    use std::path::Path;

    let loader = ScenarioLoader::new();
    let path = Path::new("scenarios/en/repeat/basic-repeat.toml");

    // Load scenarios from file
    let scenarios = loader.load(path).expect("Should load repeat scenarios");

    assert!(!scenarios.is_empty(), "Should have scenarios");

    // Find repeat_insert_001 scenario
    let scenario = scenarios
        .iter()
        .find(|s| s.id == "repeat_insert_001")
        .expect("Should find repeat_insert_001");

    // Verify cursor positions are within bounds
    assert_eq!(scenario.setup.cursor_position, Some((0, 4)));
    assert_eq!(scenario.target.cursor_position, Some((0, 8)));

    // Verify content - realistic Rust code
    assert_eq!(scenario.setup.file_content, "fn f() {}");
    assert_eq!(scenario.target.file_content, "fn f(x)x {}");

    // Try to create a game session - this validates all constraints
    let session_result = GameSession::new(scenario.clone());
    assert!(
        session_result.is_ok(),
        "Should create game session successfully: {:?}",
        session_result.err()
    );
}

// ============================================================================
// Additional tests for coverage
// ============================================================================

#[test]
fn test_available_locales_returns_at_least_en() {
    // available_locales should always return at least "en"
    let locales = ScenarioLoader::available_locales();
    assert!(
        locales.contains(&"en".to_string()),
        "Should include English locale"
    );
}

#[test]
fn test_load_from_embedded_english() {
    let loader = ScenarioLoader::new();
    let result = loader.load_from_embedded("en");
    assert!(
        result.is_ok(),
        "Should load English embedded scenarios: {:?}",
        result.err()
    );
    let scenarios = result.unwrap();
    assert!(
        !scenarios.is_empty(),
        "Should have embedded English scenarios"
    );
}

#[test]
fn test_load_from_embedded_unknown_locale() {
    let loader = ScenarioLoader::new();
    let result = loader.load_from_embedded("xx"); // Non-existent locale
    assert!(result.is_err(), "Should fail for unknown locale");
}

#[test]
fn test_load_directory_success() {
    use std::path::Path;

    let loader = ScenarioLoader::new();
    let path = Path::new("scenarios/en");

    let result = loader.load_directory(path);
    assert!(
        result.is_ok(),
        "Should load scenarios from directory: {:?}",
        result.err()
    );

    let scenarios = result.unwrap();
    assert!(!scenarios.is_empty(), "Should have scenarios in directory");
}

#[test]
fn test_load_directory_not_directory() {
    use std::path::Path;

    let loader = ScenarioLoader::new();
    // Try to load a file path, not a directory
    let path = Path::new("scenarios/en/movement/basic-movement.toml");

    let result = loader.load_directory(path);
    assert!(result.is_err(), "Should fail when path is not a directory");
}

#[test]
fn test_too_many_hints_rejection() {
    let mut hints = String::new();
    // Add more than MAX_HINTS hints (assuming MAX_HINTS is 10)
    for i in 0..11 {
        hints.push_str(&format!("\"Hint {}\"", i));
        if i < 10 {
            hints.push_str(", ");
        }
    }

    let toml = format!(
        r#"
[[scenarios]]
id = "test_001"
name = "Test"
description = "Test"
hints = [{}]

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
        "#,
        hints
    );

    let mut temp_file = NamedTempFile::new().unwrap();
    temp_file.write_all(toml.as_bytes()).unwrap();
    temp_file.flush().unwrap();

    let parent_dir = temp_file.path().parent().unwrap().canonicalize().unwrap();
    let loader = ScenarioLoader::with_allowed_paths(vec![parent_dir]);

    let result = loader.load(temp_file.path());
    assert!(result.is_err(), "Should reject too many hints");
}

#[test]
fn test_scenario_with_selection() {
    let toml = r#"
[[scenarios]]
id = "selection_test_001"
name = "Selection Test"
description = "Test with selection"

[scenarios.setup]
file_content = "Hello World"
cursor_position = [0, 0]
selection = [0, 0, 0, 5]

[scenarios.target]
file_content = "Goodbye World"
cursor_position = [0, 0]

[scenarios.solution]
commands = ["cGoodbye", "Esc"]
description = "Change Hello to Goodbye"

[scenarios.scoring]
optimal_count = 2
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
        "Should load scenario with selection: {:?}",
        result.err()
    );

    let scenarios = result.unwrap();
    assert_eq!(scenarios.len(), 1);
    assert!(scenarios[0].setup.selection.is_some());
}

#[test]
fn test_scenario_metadata_category_debug() {
    // Test Debug implementation for ScenarioCategory
    assert!(format!("{:?}", ScenarioCategory::Movement).contains("Movement"));
    assert!(format!("{:?}", ScenarioCategory::Editing).contains("Editing"));
    assert!(format!("{:?}", ScenarioCategory::Clipboard).contains("Clipboard"));
    assert!(format!("{:?}", ScenarioCategory::Search).contains("Search"));
    assert!(format!("{:?}", ScenarioCategory::Selection).contains("Selection"));
    assert!(format!("{:?}", ScenarioCategory::TextObjects).contains("TextObjects"));
    assert!(format!("{:?}", ScenarioCategory::Advanced).contains("Advanced"));
    assert!(format!("{:?}", ScenarioCategory::Multi).contains("Multi"));
    assert!(format!("{:?}", ScenarioCategory::Other).contains("Other"));
}

#[test]
fn test_difficulty_ordering() {
    // Test PartialOrd implementation for Difficulty
    assert!(Difficulty::Beginner < Difficulty::Intermediate);
    assert!(Difficulty::Intermediate < Difficulty::Advanced);
    assert!(Difficulty::Beginner < Difficulty::Advanced);
}

#[test]
fn test_difficulty_debug() {
    // Test Debug implementation for Difficulty
    assert!(format!("{:?}", Difficulty::Beginner).contains("Beginner"));
    assert!(format!("{:?}", Difficulty::Intermediate).contains("Intermediate"));
    assert!(format!("{:?}", Difficulty::Advanced).contains("Advanced"));
}

#[test]
fn test_scenario_with_all_metadata_fields() {
    let toml = r#"
[[scenarios]]
id = "metadata_test_001"
name = "Metadata Test"
description = "Test with all metadata fields"

[scenarios.setup]
file_content = "test"
cursor_position = [0, 0]

[scenarios.target]
file_content = "test"
cursor_position = [0, 0]

[scenarios.solution]
commands = ["nop"]
description = "No operation"

[scenarios.scoring]
optimal_count = 1
max_points = 100
tolerance = 0

[scenarios.metadata]
category = "movement"
difficulty = "intermediate"
tags = ["test", "metadata"]
commands_taught = ["h", "j", "k", "l"]
prerequisites = ["basic_001"]
estimated_time_seconds = 30
locale = "en"
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
        "Should load scenario with full metadata: {:?}",
        result.err()
    );

    let scenarios = result.unwrap();
    let metadata = scenarios[0].metadata.as_ref().unwrap();

    assert_eq!(metadata.category, Some(ScenarioCategory::Movement));
    assert_eq!(metadata.difficulty, Some(Difficulty::Intermediate));
    assert_eq!(metadata.tags, vec!["test", "metadata"]);
    assert_eq!(metadata.commands_taught, vec!["h", "j", "k", "l"]);
    assert_eq!(metadata.prerequisites, vec!["basic_001"]);
    assert_eq!(metadata.estimated_time_seconds, Some(30));
    assert_eq!(metadata.locale, Some("en".to_string()));
}

#[test]
fn test_invalid_toml_rejection() {
    let toml = r#"
this is not valid toml [[[
        "#;

    let mut temp_file = NamedTempFile::new().unwrap();
    temp_file.write_all(toml.as_bytes()).unwrap();
    temp_file.flush().unwrap();

    let parent_dir = temp_file.path().parent().unwrap().canonicalize().unwrap();
    let loader = ScenarioLoader::with_allowed_paths(vec![parent_dir]);

    let result = loader.load(temp_file.path());
    assert!(result.is_err(), "Should reject invalid TOML");
}

#[test]
fn test_target_cursor_position_invalid() {
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
cursor_position = [0, 50000]

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
    assert!(
        result.is_err(),
        "Should reject invalid target cursor position"
    );
}

#[test]
fn test_oversized_target_content_rejection() {
    let huge_content = "B".repeat(200_000);
    let toml = format!(
        r#"
[[scenarios]]
id = "test_001"
name = "Test"
description = "Test"

[scenarios.setup]
file_content = "start"
cursor_position = [0, 0]

[scenarios.target]
file_content = "{}"
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
    assert!(result.is_err(), "Should reject oversized target content");
}

// ============================================================================
// Multi-cursor format tests
// ============================================================================

#[test]
fn test_scenario_with_multi_cursors() {
    let toml = r#"
[[scenarios]]
id = "multi_cursor_001"
name = "Multi-cursor test"
description = "Test with multiple cursors"

[scenarios.setup]
file_content = """let a = 1;
let b = 2;
let c = 3;"""
cursor_position = [0, 4]

[scenarios.target]
file_content = """let a = 1;
let b = 2;
let c = 3;"""
cursors = [[0, 4], [1, 4], [2, 4]]

[scenarios.solution]
commands = ["C", "C"]
description = "Add cursors on next lines"

[scenarios.scoring]
optimal_count = 2
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
        "Should load scenario with multi-cursors: {:?}",
        result.err()
    );

    let scenarios = result.unwrap();
    assert_eq!(scenarios.len(), 1);
    assert!(scenarios[0].target.cursors.is_some());
    assert_eq!(scenarios[0].target.cursors.as_ref().unwrap().len(), 3);
}

#[test]
fn test_scenario_with_multi_selections() {
    let toml = r#"
[[scenarios]]
id = "multi_selection_001"
name = "Multi-selection test"
description = "Test with multiple selections"

[scenarios.setup]
file_content = """let a = 1;
let b = 2;
let c = 3;"""
cursor_position = [0, 0]
selection = [0, 0, 2, 10]

[scenarios.target]
file_content = """let a = 1;
let b = 2;
let c = 3;"""
selections = [[0, 0, 0, 10], [1, 0, 1, 10], [2, 0, 2, 10]]

[scenarios.solution]
commands = ["Alt-s"]
description = "Split selection on newlines"

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
        "Should load scenario with multi-selections: {:?}",
        result.err()
    );

    let scenarios = result.unwrap();
    assert_eq!(scenarios.len(), 1);
    assert!(scenarios[0].target.selections.is_some());
    assert_eq!(scenarios[0].target.selections.as_ref().unwrap().len(), 3);
}

#[test]
fn test_setup_effective_cursor_position() {
    let setup = Setup {
        file_content: "test".to_string(),
        cursor_position: Some((1, 5)),
        selection: None,
        cursors: None,
        selections: None,
    };
    assert_eq!(setup.effective_cursor_position(), (1, 5));

    let setup_multi = Setup {
        file_content: "test".to_string(),
        cursor_position: None,
        selection: None,
        cursors: Some(vec![[2, 3], [4, 5]]),
        selections: None,
    };
    assert_eq!(setup_multi.effective_cursor_position(), (2, 3));
}

#[test]
fn test_setup_all_cursors() {
    let setup = Setup {
        file_content: "test".to_string(),
        cursor_position: Some((1, 5)),
        selection: None,
        cursors: None,
        selections: None,
    };
    assert_eq!(setup.all_cursors(), vec![[1, 5]]);

    let setup_multi = Setup {
        file_content: "test".to_string(),
        cursor_position: None,
        selection: None,
        cursors: Some(vec![[2, 3], [4, 5]]),
        selections: None,
    };
    assert_eq!(setup_multi.all_cursors(), vec![[2, 3], [4, 5]]);
}

#[test]
fn test_setup_all_selections() {
    let setup = Setup {
        file_content: "test".to_string(),
        cursor_position: Some((0, 0)),
        selection: Some([0, 0, 0, 5]),
        cursors: None,
        selections: None,
    };
    assert_eq!(setup.all_selections(), Some(vec![[0, 0, 0, 5]]));

    let setup_multi = Setup {
        file_content: "test".to_string(),
        cursor_position: None,
        selection: None,
        cursors: None,
        selections: Some(vec![[0, 0, 0, 5], [1, 0, 1, 5]]),
    };
    assert_eq!(
        setup_multi.all_selections(),
        Some(vec![[0, 0, 0, 5], [1, 0, 1, 5]])
    );
}

#[test]
fn test_setup_is_multi_cursor() {
    let single = Setup {
        file_content: "test".to_string(),
        cursor_position: Some((0, 0)),
        selection: None,
        cursors: None,
        selections: None,
    };
    assert!(!single.is_multi_cursor());

    let multi_cursors = Setup {
        file_content: "test".to_string(),
        cursor_position: None,
        selection: None,
        cursors: Some(vec![[0, 0], [1, 0]]),
        selections: None,
    };
    assert!(multi_cursors.is_multi_cursor());

    let multi_selections = Setup {
        file_content: "test".to_string(),
        cursor_position: None,
        selection: None,
        cursors: None,
        selections: Some(vec![[0, 0, 0, 5]]),
    };
    assert!(multi_selections.is_multi_cursor());
}

#[test]
fn test_scenario_missing_cursor_rejected() {
    let toml = r#"
[[scenarios]]
id = "missing_cursor_001"
name = "Missing cursor test"
description = "Should fail - no cursor specified"

[scenarios.setup]
file_content = "test"

[scenarios.target]
file_content = "test"
cursor_position = [0, 0]

[scenarios.solution]
commands = ["nop"]
description = "No operation"

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
    assert!(
        result.is_err(),
        "Should reject scenario with no cursor specification"
    );
}

#[test]
fn test_multi_cursor_invalid_position_rejected() {
    let toml = r#"
[[scenarios]]
id = "invalid_multi_cursor_001"
name = "Invalid multi-cursor test"
description = "Should fail - cursor position out of bounds"

[scenarios.setup]
file_content = "test"
cursor_position = [0, 0]

[scenarios.target]
file_content = "test"
cursors = [[0, 0], [50000, 0]]

[scenarios.solution]
commands = ["C"]
description = "Add cursor"

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
    assert!(
        result.is_err(),
        "Should reject scenario with invalid multi-cursor position"
    );
}

// ============================================================================
// TESTING-003: Edge case tests for Setup/TargetState helpers
// ============================================================================

#[test]
fn test_setup_effective_cursor_from_selections() {
    // When only selections are present, cursor is at first selection's head
    let setup = Setup {
        file_content: "test content".to_string(),
        cursor_position: None,
        selection: None,
        cursors: None,
        selections: Some(vec![[0, 2, 0, 5], [0, 8, 0, 12]]),
    };
    // First selection's end (head) is at (0, 5)
    assert_eq!(setup.effective_cursor_position(), (0, 5));
}

#[test]
fn test_setup_all_cursors_derived_from_selections() {
    // all_cursors should derive from selections when cursors is None
    let setup = Setup {
        file_content: "test".to_string(),
        cursor_position: None,
        selection: None,
        cursors: None,
        selections: Some(vec![[0, 0, 0, 3], [0, 5, 0, 8]]),
    };
    // Cursors are at selection ends (heads)
    let cursors = setup.all_cursors();
    assert_eq!(cursors.len(), 2);
    assert_eq!(cursors[0], [0, 3]);
    assert_eq!(cursors[1], [0, 8]);
}

#[test]
fn test_setup_all_selections_derived_from_cursors() {
    // all_selections should create point selections from cursors
    let setup = Setup {
        file_content: "test".to_string(),
        cursor_position: None,
        selection: None,
        cursors: Some(vec![[0, 2], [0, 5]]),
        selections: None,
    };
    let selections = setup.all_selections().unwrap();
    assert_eq!(selections.len(), 2);
    // Point selections: start == end
    assert_eq!(selections[0], [0, 2, 0, 2]);
    assert_eq!(selections[1], [0, 5, 0, 5]);
}

#[test]
fn test_setup_all_selections_none_when_no_selection_info() {
    // When no selection/selections/cursors, all_selections returns None
    let setup = Setup {
        file_content: "test".to_string(),
        cursor_position: Some((0, 0)),
        selection: None,
        cursors: None,
        selections: None,
    };
    assert!(setup.all_selections().is_none());
}

#[test]
fn test_setup_empty_cursors_array() {
    let setup = Setup {
        file_content: "test".to_string(),
        cursor_position: Some((0, 2)),
        selection: None,
        cursors: Some(vec![]),
        selections: None,
    };
    // Empty cursors array: effective_cursor_position falls back to cursor_position
    assert_eq!(setup.effective_cursor_position(), (0, 2));
    // all_cursors returns empty vec
    assert!(setup.all_cursors().is_empty());
}

#[test]
fn test_setup_empty_selections_array() {
    let setup = Setup {
        file_content: "test".to_string(),
        cursor_position: Some((0, 2)),
        selection: None,
        cursors: None,
        selections: Some(vec![]),
    };
    // Empty selections array: effective_cursor_position falls back to cursor_position
    assert_eq!(setup.effective_cursor_position(), (0, 2));
    // all_selections returns Some(empty vec)
    assert_eq!(setup.all_selections(), Some(vec![]));
}

#[test]
fn test_target_effective_cursor_from_selections() {
    let target = TargetState {
        file_content: "test content".to_string(),
        cursor_position: None,
        selection: None,
        cursors: None,
        selections: Some(vec![[0, 0, 0, 7]]),
    };
    assert_eq!(target.effective_cursor_position(), (0, 7));
}

#[test]
fn test_target_all_cursors_derived_from_selections() {
    let target = TargetState {
        file_content: "test".to_string(),
        cursor_position: None,
        selection: None,
        cursors: None,
        selections: Some(vec![[0, 0, 0, 4]]),
    };
    let cursors = target.all_cursors();
    assert_eq!(cursors.len(), 1);
    assert_eq!(cursors[0], [0, 4]); // Head of selection
}

#[test]
fn test_cursors_ref_returns_reference() {
    let setup = Setup {
        file_content: "test".to_string(),
        cursor_position: None,
        selection: None,
        cursors: Some(vec![[0, 1], [0, 2]]),
        selections: None,
    };
    let ref_slice = setup.cursors_ref();
    assert!(ref_slice.is_some());
    assert_eq!(ref_slice.unwrap().len(), 2);
}

#[test]
fn test_selections_ref_returns_reference() {
    let setup = Setup {
        file_content: "test".to_string(),
        cursor_position: None,
        selection: None,
        cursors: None,
        selections: Some(vec![[0, 0, 0, 5]]),
    };
    let ref_slice = setup.selections_ref();
    assert!(ref_slice.is_some());
    assert_eq!(ref_slice.unwrap().len(), 1);
}

#[test]
fn test_cursors_ref_none_when_not_set() {
    let setup = Setup {
        file_content: "test".to_string(),
        cursor_position: Some((0, 0)),
        selection: None,
        cursors: None,
        selections: None,
    };
    assert!(setup.cursors_ref().is_none());
}

// ============================================================================
// TESTING-004: Validation boundary tests
// ============================================================================

#[test]
fn test_cursor_at_max_position_boundary() {
    // MAX_POSITION is 10,000 - cursor at exactly boundary should pass
    let toml = r#"
[[scenarios]]
id = "boundary_test_001"
name = "Boundary Test"
description = "Test cursor at max boundary"

[scenarios.setup]
file_content = "test"
cursor_position = [10000, 10000]

[scenarios.target]
file_content = "test"
cursor_position = [0, 0]

[scenarios.solution]
commands = ["gg"]
description = "Go to start"

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
    assert!(
        result.is_ok(),
        "Should accept cursor at exactly MAX_POSITION"
    );
}

#[test]
fn test_cursor_exceeding_max_position() {
    // Cursor position exceeding MAX_POSITION (10,000) should fail
    let toml = r#"
[[scenarios]]
id = "boundary_exceed_001"
name = "Boundary Exceed Test"
description = "Test cursor exceeding max boundary"

[scenarios.setup]
file_content = "test"
cursor_position = [10001, 0]

[scenarios.target]
file_content = "test"
cursor_position = [0, 0]

[scenarios.solution]
commands = ["gg"]
description = "Go to start"

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
    assert!(
        result.is_err(),
        "Should reject cursor exceeding MAX_POSITION"
    );
}

#[test]
fn test_point_selection_anchor_equals_head() {
    // Point selection where anchor == head should be valid
    let toml = r#"
[[scenarios]]
id = "point_selection_001"
name = "Point Selection Test"
description = "Test point selection"

[scenarios.setup]
file_content = "test content"
cursor_position = [0, 0]

[scenarios.target]
file_content = "test content"
selections = [[0, 5, 0, 5]]

[scenarios.solution]
commands = ["w"]
description = "Move to word"

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
    assert!(
        result.is_ok(),
        "Should accept point selection (anchor == head)"
    );
}

#[test]
fn test_cursors_at_max_limit() {
    // Exactly MAX_CURSORS_PER_SCENARIO cursors should be valid
    let mut cursors_str = String::from("[");
    for i in 0..100 {
        if i > 0 {
            cursors_str.push_str(", ");
        }
        cursors_str.push_str(&format!("[0, {}]", i % 4));
    }
    cursors_str.push(']');

    let toml = format!(
        r#"
[[scenarios]]
id = "max_cursors_001"
name = "Max Cursors Test"
description = "Test at max cursors limit"

[scenarios.setup]
file_content = "test"
cursor_position = [0, 0]

[scenarios.target]
file_content = "test"
cursors = {}

[scenarios.solution]
commands = ["C"]
description = "Add cursor"

[scenarios.scoring]
optimal_count = 1
max_points = 100
tolerance = 0
        "#,
        cursors_str
    );

    let mut temp_file = NamedTempFile::new().unwrap();
    temp_file.write_all(toml.as_bytes()).unwrap();
    temp_file.flush().unwrap();

    let parent_dir = temp_file.path().parent().unwrap().canonicalize().unwrap();
    let loader = ScenarioLoader::with_allowed_paths(vec![parent_dir]);

    let result = loader.load(temp_file.path());
    assert!(
        result.is_ok(),
        "Should accept exactly MAX_CURSORS_PER_SCENARIO cursors"
    );
}

#[test]
fn test_cursors_exceeding_max_limit() {
    // More than MAX_CURSORS_PER_SCENARIO cursors should fail
    let mut cursors_str = String::from("[");
    for i in 0..101 {
        if i > 0 {
            cursors_str.push_str(", ");
        }
        cursors_str.push_str(&format!("[0, {}]", i % 4));
    }
    cursors_str.push(']');

    let toml = format!(
        r#"
[[scenarios]]
id = "exceed_cursors_001"
name = "Exceed Cursors Test"
description = "Test exceeding max cursors limit"

[scenarios.setup]
file_content = "test"
cursor_position = [0, 0]

[scenarios.target]
file_content = "test"
cursors = {}

[scenarios.solution]
commands = ["C"]
description = "Add cursor"

[scenarios.scoring]
optimal_count = 1
max_points = 100
tolerance = 0
        "#,
        cursors_str
    );

    let mut temp_file = NamedTempFile::new().unwrap();
    temp_file.write_all(toml.as_bytes()).unwrap();
    temp_file.flush().unwrap();

    let parent_dir = temp_file.path().parent().unwrap().canonicalize().unwrap();
    let loader = ScenarioLoader::with_allowed_paths(vec![parent_dir]);

    let result = loader.load(temp_file.path());
    assert!(
        result.is_err(),
        "Should reject more than MAX_CURSORS_PER_SCENARIO cursors"
    );
}

#[test]
fn test_selections_at_max_limit() {
    // Exactly MAX_SELECTIONS_PER_SCENARIO selections should be valid
    let mut selections_str = String::from("[");
    for i in 0..100 {
        if i > 0 {
            selections_str.push_str(", ");
        }
        selections_str.push_str(&format!("[0, {}, 0, {}]", i % 4, (i % 4) + 1));
    }
    selections_str.push(']');

    let toml = format!(
        r#"
[[scenarios]]
id = "max_selections_001"
name = "Max Selections Test"
description = "Test at max selections limit"

[scenarios.setup]
file_content = "test"
cursor_position = [0, 0]

[scenarios.target]
file_content = "test"
selections = {}

[scenarios.solution]
commands = ["C"]
description = "Add selection"

[scenarios.scoring]
optimal_count = 1
max_points = 100
tolerance = 0
        "#,
        selections_str
    );

    let mut temp_file = NamedTempFile::new().unwrap();
    temp_file.write_all(toml.as_bytes()).unwrap();
    temp_file.flush().unwrap();

    let parent_dir = temp_file.path().parent().unwrap().canonicalize().unwrap();
    let loader = ScenarioLoader::with_allowed_paths(vec![parent_dir]);

    let result = loader.load(temp_file.path());
    assert!(
        result.is_ok(),
        "Should accept exactly MAX_SELECTIONS_PER_SCENARIO selections"
    );
}

#[test]
fn test_selections_exceeding_max_limit() {
    // More than MAX_SELECTIONS_PER_SCENARIO selections should fail
    let mut selections_str = String::from("[");
    for i in 0..101 {
        if i > 0 {
            selections_str.push_str(", ");
        }
        selections_str.push_str(&format!("[0, {}, 0, {}]", i % 4, (i % 4) + 1));
    }
    selections_str.push(']');

    let toml = format!(
        r#"
[[scenarios]]
id = "exceed_selections_001"
name = "Exceed Selections Test"
description = "Test exceeding max selections limit"

[scenarios.setup]
file_content = "test"
cursor_position = [0, 0]

[scenarios.target]
file_content = "test"
selections = {}

[scenarios.solution]
commands = ["C"]
description = "Add selection"

[scenarios.scoring]
optimal_count = 1
max_points = 100
tolerance = 0
        "#,
        selections_str
    );

    let mut temp_file = NamedTempFile::new().unwrap();
    temp_file.write_all(toml.as_bytes()).unwrap();
    temp_file.flush().unwrap();

    let parent_dir = temp_file.path().parent().unwrap().canonicalize().unwrap();
    let loader = ScenarioLoader::with_allowed_paths(vec![parent_dir]);

    let result = loader.load(temp_file.path());
    assert!(
        result.is_err(),
        "Should reject more than MAX_SELECTIONS_PER_SCENARIO selections"
    );
}

#[test]
fn test_empty_cursors_array_valid() {
    // Empty cursors array should be valid (falls back to cursor_position)
    let toml = r#"
[[scenarios]]
id = "empty_cursors_001"
name = "Empty Cursors Test"
description = "Test empty cursors array"

[scenarios.setup]
file_content = "test"
cursor_position = [0, 0]

[scenarios.target]
file_content = "test"
cursor_position = [0, 0]
cursors = []

[scenarios.solution]
commands = ["nop"]
description = "No operation"

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
    assert!(result.is_ok(), "Should accept empty cursors array");
}

#[test]
fn test_empty_selections_array_valid() {
    // Empty selections array should be valid
    let toml = r#"
[[scenarios]]
id = "empty_selections_001"
name = "Empty Selections Test"
description = "Test empty selections array"

[scenarios.setup]
file_content = "test"
cursor_position = [0, 0]

[scenarios.target]
file_content = "test"
cursor_position = [0, 0]
selections = []

[scenarios.solution]
commands = ["nop"]
description = "No operation"

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
    assert!(result.is_ok(), "Should accept empty selections array");
}
