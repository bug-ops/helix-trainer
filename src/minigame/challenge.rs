//! Challenge mode logic and progress tracking
//!
//! This module handles daily challenge functionality:
//! - Progress tracking with attempt limits
//! - Best score tracking (daily and all-time)
//! - Deterministic scenario selection using seeded RNG

use chrono::NaiveDate;
use rand::SeedableRng;
use rand::prelude::SliceRandom;
use serde::{Deserialize, Serialize};

use crate::config::Scenario;
use crate::constants::{CHALLENGE_MAX_ATTEMPTS, CHALLENGE_SCENARIO_COUNT};

use super::ChallengeConfig;

/// Tracks player's progress on daily challenges.
///
/// Persisted to UserProfile for cross-session tracking.
///
/// # TODO: Performance Optimization
///
/// Consider storing `NaiveDate` directly instead of `String` format to avoid
/// repeated string allocations in `can_attempt()`, `attempts_remaining()`,
/// `start_attempt()`, and `is_today()` methods. `NaiveDate`'s serde format is
/// the same `"YYYY-MM-DD"` string already used here (and already persisted
/// elsewhere in `profile.json` as a bare `NaiveDate`, e.g.
/// `ScenarioCompletion::last_attempt_date`), so this is not a wire-format
/// migration. The real, narrower risk: `ProfileStorage::load` currently hard-
/// errors the whole profile load on any parse failure, so a malformed stored
/// date for this field would need `#[serde(default, deserialize_with = ...)]`
/// mapping bad values to `None` instead of propagating a parse error. Out of
/// scope for this batch — deferred as a small follow-up, not a blocked one.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChallengeProgress {
    /// Date of current/last challenge (YYYY-MM-DD format for serialization)
    // TODO: CR-013: Consider storing chrono::NaiveDate directly with custom serde.
    // Deferred: needs a lenient deserializer so a malformed value doesn't hard-error
    // the whole profile load; see struct-level doc above.
    pub last_challenge_date: Option<String>,

    /// Attempts used today (0-3)
    pub attempts_used_today: u8,

    /// Best score achieved today
    pub best_score_today: u64,

    /// Best scenarios completed today (out of 10)
    pub best_scenarios_today: u8,

    /// All-time best challenge score
    pub all_time_best_score: u64,

    /// All-time best scenarios completed in a single challenge
    pub all_time_best_scenarios: u8,

    /// Total challenges attempted (lifetime)
    pub total_challenges_attempted: u32,

    /// Total challenges completed (all 10 scenarios)
    pub total_challenges_completed: u32,
}

impl ChallengeProgress {
    /// Create a new empty challenge progress.
    ///
    /// Note: This is identical to `Default::default()` but provides a conventional
    /// constructor for explicit initialization. Using `new()` signals intentional
    /// creation of an empty progress state, while `Default` is typically used
    /// for struct field initialization in derives.
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if player can attempt today's challenge
    pub fn can_attempt(&self, today: NaiveDate) -> bool {
        let today = Self::today_string(today);

        // Different day = reset attempts
        if self.last_challenge_date.as_deref() != Some(&today) {
            return true;
        }

        self.attempts_used_today < CHALLENGE_MAX_ATTEMPTS
    }

    /// Get remaining attempts for today
    pub fn attempts_remaining(&self, today: NaiveDate) -> u8 {
        let today = Self::today_string(today);

        if self.last_challenge_date.as_deref() != Some(&today) {
            return CHALLENGE_MAX_ATTEMPTS;
        }

        CHALLENGE_MAX_ATTEMPTS.saturating_sub(self.attempts_used_today)
    }

    /// Start a new attempt, resetting if new day
    pub fn start_attempt(&mut self, today: NaiveDate) {
        let today = Self::today_string(today);

        // Reset for new day
        if self.last_challenge_date.as_deref() != Some(&today) {
            self.last_challenge_date = Some(today);
            self.attempts_used_today = 0;
            self.best_score_today = 0;
            self.best_scenarios_today = 0;
        }

        self.attempts_used_today = self.attempts_used_today.saturating_add(1);
        self.total_challenges_attempted = self.total_challenges_attempted.saturating_add(1);
    }

