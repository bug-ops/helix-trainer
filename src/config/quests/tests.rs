//! Tests for quest template loading

use super::*;
use crate::testing::{ncmd, qid, sid};
use serde::Deserialize;
use std::assert_matches;
use std::path::PathBuf;

#[test]
fn test_quest_loader_creation() {
    let loader = QuestLoader::new();
    assert_eq!(loader.allowed_base_paths.len(), 2);
}

#[test]
fn test_quest_loader_with_custom_paths() {
    let paths = vec![PathBuf::from("./custom")];
    let loader = QuestLoader::with_allowed_paths(paths.clone());
    assert_eq!(loader.allowed_base_paths, paths);
}

#[test]
fn quest_id_is_validated_when_loading_toml() {
    #[derive(Deserialize)]
    struct Test {
        id: QuestId,
    }

    let parse = |id: &str| toml::from_str::<Test>(&format!("id = {id:?}"));
    assert_eq!(parse("cmd_x_easy").unwrap().id.as_str(), "cmd_x_easy");
    assert!(parse(&"a".repeat(65)).is_err());
    assert!(parse("invalid-id-with-hyphens").is_err());
    assert!(parse("").is_err(), "Empty ID should be rejected");
}

#[test]
fn test_quest_spec_type_tag_deserialization() {
    let valid = r#"
type = "command_practice"
[params]
command = "x"
target = 3
"#;
    let result: QuestSpec = toml::from_str(valid).unwrap();
    assert_matches!(result, QuestSpec::CommandPractice { .. });

    let valid = r#"
type = "speed_run"
[params]
scenario_id = "delete_line_001"
time_limit_seconds = 5
"#;
    let result: QuestSpec = toml::from_str(valid).unwrap();
    assert_matches!(result, QuestSpec::SpeedRun { .. });
}

#[test]
fn test_quest_difficulty_deserialization() {
    #[derive(Deserialize)]
    struct Test {
        difficulty: QuestDifficulty,
    }

    let easy = r#"difficulty = "easy""#;
    let result: Test = toml::from_str(easy).unwrap();
    assert_eq!(result.difficulty, QuestDifficulty::Easy);

    let medium = r#"difficulty = "medium""#;
    let result: Test = toml::from_str(medium).unwrap();
    assert_eq!(result.difficulty, QuestDifficulty::Medium);

    let hard = r#"difficulty = "hard""#;
    let result: Test = toml::from_str(hard).unwrap();
    assert_eq!(result.difficulty, QuestDifficulty::Hard);
}

#[test]
fn test_quest_spec_command_practice() {
    let toml_str = r#"
type = "command_practice"
[params]
command = "x"
target = 3
"#;
    let spec: QuestSpec = toml::from_str(toml_str).unwrap();
    match spec {
        QuestSpec::CommandPractice { command, target } => {
            assert_eq!(command.as_str(), "x");
            assert_eq!(target, 3);
        }
        other => panic!("Wrong variant: {other:?}"),
    }
}

#[test]
fn test_quest_spec_scenario_completion() {
    let toml_str = r#"
type = "scenario_completion"
[params]
target = 5
"#;
    let spec: QuestSpec = toml::from_str(toml_str).unwrap();
    match spec {
        QuestSpec::ScenarioCompletion { target } => {
            assert_eq!(target, 5);
        }
        other => panic!("Wrong variant: {other:?}"),
    }
}

#[test]
fn test_quest_spec_speed_run() {
    let toml_str = r#"
type = "speed_run"
[params]
scenario_id = "delete_line_001"
time_limit_seconds = 5
"#;
    let spec: QuestSpec = toml::from_str(toml_str).unwrap();
    match spec {
        QuestSpec::SpeedRun {
            scenario_id,
            time_limit_seconds,
        } => {
            assert_eq!(scenario_id.as_str(), "delete_line_001");
            assert_eq!(time_limit_seconds, 5);
        }
        other => panic!("Wrong variant: {other:?}"),
    }
}

