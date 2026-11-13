//! Tests for scenario loading and validation

use super::*;
use crate::testing::{ncmd, point, sid};
use std::assert_matches;
use std::fmt::Write as _;
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
    assert_eq!(scenarios[0].id.as_str(), "test_001");
}

#[test]
fn test_scenario_without_language_field_defaults_to_none() {
    // FR-001/FR-002: existing scenarios omitting `language` must keep parsing
    // and resolve to `None` (treated as "rs" by the highlighter).
    let toml = create_test_scenario_toml();
    let result: Result<ScenariosFile, _> = toml::from_str(&toml);
    let file = result.expect("scenario without language field should parse");
    assert_eq!(file.scenarios[0].setup.language, None);
}

#[test]
fn test_scenario_with_valid_language_field() {
    let toml = format!(
        "{}\n",
        create_test_scenario_toml().replace(
            "file_content = \"Hello, World!\"",
            "file_content = \"Hello, World!\"\nlanguage = \"md\""
        )
    );
    let result: Result<ScenariosFile, _> = toml::from_str(&toml);
    let file = result.expect("scenario with valid language field should parse");
    assert_eq!(file.scenarios[0].setup.language, Some(Language::Md));
}

#[test]
fn test_scenario_with_unknown_language_is_a_load_error() {
    // FR-004/FR-006: the closed `Language` enum rejects every other token,
    // including the empty string, whitespace and a different case.
    for token in ["", "c++", "js", "RS", "Rs", "python", "rs drop"] {
        let toml = format!(
            "{}\n",
            create_test_scenario_toml().replace(
                "file_content = \"Hello, World!\"",
                &format!("file_content = \"Hello, World!\"\nlanguage = \"{token}\"")
            )
        );
        let result: Result<ScenariosFile, _> = toml::from_str(&toml);
        assert!(result.is_err(), "{token:?}");
    }
}