    /// Record attempt result
    pub fn record_result(&mut self, score: u64, scenarios_completed: u8) {
        // Update today's best
        if score > self.best_score_today {
            self.best_score_today = score;
        }
        if scenarios_completed > self.best_scenarios_today {
            self.best_scenarios_today = scenarios_completed;
        }

        // Update all-time best
        if score > self.all_time_best_score {
            self.all_time_best_score = score;
        }
        if scenarios_completed > self.all_time_best_scenarios {
            self.all_time_best_scenarios = scenarios_completed;
        }

        // Track completions
        if scenarios_completed >= CHALLENGE_SCENARIO_COUNT as u8 {
            self.total_challenges_completed = self.total_challenges_completed.saturating_add(1);
        }
    }

    /// Get the given date as a YYYY-MM-DD string
    fn today_string(today: NaiveDate) -> String {
        today.to_string()
    }

    /// Check if progress is for the given date
    pub fn is_today(&self, today: NaiveDate) -> bool {
        self.last_challenge_date.as_deref() == Some(&Self::today_string(today))
    }
}

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
/// Uses index-based shuffling to reduce from O(n) clones to O(k) clones,
/// where k is `config.scenario_count` (typically 10).
///
/// # TODO: `Arc<Scenario>` optimization
///
/// Consider using `Arc<Scenario>` to avoid deep clones entirely. Currently
/// bounded to k=10 clones per call, which is acceptable but could be
/// eliminated with reference counting. Out of scope for now: the selection
/// this function returns is stored as owned `Scenario` end-to-end (through
/// `MiniGameSession::with_mode`, `refill_queue`'s `VecDeque<Scenario>` queue,
/// and the public `ActiveMiniScenario::scenario` field also used by the
/// non-Challenge Arcade/Survival path via `DifficultyController::next_scenario`),
/// so switching to `Arc<Scenario>` here alone would just move the clone rather
/// than remove it — it needs a coordinated type change across all of those
/// call sites.
///
/// # Examples
///
/// ```ignore
/// use helix_trainer::minigame::{ChallengeConfig, select_challenge_scenarios};
///
/// let scenarios = load_scenarios();
/// let config = ChallengeConfig::for_date(chrono::Utc::now().date_naive());
/// let selected = select_challenge_scenarios(&scenarios, &config);
/// assert_eq!(selected.len(), 10);
/// ```
pub fn select_challenge_scenarios(
    scenarios: &[Scenario],
    config: &ChallengeConfig,
) -> Vec<Scenario> {
    if scenarios.is_empty() {
        return Vec::new();
    }

    if scenarios.len() < config.scenario_count {
        // Not enough scenarios, return all available
        return scenarios.to_vec();
    }

    // CR-004: Use index-based shuffling to reduce clones from O(n) to O(k)
    let mut rng = rand::rngs::StdRng::seed_from_u64(config.seed);
    let mut indices: Vec<usize> = (0..scenarios.len()).collect();
    indices.shuffle(&mut rng);
    indices.truncate(config.scenario_count);
    indices.into_iter().map(|i| scenarios[i].clone()).collect()
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

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
        assert!(progress.last_challenge_date.is_none());
        assert_eq!(progress.attempts_used_today, 0);
        assert_eq!(progress.best_score_today, 0);
        assert_eq!(progress.all_time_best_score, 0);
    }

    #[test]
    fn test_can_attempt_new_day() {
        let today = NaiveDate::from_ymd_opt(2026, 1, 15).unwrap();
        let progress = ChallengeProgress::new();
        assert!(progress.can_attempt(today));
        assert_eq!(progress.attempts_remaining(today), CHALLENGE_MAX_ATTEMPTS);
    }

    #[test]
    fn test_can_attempt_after_using_attempts() {
        let today = NaiveDate::from_ymd_opt(2026, 1, 15).unwrap();
        let mut progress = ChallengeProgress::new();
        progress.last_challenge_date = Some(today.to_string());
        progress.attempts_used_today = 2;

        assert!(progress.can_attempt(today));
        assert_eq!(progress.attempts_remaining(today), 1);

        progress.attempts_used_today = 3;
        assert!(!progress.can_attempt(today));
        assert_eq!(progress.attempts_remaining(today), 0);
    }

    #[test]
    fn test_start_attempt_new_day() {
        let today = NaiveDate::from_ymd_opt(2026, 1, 15).unwrap();
        let mut progress = ChallengeProgress::new();
        progress.last_challenge_date = Some("2020-01-01".to_string());
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
        let today = NaiveDate::from_ymd_opt(2026, 1, 15).unwrap();
        let mut progress = ChallengeProgress::new();
        progress.last_challenge_date = Some(today.to_string());
        progress.attempts_used_today = 1;
        progress.best_score_today = 5000;

        progress.start_attempt(today);

        // Should not reset, just increment
        assert_eq!(progress.attempts_used_today, 2);
        assert_eq!(progress.best_score_today, 5000);
        assert_eq!(progress.last_challenge_date, Some(today.to_string()));
    }

    #[test]
    fn test_record_result_updates_daily_best() {
        let today = NaiveDate::from_ymd_opt(2026, 1, 15).unwrap();
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
        let today = NaiveDate::from_ymd_opt(2026, 1, 15).unwrap();
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
        let today = NaiveDate::from_ymd_opt(2026, 1, 15).unwrap();
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
            .map(|i| create_test_scenario(&format!("scenario_{}", i)))
            .collect();

        let config =
            ChallengeConfig::for_date(chrono::NaiveDate::from_ymd_opt(2025, 1, 15).unwrap());

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
            .map(|i| create_test_scenario(&format!("scenario_{}", i)))
            .collect();

        let config1 =
            ChallengeConfig::for_date(chrono::NaiveDate::from_ymd_opt(2025, 1, 15).unwrap());
        let config2 =
            ChallengeConfig::for_date(chrono::NaiveDate::from_ymd_opt(2025, 1, 16).unwrap());

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
            .map(|i| create_test_scenario(&format!("scenario_{}", i)))
            .collect();

        let today = NaiveDate::from_ymd_opt(2026, 1, 15).unwrap();
        let config = ChallengeConfig::for_date(today);
        let selected = select_challenge_scenarios(&scenarios, &config);

        // Should return all available scenarios
        assert_eq!(selected.len(), 5);
    }

    #[test]
    fn test_select_challenge_scenarios_truncates() {
        let scenarios: Vec<Scenario> = (0..50)
            .map(|i| create_test_scenario(&format!("scenario_{}", i)))
            .collect();

        let today = NaiveDate::from_ymd_opt(2026, 1, 15).unwrap();
        let config = ChallengeConfig::for_date(today);
        let selected = select_challenge_scenarios(&scenarios, &config);

        // Should only return scenario_count scenarios
        assert_eq!(selected.len(), CHALLENGE_SCENARIO_COUNT);
    }

    #[test]
    fn test_challenge_max_attempts_enforced() {
        let today = NaiveDate::from_ymd_opt(2026, 1, 15).unwrap();
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
        let today = NaiveDate::from_ymd_opt(2026, 1, 15).unwrap();
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
        assert!(progress.last_challenge_date.is_none());
        assert_eq!(progress.attempts_used_today, 0);
        assert_eq!(progress.best_score_today, 0);
        assert_eq!(progress.best_scenarios_today, 0);
        assert_eq!(progress.all_time_best_score, 0);
        assert_eq!(progress.all_time_best_scenarios, 0);
        assert_eq!(progress.total_challenges_attempted, 0);
        assert_eq!(progress.total_challenges_completed, 0);
    }

    // CR-008: Test is_today() method
    #[test]
    fn test_challenge_progress_is_today() {
        let today = NaiveDate::from_ymd_opt(2026, 1, 15).unwrap();
        let mut progress = ChallengeProgress::new();
        assert!(!progress.is_today(today)); // No date set yet

        progress.start_attempt(today);
        assert!(progress.is_today(today)); // Now has today's date

        progress.last_challenge_date = Some("2020-01-01".to_string());
        assert!(!progress.is_today(today)); // Past date
    }

    // CR-018: Test empty scenario list
    #[test]
    fn test_select_challenge_scenarios_empty_input() {
        let scenarios: Vec<Scenario> = vec![];
        let today = NaiveDate::from_ymd_opt(2026, 1, 15).unwrap();
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
                ChallengeConfig::for_date(NaiveDate::from_ymd_opt(2026, 1, 15).unwrap());
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
                ChallengeConfig::for_date(NaiveDate::from_ymd_opt(2026, 1, 15).unwrap());
            config.scenario_count = scenario_count;

            let outcomes: HashSet<Vec<String>> = seeds
                .into_iter()
                .map(|seed| {
                    config.seed = seed;
                    select_challenge_scenarios(&scenarios, &config)
                        .into_iter()
                        .map(|s| s.id)
                        .collect()
                })
                .collect();

            // With a pool much larger than the requested count, distinct seeds
            // must not all collapse onto the same selection/order.
            prop_assert!(outcomes.len() >= 2);
        }
    }
}