#[test]
fn test_quest_spec_type_params_mismatch_is_deserialization_error() {
    // `type` says speed_run but `params` is shaped like command_practice: this
    // must fail to deserialize rather than silently picking a mismatched variant.
    let toml_str = r#"
type = "speed_run"
[params]
command = "x"
target = 3
"#;
    let result: Result<QuestSpec, _> = toml::from_str(toml_str);
    assert!(
        result.is_err(),
        "type/params mismatch should be a deserialization error"
    );
}

#[test]
fn test_quest_conditions_default() {
    let conditions = QuestConditions::default();
    assert!(conditions.min_level.is_none());
    assert!(conditions.max_level.is_none());
    assert!(conditions.requires_commands.is_empty());
    assert!(conditions.requires_scenarios.is_empty());
}

#[test]
fn test_xp_config_default() {
    let xp_config = XpConfig::default();
    assert!(xp_config.base_reward.is_none());
}

#[test]
fn test_load_daily_quests_toml() {
    use std::path::Path;

    let loader = QuestLoader::new();
    let quest_file = Path::new("./quests/en/daily.toml");

    // Only run if file exists (for CI compatibility)
    if !quest_file.exists() {
        return;
    }

    let result = loader.load(quest_file);
    assert!(result.is_ok(), "Failed to load daily quests: {result:?}");

    let quests = result.unwrap();
    assert_eq!(quests.len(), 70, "Expected 70 quest templates");

    // Verify we have expected quest IDs across all difficulty levels
    let ids: Vec<_> = quests.iter().map(|q| q.id.as_str()).collect();

    // Easy quests
    assert!(ids.contains(&"cmd_h_easy"));
    assert!(ids.contains(&"cmd_j_easy"));
    assert!(ids.contains(&"cmd_k_easy"));
    assert!(ids.contains(&"cmd_l_easy"));
    assert!(ids.contains(&"cmd_w_easy"));
    assert!(ids.contains(&"cmd_b_easy"));
    assert!(ids.contains(&"cmd_0_easy"));
    assert!(ids.contains(&"cmd_gg_easy"));
    assert!(ids.contains(&"cmd_G_easy"));
    assert!(ids.contains(&"cmd_x_easy"));
    assert!(ids.contains(&"cmd_x_easy"));
    assert!(ids.contains(&"cmd_yy_easy"));
    assert!(ids.contains(&"scenario_1_easy"));
    assert!(ids.contains(&"time_2_easy"));

    // Medium quests
    assert!(ids.contains(&"cmd_w_medium"));
    assert!(ids.contains(&"cmd_i_medium"));
    assert!(ids.contains(&"cmd_c_medium"));
    assert!(ids.contains(&"scenario_2_medium"));
    assert!(ids.contains(&"time_5_medium"));
    assert!(ids.contains(&"explore_5_medium"));

    // Hard quests
    assert!(ids.contains(&"scenario_5_hard"));
    assert!(ids.contains(&"speed_delete_hard"));
    assert!(ids.contains(&"time_15_hard"));
    assert!(ids.contains(&"explore_10_hard"));
    assert!(ids.contains(&"explore_20_hard"));
}

#[test]
fn test_quest_template_to_quest_conversion() {
    let template = QuestTemplate {
        id: qid("test_quest"),
        difficulty: QuestDifficulty::Easy,
        spec: QuestSpec::CommandPractice {
            command: ncmd("x"),
            target: 3,
        },
        xp: None,
        conditions: QuestConditions::default(),
    };

    let quest = crate::gamification::Quest::from(&template);
    assert_eq!(quest.id.as_str(), "test_quest");
    assert!(!quest.completed);
}

#[test]
fn test_quest_spec_time_invested() {
    let toml_str = r#"
type = "time_invested"
[params]
target_minutes = 10
"#;
    let spec: QuestSpec = toml::from_str(toml_str).unwrap();
    match spec {
        QuestSpec::TimeInvested { target_minutes } => {
            assert_eq!(target_minutes, 10);
        }
        other => panic!("Wrong variant: {other:?}"),
    }
}

