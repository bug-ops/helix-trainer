//! FSRS-based scenario prioritization for arcade mode
//!
//! This module provides scenario prioritization based on FSRS learning data.
//! Scenarios containing commands that are overdue, weak, or never practiced
//! are scored higher than scenarios with mastered commands.
//!
//! # Scoring Formula
//!
//! For each command in a scenario:
//!
//! ```text
//! score = overdue_weight * overdue_factor + weakness_weight * weakness_factor
//! ```
//!
//! Where:
//! - `overdue_factor` = `min(overdue_ratio, 1.0)`, the days past due relative to the
//!   scheduled interval (from [`command_urgency`])
//! - `weakness_factor` = 1.0 - `success_rate`
//!
//! Novel commands (no tracker entry) receive a fixed novelty weight.
//!
//! The scenario score is the **maximum** score among all its commands,
//! since a scenario is as urgent as its most urgent command.
//!
//! # Examples
//!
//! ```
//! use helix_trainer::config::ScenarioLoader;
//! use helix_trainer::learning::PerformanceTracker;
//! use helix_trainer::minigame::ScenarioPrioritizer;
//!
//! let scenarios = ScenarioLoader::new().load_from_embedded("en")?;
//! let tracker = PerformanceTracker::new();
//! let scorer = ScenarioPrioritizer::new(&tracker);
//!
//! let score = scorer.score(&scenarios[0]);
//! assert!((0.0..=1.0).contains(&score));
//! # Ok::<(), helix_trainer::security::UserError>(())
//! ```

use chrono::{DateTime, Utc};

use crate::config::Scenario;
use crate::constants::{FSRS_NOVELTY_WEIGHT, FSRS_OVERDUE_WEIGHT, FSRS_WEAKNESS_WEIGHT};
use crate::ids::CommandId;
use crate::learning::{PerformanceTracker, Urgency, command_urgency};

/// Scores scenarios based on FSRS learning priority.
///
/// Higher scores indicate scenarios that should be practiced sooner:
/// - 0.0 = low priority (already mastered)
/// - 1.0 = high priority (needs practice)
///
/// The scorer uses FSRS data from the `PerformanceTracker` to determine
/// which commands need review based on their overdue status and success rate.
pub struct ScenarioPrioritizer<'a> {
    tracker: &'a PerformanceTracker,
    now: DateTime<Utc>,
}

impl std::fmt::Debug for ScenarioPrioritizer<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ScenarioPrioritizer")
            .field("now", &self.now)
            .finish_non_exhaustive()
    }
}

impl<'a> ScenarioPrioritizer<'a> {
    /// Creates a new scorer reading the time from the tracker's clock.
    ///
    /// Using the tracker's clock (not the system clock) keeps arcade selection
    /// deterministic under an injected `FakeClock`.
    ///
    /// # Arguments
    ///
    /// * `tracker` - Reference to the performance tracker containing FSRS data
    ///
    /// # Examples
    ///
    /// ```
    /// use helix_trainer::ids::CommandId;
    /// use helix_trainer::learning::PerformanceTracker;
    /// use helix_trainer::minigame::ScenarioPrioritizer;
    ///
    /// let tracker = PerformanceTracker::new();
    /// let prioritizer = ScenarioPrioritizer::new(&tracker);
    /// let dd = CommandId::from_card_id("dd").unwrap();
    /// assert_eq!(prioritizer.score_command(&dd), helix_trainer::constants::FSRS_NOVELTY_WEIGHT);
    /// ```
    #[must_use]
    pub fn new(tracker: &'a PerformanceTracker) -> Self {
        Self {
            tracker,
            now: tracker.clock().now(),
        }
    }

    /// Creates a scorer with a specific timestamp.
    ///
    /// This is primarily useful for testing to ensure deterministic behavior.
    ///
    /// # Arguments
    ///
    /// * `tracker` - Reference to the performance tracker
    /// * `now` - The timestamp to use for overdue calculations
    #[must_use]
    pub fn with_time(tracker: &'a PerformanceTracker, now: DateTime<Utc>) -> Self {
        Self { tracker, now }
    }