#[test]
fn test_multi_scenario_toml_resolves_language_independently() {
    // Tester gap: two scenarios in the same TOML file must each keep their own
    // `language` value - no cross-scenario leakage between deserialized entries.
    let toml = format!(
        "{}\n{}\n",
        create_test_scenario_toml().replace(
            "file_content = \"Hello, World!\"",
            "file_content = \"Hello, World!\"\nlanguage = \"md\""
        ),
        create_test_scenario_toml()
            .replace("id = \"test_001\"", "id = \"test_002\"")
            .replace(
                "file_content = \"Hello, World!\"",
                "file_content = \"Hello, World!\"\nlanguage = \"py\""
            )
    );
    let result: Result<ScenariosFile, _> = toml::from_str(&toml);
    let file = result.expect("multi-scenario TOML with distinct languages should parse");
    assert_eq!(file.scenarios.len(), 2);
    assert_eq!(file.scenarios[0].setup.language, Some(Language::Md));
    assert_eq!(file.scenarios[1].setup.language, Some(Language::Py));
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
fn test_empty_id_rejection() {
    let toml = r#"
[[scenarios]]
id = ""
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

    let mut temp_file = NamedTempFile::new().unwrap();
    temp_file.write_all(toml.as_bytes()).unwrap();
    temp_file.flush().unwrap();

    let parent_dir = temp_file.path().parent().unwrap().canonicalize().unwrap();
    let loader = ScenarioLoader::with_allowed_paths(vec![parent_dir]);

    let result = loader.load(temp_file.path());
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
file_content = "{huge_content}"
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
        "#
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
    assert_eq!(scenarios[0].id.as_str(), "test_001");
    assert_eq!(scenarios[1].id.as_str(), "test_002");
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
fn test_command_sequence_too_long() {
    let commands = vec!["cmd".to_string(); MAX_COMMAND_SEQUENCE_LENGTH + 1];
    let commands_str = commands
        .iter()
        .map(|c| format!("\"{c}\""))
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
commands = [{commands_str}]
description = "test"

[scenarios.scoring]
optimal_count = 1
max_points = 100
tolerance = 0
        "#
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
fn test_scenario_with_hints() {
    let toml = r#"
[[scenarios]]
id = "comprehensive_test_001"
name = "Comprehensive Test"
description = "A test with hints"
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
    assert_eq!(scenario.id.as_str(), "comprehensive_test_001");
    assert_eq!(scenario.hints.len(), 2);
    assert_eq!(scenario.solution.commands.len(), 2);
}

#[test]
fn test_removed_alternatives_field_is_rejected() {
    let base = create_test_scenario_toml();
    let base = base.split("[scenarios.metadata]").next().unwrap();
    let toml = format!(
        "{base}\n[[scenarios.alternatives]]\ncommands = [\"j\"]\npoints_multiplier = 1.0\ndescription = \"x\"\n"
    );
    assert!(toml::from_str::<ScenariosFile>(&toml).is_err());
    assert!(toml::from_str::<ScenariosFile>(base).is_ok());
}

/// Stale keys inside `[scenarios.metadata]` are a load error naming the key.
#[test]
fn test_unknown_keys_in_metadata_table_are_rejected() {
    for key in ["estimated_time_seconds = 30", "locale = \"en\""] {
        let toml = format!("{}\n{key}\n", create_test_scenario_toml());
        let err = toml::from_str::<ScenariosFile>(&toml)
            .unwrap_err()
            .to_string();
        let name = key.split(' ').next().unwrap();
        assert!(err.contains(name), "{err}");
    }
}

#[test]
#[expect(
    clippy::assert_is_empty,
    reason = "assert!(x.is_empty()) reads better than the machine fix"
)]
fn test_scenario_without_metadata_table_gets_default_metadata() {
    let toml = create_test_scenario_toml();
    let toml = toml.split("[scenarios.metadata]").next().unwrap();
    let file: ScenariosFile = toml::from_str(toml).unwrap();
    let metadata = &file.scenarios[0].metadata;
    assert_eq!(metadata.category, None);
    assert!(metadata.tags.is_empty());
}

#[test]
#[expect(
    clippy::assert_is_empty,
    reason = "assert!(x.is_empty()) reads better than the machine fix"
)]
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
        .find(|s| s.id.as_str() == "repeat_insert_001")
        .expect("Should find repeat_insert_001");

    // Verify cursor positions are within bounds
    assert_eq!(scenario.setup.cursor, point(0, 4));
    assert_eq!(scenario.target.cursor, point(0, 7));

    // Verify content - realistic Rust code
    assert_eq!(scenario.setup.file_content, "fn f() {}");
    assert_eq!(scenario.target.file_content, "fn f(xx) {}");

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
    let locales = ScenarioLoader::new().available_locales();
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
fn test_embedded_scenarios_optimal_count_equals_solution_commands() {
    let scenarios = ScenarioLoader::new()
        .load_from_embedded("en")
        .expect("embedded scenarios must load");

    let mismatches: Vec<String> = scenarios
        .iter()
        .filter(|s| s.scoring.optimal_count.get() != s.solution.commands.len())
        .map(|s| {
            format!(
                "{}: optimal_count={} but solution has {} commands",
                s.id,
                s.scoring.optimal_count,
                s.solution.commands.len()
            )
        })
        .collect();

    assert!(mismatches.is_empty(), "{}", mismatches.join("\n"));
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
        write!(hints, "\"Hint {i}\"").unwrap();
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
hints = [{hints}]

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
    assert_matches!(scenarios[0].setup.cursor, CursorSpec::Range(_));
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

/// Exhaustive match forces a compile error if a `Difficulty` variant is added without
/// updating `Difficulty::ALL` (used by the Polyglot achievement check), catching drift
/// at compile time rather than silently under-counting difficulties.
#[test]
fn difficulty_all_is_exhaustive() {
    fn assert_variant_covered(d: Difficulty) {
        match d {
            Difficulty::Beginner | Difficulty::Intermediate | Difficulty::Advanced => {}
        }
    }
    for d in Difficulty::ALL {
        assert_variant_covered(d);
    }
    assert_eq!(Difficulty::ALL.len(), 3);
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
    let metadata = &scenarios[0].metadata;

    assert_eq!(metadata.category, Some(ScenarioCategory::Movement));
    assert_eq!(metadata.difficulty, Some(Difficulty::Intermediate));
    assert_eq!(
        metadata.tags,
        vec![ScenarioTag::new("test"), ScenarioTag::new("metadata")]
    );
    assert_eq!(
        metadata.commands_taught,
        vec![ncmd("h"), ncmd("j"), ncmd("k"), ncmd("l")]
    );
    assert_eq!(metadata.prerequisites, vec![sid("basic_001")]);
}

#[test]
fn test_invalid_toml_rejection() {
    let toml = r"
this is not valid toml [[[
        ";

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
file_content = "{huge_content}"
cursor_position = [0, 0]

[scenarios.solution]
commands = ["test"]
description = "test"

[scenarios.scoring]
optimal_count = 1
max_points = 100
tolerance = 0
        "#
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
    assert_matches!(
        &scenarios[0].target.cursor,
        CursorSpec::Points(p) if p.as_slice().len() == 3
    );
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
    assert_matches!(
        &scenarios[0].target.cursor,
        CursorSpec::Ranges(r) if r.as_slice().len() == 3
    );
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
        write!(cursors_str, "[0, {}]", i % 4).unwrap();
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
cursors = {cursors_str}

[scenarios.solution]
commands = ["C"]
description = "Add cursor"

[scenarios.scoring]
optimal_count = 1
max_points = 100
tolerance = 0
        "#
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
        write!(cursors_str, "[0, {}]", i % 4).unwrap();
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
cursors = {cursors_str}

[scenarios.solution]
commands = ["C"]
description = "Add cursor"

[scenarios.scoring]
optimal_count = 1
max_points = 100
tolerance = 0
        "#
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
        write!(selections_str, "[0, {}, 0, {}]", i % 4, (i % 4) + 1).unwrap();
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
selections = {selections_str}

[scenarios.solution]
commands = ["C"]
description = "Add selection"

[scenarios.scoring]
optimal_count = 1
max_points = 100
tolerance = 0
        "#
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
        write!(selections_str, "[0, {}, 0, {}]", i % 4, (i % 4) + 1).unwrap();
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
selections = {selections_str}

[scenarios.solution]
commands = ["C"]
description = "Add selection"

[scenarios.scoring]
optimal_count = 1
max_points = 100
tolerance = 0
        "#
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

mod completion_rule {
    use super::*;

    fn taught(commands: &[&str]) -> Vec<CommandId> {
        commands.iter().map(|c| id(c).unwrap()).collect()
    }

    fn id(raw: &str) -> Option<CommandId> {
        CommandId::from_action(raw)
    }

    fn last(command: Option<&CommandId>, target_held_before: bool) -> LastAction<'_> {
        LastAction {
            command,
            target_held_before,
        }
    }

    #[test]
    fn test_is_satisfied_table() {
        let undo_redo = taught(&["u", "U"]);
        let after = CompletionRule::AfterTaughtCommands;
        let target = CompletionRule::TargetState;
        let cases = [
            (
                "target not reached",
                after,
                false,
                Some(("u", false)),
                false,
            ),
            ("no last action", after, true, None, false),
            (
                "unparsable command",
                after,
                true,
                Some(("???", false)),
                false,
            ),
            ("untaught command", after, true, Some(("d", false)), false),
            ("taught u", after, true, Some(("u", false)), true),
            ("taught U", after, true, Some(("U", false)), true),
            ("counted taught", after, true, Some(("2u", false)), true),
            (
                "no-op U (target held before)",
                after,
                true,
                Some(("U", true)),
                false,
            ),
            ("TargetState ignores last action", target, true, None, true),
            (
                "TargetState needs target",
                target,
                false,
                Some(("u", false)),
                false,
            ),
        ];
        for (name, rule, reached, action, expected) in cases {
            let ids = action.map(|(command, held)| (id(command), held));
            let action = ids
                .as_ref()
                .map(|(command, held)| last(command.as_ref(), *held));
            assert_eq!(
                rule.is_satisfied(reached, &undo_redo, action),
                expected,
                "{name}"
            );
        }
    }

    #[test]
    fn test_u_does_not_match_taught_redo_only() {
        let rule = CompletionRule::AfterTaughtCommands;
        let undo = id("u");
        assert!(!rule.is_satisfied(true, &taught(&["U"]), Some(last(undo.as_ref(), false))));
    }
}

fn load_toml(toml: &str) -> Result<Vec<Scenario>, crate::config::ConfigError> {
    let mut temp_file = NamedTempFile::new().unwrap();
    temp_file.write_all(toml.as_bytes()).unwrap();
    temp_file.flush().unwrap();
    let parent_dir = temp_file.path().parent().unwrap().canonicalize().unwrap();
    ScenarioLoader::with_allowed_paths(vec![parent_dir]).load(temp_file.path())
}

fn scenario_toml_with_setup_cursor(setup_cursor: &str) -> String {
    scenario_toml_with_cursors(setup_cursor, "cursor_position = [0, 0]")
}

fn scenario_toml_with_cursors(setup_cursor: &str, target_cursor: &str) -> String {
    format!(
        r#"
[[scenarios]]
id = "cursor_spec_001"
name = "Cursor spec"
description = "Cursor spec test"

[scenarios.setup]
file_content = "hello world"
{setup_cursor}

[scenarios.target]
file_content = "hello world"
{target_cursor}

[scenarios.solution]
commands = ["nop"]
description = "No operation"

[scenarios.scoring]
optimal_count = 1
max_points = 100
tolerance = 0
"#
    )
}

#[test]
fn test_setup_with_two_cursor_fields_is_rejected_naming_both() {
    let toml =
        scenario_toml_with_setup_cursor("cursor_position = [0, 0]\nselection = [0, 0, 0, 3]");
    let err = load_toml(&toml).unwrap_err();
    let cause = format!("{err:?}");
    assert!(
        cause.contains("cursor_position, selection"),
        "error should name both fields: {cause}"
    );
}

#[test]
fn test_target_with_two_cursor_fields_is_rejected_naming_both() {
    let toml = scenario_toml_with_cursors(
        "cursor_position = [0, 0]",
        "cursor_position = [0, 0]\nselection = [0, 0, 0, 3]",
    );
    let err = load_toml(&toml).unwrap_err();
    let cause = format!("{err:?}");
    assert!(
        cause.contains("cursor_position, selection"),
        "error should name both fields: {cause}"
    );
}

#[test]
fn test_setup_with_only_selections_is_a_ranges_spec() {
    let toml = scenario_toml_with_setup_cursor("selections = [[0, 0, 0, 3], [0, 6, 0, 9]]");
    let scenarios = load_toml(&toml).unwrap();
    assert_matches!(
        &scenarios[0].setup.cursor,
        CursorSpec::Ranges(r) if r.as_slice().len() == 2
    );
}

#[test]
fn test_setup_with_an_empty_cursor_list_is_rejected() {
    assert!(load_toml(&scenario_toml_with_setup_cursor("cursors = []")).is_err());
}
