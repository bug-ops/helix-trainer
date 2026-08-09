//! Scenario completion service
//!
//! Handles XP calculation, mastery tracking, and FSRS recording

use crate::game::Feedback;
use crate::gamification::UserProfile;
use crate::learning::{PerformanceTracker, ScenarioMastery, Scheduler};
use chrono::{DateTime, Utc};
use std::time::Duration;

/// Components of XP calculation before mastery scaling
#[derive(Debug, Clone, Default)]
pub struct XPComponents {
    pub base_xp: u64,
    pub perfect_bonus: u64,
    pub first_today_bonus: u64,
    pub total_base_xp: u64,
}

/// Result of completing a scenario
#[derive(Debug, Clone)]
pub struct CompletionResult {
    pub xp_components: XPComponents,
    pub actual_xp: u64,
    pub mastery_level: ScenarioMastery,
    pub mastery_multiplier: f64,
    pub leveled_up: bool,
}

/// Result of recording a scenario completion and scaling its XP by mastery
///
/// Fields are a mix of post-recording values (`actual_xp`, `mastery_level`)
/// and pre-recording values (`applied_multiplier`, `applied_mastery_factor`,
/// `applied_repeat_penalty`), each documented individually below to avoid
/// ambiguity.
#[derive(Debug, Clone)]
pub struct XPScalingResult {
    /// XP actually awarded for this completion, after mastery/repeat scaling;
    /// reflects the post-recording state
    pub actual_xp: u64,
    /// Mastery level *after* recording this completion, for display purposes
    pub mastery_level: ScenarioMastery,
    /// Combined multiplier (`applied_mastery_factor * applied_repeat_penalty`)
    /// that was applied to this completion, captured *before* recording
    pub applied_multiplier: f64,
    /// Mastery-level XP factor that was applied to this completion,
    /// captured *before* recording
    pub applied_mastery_factor: f64,
    /// Session repeat-penalty factor that was applied to this completion,
    /// captured *before* recording
    pub applied_repeat_penalty: f64,
}

/// Service for scenario completion operations
pub struct ScenarioCompletionService;

impl ScenarioCompletionService {
    /// Calculate XP components from scenario feedback (pure function)
    #[must_use]
    pub fn calculate_xp_components(feedback: &Feedback, is_first_today: bool) -> XPComponents {
        let score = feedback.score;
        let is_perfect = feedback.score == feedback.max_points;

        let base_xp = (u64::from(score) * 50) / 100;
        let perfect_bonus = if is_perfect { base_xp / 5 } else { 0 };
        let first_today_bonus = if is_first_today { 10 } else { 0 };
        let total_base_xp = base_xp + perfect_bonus + first_today_bonus;

        XPComponents {
            base_xp,
            perfect_bonus,
            first_today_bonus,
            total_base_xp,
        }
    }

    /// Record scenario completion and return mastery-scaled XP
    ///
    /// See [`XPScalingResult`] for the distinction between the post-recording
    /// `mastery_level` field and the pre-recording multiplier fields.
    #[must_use]
    pub fn record_and_scale_xp(
        profile: &mut UserProfile,
        scenario_id: &str,
        score: u32,
        base_xp: u64,
        now: DateTime<Utc>,
    ) -> XPScalingResult {
        // Capture multipliers BEFORE recording (what will be applied)
        let (_pre_mastery_level, pre_mastery_factor, pre_repeat_penalty) = profile
            .scenario_history
            .get(scenario_id)
            .map(|c| (c.mastery_level, c.mastery_factor(), c.repeat_penalty()))
            .unwrap_or((ScenarioMastery::Learning, 1.0, 1.0));

        let actual_xp =
            profile
                .scenario_history
                .record_completion(scenario_id, score, base_xp, now);

        // Get post-recording mastery level (may have changed due to this completion)
        let post_mastery_level = profile
            .scenario_history
            .get(scenario_id)
            .map(|c| c.mastery_level)
            .unwrap_or(ScenarioMastery::Learning);

        let applied_multiplier = pre_mastery_factor * pre_repeat_penalty;

        // Return post-mastery level (for display) but pre-multipliers (what was applied)
        XPScalingResult {
            actual_xp,
            mastery_level: post_mastery_level,
            applied_multiplier,
            applied_mastery_factor: pre_mastery_factor,
            applied_repeat_penalty: pre_repeat_penalty,
        }
    }