    /// Scores a scenario based on FSRS priority of its commands.
    ///
    /// Returns a score from 0.0 (low priority - already mastered)
    /// to approximately 1.0 (high priority - needs practice).
    ///
    /// The score is calculated as the maximum score among all commands
    /// in the scenario's solution, since a scenario is as urgent as
    /// its most urgent command.
    ///
    /// # Arguments
    ///
    /// * `scenario` - The scenario to score
    ///
    /// # Returns
    ///
    /// A score between 0.0 and ~0.8 (overdue + weakness) or 0.2 (novelty).
    /// Returns 0.5 for scenarios without clear commands.
    ///
    /// # Examples
    ///
    /// ```
    /// use helix_trainer::config::ScenarioLoader;
    /// use helix_trainer::learning::PerformanceTracker;
    /// use helix_trainer::minigame::ScenarioPrioritizer;
    ///
    /// let scenarios = ScenarioLoader::new().load_from_embedded("en")?;
    /// let tracker = PerformanceTracker::new();
    /// let scorer = ScenarioPrioritizer::new(&tracker);
    ///
    /// // An untracked scenario consists of novel commands and gets the novelty weight.
    /// assert!(scorer.score(&scenarios[0]) > 0.0);
    /// # Ok::<(), helix_trainer::security::UserError>(())
    /// ```
    #[must_use]
    pub fn score(&self, scenario: &Scenario) -> f64 {
        let commands = Self::extract_commands(scenario);

        if commands.is_empty() {
            // Neutral score for scenarios without clear commands
            return 0.5;
        }

        // Calculate score for each command and take the maximum
        commands
            .iter()
            .map(|cmd| self.score_command(cmd))
            .fold(0.0_f64, f64::max)
    }

    /// Extracts the review-card ids of a scenario's solution.
    ///
    /// # Arguments
    ///
    /// * `scenario` - The scenario to extract commands from
    ///
    /// # Returns
    ///
    /// Card ids minted the same way as when a completed scenario is recorded.
    fn extract_commands(scenario: &Scenario) -> Vec<CommandId> {
        CommandId::from_actions(scenario.solution.commands.iter().map(String::as_str))
    }

