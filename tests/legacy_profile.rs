//! Legacy-profile loading with the strict identifier types (specs/data-and-runtime-hardening,
//! FR-154, FR-155, SC-014). A profile written before the id newtypes existed must still load.

use helix_trainer::config::keymap::fingerprint::{KeymapFingerprint, StoredKeymapFingerprint};
use helix_trainer::gamification::{
    ChallengeDay, ProfileStorage, QuestStatus, QuestType, UserProfile,
};
use helix_trainer::ids::{CommandId, NormalizedCommand, QuestId, ScenarioId};
use helix_trainer::time::PlayerDay;
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
            "completed": true
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

fn load_profile_json(profile: &Value) -> UserProfile {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("profile.json");
    std::fs::write(&path, json!({ "profile": profile }).to_string()).unwrap();
    ProfileStorage::with_path(&path)
        .load()
        .expect("profile loads")
}

fn load_legacy_profile() -> UserProfile {
    load_profile_json(&legacy_profile_json())
}

#[test]
fn legacy_profile_loads_with_strict_ids() {
    let profile = load_legacy_profile();

    let quest = QuestId::parse("quest_easy_1").unwrap();
    assert!(profile.is_quest_completed(&quest));
    assert_eq!(profile.daily_quests().len(), 3);
    assert_eq!(profile.daily_quests()[0].id, quest);
    assert!(
        profile
            .scenario_history()
            .get(&ScenarioId::parse("delete_line_001").unwrap())
            .is_some()
    );
}

/// FR-037: a legacy `completed` quest loads as completed, rewarded exactly when its id was in
/// the legacy `completed_quests_today` set; the set itself is not written back.
#[test]
fn legacy_quest_completion_loads_into_the_quest_status() {
    let profile = load_legacy_profile();

    assert_eq!(
        profile.daily_quests()[0].status(),
        QuestStatus::Completed { rewarded: true }
    );
    assert_eq!(profile.daily_quests()[1].status(), QuestStatus::InProgress);
    assert_eq!(
        profile.daily_quests()[2].status(),
        QuestStatus::Completed { rewarded: false }
    );
    assert_eq!(profile.completed_quests_today().count(), 2);

    let written = serde_json::to_value(&profile).unwrap();
    assert!(written.get("completed_quests_today").is_none());
    assert!(written["daily_quests"][0].get("completed").is_none());
}

#[test]
fn legacy_quest_commands_keep_their_normalized_form() {
    let profile = load_legacy_profile();

    let QuestType::Exploration { commands_used, .. } = &profile.daily_quests()[1].quest_type else {
        panic!("expected the exploration quest");
    };
    assert!(commands_used.contains(&NormalizedCommand::from_raw("3h").unwrap()));
    assert!(commands_used.contains(&NormalizedCommand::from_raw("\"y").unwrap()));
}

#[test]
fn legacy_cards_with_non_card_keys_are_dropped_not_fatal() {
    let profile = load_legacy_profile();
    let data = &profile.performance_data();

    assert_eq!(data.len(), 5, "only the five canonical fixture cards stay");
    for key in ["ciw", "dd", "x", "w", "gg"] {
        assert!(
            data.contains_key(&CommandId::from_card_id(key).unwrap()),
            "{key}"
        );
    }
}

/// #560: a profile written before the fingerprint carried an algorithm tag holds a bare number
#[test]
fn legacy_numeric_keymap_fingerprint_loads_as_legacy() {
    let mut profile = legacy_profile_json();
    profile["keymap_fingerprint"] = json!(11_400_714_819_323_198_485_u64);

    let loaded = load_profile_json(&profile);

    assert_eq!(
        loaded.keymap_fingerprint(),
        Some(StoredKeymapFingerprint::Legacy(11_400_714_819_323_198_485))
    );
}

#[test]
fn tagged_keymap_fingerprint_survives_a_save_and_load() {
    let mut profile = legacy_profile_json();
    profile["keymap_fingerprint"] = json!({ "Fnv1a64V1": 42 });

    let loaded = load_profile_json(&profile);

    assert_eq!(
        loaded.keymap_fingerprint(),
        Some(StoredKeymapFingerprint::Current(
            KeymapFingerprint::Fnv1a64V1(42)
        ))
    );
    let rewritten = serde_json::to_value(&loaded).unwrap();
    assert_eq!(rewritten["keymap_fingerprint"], json!({ "Fnv1a64V1": 42 }));
}

#[test]
fn profile_without_a_keymap_fingerprint_loads_with_none() {
    let mut profile = legacy_profile_json();
    profile
        .as_object_mut()
        .unwrap()
        .remove("keymap_fingerprint");

    assert_eq!(load_profile_json(&profile).keymap_fingerprint(), None);
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
    let mut original = legacy_profile_json();
    let profile: UserProfile = serde_json::from_value(original.clone()).unwrap();
    let rewritten = serde_json::to_value(&profile).unwrap();

    // Quest text moved to the locale files (#612); an old `description` is read and dropped.
    for quest in original["daily_quests"].as_array_mut().unwrap() {
        quest.as_object_mut().unwrap().remove("description");
    }
    // Quest completion lives on the quests (#652): the legacy flag becomes `status` (a plain
    // deserialize leaves it unrewarded; `ProfileStorage::load` settles it), and the legacy
    // completed-quest set is read but never written.
    for quest in original["daily_quests"].as_array_mut().unwrap() {
        let quest = quest.as_object_mut().unwrap();
        let completed = quest.remove("completed") == Some(json!(true));
        quest.insert(
            "status".to_string(),
            if completed {
                json!({ "Completed": { "rewarded": false } })
            } else {
                json!("InProgress")
            },
        );
    }
    assert!(rewritten.get("completed_quests_today").is_none());
    for field in ["daily_quests", "scenario_history"] {
        let mut expected = original[field].clone();
        let mut actual = rewritten[field].clone();
        sort_string_sets(&mut expected);
        sort_string_sets(&mut actual);
        assert_eq!(actual, expected, "{field}");
    }
}

