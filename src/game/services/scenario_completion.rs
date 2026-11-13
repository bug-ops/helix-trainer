//! Scenario completion service
//!
//! Handles XP calculation, mastery tracking, and FSRS recording

use crate::config::Difficulty;
use crate::constants::{FLASH_TIME_RATIO, SPEED_DEMON_TIME_RATIO};
use crate::game::Feedback;
use crate::gamification::{
    AchievementEngine, AchievementId, UserProfile, XPCalculator, XPComponents, speed_time_ratio,
};
use crate::ids::{CommandId, ScenarioId};
use crate::learning::{
    AttemptOutcome, MasteryLevel, PerformanceTracker, ReferenceTime, ScenarioMastery, Scheduler,
};
use chrono::{DateTime, Utc};
use std::time::Duration;

/// Result of completing a scenario
#[derive(Debug, Clone)]
pub struct CompletionResult {
    /// Itemised XP breakdown (base, bonuses) before mastery scaling.
    pub xp_components: XPComponents,
    /// XP actually awarded after mastery scaling.
    pub actual_xp: u64,
    /// Mastery tier of the scenario after this completion was recorded.
    pub mastery_level: ScenarioMastery,
    /// Multiplier the mastery tier applied to the base XP.
    pub mastery_multiplier: f64,
    /// Whether the awarded XP raised the player's level.
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

/// What a scenario completion earned that the player should be told about.
///
/// The game layer reports the event; the UI decides how to show it (it converts
/// each into its notification type).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompletionEvent {
    /// An achievement was unlocked.
    AchievementUnlocked {
        /// The achievement that was unlocked.
        id: AchievementId,
    },
    /// The account level went up.
    LevelUp {
        /// The level reached.
        new_level: u32,
    },
}

/// Service for scenario completion operations
pub struct ScenarioCompletionService;

impl ScenarioCompletionService {
    /// Calculate XP components from scenario feedback (pure function)
    #[must_use]
    pub fn calculate_xp_components(feedback: &Feedback, is_first_today: bool) -> XPComponents {
        XPCalculator::training_xp_components(
            feedback.score,
            feedback.score == feedback.max_points,
            is_first_today,
        )
    }

    /// Record scenario completion and return mastery-scaled XP
    ///
    /// See `XPScalingResult` for the distinction between the post-recording
    /// `mastery_level` field and the pre-recording multiplier fields.
    #[must_use]
    pub fn record_and_scale_xp(
        profile: &mut UserProfile,
        scenario_id: &ScenarioId,
        score: u32,
        base_xp: u64,
        now: DateTime<Utc>,
    ) -> XPScalingResult {
        // Capture multipliers BEFORE recording (what will be applied)
        let (_pre_mastery_level, pre_mastery_factor, pre_repeat_penalty) = profile
            .scenario_history
            .get(scenario_id)
            .map_or((ScenarioMastery::Learning, 1.0, 1.0), |c| {
                (c.mastery_level, c.mastery_factor(), c.repeat_penalty())
            });

        let actual_xp =
            profile
                .scenario_history
                .record_completion(scenario_id, score, base_xp, now);

        // Get post-recording mastery level (may have changed due to this completion)
        let post_mastery_level = profile
            .scenario_history
            .get(scenario_id)
            .map_or(ScenarioMastery::Learning, |c| c.mastery_level);

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
        profile.scenarios_completed = profile.scenarios_completed.saturating_add(1);
        if is_perfect {
            profile.perfect_scenarios = profile.perfect_scenarios.saturating_add(1);
        }
    }

    /// Record commands and return mastery level changes
    ///
    /// Returns the commands that leveled up with their new level, in first-occurrence order.
    pub fn record_fsrs_data_with_mastery(
        scheduler: &mut Scheduler,
        tracker: &mut PerformanceTracker,
        commands: &[CommandId],
        duration: Duration,
        outcome: AttemptOutcome,
        reference: ReferenceTime,
    ) -> Vec<(CommandId, MasteryLevel)> {
        scheduler
            .record_scenario_commands_with_mastery(tracker, commands, duration, outcome, reference)
    }