#[test]
fn test_quest_spec_exploration() {
    let toml_str = r#"
type = "exploration"
[params]
target_commands = 15
"#;
    let spec: QuestSpec = toml::from_str(toml_str).unwrap();
    match spec {
        QuestSpec::Exploration { target_commands } => {
            assert_eq!(target_commands, 15);
        }
        other => panic!("Wrong variant: {other:?}"),
    }
}

#[test]
fn test_quest_conditions_with_values() {
    let toml_str = r#"
min_level = 5
max_level = 20
requires_commands = ["x", "yy"]
requires_scenarios = ["basic_001"]
"#;
    let conditions: QuestConditions = toml::from_str(toml_str).unwrap();
    assert_eq!(conditions.min_level, Some(5));
    assert_eq!(conditions.max_level, Some(20));
    assert_eq!(conditions.requires_commands.len(), 2);
    assert_eq!(conditions.requires_scenarios.len(), 1);
}

#[test]
fn test_metadata_version_validation() {
    use serde::Deserialize;

    #[expect(
        dead_code,
        reason = "the field exists only to run its deserialize validator"
    )]
    #[derive(Deserialize)]
    struct Test {
        #[serde(deserialize_with = "validate_version_field")]
        version: String,
    }

    // Valid version
    let valid = r#"version = "1.0""#;
    let result: Result<Test, _> = toml::from_str(valid);
    assert!(result.is_ok());

    // Empty version should fail
    let empty = r#"version = """#;
    let result: Result<Test, _> = toml::from_str(empty);
    assert!(result.is_err(), "Empty version should be rejected");

    // Too long version should fail
    let too_long = r#"version = "1.0.0.0.0.0.0.0.0.0.0""#;
    let result: Result<Test, _> = toml::from_str(too_long);
    assert!(result.is_err(), "Too long version should be rejected");
}

#[test]
fn test_metadata_locale_validation() {
    use serde::Deserialize;

    #[expect(
        dead_code,
        reason = "the field exists only to run its deserialize validator"
    )]
    #[derive(Deserialize)]
    struct Test {
        #[serde(default, deserialize_with = "validate_locale_field")]
        locale: Option<String>,
    }

    // Valid locale
    let valid = r#"locale = "en""#;
    let result: Result<Test, _> = toml::from_str(valid);
    assert!(result.is_ok());

    // Valid locale with underscore
    let valid_underscore = r#"locale = "en_US""#;
    let result: Result<Test, _> = toml::from_str(valid_underscore);
    assert!(result.is_ok());

    // Invalid locale with hyphen
    let invalid_hyphen = r#"locale = "en-US""#;
    let result: Result<Test, _> = toml::from_str(invalid_hyphen);
    assert!(result.is_err(), "Locale with hyphen should be rejected");
}

#[test]
fn test_deny_unknown_fields_quests_file() {
    let invalid_toml = r#"
[metadata]
version = "1.0"
unknown_field = "value"

[[quests]]
id = "test"
type = "command_practice"
difficulty = "easy"

[quests.params]
command = "x"
target = 3
"#;
    let result: Result<QuestsFile, _> = toml::from_str(invalid_toml);
    assert!(
        result.is_err(),
        "Unknown field in metadata should be rejected"
    );
}

#[test]
fn test_deny_unknown_fields_quest_template() {
    let invalid_toml = r#"
[metadata]
version = "1.0"

[[quests]]
id = "test"
type = "command_practice"
difficulty = "easy"
unknown_field = "value"

[quests.params]
command = "x"
target = 3
"#;
    let result: Result<QuestsFile, _> = toml::from_str(invalid_toml);
    assert!(
        result.is_err(),
        "Unknown field in quest template should be rejected"
    );
}

