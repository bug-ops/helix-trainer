//! Legacy-profile loading with the strict identifier types (specs/data-and-runtime-hardening,
//! FR-154, FR-155, SC-014). A profile written before the id newtypes existed must still load.

use helix_trainer::gamification::{ProfileStorage, QuestType, UserProfile};
use helix_trainer::ids::{CommandId, NormalizedCommand, QuestId, ScenarioId};
use serde_json::{Value, json};
use tempfile::TempDir;

const PERFORMANCE_FIXTURE: &str = include_str!("fixtures/fsrs6_profile_performance.json");

fn legacy_profile_json() -> Value {
    let mut performance: Value = serde_json::from_str(PERFORMANCE_FIXTURE).unwrap();
    let card = performance["x"].clone();
    performance["3h"] = card.clone();
    performance[format!(":{}", "n".repeat(64))] = card.clone();
    performance["Escape"] = card;

    let mut profile = serde_json::to_value(UserProfile::new()).unwrap();
    profile["performance_data"] = performance;
    profile["completed_quests_today"] = json!(["quest_easy_1", "cmd_x_easy"]);
    profile["daily_quests"] = json!([
        {
            "id": "quest_easy_1",
            "quest_type": {"CommandPractice": {"command": "x", "target": 3, "current": 1}},
            "description": "Use x",
            "difficulty": "Easy",
            "xp_reward": 25,
            "completed": false
        },
        {
            "id": "quest_explore_2",
            "quest_type": {"Exploration": {"target_commands": 3, "commands_used": ["3h", "\"y"]}},
            "description": "Explore",
            "difficulty": "Medium",
            "xp_reward": 40,
            "completed": false
        },
        {
            "id": "quest_speed_3",
            "quest_type": {"SpeedRun": {"scenario_id": "delete_line_001", "time_limit": 30}},
            "description": "Speed",
            "difficulty": "Hard",
            "xp_reward": 60,
            "completed": true
        }
    ]);
    profile["scenario_history"] = json!({
        "completions": {
            "delete_line_001": {
                "scenario_id": "delete_line_001",
                "attempts": 2,
                "best_score": 95,
                "perfect_count": 0,
                "total_xp_earned": 80,
                "first_attempt": "2026-01-10T10:00:00Z",
                "last_attempt": "2026-01-12T10:00:00Z",
                "mastery_level": "Learning",
                "attempts_today": 1,
                "last_attempt_date": "2026-01-12"
            }
        }
    });
    profile
}

fn load_legacy_profile() -> UserProfile {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("profile.json");
    std::fs::write(
        &path,
        json!({ "profile": legacy_profile_json() }).to_string(),
    )
    .unwrap();
    ProfileStorage::with_path(&path)
        .load()
        .expect("legacy profile loads")
}

#[test]
fn legacy_profile_loads_with_strict_ids() {
    let profile = load_legacy_profile();

    let quest = QuestId::parse("quest_easy_1").unwrap();
    assert!(profile.is_quest_completed(&quest));
    assert_eq!(profile.daily_quests.len(), 3);
    assert_eq!(profile.daily_quests[0].id, quest);
    assert!(
        profile
            .scenario_history
            .get(&ScenarioId::parse("delete_line_001").unwrap())
            .is_some()
    );
}

#[test]
fn legacy_quest_commands_keep_their_normalized_form() {
    let profile = load_legacy_profile();

    let QuestType::Exploration { commands_used, .. } = &profile.daily_quests[1].quest_type else {
        panic!("expected the exploration quest");
    };
    assert!(commands_used.contains(&NormalizedCommand::from_raw("3h").unwrap()));
    assert!(commands_used.contains(&NormalizedCommand::from_raw("\"y").unwrap()));
}

#[test]
fn legacy_cards_with_non_card_keys_are_dropped_not_fatal() {
    let profile = load_legacy_profile();
    let data = &profile.performance_data;

    assert_eq!(data.len(), 5, "only the five canonical fixture cards stay");
    for key in ["ciw", "dd", "x", "w", "gg"] {
        assert!(
            data.contains_key(&CommandId::from_card_id(key).unwrap()),
            "{key}"
        );
    }
}

/// Sets serialize in hash order, so arrays of strings are compared sorted.
fn sort_string_sets(value: &mut Value) {
    match value {
        Value::Array(items) => {
            items.iter_mut().for_each(sort_string_sets);
            if items.iter().all(Value::is_string) {
                items.sort_by_key(Value::to_string);
            }
        }
        Value::Object(map) => map.values_mut().for_each(sort_string_sets),
        _ => {}
    }
}

#[test]
fn profile_json_shape_is_unchanged_by_the_id_types() {
    let original = legacy_profile_json();
    let profile: UserProfile = serde_json::from_value(original.clone()).unwrap();
    let rewritten = serde_json::to_value(&profile).unwrap();

    for field in ["completed_quests_today", "daily_quests", "scenario_history"] {
        let mut expected = original[field].clone();
        let mut actual = rewritten[field].clone();
        sort_string_sets(&mut expected);
        sort_string_sets(&mut actual);
        assert_eq!(actual, expected, "{field}");
    }
}
