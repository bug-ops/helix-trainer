//! Scenario completion service
//!
//! Handles XP calculation, mastery tracking, and FSRS recording

use crate::game::Feedback;
use crate::gamification::UserProfile;
use crate::learning::{PerformanceTracker, ScenarioMastery, Scheduler};
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
    /// Returns: (actual_xp, mastery_level, applied_multiplier, applied_mastery_factor, applied_repeat_penalty)
    /// Note: multipliers returned are what was APPLIED to this completion, not current state
    #[must_use]
    pub fn record_and_scale_xp(
        profile: &mut UserProfile,
        scenario_id: &str,
        score: u32,
        base_xp: u64,
    ) -> (u64, ScenarioMastery, f64, f64, f64) {
        // Capture multipliers BEFORE recording (what will be applied)
        let (_pre_mastery_level, pre_mastery_factor, pre_repeat_penalty) = profile
            .scenario_history
            .get(scenario_id)
            .map(|c| (c.mastery_level, c.mastery_factor(), c.repeat_penalty()))
            .unwrap_or((ScenarioMastery::Learning, 1.0, 1.0));

        let actual_xp = profile
            .scenario_history
            .record_completion(scenario_id, score, base_xp);

        // Get post-recording mastery level (may have changed due to this completion)
        let post_mastery_level = profile
            .scenario_history
            .get(scenario_id)
            .map(|c| c.mastery_level)
            .unwrap_or(ScenarioMastery::Learning);

        let applied_multiplier = pre_mastery_factor * pre_repeat_penalty;

        // Return post-mastery level (for display) but pre-multipliers (what was applied)
        (
            actual_xp,
            post_mastery_level,
            applied_multiplier,
            pre_mastery_factor,
            pre_repeat_penalty,
        )
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
        let (actual_xp, mastery, multiplier, mastery_factor, repeat_penalty) =
            ScenarioCompletionService::record_and_scale_xp(&mut profile, "test_scenario", 100, 50);

        // First completion gets full XP (no penalty)
        assert_eq!(actual_xp, 50);
        assert_eq!(mastery, ScenarioMastery::Learning);
        // Pre-recording multipliers were 1.0 (no prior completion)
        assert!((multiplier - 1.0).abs() < 0.01);
        assert!((mastery_factor - 1.0).abs() < 0.01);
        assert!((repeat_penalty - 1.0).abs() < 0.01);
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
            );
        }

        let completion = profile.scenario_history.get("test_scenario").unwrap();
        assert_eq!(completion.attempts, 5);
    }
}