#[test]
fn test_quest_text_keys_are_rejected_in_templates() {
    for key in ["name", "description"] {
        let toml = format!(
            "[metadata]\nversion = \"1.0\"\n\n[[quests]]\nid = \"test\"\n{key} = \"Test\"\ntype = \"scenario_completion\"\ndifficulty = \"easy\"\n\n[quests.params]\ntarget = 1\n"
        );
        assert!(toml::from_str::<QuestsFile>(&toml).is_err(), "{key}");
    }
}

#[test]
fn test_validate_all_quest_templates() {
    use std::collections::{HashMap, HashSet};

    // Load quest templates for English locale
    let loader = QuestLoader::default();
    let templates = loader
        .load_for_locale("en")
        .expect("Failed to load quest templates for locale 'en'");

    // Verify minimum count (12 templates in daily.toml)
    assert!(
        templates.len() >= 12,
        "Expected at least 12 quest templates, got {}",
        templates.len()
    );

    // Track unique IDs
    let mut seen_ids = HashSet::new();
    let mut commands_used = HashSet::new();
    let mut scenario_ids_used = HashSet::new();

    // Validate each template
    for template in &templates {
        // 1. Check unique IDs
        assert!(
            seen_ids.insert(&template.id),
            "Duplicate quest ID found: {}",
            template.id
        );

        // 2. ID format is guaranteed by the `QuestId` type

        // 4. Validate quest spec parameters (type/params mismatch is unrepresentable
        //    at the type level, so no wildcard arm is needed here)
        match &template.spec {
            QuestSpec::CommandPractice { command, target } => {
                assert!(
                    command.as_str().len() <= 10,
                    "Command name too long for ID: {}",
                    template.id
                );
                assert!(*target > 0, "Target must be > 0 for ID: {}", template.id);
                assert!(
                    *target <= 100,
                    "Target exceeds maximum (100) for ID: {}",
                    template.id
                );
                commands_used.insert(command.clone());
            }
            QuestSpec::ScenarioCompletion { target } => {
                assert!(*target > 0, "Target must be > 0 for ID: {}", template.id);
                assert!(
                    *target <= 100,
                    "Target exceeds maximum (100) for ID: {}",
                    template.id
                );
            }
            QuestSpec::SpeedRun {
                scenario_id,
                time_limit_seconds,
            } => {
                assert!(
                    *time_limit_seconds > 0,
                    "Time limit must be > 0 for ID: {}",
                    template.id
                );
                assert!(
                    *time_limit_seconds <= 3600,
                    "Time limit exceeds 1 hour for ID: {}",
                    template.id
                );
                scenario_ids_used.insert(scenario_id.clone());
            }
            QuestSpec::TimeInvested { target_minutes } => {
                assert!(
                    *target_minutes > 0,
                    "Target minutes must be > 0 for ID: {}",
                    template.id
                );
                assert!(
                    *target_minutes <= 100,
                    "Target minutes exceeds maximum (100) for ID: {}",
                    template.id
                );
            }
            QuestSpec::Exploration { target_commands } => {
                assert!(
                    *target_commands > 0,
                    "Target commands must be > 0 for ID: {}",
                    template.id
                );
                assert!(
                    *target_commands <= 100,
                    "Target commands exceeds maximum (100) for ID: {}",
                    template.id
                );
            }
        }

        // 5. Validate custom XP reward if present
        if let Some(xp_config) = &template.xp
            && let Some(reward) = xp_config.base_reward
        {
            assert!(
                reward <= 1000,
                "XP reward exceeds maximum (1000) for ID: {}",
                template.id
            );
        }

        // 6. Validate conditions
        assert!(
            template.conditions.requires_commands.len() <= 20,
            "Too many required commands (max 20) for ID: {}",
            template.id
        );
        assert!(
            template.conditions.requires_scenarios.len() <= 20,
            "Too many required scenarios (max 20) for ID: {}",
            template.id
        );

        // Spot-check that `conditions` (a sibling of the flattened `spec` field)
        // deserializes correctly and isn't swallowed by the adjacent tagging.
        if template.id.as_str() == "speed_delete_hard" {
            assert_eq!(
                template.conditions.requires_scenarios,
                vec![sid("delete_line_001")],
                "requires_scenarios did not survive deserialization for ID: {}",
                template.id
            );
        }

        // Collect scenario IDs from conditions
        for scenario_id in &template.conditions.requires_scenarios {
            scenario_ids_used.insert(scenario_id.clone());
        }

        // 7. Validate template converts to runtime Quest
        let quest = crate::gamification::Quest::from(template);
        assert_eq!(quest.id, template.id);
        assert!(!quest.completed, "New quest should not be completed");
    }

    // 8. Validate referenced commands are valid Helix commands
    let valid_commands: HashSet<&str> = [
        // Movement
        "h", "j", "k", "l", "w", "b", "e", "0", "$", "gg", "G", // Paragraph movement
        "[p", "]p", // Editing
        "x", "i", "a", "I", "A", "o", "O", "c", "J", ">", "<", "~", // Clipboard
        "y", "yy", "p", "P", // Undo/Redo
        "u", "U", // Repeat
        ".", // Replace (r + char is handled specially)
        "r", // Selection
        "_", "C", // Search
        "*", "n", "N", // Text objects (match mode prefix + object)
        "miw", "maw", "mip", "map", // Surround (match mode prefix + surround command)
        "ms", "md", "mr",
    ]
    .iter()
    .copied()
    .collect();

    for command in &commands_used {
        // Check if command is valid or is a replace command (rx, ry, etc.)
        let command = command.as_str();
        let is_valid = valid_commands.contains(command)
            || (command.starts_with('r')
                && command.len() == 2
                && command.chars().nth(1).unwrap().is_alphanumeric());

        assert!(
            is_valid,
            "Quest references invalid/unknown Helix command: {command}"
        );
    }

    // 10. Verify distribution of difficulties
    let difficulty_counts: HashMap<QuestDifficulty, usize> =
        templates.iter().fold(HashMap::new(), |mut acc, t| {
            *acc.entry(t.difficulty).or_insert(0) += 1;
            acc
        });

    // Should have quests of all difficulty levels
    assert!(
        difficulty_counts.contains_key(&QuestDifficulty::Easy),
        "No easy quests found"
    );
    assert!(
        difficulty_counts.contains_key(&QuestDifficulty::Medium),
        "No medium quests found"
    );
    assert!(
        difficulty_counts.contains_key(&QuestDifficulty::Hard),
        "No hard quests found"
    );

    // Report summary
    println!("\n=== Quest Template Validation Summary ===");
    println!("Total templates loaded: {}", templates.len());
    println!("Unique quest IDs: {}", seen_ids.len());
    println!("Commands referenced: {}", commands_used.len());
    println!("Scenario IDs referenced: {}", scenario_ids_used.len());
    println!("\nDifficulty distribution:");
    println!(
        "  Easy: {}",
        difficulty_counts.get(&QuestDifficulty::Easy).unwrap_or(&0)
    );
    println!(
        "  Medium: {}",
        difficulty_counts
            .get(&QuestDifficulty::Medium)
            .unwrap_or(&0)
    );
    println!(
        "  Hard: {}",
        difficulty_counts.get(&QuestDifficulty::Hard).unwrap_or(&0)
    );
    println!("\nAll quest templates are valid!");
}