    /// Update profile counters after scenario completion
    pub fn update_profile_counters(profile: &mut UserProfile, is_perfect: bool) {
        profile.scenarios_completed += 1;
        if is_perfect {
            profile.perfect_scenarios += 1;
        }
    }

    /// Record commands in FSRS scheduler for spaced repetition
    pub fn record_fsrs_data(
        scheduler: &mut Scheduler,
        tracker: &mut PerformanceTracker,
        commands: &[String],
        duration: Duration,
        success: bool,
    ) {
        scheduler.record_scenario_commands(tracker, commands, duration, success);
    }

    /// Record commands and return mastery level changes
    ///
    /// Returns list of (command, new_level_name) for commands that leveled up
    pub fn record_fsrs_data_with_mastery(
        scheduler: &mut Scheduler,
        tracker: &mut PerformanceTracker,
        commands: &[String],
        duration: Duration,
        success: bool,
    ) -> Vec<(String, String)> {
        scheduler.record_scenario_commands_with_mastery(tracker, commands, duration, success)
    }

    /// Extract command strings from feedback
    #[must_use]
    pub fn extract_commands(feedback: &Feedback) -> Vec<String> {
        feedback
            .user_actions
            .iter()
            .map(|action| action.command.clone())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::PerformanceRating;

    fn create_test_feedback(score: u32, max_points: u32) -> Feedback {
        Feedback {
            scenario_id: "test".to_string(),
            success: true,
            score,
            max_points,
            rating: PerformanceRating::Perfect,
            actions_taken: 1,
            optimal_actions: 1,
            duration: Duration::from_secs(5),
            hint: None,
            is_optimal: true,
            user_actions: vec![],
        }
    }

    #[test]
    fn test_xp_components_base_only() {
        let feedback = create_test_feedback(100, 150);
        let components = ScenarioCompletionService::calculate_xp_components(&feedback, false);

        assert_eq!(components.base_xp, 50);
        assert_eq!(components.perfect_bonus, 0);
        assert_eq!(components.first_today_bonus, 0);
        assert_eq!(components.total_base_xp, 50);
    }

    #[test]
    fn test_xp_components_perfect_bonus() {
        let feedback = create_test_feedback(100, 100);
        let components = ScenarioCompletionService::calculate_xp_components(&feedback, false);

        assert_eq!(components.base_xp, 50);
        assert_eq!(components.perfect_bonus, 10);
        assert_eq!(components.total_base_xp, 60);
    }

    #[test]
    fn test_xp_components_first_today_bonus() {
        let feedback = create_test_feedback(100, 150);
        let components = ScenarioCompletionService::calculate_xp_components(&feedback, true);

        assert_eq!(components.first_today_bonus, 10);
        assert_eq!(components.total_base_xp, 60);
    }

    #[test]
    fn test_xp_components_all_bonuses() {
        let feedback = create_test_feedback(100, 100);
        let components = ScenarioCompletionService::calculate_xp_components(&feedback, true);

        assert_eq!(components.base_xp, 50);
        assert_eq!(components.perfect_bonus, 10);
        assert_eq!(components.first_today_bonus, 10);
        assert_eq!(components.total_base_xp, 70);
    }

    #[test]
    fn test_xp_components_zero_score() {
        let feedback = create_test_feedback(0, 100);
        let components = ScenarioCompletionService::calculate_xp_components(&feedback, false);

        assert_eq!(components.base_xp, 0);
        assert_eq!(components.perfect_bonus, 0);
        assert_eq!(components.total_base_xp, 0);
    }

    #[test]
    fn test_update_profile_counters() {
        let mut profile = UserProfile::new();
        assert_eq!(profile.scenarios_completed, 0);
        assert_eq!(profile.perfect_scenarios, 0);

        ScenarioCompletionService::update_profile_counters(&mut profile, false);
        assert_eq!(profile.scenarios_completed, 1);
        assert_eq!(profile.perfect_scenarios, 0);

        ScenarioCompletionService::update_profile_counters(&mut profile, true);
        assert_eq!(profile.scenarios_completed, 2);
        assert_eq!(profile.perfect_scenarios, 1);
    }

    #[test]
    fn test_xp_components_partial_score() {
        let feedback = create_test_feedback(75, 100);
        let components = ScenarioCompletionService::calculate_xp_components(&feedback, false);

        assert_eq!(components.base_xp, 37);
        assert_eq!(components.perfect_bonus, 0);
        assert_eq!(components.total_base_xp, 37);
    }

    #[test]
    fn test_xp_components_high_score() {
        let feedback = create_test_feedback(200, 200);
        let components = ScenarioCompletionService::calculate_xp_components(&feedback, true);

        assert_eq!(components.base_xp, 100);
        assert_eq!(components.perfect_bonus, 20);
        assert_eq!(components.first_today_bonus, 10);
        assert_eq!(components.total_base_xp, 130);
    }

    #[test]
    fn test_xp_components_low_score() {
        let feedback = create_test_feedback(10, 100);
        let components = ScenarioCompletionService::calculate_xp_components(&feedback, false);

        assert_eq!(components.base_xp, 5);
        assert_eq!(components.perfect_bonus, 0);
        assert_eq!(components.total_base_xp, 5);
    }

    #[test]
    fn test_xp_components_rounding() {
        let feedback = create_test_feedback(51, 100);
        let components = ScenarioCompletionService::calculate_xp_components(&feedback, false);

        assert_eq!(components.base_xp, 25);
        assert_eq!(components.total_base_xp, 25);
    }

    #[test]
    fn test_extract_commands_empty() {
        let feedback = create_test_feedback(100, 100);
        let commands = ScenarioCompletionService::extract_commands(&feedback);
        assert!(commands.is_empty());
    }

    #[test]
    fn test_extract_commands_single() {
        use crate::game::UserAction;

        let mut feedback = create_test_feedback(100, 100);
        feedback.user_actions = vec![UserAction {
            command: "dd".to_string(),
            timestamp: Duration::from_millis(100),
        }];

        let commands = ScenarioCompletionService::extract_commands(&feedback);
        assert_eq!(commands, vec!["dd"]);
    }

    #[test]
    fn test_extract_commands_multiple() {
        use crate::game::UserAction;

        let mut feedback = create_test_feedback(100, 100);
        feedback.user_actions = vec![
            UserAction {
                command: "d".to_string(),
                timestamp: Duration::from_millis(100),
            },
            UserAction {
                command: "w".to_string(),
                timestamp: Duration::from_millis(200),
            },
            UserAction {
                command: "j".to_string(),
                timestamp: Duration::from_millis(300),
            },
        ];

        let commands = ScenarioCompletionService::extract_commands(&feedback);
        assert_eq!(commands, vec!["d", "w", "j"]);
    }

    #[test]
    fn test_record_and_scale_xp_first_completion() {
        let mut profile = UserProfile::new();
        let result = ScenarioCompletionService::record_and_scale_xp(
            &mut profile,
            "test_scenario",
            100,
            50,
            Utc::now(),
        );

        // First completion gets full XP (no penalty)
        assert_eq!(result.actual_xp, 50);
        assert_eq!(result.mastery_level, ScenarioMastery::Learning);
        // Pre-recording multipliers were 1.0 (no prior completion)
        assert!((result.applied_multiplier - 1.0).abs() < 0.01);
        assert!((result.applied_mastery_factor - 1.0).abs() < 0.01);
        assert!((result.applied_repeat_penalty - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_record_and_scale_xp_tracks_attempts() {
        let mut profile = UserProfile::new();

        for _ in 0..5 {
            let _ = ScenarioCompletionService::record_and_scale_xp(
                &mut profile,
                "test_scenario",
                100,
                50,
                Utc::now(),
            );
        }

        let completion = profile.scenario_history.get("test_scenario").unwrap();
        assert_eq!(completion.attempts, 5);
    }
}