    /// Extract reviewable command ids from feedback
    ///
    /// Goes through [`CommandId`]: typed text, `Escape` and argument
    /// characters yield no card, and register ops (`"ay` -> `"y`) and
    /// command-line invocations (`:g 3` -> `:goto`) are normalized so FSRS mints
    /// one learning card per skill.
    #[must_use]
    pub fn extract_commands(feedback: &Feedback) -> Vec<CommandId> {
        feedback
            .user_actions
            .iter()
            .filter_map(|action| action.command_id().cloned())
            .collect()
    }

    /// Track exploration/speed achievement signals for a scenario completion:
    /// records the completed difficulty tier and increments the speed/flash
    /// run counters when the completion fell under the respective time budget.
    ///
    /// Shared by Training and Arcade so speed/exploration achievements
    /// (`SpeedDemon`, Speedrunner, Flash, Polyglot) are reachable identically
    /// from either mode - see [`speed_time_ratio`]'s doc comment for why both
    /// modes derive "speed" from the same base per-difficulty budget.
    ///
    /// # Examples
    ///
    /// ```
    /// use helix_trainer::config::Difficulty;
    /// use helix_trainer::game::services::ScenarioCompletionService;
    /// use helix_trainer::gamification::UserProfile;
    /// use std::time::Duration;
    ///
    /// let mut profile = UserProfile::new();
    /// // Beginner budget is 10s; 1s elapsed is under both the 50% speed-run and
    /// // 25% flash-run thresholds.
    /// ScenarioCompletionService::track_speed_and_difficulty(
    ///     &mut profile,
    ///     Duration::from_secs(1),
    ///     Some(Difficulty::Beginner),
    /// );
    ///
    /// assert!(profile.difficulties_completed.contains(&Difficulty::Beginner));
    /// assert_eq!(profile.speed_run_count, 1);
    /// assert_eq!(profile.flash_run_count, 1);
    /// ```
    pub fn track_speed_and_difficulty(
        profile: &mut UserProfile,
        duration: Duration,
        difficulty: Option<Difficulty>,
    ) {
        if let Some(difficulty) = difficulty {
            profile.difficulties_completed.insert(difficulty);
        }
        let time_ratio = speed_time_ratio(duration, difficulty);
        if time_ratio < SPEED_DEMON_TIME_RATIO {
            profile.speed_run_count = profile.speed_run_count.saturating_add(1);
        }
        if time_ratio < FLASH_TIME_RATIO {
            profile.flash_run_count = profile.flash_run_count.saturating_add(1);
        }
    }

    /// Check for achievements newly unlocked by this completion and build
    /// their notifications, without pushing them anywhere.
    ///
    /// Training and Arcade hold different context types (`HandlerContext` vs
    /// `AppState`), so the resulting notifications are returned for the
    /// caller to push into its own notification queue.
    ///
    /// # Examples
    ///
    /// ```
    /// use helix_trainer::game::services::ScenarioCompletionService;
    /// use helix_trainer::gamification::UserProfile;
    /// use helix_trainer::learning::PerformanceTracker;
    /// use helix_trainer::game::services::CompletionEvent;
    /// use std::assert_matches;
    ///
    /// let mut profile = UserProfile::new();
    /// profile.perfect_scenarios = 1;
    ///
    /// let tracker = PerformanceTracker::new();
    /// let events =
    ///     ScenarioCompletionService::check_and_notify_achievements(&mut profile, &tracker);
    ///
    /// // Should unlock FirstPerfect
    /// assert_eq!(events.len(), 1);
    /// assert_matches!(events[0], CompletionEvent::AchievementUnlocked { .. });
    /// ```
    #[must_use]
    pub fn check_and_notify_achievements(
        profile: &mut UserProfile,
        tracker: &PerformanceTracker,
    ) -> Vec<CompletionEvent> {
        AchievementEngine::check_and_unlock(profile, tracker)
            .into_iter()
            .map(|id| CompletionEvent::AchievementUnlocked { id })
            .collect()
    }