mod validation {
    use super::*;

    fn valid_template() -> QuestTemplate {
        let file: QuestsFile = toml::from_str(
            r#"
[metadata]
version = "1.0"

[[quests]]
id = "valid"
type = "command_practice"
difficulty = "easy"

[quests.params]
command = "x"
target = 3
"#,
        )
        .unwrap();
        file.quests.into_iter().next().unwrap()
    }

    fn with_spec(spec: QuestSpec) -> QuestTemplate {
        QuestTemplate {
            spec,
            ..valid_template()
        }
    }

    fn check(quest: &QuestTemplate) -> Result<(), QuestTemplateError> {
        QuestLoader::check_quest(quest)
    }

    #[test]
    fn a_valid_template_passes() {
        assert_eq!(check(&valid_template()), Ok(()));
        assert!(QuestLoader::validate_quest(&valid_template()).is_ok());
    }

    #[test]
    fn the_xp_limit_has_its_own_variant() {
        let rich = QuestTemplate {
            xp: Some(XpConfig {
                base_reward: Some(MAX_CUSTOM_XP_REWARD + 1),
            }),
            ..valid_template()
        };
        assert_matches!(
            check(&rich),
            Err(QuestTemplateError::XpRewardTooHigh { .. })
        );
    }

    #[test]
    fn condition_limits_have_their_own_variants() {
        let commands = QuestTemplate {
            conditions: QuestConditions {
                requires_commands: vec![ncmd("x"); MAX_REQUIRED_CONDITIONS + 1],
                ..QuestConditions::default()
            },
            ..valid_template()
        };
        assert_matches!(
            check(&commands),
            Err(QuestTemplateError::TooManyRequiredCommands { .. })
        );

        let scenarios = QuestTemplate {
            conditions: QuestConditions {
                requires_scenarios: vec![sid("s"); MAX_REQUIRED_CONDITIONS + 1],
                ..QuestConditions::default()
            },
            ..valid_template()
        };
        assert_matches!(
            check(&scenarios),
            Err(QuestTemplateError::TooManyRequiredScenarios { .. })
        );

        let inverted = QuestTemplate {
            conditions: QuestConditions {
                min_level: Some(5),
                max_level: Some(2),
                ..QuestConditions::default()
            },
            ..valid_template()
        };
        assert_eq!(
            check(&inverted),
            Err(QuestTemplateError::LevelRangeInverted { min: 5, max: 2 })
        );
    }