    /// Scores an individual command based on its FSRS state.
    ///
    /// # Scoring Formula
    ///
    /// For practiced commands:
    /// ```text
    /// score = OVERDUE_WEIGHT * overdue_factor + WEAKNESS_WEIGHT * weakness_factor
    /// ```
    ///
    /// Where (both from [`command_urgency`]):
    /// - `overdue_factor` = `min(overdue_ratio, 1.0)`, the days past due relative to the
    ///   scheduled interval
    /// - `weakness_factor` = 1.0 - `success_rate`
    ///
    /// For novel commands (no tracker entry):
    /// ```text
    /// score = NOVELTY_WEIGHT
    /// ```
    ///
    /// # Arguments
    ///
    /// * `command` - The command string to score
    ///
    /// # Returns
    ///
    /// A score indicating the command's priority for practice.
    #[must_use]
    pub fn score_command(&self, command: &CommandId) -> f64 {
        match command_urgency(self.tracker.performance(command), self.now) {
            Urgency::Practiced {
                overdue_ratio,
                weakness,
                ..
            } => FSRS_OVERDUE_WEIGHT * overdue_ratio.min(1.0) + FSRS_WEAKNESS_WEIGHT * weakness,
            Urgency::Novel => FSRS_NOVELTY_WEIGHT,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{ScenarioMetadata, ScoringConfig, Setup, Solution, TargetState};
    use crate::testing::{cid, sid};
    use crate::time::{Clock, FakeClock};
    use chrono::Duration;
    use std::sync::Arc;
    use std::time::Duration as StdDuration;

    /// Creates a test scenario with the given commands.
    fn make_scenario(commands: Vec<&str>) -> Scenario {
        Scenario {
            id: sid("test_scenario"),
            name: "Test Scenario".to_string(),
            description: "A test scenario".to_string(),
            setup: Setup {
                file_content: "test content".to_string(),
                language: None,
                cursor: crate::testing::point(0, 0),
            },
            target: TargetState {
                file_content: "target content".to_string(),
                cursor: crate::testing::point(0, 0),
            },
            solution: Solution {
                commands: commands.into_iter().map(String::from).collect(),
                description: "Test solution".to_string(),
            },
            hints: vec![],
            scoring: ScoringConfig {
                optimal_count: std::num::NonZeroUsize::new(1).unwrap(),
                max_points: 100,
                tolerance: 0,
            },
            metadata: ScenarioMetadata::default(),
        }
    }

    #[test]
    fn test_score_command_never_practiced() {
        let tracker = PerformanceTracker::new();
        let scorer = ScenarioPrioritizer::new(&tracker);

        let score = scorer.score_command(&cid("dd"));

        // Never practiced command should get novelty weight
        assert!(
            (score - FSRS_NOVELTY_WEIGHT).abs() < 0.001,
            "Expected novelty weight {FSRS_NOVELTY_WEIGHT}, got {score}"
        );
    }

    #[test]
    fn test_score_command_mastered() {
        let mut tracker = PerformanceTracker::new();

        // Record many successful attempts to build up mastery
        for _ in 0..10 {
            tracker.record_attempt(
                &cid("dd"),
                StdDuration::from_secs(1),
                true,
                StdDuration::from_secs(1),
            );
        }

        // Score at the due time (not overdue)
        let perf = tracker.performance(&cid("dd")).unwrap();
        let scorer = ScenarioPrioritizer::with_time(&tracker, perf.due());

        let score = scorer.score_command(&cid("dd"));

        // Not overdue (overdue_factor = 0) and high success rate (weakness_factor ~ 0)
        // Score should be very low
        assert!(
            score < 0.1,
            "Expected low score for mastered command, got {score}"
        );
    }

    #[test]
    fn test_score_command_weak() {
        let mut tracker = PerformanceTracker::new();

        // Record mostly failures to create a weak command
        for _ in 0..8 {
            tracker.record_attempt(
                &cid("dd"),
                StdDuration::from_secs(10),
                false,
                StdDuration::from_secs(1),
            );
        }
        for _ in 0..2 {
            tracker.record_attempt(
                &cid("dd"),
                StdDuration::from_secs(1),
                true,
                StdDuration::from_secs(1),
            );
        }

        // Score at due time
        let perf = tracker.performance(&cid("dd")).unwrap();
        let scorer = ScenarioPrioritizer::with_time(&tracker, perf.due());

        let score = scorer.score_command(&cid("dd"));

        // 20% success rate -> weakness_factor = 0.8
        // Expected: 0.4 * 0.8 = 0.32
        assert!(
            score >= 0.2,
            "Expected higher score for weak command, got {score}"
        );
    }

    #[test]
    fn test_score_scenario_uses_max() {
        let mut tracker = PerformanceTracker::new();

        // Create one mastered command
        for _ in 0..5 {
            tracker.record_attempt(
                &cid("h"),
                StdDuration::from_secs(1),
                true,
                StdDuration::from_secs(1),
            );
        }

        // "dd" is never practiced (will get novelty score)

        // Create scenario with both commands
        let scenario = make_scenario(vec!["h", "dd"]);

        let perf = tracker.performance(&cid("h")).unwrap();
        let scorer = ScenarioPrioritizer::with_time(&tracker, perf.due());

        let score = scorer.score(&scenario);
        let h_score = scorer.score_command(&cid("h"));
        let dd_score = scorer.score_command(&cid("dd"));

        // Should use maximum (dd's novelty score > h's mastered score)
        assert!(
            (score - dd_score.max(h_score)).abs() < 0.001,
            "Expected max score {}, got {}",
            dd_score.max(h_score),
            score
        );

        // The novelty score should be higher than the mastered score
        assert!(
            dd_score > h_score,
            "Novel command should have higher score than mastered"
        );
    }

    #[test]
    fn test_score_scenario_empty_commands() {
        let tracker = PerformanceTracker::new();
        let scorer = ScenarioPrioritizer::new(&tracker);

        let scenario = make_scenario(vec![]);
        let score = scorer.score(&scenario);

        // Empty commands should return neutral score
        assert!(
            (score - 0.5).abs() < 0.001,
            "Expected neutral score 0.5 for empty commands, got {score}"
        );
    }

    #[test]
    fn test_empty_tracker_gives_novelty_scores() {
        let tracker = PerformanceTracker::new();
        let scorer = ScenarioPrioritizer::new(&tracker);

        let scenario1 = make_scenario(vec!["dd"]);
        let scenario2 = make_scenario(vec!["x", "p"]);
        let scenario3 = make_scenario(vec!["w", "d", "w"]);

        let score1 = scorer.score(&scenario1);
        let score2 = scorer.score(&scenario2);
        let score3 = scorer.score(&scenario3);

        // All should get novelty weight since tracker is empty
        assert!(
            (score1 - FSRS_NOVELTY_WEIGHT).abs() < 0.001,
            "Expected novelty weight for scenario1, got {score1}"
        );
        assert!(
            (score2 - FSRS_NOVELTY_WEIGHT).abs() < 0.001,
            "Expected novelty weight for scenario2, got {score2}"
        );
        assert!(
            (score3 - FSRS_NOVELTY_WEIGHT).abs() < 0.001,
            "Expected novelty weight for scenario3, got {score3}"
        );
    }

    #[test]
    fn test_extract_commands() {
        let scenario = make_scenario(vec!["d", "w", "i"]);
        let commands = ScenarioPrioritizer::extract_commands(&scenario);

        assert_eq!(commands.len(), 3);
        assert_eq!(commands[0].as_str(), "d");
        assert_eq!(commands[1].as_str(), "w");
        assert_eq!(commands[2].as_str(), "i");
    }

    #[test]
    fn test_score_bounds() {
        let tracker = PerformanceTracker::new();
        let scorer = ScenarioPrioritizer::new(&tracker);

        // Test various commands - all scores should be reasonable
        for cmd in ["dd", "x", "yy", "p", "w", "b", "h", "j", "k", "l"] {
            let score = scorer.score_command(&cid(cmd));
            assert!(
                score >= 0.0,
                "Score for {cmd} should be >= 0.0, got {score}"
            );
            assert!(
                score <= 1.0,
                "Score for {cmd} should be <= 1.0, got {score}"
            );
        }
    }

    #[test]
    fn test_score_mixed_scenario() {
        let mut tracker = PerformanceTracker::new();

        // Create different states for different commands:
        // - "h": mastered (many successes, not overdue)
        // - "dd": weak (mostly failures)
        // - "p": never practiced

        // Master "h"
        for _ in 0..10 {
            tracker.record_attempt(
                &cid("h"),
                StdDuration::from_secs(1),
                true,
                StdDuration::from_secs(1),
            );
        }

        // Make "dd" weak
        for _ in 0..8 {
            tracker.record_attempt(
                &cid("dd"),
                StdDuration::from_secs(10),
                false,
                StdDuration::from_secs(1),
            );
        }

        // Score at the later due date to avoid overdue effects
        let h_perf = tracker.performance(&cid("h")).unwrap();
        let dd_perf = tracker.performance(&cid("dd")).unwrap();
        let now = h_perf.due().max(dd_perf.due());

        let scorer = ScenarioPrioritizer::with_time(&tracker, now);

        let h_score = scorer.score_command(&cid("h"));
        let dd_score = scorer.score_command(&cid("dd"));
        let p_score = scorer.score_command(&cid("p"));

        // Verify relative ordering: weak > novelty > mastered
        assert!(
            dd_score > p_score || (dd_score - p_score).abs() < 0.1,
            "Weak command should score >= novelty: dd={dd_score}, p={p_score}"
        );
        assert!(
            p_score > h_score,
            "Novelty should score higher than mastered: p={p_score}, h={h_score}"
        );

        // Scenario with all three should use max
        let scenario = make_scenario(vec!["h", "dd", "p"]);
        let scenario_score = scorer.score(&scenario);
        let expected_max = h_score.max(dd_score).max(p_score);

        assert!(
            (scenario_score - expected_max).abs() < 0.001,
            "Scenario should use max score: expected {expected_max}, got {scenario_score}"
        );
    }

    /// Tracker with one perfect attempt on `cmd` and the interval FSRS scheduled for it.
    fn tracker_with_one_attempt(cmd: &str) -> (PerformanceTracker, i64) {
        let mut tracker = PerformanceTracker::new();
        tracker.record_attempt(
            &cid(cmd),
            StdDuration::from_secs(1),
            true,
            StdDuration::from_secs(1),
        );
        let interval = i64::from(tracker.performance(&cid(cmd)).unwrap().scheduled_days());
        (tracker, interval)
    }

    #[test]
    fn test_score_command_overdue_by_one_interval_gets_full_overdue_weight() {
        let (tracker, interval) = tracker_with_one_attempt("dd");
        let due = tracker.performance(&cid("dd")).unwrap().due();

        let scorer = ScenarioPrioritizer::with_time(&tracker, due + Duration::days(interval));

        // success rate 1.0, so only the overdue term remains
        assert!((scorer.score_command(&cid("dd")) - FSRS_OVERDUE_WEIGHT).abs() < 1e-9);
    }

    #[test]
    #[expect(
        clippy::cast_precision_loss,
        reason = "integer counters and scores stay far below 2^52, so the f64 value is exact"
    )]
    fn test_score_command_overdue_is_relative_to_the_interval() {
        let (tracker, interval) = tracker_with_one_attempt("x");
        let due = tracker.performance(&cid("x")).unwrap().due();

        let scorer = ScenarioPrioritizer::with_time(&tracker, due + Duration::days(1));

        let expected = FSRS_OVERDUE_WEIGHT * (1.0 / interval as f64).min(1.0);
        assert!((scorer.score_command(&cid("x")) - expected).abs() < 1e-9);
    }