fn load_error_for(
    mutate: impl FnOnce(&mut Value),
) -> helix_trainer::config::json_store::StorageError {
    let mut profile = legacy_profile_json();
    mutate(&mut profile);
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("profile.json");
    std::fs::write(&path, json!({ "profile": profile }).to_string()).unwrap();
    ProfileStorage::with_path(&path)
        .load()
        .expect_err("an invalid persisted id must not load")
}

fn rename_history_entry(profile: &mut Value, new_key: &str) {
    let completions = &mut profile["scenario_history"]["completions"];
    let entry = completions["delete_line_001"].take();
    *completions = json!({ new_key: entry });
}

fn assert_corrupt(site: &str, mutate: impl FnOnce(&mut Value)) {
    let err = load_error_for(mutate);
    assert!(
        matches!(
            err,
            helix_trainer::config::json_store::StorageError::Corrupt { .. }
        ),
        "{site}: {err}"
    );
}

#[test]
fn invalid_persisted_ids_are_a_corrupt_load_error() {
    assert_corrupt("completed_quests_today", |p| {
        p["completed_quests_today"] = json!(["bad id"]);
    });
    assert_corrupt("daily_quests[].id", |p| {
        p["daily_quests"][0]["id"] = json!("bad id");
    });
    assert_corrupt("scenario_history key with a space", |p| {
        rename_history_entry(p, "bad id");
    });
    assert_corrupt("scenario_history key over 64 bytes", |p| {
        rename_history_entry(p, &"a".repeat(65));
    });
}

/// #647 FR-002/FR-003: the stored level is ignored in favor of the one derived from the XP,
/// and a later save carries no `level` key.
#[test]
fn legacy_level_key_is_ignored_and_never_written() {
    let mut profile = legacy_profile_json();
    profile["total_xp"] = json!(1500);
    profile["level"] = json!(99);

    let loaded = load_profile_json(&profile);

    assert_eq!(
        loaded.level(),
        helix_trainer::gamification::XPCalculator::level_from_xp(1500)
    );
    assert!(
        serde_json::to_value(&loaded)
            .unwrap()
            .get("level")
            .is_none()
    );
}

/// #647 FR-004/FR-005: every repair, one fixture each; the profile loads, never rejected.
#[test]
fn legacy_streak_and_completion_bounds_are_repaired_on_load() {
    let mut profile = legacy_profile_json();
    profile["current_streak"] = json!(9);
    profile["longest_streak"] = json!(4);
    profile["perfect_scenarios"] = json!(8);
    profile["scenarios_completed"] = json!(6);

    let loaded = load_profile_json(&profile);

    assert_eq!(loaded.longest_streak(), 9);
    assert_eq!(loaded.scenarios_completed(), 8);
}

#[test]
fn legacy_challenge_counters_are_repaired_on_load() {
    let mut profile = legacy_profile_json();
    profile["challenge_progress"] = json!({
        "last_challenge_date": "2026-01-12",
        "attempts_used_today": 7,
        "best_score_today": 900,
        "best_scenarios_today": 6,
        "all_time_best_score": 400,
        "all_time_best_scenarios": 3,
        "total_challenges_attempted": 3,
        "total_challenges_completed": 5
    });

    let loaded = load_profile_json(&profile);
    let challenge = loaded.challenge_progress();

    // The daily attempt limit is 3; the 7 stored attempts stay spent.
    assert_eq!(challenge.attempts_used_today(), 3);
    assert_eq!(challenge.all_time_best_score(), 900);
    assert_eq!(challenge.all_time_best_scenarios(), 6);
    assert_eq!(challenge.total_challenges_attempted(), 5);
}

/// #660 FR-006: a challenge date stored as a UTC day by the previous build loads as a player day.
#[test]
fn legacy_utc_challenge_date_loads_as_a_player_day() {
    let mut profile = legacy_profile_json();
    profile["challenge_progress"] = json!({
        "last_challenge_date": "2026-01-12",
        "attempts_used_today": 2,
        "best_score_today": 100,
        "best_scenarios_today": 1,
        "all_time_best_score": 100,
        "all_time_best_scenarios": 1,
        "total_challenges_attempted": 2,
        "total_challenges_completed": 1
    });

    let loaded = load_profile_json(&profile);
    let challenge = loaded.challenge_progress();
    let stored = PlayerDay::from_ymd(2026, 1, 12).unwrap();

    assert_eq!(challenge.last_challenge_date(), ChallengeDay::On(stored));
    assert_eq!(challenge.attempts_remaining(stored), 1);
    assert!(challenge.can_attempt(PlayerDay::from_ymd(2026, 1, 13).unwrap()));
    assert_eq!(challenge.attempts_used_today(), 2);
}

#[test]
fn a_consistent_profile_loads_unchanged() {
    let loaded = load_legacy_profile();

    assert!(loaded.longest_streak() >= loaded.current_streak());
    assert!(loaded.scenarios_completed() >= loaded.perfect_scenarios());
}
