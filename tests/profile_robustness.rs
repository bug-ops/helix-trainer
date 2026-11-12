//! Property test over mutated profile documents (specs/data-and-runtime-hardening, FR-131).
//!
//! A valid profile JSON gets integer leaves replaced by extreme values, date leaves replaced by
//! the chrono limits and unknown fields added. Whenever the result still loads, every
//! consumer of loaded data runs on it; none may panic.

use chrono::{DateTime, Utc};
use helix_trainer::gamification::{
    AchievementEngine, AchievementId, ProfileStorage, QuestGenerator, QuestTemplateRegistry,
    StreakManager, UserProfile,
};
use helix_trainer::ids::{CommandId, ScenarioId};
use helix_trainer::learning::PerformanceTracker;
use proptest::prelude::*;
use serde_json::{Value, json};
use std::fmt::Write as _;
use std::time::Duration;

const PERFORMANCE_FIXTURE: &str = include_str!("fixtures/fsrs6_profile_performance.json");

fn base_profile() -> UserProfile {
    let mut profile = UserProfile::new();
    let now = Utc::now();
    profile.add_xp(1_500);
    profile.scenarios_completed = 7;
    profile.current_streak = 3;
    profile.longest_streak = 5;
    profile.scenario_history.record_completion(
        &ScenarioId::parse("delete_line_001").unwrap(),
        100,
        50,
        now,
    );
    profile.unlock_achievement(AchievementId::FirstPerfect);
    profile.performance_data = serde_json::from_str(PERFORMANCE_FIXTURE).unwrap();
    let registry = QuestTemplateRegistry::from_embedded("en").unwrap();
    profile.daily_quests =
        QuestGenerator::generate_quests(&profile, &registry, now.date_naive()).quests;
    profile
}

/// JSON pointers of every integer leaf and every RFC 3339 timestamp leaf.
fn leaf_pointers(value: &Value) -> Vec<(String, bool)> {
    fn walk(value: &Value, pointer: &mut String, out: &mut Vec<(String, bool)>) {
        match value {
            Value::Number(n) if n.is_u64() || n.is_i64() => out.push((pointer.clone(), false)),
            Value::String(s) if s.parse::<DateTime<Utc>>().is_ok() => {
                out.push((pointer.clone(), true));
            }
            Value::Array(items) => {
                for (i, item) in items.iter().enumerate() {
                    let len = pointer.len();
                    write!(pointer, "/{i}").unwrap();
                    walk(item, pointer, out);
                    pointer.truncate(len);
                }
            }
            Value::Object(map) => {
                for (key, item) in map {
                    let len = pointer.len();
                    pointer.push('/');
                    pointer.push_str(&key.replace('~', "~0").replace('/', "~1"));
                    walk(item, pointer, out);
                    pointer.truncate(len);
                }
            }
            _ => {}
        }
    }
    let mut out = Vec::new();
    walk(value, &mut String::new(), &mut out);
    out
}

fn extreme_number(selector: usize) -> Value {
    match selector % 6 {
        0 => json!(0),
        1 => json!(1),
        2 => json!(u32::MAX),
        3 => json!(u64::MAX),
        4 => json!(i64::MIN),
        _ => json!(-1),
    }
}

fn extreme_date(selector: usize) -> Value {
    let date = if selector.is_multiple_of(2) {
        DateTime::<Utc>::MIN_UTC
    } else {
        DateTime::<Utc>::MAX_UTC
    };
    serde_json::to_value(date).unwrap()
}

#[derive(Debug, Clone)]
struct Mutation {
    leaf: prop::sample::Index,
    selector: usize,
}

fn mutate(document: &mut Value, mutations: &[Mutation], unknown_fields: usize) {
    let leaves = leaf_pointers(document);
    for mutation in mutations {
        let (pointer, is_date) = &leaves[mutation.leaf.index(leaves.len())];
        let replacement = if *is_date {
            extreme_date(mutation.selector)
        } else {
            extreme_number(mutation.selector)
        };
        *document.pointer_mut(pointer).unwrap() = replacement;
    }
    let root = document.as_object_mut().unwrap();
    for i in 0..unknown_fields {
        root.insert(format!("unknown_field_{i}"), json!({"nested": [u64::MAX]}));
    }
}

#[expect(
    clippy::assert_is_empty,
    reason = "assert!(x.is_empty()) reads better than the machine fix"
)]
fn exercise_consumers(mut profile: UserProfile, clock_selector: usize) {
    let level_before = profile.level;
    let now = match clock_selector % 3 {
        0 => Utc::now(),
        1 => DateTime::<Utc>::MIN_UTC,
        _ => DateTime::<Utc>::MAX_UTC,
    };
    let registry = QuestTemplateRegistry::from_embedded("en").unwrap();
    let mut tracker = PerformanceTracker::from_stats(profile.performance_data.clone());

    let command = CommandId::from_action("x").unwrap();
    tracker.record_attempt(&command, Duration::MAX, true, Duration::ZERO);
    tracker.record_attempt(&command, Duration::ZERO, false, Duration::MAX);

    let id = ScenarioId::parse("delete_line_001").unwrap();
    profile
        .scenario_history
        .record_completion(&id, u32::MAX, u64::MAX, now);
    profile
        .scenario_history
        .record_completion(&id, 100, u64::MAX, now);

    StreakManager::update_streak(&mut profile, now);
    AchievementEngine::check_and_unlock(&mut profile, &tracker);
    let generated = QuestGenerator::generate_quests(&profile, &registry, Utc::now().date_naive());
    assert!(!generated.quests.is_empty());
    assert_eq!(
        generated.unfilled_slots, 0,
        "the embedded registry fills every slot"
    );
    profile.add_xp(u64::MAX);
    let progress = profile.xp_progress();
    assert!(
        (0.0..=1.0).contains(&progress),
        "xp progress {progress} out of range"
    );
    assert!(profile.level >= 1, "level started at {level_before}");
}

fn mutation_strategy() -> impl Strategy<Value = Mutation> {
    (any::<prop::sample::Index>(), any::<usize>())
        .prop_map(|(leaf, selector)| Mutation { leaf, selector })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    #[test]
    fn mutated_profiles_never_panic_when_loaded_and_used(
        mutations in prop::collection::vec(mutation_strategy(), 0..6),
        unknown_fields in 0usize..3,
        clock_selector in any::<usize>(),
    ) {
        let mut document = json!({ "profile": serde_json::to_value(base_profile()).unwrap() });
        mutate(&mut document["profile"], &mutations, unknown_fields);

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("profile.json");
        std::fs::write(&path, document.to_string()).unwrap();

        let loaded = ProfileStorage::with_path(&path).load();
        if mutations.is_empty() {
            prop_assert!(
                loaded.is_ok(),
                "a valid document with only unknown fields must load: {:?}",
                loaded.err()
            );
        }
        if let Ok(profile) = loaded {
            exercise_consumers(profile, clock_selector);
        }
    }
}

#[test]
fn unmutated_base_profile_loads_and_survives_the_consumers() {
    let document = json!({ "profile": serde_json::to_value(base_profile()).unwrap() });
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("profile.json");
    std::fs::write(&path, document.to_string()).unwrap();

    let profile = ProfileStorage::with_path(&path)
        .load()
        .expect("the unmutated base profile is valid");
    exercise_consumers(profile, 0);
}