    /// Build a `LevelUp` event if this completion crossed an account-level
    /// threshold, for the caller to show in its own notification queue.
    ///
    /// # Examples
    ///
    /// ```
    /// use helix_trainer::game::services::ScenarioCompletionService;
    /// use helix_trainer::game::services::CompletionEvent;
    /// use std::assert_matches;
    ///
    /// let event = ScenarioCompletionService::level_up_notification(true, 5);
    /// assert_matches!(event, Some(CompletionEvent::LevelUp { new_level: 5 }));
    ///
    /// assert!(ScenarioCompletionService::level_up_notification(false, 5).is_none());
    /// ```
    #[must_use]
    pub fn level_up_notification(leveled_up: bool, new_level: u32) -> Option<CompletionEvent> {
        leveled_up.then_some(CompletionEvent::LevelUp { new_level })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::PerformanceRating;
    use crate::testing::{cid, sid};

    fn create_test_feedback(score: u32, max_points: u32) -> Feedback {
        Feedback {
            scenario_id: sid("test"),
            outcome: crate::game::FeedbackOutcome::Optimal,
            score,
            max_points,
            rating: PerformanceRating::Perfect,
            actions_taken: 1,
            optimal_actions: 1,
            duration: Duration::from_secs(5),
            hint: None,
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
    fn test_update_profile_counters_saturates() {
        let mut profile = UserProfile::new();
        profile.scenarios_completed = u32::MAX;
        profile.perfect_scenarios = u32::MAX;

        ScenarioCompletionService::update_profile_counters(&mut profile, true);

        assert_eq!(profile.scenarios_completed, u32::MAX);
        assert_eq!(profile.perfect_scenarios, u32::MAX);
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
    #[expect(
        clippy::assert_is_empty,
        reason = "assert!(x.is_empty()) reads better than the machine fix"
    )]
    fn test_extract_commands_empty() {
        let feedback = create_test_feedback(100, 100);
        let commands = ScenarioCompletionService::extract_commands(&feedback);
        assert!(commands.is_empty());
    }

    #[test]
    fn test_extract_commands_single() {
        let mut feedback = create_test_feedback(100, 100);
        feedback.user_actions = crate::testing::user_actions(&["d"]);

        let commands = ScenarioCompletionService::extract_commands(&feedback);
        assert_eq!(commands, vec![cid("d")]);
    }

    #[test]
    fn test_extract_commands_multiple() {
        let mut feedback = create_test_feedback(100, 100);
        feedback.user_actions = crate::testing::user_actions(&["d", "w", "j"]);

        let commands = ScenarioCompletionService::extract_commands(&feedback);
        assert_eq!(commands, vec![cid("d"), cid("w"), cid("j")]);
    }

    #[test]
    fn test_extract_commands_skips_typed_text_and_arguments() {
        let mut feedback = create_test_feedback(100, 100);
        feedback.user_actions = crate::testing::user_actions(&[
            "!", "Escape", " ", "3h", "f=", "T,", "r_", "/ sum", "ma(",
        ]);

        let commands = ScenarioCompletionService::extract_commands(&feedback);
        assert_eq!(commands, ["h", "f", "T", "r", "/", "ma("].map(cid).to_vec());
    }

    #[test]
    fn test_record_and_scale_xp_first_completion() {
        let mut profile = UserProfile::new();
        let result = ScenarioCompletionService::record_and_scale_xp(
            &mut profile,
            &sid("test_scenario"),
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
                &sid("test_scenario"),
                100,
                50,
                Utc::now(),
            );
        }

        let completion = profile.scenario_history.get(&sid("test_scenario")).unwrap();
        assert_eq!(completion.attempts, 5);
    }
}