    #[test]
    fn every_spec_variant_rejects_a_zero_and_an_oversized_target() {
        for target in [0, MAX_QUEST_TARGET + 1] {
            assert_matches!(
                check(&with_spec(QuestSpec::CommandPractice {
                    command: ncmd("x"),
                    target
                })),
                Err(QuestTemplateError::InvalidCommandPracticeTarget { .. })
            );
            assert_matches!(
                check(&with_spec(QuestSpec::ScenarioCompletion { target })),
                Err(QuestTemplateError::InvalidScenarioCompletionTarget { .. })
            );
            assert_matches!(
                check(&with_spec(QuestSpec::TimeInvested {
                    target_minutes: target
                })),
                Err(QuestTemplateError::InvalidTimeInvestedTarget { .. })
            );
            assert_matches!(
                check(&with_spec(QuestSpec::Exploration {
                    target_commands: target
                })),
                Err(QuestTemplateError::InvalidExplorationTarget { .. })
            );
        }
        for limit in [0, MAX_SPEED_RUN_TIME_SECONDS + 1] {
            assert_matches!(
                check(&with_spec(QuestSpec::SpeedRun {
                    scenario_id: sid("s"),
                    time_limit_seconds: limit
                })),
                Err(QuestTemplateError::InvalidSpeedRunTimeLimit { .. })
            );
        }
    }

    #[test]
    fn a_long_command_name_is_rejected() {
        let long = ncmd(&"x".repeat(MAX_QUEST_COMMAND_NAME_LENGTH + 1));
        assert_matches!(
            check(&with_spec(QuestSpec::CommandPractice {
                command: long,
                target: 1
            })),
            Err(QuestTemplateError::CommandNameTooLong { .. })
        );
    }

    #[test]
    fn the_security_error_keeps_the_typed_cause() {
        let err = QuestLoader::validate_quest(&QuestTemplate {
            xp: Some(XpConfig {
                base_reward: Some(MAX_CUSTOM_XP_REWARD + 1),
            }),
            ..valid_template()
        })
        .unwrap_err();
        assert_matches!(
            err,
            SecurityError::QuestTemplate(QuestTemplateError::XpRewardTooHigh { .. })
        );
    }
}
