//! Challenge mode logic and progress tracking
//!
//! This module handles daily challenge functionality:
//! - Progress tracking with attempt limits
//! - Best score tracking (daily and all-time)
//! - Deterministic scenario selection using seeded RNG

use std::sync::Arc;

use rand::SeedableRng;
use rand::prelude::SliceRandom;

use crate::config::Scenario;

use super::ChallengeConfig;

/// Select scenarios for a challenge using deterministic seeded RNG.
///
/// This ensures all players get the same scenarios on the same day.
///
/// # Arguments
///
/// * `scenarios` - All available scenarios
/// * `config` - Challenge configuration with seed and scenario count
///
/// # Returns
///
/// A vector of selected scenarios, or empty if input is empty.
///
/// # Performance
///
/// Uses index-based shuffling and returns `Arc<Scenario>` handles, so the
/// selection costs k deep clones once (k is `config.scenario_count`) and the
/// session shares them through its queue and active scenario without cloning.
///
/// # Examples
///
/// ```
/// use chrono::Utc;
/// use helix_trainer::config::ScenarioLoader;
/// use helix_trainer::minigame::{ChallengeConfig, select_challenge_scenarios};
/// use helix_trainer::time::Moment;
///
/// let scenarios = ScenarioLoader::new().load_from_embedded("en")?;
/// let config = ChallengeConfig::for_date(Moment::from(Utc::now()).day());
/// let selected = select_challenge_scenarios(&scenarios, &config);
/// assert_eq!(selected.len(), 10);
/// # Ok::<(), helix_trainer::error::UserError>(())
/// ```
pub fn select_challenge_scenarios(
    scenarios: &[Scenario],
    config: &ChallengeConfig,
) -> Vec<Arc<Scenario>> {
    if scenarios.is_empty() {
        return Vec::new();
    }

    if scenarios.len() < config.scenario_count {
        // Not enough scenarios, return all available
        return scenarios.iter().cloned().map(Arc::new).collect();
    }

    // CR-004: Use index-based shuffling to reduce clones from O(n) to O(k)
    let mut rng = rand::rngs::StdRng::seed_from_u64(config.seed);
    let mut indices: Vec<usize> = (0..scenarios.len()).collect();
    indices.shuffle(&mut rng);
    indices.truncate(config.scenario_count);
    indices
        .into_iter()
        .map(|i| Arc::new(scenarios[i].clone()))
        .collect()
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use crate::constants::{CHALLENGE_MAX_ATTEMPTS, CHALLENGE_SCENARIO_COUNT};
    use crate::gamification::{ChallengeDay, ChallengeProgress};

    use proptest::prelude::*;

    use super::*;
    use crate::config::Difficulty;
    use crate::testing::ScenarioBuilder;

    fn create_test_scenario(id: &str) -> Scenario {
        ScenarioBuilder::new()
            .id(id)
            .setup_content("line 1\nline 2\n")
            .setup_cursor(1, 0)
            .target_content("line 1\n")
            .target_cursor(1, 0)
            .optimal_count(1)
            .difficulty(Difficulty::Beginner)
            .build()
    }

    #[test]
    fn test_challenge_progress_new() {
        let progress = ChallengeProgress::new();
        assert_eq!(progress.last_challenge_date, ChallengeDay::NotStarted);
        assert_eq!(progress.attempts_used_today, 0);
        assert_eq!(progress.best_score_today, 0);
        assert_eq!(progress.all_time_best_score, 0);
    }

    /// FR-006/FR-003: a stored day after today (written west of UTC by the previous build)
    /// counts as today: no reset, and the stored day does not move backward.
    #[test]
    fn test_a_stored_day_after_today_is_today() {
        let today = crate::time::PlayerDay::from_ymd(2026, 1, 15).unwrap();
        let tomorrow = crate::time::PlayerDay::from_ymd(2026, 1, 16).unwrap();
        let mut progress = ChallengeProgress::new();
        progress.last_challenge_date = ChallengeDay::On(tomorrow);
        progress.attempts_used_today = 3;

        assert!(!progress.can_attempt(today));
        assert_eq!(progress.attempts_remaining(today), 0);
        assert!(progress.is_today(today));
        progress.start_attempt(today);
        assert_eq!(progress.last_challenge_date, ChallengeDay::On(tomorrow));
    }

    /// FR-007: east, west, east on one real day resets exactly once.
    #[test]
    fn test_zone_toggling_resets_once() {
        use crate::time::{Clock, FakeClock};

        let clock = FakeClock::at_offset("2026-01-15T20:00:00Z", 9);
        let mut progress = ChallengeProgress::new();
        progress.last_challenge_date =
            ChallengeDay::On(crate::time::PlayerDay::from_ymd(2026, 1, 15).unwrap());
        progress.attempts_used_today = 3;

        let mut resets = 0;
        for hours in [9, -8, 9] {
            clock.set_utc_offset_hours(hours);
            if progress.can_attempt(clock.player_day()) && progress.attempts_used_today == 3 {
                resets += 1;
            }
            progress.start_attempt(clock.player_day());
            progress.attempts_used_today = 3;
        }

        assert_eq!(resets, 1);
    }

    #[test]
    fn test_can_attempt_new_day() {
        let today = crate::time::PlayerDay::from_ymd(2026, 1, 15).unwrap();
        let progress = ChallengeProgress::new();
        assert!(progress.can_attempt(today));
        assert_eq!(progress.attempts_remaining(today), CHALLENGE_MAX_ATTEMPTS);
    }

    #[test]
    fn test_can_attempt_after_using_attempts() {
        let today = crate::time::PlayerDay::from_ymd(2026, 1, 15).unwrap();
        let mut progress = ChallengeProgress::new();
        progress.last_challenge_date = ChallengeDay::On(today);
        progress.attempts_used_today = 2;

        assert!(progress.can_attempt(today));
        assert_eq!(progress.attempts_remaining(today), 1);

        progress.attempts_used_today = 3;
        assert!(!progress.can_attempt(today));
        assert_eq!(progress.attempts_remaining(today), 0);
    }

    #[test]
    fn test_start_attempt_new_day() {
        let today = crate::time::PlayerDay::from_ymd(2026, 1, 15).unwrap();
        let mut progress = ChallengeProgress::new();
        progress.last_challenge_date =
            ChallengeDay::On(crate::time::PlayerDay::from_ymd(2020, 1, 1).unwrap());
        progress.attempts_used_today = 3;
        progress.best_score_today = 5000;
        progress.best_scenarios_today = 8;

        progress.start_attempt(today);

        // Should reset for new day
        assert_eq!(progress.attempts_used_today, 1);
        assert_eq!(progress.best_score_today, 0);
        assert_eq!(progress.best_scenarios_today, 0);
        assert_eq!(progress.total_challenges_attempted, 1);
    }

    #[test]
    fn test_start_attempt_same_day() {
        let today = crate::time::PlayerDay::from_ymd(2026, 1, 15).unwrap();
        let mut progress = ChallengeProgress::new();
        progress.last_challenge_date = ChallengeDay::On(today);
        progress.attempts_used_today = 1;
        progress.best_score_today = 5000;

        progress.start_attempt(today);

        // Should not reset, just increment
        assert_eq!(progress.attempts_used_today, 2);
        assert_eq!(progress.best_score_today, 5000);
        assert_eq!(progress.last_challenge_date, ChallengeDay::On(today));
    }

    #[test]
    fn test_record_result_updates_daily_best() {
        let today = crate::time::PlayerDay::from_ymd(2026, 1, 15).unwrap();
        let mut progress = ChallengeProgress::new();
        progress.start_attempt(today);

        progress.record_result(1000, 5);
        assert_eq!(progress.best_score_today, 1000);
        assert_eq!(progress.best_scenarios_today, 5);

        // Higher score should update
        progress.record_result(2000, 7);
        assert_eq!(progress.best_score_today, 2000);
        assert_eq!(progress.best_scenarios_today, 7);

        // Lower score should not update
        progress.record_result(500, 3);
        assert_eq!(progress.best_score_today, 2000);
        assert_eq!(progress.best_scenarios_today, 7);
    }

    #[test]
    fn test_record_result_updates_all_time_best() {
        let today = crate::time::PlayerDay::from_ymd(2026, 1, 15).unwrap();
        let mut progress = ChallengeProgress::new();
        progress.all_time_best_score = 3000;
        progress.all_time_best_scenarios = 8;
        progress.start_attempt(today);

        // Should not update (lower)
        progress.record_result(2000, 5);
        assert_eq!(progress.all_time_best_score, 3000);
        assert_eq!(progress.all_time_best_scenarios, 8);

        // Should update (higher)
        progress.record_result(5000, 10);
        assert_eq!(progress.all_time_best_score, 5000);
        assert_eq!(progress.all_time_best_scenarios, 10);
    }

    #[test]
    fn test_record_result_tracks_completions() {
        let today = crate::time::PlayerDay::from_ymd(2026, 1, 15).unwrap();
        let mut progress = ChallengeProgress::new();
        progress.start_attempt(today);

        // Incomplete challenge
        progress.record_result(1000, 9);
        assert_eq!(progress.total_challenges_completed, 0);

        // Complete challenge
        progress.record_result(2000, 10);
        assert_eq!(progress.total_challenges_completed, 1);
    }

    #[test]
    fn test_select_challenge_scenarios_deterministic() {
        let scenarios: Vec<Scenario> = (0..20)
            .map(|i| create_test_scenario(&format!("scenario_{i}")))
            .collect();

        let config =
            ChallengeConfig::for_date(crate::time::PlayerDay::from_ymd(2025, 1, 15).unwrap());

        let selected1 = select_challenge_scenarios(&scenarios, &config);
        let selected2 = select_challenge_scenarios(&scenarios, &config);

        // Same config should produce same selection
        assert_eq!(selected1.len(), selected2.len());
        for (s1, s2) in selected1.iter().zip(selected2.iter()) {
            assert_eq!(s1.id, s2.id);
        }
    }

    #[test]
    fn test_select_challenge_scenarios_different_days() {
        let scenarios: Vec<Scenario> = (0..20)
            .map(|i| create_test_scenario(&format!("scenario_{i}")))
            .collect();

        let config1 =
            ChallengeConfig::for_date(crate::time::PlayerDay::from_ymd(2025, 1, 15).unwrap());
        let config2 =
            ChallengeConfig::for_date(crate::time::PlayerDay::from_ymd(2025, 1, 16).unwrap());

        let selected1 = select_challenge_scenarios(&scenarios, &config1);
        let selected2 = select_challenge_scenarios(&scenarios, &config2);

        // Different days should produce different selections
        let ids1: Vec<_> = selected1.iter().map(|s| &s.id).collect();
        let ids2: Vec<_> = selected2.iter().map(|s| &s.id).collect();
        assert_ne!(ids1, ids2);
    }

    #[test]
    fn test_select_challenge_scenarios_not_enough() {
        let scenarios: Vec<Scenario> = (0..5)
            .map(|i| create_test_scenario(&format!("scenario_{i}")))
            .collect();

        let today = crate::time::PlayerDay::from_ymd(2026, 1, 15).unwrap();
        let config = ChallengeConfig::for_date(today);
        let selected = select_challenge_scenarios(&scenarios, &config);

        // Should return all available scenarios
        assert_eq!(selected.len(), 5);
    }

    #[test]
    fn test_select_challenge_scenarios_truncates() {
        let scenarios: Vec<Scenario> = (0..50)
            .map(|i| create_test_scenario(&format!("scenario_{i}")))
            .collect();

        let today = crate::time::PlayerDay::from_ymd(2026, 1, 15).unwrap();
        let config = ChallengeConfig::for_date(today);
        let selected = select_challenge_scenarios(&scenarios, &config);

        // Should only return scenario_count scenarios
        assert_eq!(selected.len(), CHALLENGE_SCENARIO_COUNT);
    }

    #[test]
    fn test_challenge_max_attempts_enforced() {
        let today = crate::time::PlayerDay::from_ymd(2026, 1, 15).unwrap();
        let mut progress = ChallengeProgress::new();

        // Use all attempts
        for _ in 0..CHALLENGE_MAX_ATTEMPTS {
            assert!(progress.can_attempt(today));
            progress.start_attempt(today);
        }

        // Should not be able to attempt anymore
        assert!(!progress.can_attempt(today));
        assert_eq!(progress.attempts_remaining(today), 0);
    }

    // CR-003: Test ChallengeProgress TOML serialization roundtrip
    #[test]
    fn test_challenge_progress_serialization_roundtrip() {
        let today = crate::time::PlayerDay::from_ymd(2026, 1, 15).unwrap();
        let mut progress = ChallengeProgress::new();
        progress.start_attempt(today);
        progress.record_result(5000, 8);

        let toml = toml::to_string(&progress).unwrap();
        let restored: ChallengeProgress = toml::from_str(&toml).unwrap();

        assert_eq!(restored.attempts_used_today, progress.attempts_used_today);
        assert_eq!(restored.best_score_today, progress.best_score_today);
        assert_eq!(restored.all_time_best_score, progress.all_time_best_score);
        assert_eq!(restored.best_scenarios_today, progress.best_scenarios_today);
        assert_eq!(
            restored.total_challenges_attempted,
            progress.total_challenges_attempted
        );
    }

    // CR-003: Test backward compatibility - verify struct has proper serde defaults
    #[test]
    fn test_challenge_progress_default_values() {
        // Verify that Default provides sensible initial values
        let progress = ChallengeProgress::default();
        assert_eq!(progress.last_challenge_date, ChallengeDay::NotStarted);
        assert_eq!(progress.attempts_used_today, 0);
        assert_eq!(progress.best_score_today, 0);
        assert_eq!(progress.best_scenarios_today, 0);
        assert_eq!(progress.all_time_best_score, 0);
        assert_eq!(progress.all_time_best_scenarios, 0);
        assert_eq!(progress.total_challenges_attempted, 0);
        assert_eq!(progress.total_challenges_completed, 0);
    }

    fn load_with_date(date: Option<serde_json::Value>, attempts: u8) -> ChallengeProgress {
        let mut json = serde_json::to_value(ChallengeProgress {
            all_time_best_score: 42,
            attempts_used_today: attempts,
            ..ChallengeProgress::new()
        })
        .unwrap();
        let object = json.as_object_mut().unwrap();
        match date {
            Some(value) => object.insert("last_challenge_date".to_owned(), value),
            None => object.remove("last_challenge_date"),
        };
        // from_str, not from_value: production loads via the streaming deserializer.
        serde_json::from_str(&json.to_string()).unwrap()
    }

    #[test]
    fn test_last_challenge_date_deserialize_valid() {
        let p = load_with_date(Some("2026-01-15".into()), 2);
        assert_eq!(
            p.last_challenge_date,
            ChallengeDay::On(crate::time::PlayerDay::from_ymd(2026, 1, 15).unwrap())
        );
    }

    #[test]
    fn test_last_challenge_date_deserialize_malformed_is_unreadable() {
        for text in ["not-a-date", "2026-02-30", "2026-01-15T10:00:00"] {
            let p = load_with_date(Some(text.into()), 3);
            assert_eq!(p.last_challenge_date, ChallengeDay::Unreadable, "{text}");
            assert_eq!(p.all_time_best_score, 42, "{text}");
            assert_eq!(p.attempts_used_today, 3, "{text}");
        }
    }

    #[test]
    fn test_last_challenge_date_deserialize_wrong_type_is_unreadable() {
        for value in [
            serde_json::json!(12345),
            serde_json::json!([1, 2]),
            serde_json::json!({"a": 1}),
        ] {
            let p = load_with_date(Some(value), 0);
            assert_eq!(p.last_challenge_date, ChallengeDay::Unreadable);
            assert_eq!(p.all_time_best_score, 42);
        }
    }

    #[test]
    fn test_last_challenge_date_missing_or_null_depends_on_attempts() {
        for date in [None, Some(serde_json::Value::Null)] {
            let fresh = load_with_date(date.clone(), 0);
            assert_eq!(fresh.last_challenge_date, ChallengeDay::NotStarted);
            let used = load_with_date(date, 2);
            assert_eq!(used.last_challenge_date, ChallengeDay::Unreadable);
        }
    }

    #[test]
    fn test_unreadable_day_never_resets_attempts() {
        let today = crate::time::PlayerDay::from_ymd(2026, 1, 15).unwrap();
        let mut p = load_with_date(Some("garbage".into()), CHALLENGE_MAX_ATTEMPTS);
        assert!(!p.can_attempt(today));
        assert_eq!(p.attempts_remaining(today), 0);

        p.settle_unreadable_day(today);
        assert_eq!(p.last_challenge_date, ChallengeDay::On(today));
        assert!(!p.can_attempt(today));
        assert_eq!(p.attempts_used_today, CHALLENGE_MAX_ATTEMPTS);
    }

    #[test]
    fn test_unreadable_day_serializes_as_null_and_reloads_unreadable() {
        let p = load_with_date(Some("garbage".into()), 2);
        let json = serde_json::to_value(&p).unwrap();
        assert!(json["last_challenge_date"].is_null());
        let reloaded: ChallengeProgress = serde_json::from_str(&json.to_string()).unwrap();
        assert_eq!(reloaded.last_challenge_date, ChallengeDay::Unreadable);
    }

    #[test]
    fn test_last_challenge_date_serializes_as_iso_string() {
        let mut p = ChallengeProgress::new();
        p.start_attempt(crate::time::PlayerDay::from_ymd(2026, 1, 15).unwrap());
        let json = serde_json::to_value(&p).unwrap();
        assert_eq!(json["last_challenge_date"], "2026-01-15");
    }

    // CR-008: Test is_today() method
    #[test]
    fn test_challenge_progress_is_today() {
        let today = crate::time::PlayerDay::from_ymd(2026, 1, 15).unwrap();
        let mut progress = ChallengeProgress::new();
        assert!(!progress.is_today(today)); // No date set yet

        progress.start_attempt(today);
        assert!(progress.is_today(today)); // Now has today's date

        progress.last_challenge_date =
            ChallengeDay::On(crate::time::PlayerDay::from_ymd(2020, 1, 1).unwrap());
        assert!(!progress.is_today(today)); // Past date
    }

    // CR-018: Test empty scenario list
    #[test]
    fn test_select_challenge_scenarios_empty_input() {
        let scenarios: Vec<Scenario> = vec![];
        let today = crate::time::PlayerDay::from_ymd(2026, 1, 15).unwrap();
        let config = ChallengeConfig::for_date(today);
        let selected = select_challenge_scenarios(&scenarios, &config);
        assert!(selected.is_empty());
    }

    proptest! {
        #[test]
        fn prop_select_challenge_scenarios_is_deterministic_and_bounded(
            pool_size in 0usize..30,
            scenario_count in 1usize..15,
            seed in any::<u64>(),
        ) {
            let scenarios: Vec<Scenario> = (0..pool_size)
                .map(|i| create_test_scenario(&format!("scenario_{i}")))
                .collect();

            let mut config =
                ChallengeConfig::for_date(crate::time::PlayerDay::from_ymd(2026, 1, 15).unwrap());
            config.scenario_count = scenario_count;
            config.seed = seed;

            let selected1 = select_challenge_scenarios(&scenarios, &config);
            let selected2 = select_challenge_scenarios(&scenarios, &config);

            // Same config must always select the same scenarios, in the same order
            prop_assert_eq!(
                selected1.iter().map(|s| s.id.clone()).collect::<Vec<_>>(),
                selected2.iter().map(|s| s.id.clone()).collect::<Vec<_>>()
            );

            // Selection length is bounded by both the request and the available pool
            prop_assert_eq!(selected1.len(), pool_size.min(scenario_count));

            // No scenario is selected more than once
            let unique_ids: HashSet<_> = selected1.iter().map(|s| &s.id).collect();
            prop_assert_eq!(unique_ids.len(), selected1.len());

            // Every selected scenario comes from the original pool
            let original_ids: HashSet<_> = scenarios.iter().map(|s| &s.id).collect();
            prop_assert!(selected1.iter().all(|s| original_ids.contains(&s.id)));
        }

        #[test]
        fn prop_select_challenge_scenarios_seed_changes_the_selection(
            pool_size in 20usize..30,
            scenario_count in 1usize..5,
            seeds in prop::collection::hash_set(any::<u64>(), 5..10),
        ) {
            let scenarios: Vec<Scenario> = (0..pool_size)
                .map(|i| create_test_scenario(&format!("scenario_{i}")))
                .collect();

            let mut config =
                ChallengeConfig::for_date(crate::time::PlayerDay::from_ymd(2026, 1, 15).unwrap());
            config.scenario_count = scenario_count;

            let outcomes: HashSet<Vec<crate::ids::ScenarioId>> = seeds
                .into_iter()
                .map(|seed| {
                    config.seed = seed;
                    select_challenge_scenarios(&scenarios, &config)
                        .into_iter()
                        .map(|s| s.id.clone())
                        .collect()
                })
                .collect();

            // With a pool much larger than the requested count, distinct seeds
            // must not all collapse onto the same selection/order.
            prop_assert!(outcomes.len() >= 2);
        }
    }
}