    #[test]
    fn test_overdue_term_is_capped_at_one_interval() {
        let (tracker, interval) = tracker_with_one_attempt("dd");
        let due = tracker.performance(&cid("dd")).unwrap().due();

        let at_cap = ScenarioPrioritizer::with_time(&tracker, due + Duration::days(interval))
            .score_command(&cid("dd"));
        let far_beyond =
            ScenarioPrioritizer::with_time(&tracker, due + Duration::days(interval * 50))
                .score_command(&cid("dd"));

        assert!((at_cap - far_beyond).abs() < 1e-9);
    }

    #[test]
    fn test_new_reads_the_trackers_injected_clock() {
        let clock = Arc::new(FakeClock::at("2030-01-01T00:00:00Z"));
        let mut tracker = PerformanceTracker::with_clock(clock.clone());
        tracker.record_attempt(
            &cid("dd"),
            StdDuration::from_secs(1),
            true,
            StdDuration::from_secs(1),
        );
        let interval = i64::from(tracker.performance(&cid("dd")).unwrap().scheduled_days());

        let before = ScenarioPrioritizer::new(&tracker).score_command(&cid("dd"));
        clock.advance_days(interval);
        let after = ScenarioPrioritizer::new(&tracker).score_command(&cid("dd"));

        assert!(after > before, "overdue score must follow the fake clock");
        assert_eq!(
            after,
            ScenarioPrioritizer::with_time(&tracker, clock.now()).score_command(&cid("dd"))
        );
    }

    #[test]
    fn test_novelty_requires_no_tracker_entry() {
        let (tracker, _) = tracker_with_one_attempt("dd");
        let scorer = ScenarioPrioritizer::with_time(&tracker, tracker.clock().now());

        assert_ne!(scorer.score_command(&cid("dd")), FSRS_NOVELTY_WEIGHT);
        assert_eq!(scorer.score_command(&cid("zz")), FSRS_NOVELTY_WEIGHT);
    }
}
