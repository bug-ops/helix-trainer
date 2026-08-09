//! FSRS-based scenario scoring for arcade mode
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
//! - `overdue_factor` = min(days_overdue / max_overdue_days, 1.0)
//! - `weakness_factor` = 1.0 - success_rate
//!
//! Novel (never practiced) commands receive a fixed novelty weight.
//!
//! The scenario score is the **maximum** score among all its commands,
//! since a scenario is as urgent as its most urgent command.
//!
//! # Examples
//!
//! ```ignore
//! use helix_trainer::learning::PerformanceTracker;
//! use helix_trainer::minigame::ScenarioScorer;
//!
//! let tracker = PerformanceTracker::new();
//! let scorer = ScenarioScorer::new(&tracker);
//!
//! let score = scorer.score(&scenario);
//! assert!(score >= 0.0 && score <= 1.0);
//! ```

use chrono::{DateTime, Utc};

use crate::config::Scenario;
use crate::constants::{
    FSRS_MAX_OVERDUE_DAYS, FSRS_NOVELTY_WEIGHT, FSRS_OVERDUE_WEIGHT, FSRS_WEAKNESS_WEIGHT,
};
use crate::learning::PerformanceTracker;

/// Scores scenarios based on FSRS learning priority.
///
/// Higher scores indicate scenarios that should be practiced sooner:
/// - 0.0 = low priority (already mastered)
/// - 1.0 = high priority (needs practice)
///
/// The scorer uses FSRS data from the `PerformanceTracker` to determine
/// which commands need review based on their overdue status and success rate.
pub struct ScenarioScorer<'a> {
    tracker: &'a PerformanceTracker,
    now: DateTime<Utc>,
}

impl std::fmt::Debug for ScenarioScorer<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ScenarioScorer")
            .field("now", &self.now)
            .finish_non_exhaustive()
    }
}

impl<'a> ScenarioScorer<'a> {
    /// Creates a new scorer using the current time.
    ///
    /// # Arguments
    ///
    /// * `tracker` - Reference to the performance tracker containing FSRS data
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use helix_trainer::learning::PerformanceTracker;
    /// use helix_trainer::minigame::ScenarioScorer;
    ///
    /// let tracker = PerformanceTracker::new();
    /// let scorer = ScenarioScorer::new(&tracker);
    /// ```
    #[must_use]
    pub fn new(tracker: &'a PerformanceTracker) -> Self {
        Self {
            tracker,
            now: Utc::now(),
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
    /// ```ignore
    /// let score = scorer.score(&scenario);
    /// if score > 0.5 {
    ///     println!("This scenario needs practice!");
    /// }
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

    /// Extracts command strings from a scenario's solution.
    ///
    /// # Arguments
    ///
    /// * `scenario` - The scenario to extract commands from
    ///
    /// # Returns
    ///
    /// A vector of command strings from the scenario's solution.
    fn extract_commands(scenario: &Scenario) -> Vec<String> {
        scenario.solution.commands.to_vec()
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
    /// Where:
    /// - `overdue_factor` = min(days_overdue / MAX_OVERDUE_DAYS, 1.0)
    /// - `weakness_factor` = 1.0 - success_rate
    ///
    /// For novel (never practiced) commands:
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
    pub fn score_command(&self, command: &str) -> f64 {
        match self.tracker.get_performance(command) {
            Some(perf) => {
                // Command has been practiced before
                let days_overdue = (self.now - perf.due).num_days().max(0) as f64;
                let overdue_factor = (days_overdue / FSRS_MAX_OVERDUE_DAYS).min(1.0);
                let weakness_factor = 1.0 - perf.success_rate();

                FSRS_OVERDUE_WEIGHT * overdue_factor + FSRS_WEAKNESS_WEIGHT * weakness_factor
            }
            None => {
                // Command never practiced - novelty bonus
                FSRS_NOVELTY_WEIGHT
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{CursorSpec, ScoringConfig, Setup, Solution, TargetState};
    use chrono::Duration;
    use std::time::Duration as StdDuration;

    /// Creates a test scenario with the given commands.
    fn make_scenario(commands: Vec<&str>) -> Scenario {
        Scenario {
            id: "test_scenario".to_string(),
            name: "Test Scenario".to_string(),
            description: "A test scenario".to_string(),
            setup: Setup {
                file_content: "test content".to_string(),
                cursor: CursorSpec {
                    cursor_position: Some((0, 0)),
                    selection: None,
                    cursors: None,
                    selections: None,
                },
            },
            target: TargetState {
                file_content: "target content".to_string(),
                cursor: CursorSpec {
                    cursor_position: Some((0, 0)),
                    selection: None,
                    cursors: None,
                    selections: None,
                },
            },
            solution: Solution {
                commands: commands.into_iter().map(String::from).collect(),
                description: "Test solution".to_string(),
            },
            alternatives: vec![],
            hints: vec![],
            scoring: ScoringConfig {
                optimal_count: std::num::NonZeroUsize::new(1).unwrap(),
                max_points: 100,
                tolerance: 0,
            },
            metadata: None,
        }
    }

    #[test]
    fn test_score_command_never_practiced() {
        let tracker = PerformanceTracker::new();
        let scorer = ScenarioScorer::new(&tracker);

        let score = scorer.score_command("dd");

        // Never practiced command should get novelty weight
        assert!(
            (score - FSRS_NOVELTY_WEIGHT).abs() < 0.001,
            "Expected novelty weight {}, got {}",
            FSRS_NOVELTY_WEIGHT,
            score
        );
    }

    #[test]
    fn test_score_command_overdue() {
        let mut tracker = PerformanceTracker::new();

        // Record an attempt to create initial FSRS state
        tracker.record_attempt(
            "dd",
            StdDuration::from_secs(1),
            true,
            StdDuration::from_secs(1),
        );

        // Get the due date and create a scorer 7 days after
        let perf = tracker.get_performance("dd").unwrap();
        let seven_days_later = perf.due + Duration::days(7);

        let scorer = ScenarioScorer::with_time(&tracker, seven_days_later);
        let score = scorer.score_command("dd");

        // 7 days overdue should give max overdue factor (1.0)
        // With high success rate (1.0), weakness_factor = 0
        // Score = 0.4 * 1.0 + 0.4 * 0.0 = 0.4
        assert!(
            score >= FSRS_OVERDUE_WEIGHT * 0.9,
            "Expected overdue score >= {}, got {}",
            FSRS_OVERDUE_WEIGHT * 0.9,
            score
        );
    }

    #[test]
    fn test_score_command_partially_overdue() {
        let mut tracker = PerformanceTracker::new();

        // Record an attempt
        tracker.record_attempt(
            "x",
            StdDuration::from_secs(1),
            true,
            StdDuration::from_secs(1),
        );

        // Get the due date and create a scorer 3.5 days after (half of max overdue)
        let perf = tracker.get_performance("x").unwrap();
        let half_overdue = perf.due + Duration::days(3) + Duration::hours(12);

        let scorer = ScenarioScorer::with_time(&tracker, half_overdue);
        let score = scorer.score_command("x");

        // 3.5 days overdue should give ~0.5 overdue factor
        // Expected: 0.4 * 0.5 = 0.2 (approximately)
        assert!(
            (0.15..=0.25).contains(&score),
            "Expected score around 0.2 for half-overdue, got {}",
            score
        );
    }

    #[test]
    fn test_score_command_mastered() {
        let mut tracker = PerformanceTracker::new();

        // Record many successful attempts to build up mastery
        for _ in 0..10 {
            tracker.record_attempt(
                "dd",
                StdDuration::from_secs(1),
                true,
                StdDuration::from_secs(1),
            );
        }

        // Score at the due time (not overdue)
        let perf = tracker.get_performance("dd").unwrap();
        let scorer = ScenarioScorer::with_time(&tracker, perf.due);

        let score = scorer.score_command("dd");

        // Not overdue (overdue_factor = 0) and high success rate (weakness_factor ~ 0)
        // Score should be very low
        assert!(
            score < 0.1,
            "Expected low score for mastered command, got {}",
            score
        );
    }

    #[test]
    fn test_score_command_weak() {
        let mut tracker = PerformanceTracker::new();

        // Record mostly failures to create a weak command
        for _ in 0..8 {
            tracker.record_attempt(
                "dd",
                StdDuration::from_secs(10),
                false,
                StdDuration::from_secs(1),
            );
        }
        for _ in 0..2 {
            tracker.record_attempt(
                "dd",
                StdDuration::from_secs(1),
                true,
                StdDuration::from_secs(1),
            );
        }

        // Score at due time
        let perf = tracker.get_performance("dd").unwrap();
        let scorer = ScenarioScorer::with_time(&tracker, perf.due);

        let score = scorer.score_command("dd");

        // 20% success rate -> weakness_factor = 0.8
        // Expected: 0.4 * 0.8 = 0.32
        assert!(
            score >= 0.2,
            "Expected higher score for weak command, got {}",
            score
        );
    }

    #[test]
    fn test_score_scenario_uses_max() {
        let mut tracker = PerformanceTracker::new();

        // Create one mastered command
        for _ in 0..5 {
            tracker.record_attempt(
                "h",
                StdDuration::from_secs(1),
                true,
                StdDuration::from_secs(1),
            );
        }

        // "dd" is never practiced (will get novelty score)

        // Create scenario with both commands
        let scenario = make_scenario(vec!["h", "dd"]);

        let perf = tracker.get_performance("h").unwrap();
        let scorer = ScenarioScorer::with_time(&tracker, perf.due);

        let score = scorer.score(&scenario);
        let h_score = scorer.score_command("h");
        let dd_score = scorer.score_command("dd");

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
        let scorer = ScenarioScorer::new(&tracker);

        let scenario = make_scenario(vec![]);
        let score = scorer.score(&scenario);

        // Empty commands should return neutral score
        assert!(
            (score - 0.5).abs() < 0.001,
            "Expected neutral score 0.5 for empty commands, got {}",
            score
        );
    }

    #[test]
    fn test_empty_tracker_gives_novelty_scores() {
        let tracker = PerformanceTracker::new();
        let scorer = ScenarioScorer::new(&tracker);

        let scenario1 = make_scenario(vec!["dd"]);
        let scenario2 = make_scenario(vec!["x", "p"]);
        let scenario3 = make_scenario(vec!["w", "d", "w"]);

        let score1 = scorer.score(&scenario1);
        let score2 = scorer.score(&scenario2);
        let score3 = scorer.score(&scenario3);

        // All should get novelty weight since tracker is empty
        assert!(
            (score1 - FSRS_NOVELTY_WEIGHT).abs() < 0.001,
            "Expected novelty weight for scenario1, got {}",
            score1
        );
        assert!(
            (score2 - FSRS_NOVELTY_WEIGHT).abs() < 0.001,
            "Expected novelty weight for scenario2, got {}",
            score2
        );
        assert!(
            (score3 - FSRS_NOVELTY_WEIGHT).abs() < 0.001,
            "Expected novelty weight for scenario3, got {}",
            score3
        );
    }

    #[test]
    fn test_extract_commands() {
        let scenario = make_scenario(vec!["d", "w", "i"]);
        let commands = ScenarioScorer::extract_commands(&scenario);

        assert_eq!(commands.len(), 3);
        assert_eq!(commands[0], "d");
        assert_eq!(commands[1], "w");
        assert_eq!(commands[2], "i");
    }

    #[test]
    fn test_score_bounds() {
        let tracker = PerformanceTracker::new();
        let scorer = ScenarioScorer::new(&tracker);

        // Test various commands - all scores should be reasonable
        for cmd in ["dd", "x", "yy", "p", "w", "b", "h", "j", "k", "l"] {
            let score = scorer.score_command(cmd);
            assert!(
                score >= 0.0,
                "Score for {} should be >= 0.0, got {}",
                cmd,
                score
            );
            assert!(
                score <= 1.0,
                "Score for {} should be <= 1.0, got {}",
                cmd,
                score
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
                "h",
                StdDuration::from_secs(1),
                true,
                StdDuration::from_secs(1),
            );
        }

        // Make "dd" weak
        for _ in 0..8 {
            tracker.record_attempt(
                "dd",
                StdDuration::from_secs(10),
                false,
                StdDuration::from_secs(1),
            );
        }

        // Score at the later due date to avoid overdue effects
        let h_perf = tracker.get_performance("h").unwrap();
        let dd_perf = tracker.get_performance("dd").unwrap();
        let now = h_perf.due.max(dd_perf.due);

        let scorer = ScenarioScorer::with_time(&tracker, now);

        let h_score = scorer.score_command("h");
        let dd_score = scorer.score_command("dd");
        let p_score = scorer.score_command("p");

        // Verify relative ordering: weak > novelty > mastered
        assert!(
            dd_score > p_score || (dd_score - p_score).abs() < 0.1,
            "Weak command should score >= novelty: dd={}, p={}",
            dd_score,
            p_score
        );
        assert!(
            p_score > h_score,
            "Novelty should score higher than mastered: p={}, h={}",
            p_score,
            h_score
        );

        // Scenario with all three should use max
        let scenario = make_scenario(vec!["h", "dd", "p"]);
        let scenario_score = scorer.score(&scenario);
        let expected_max = h_score.max(dd_score).max(p_score);

        assert!(
            (scenario_score - expected_max).abs() < 0.001,
            "Scenario should use max score: expected {}, got {}",
            expected_max,
            scenario_score
        );
    }

    #[test]
    fn test_overdue_capped_at_max_days() {
        let mut tracker = PerformanceTracker::new();

        // Record an attempt
        tracker.record_attempt(
            "dd",
            StdDuration::from_secs(1),
            true,
            StdDuration::from_secs(1),
        );

        let perf = tracker.get_performance("dd").unwrap();

        // Test at exactly max overdue days
        let at_max = perf.due + Duration::days(FSRS_MAX_OVERDUE_DAYS as i64);
        let scorer_max = ScenarioScorer::with_time(&tracker, at_max);
        let score_max = scorer_max.score_command("dd");

        // Test at double max overdue days - should be capped
        let beyond_max = perf.due + Duration::days((FSRS_MAX_OVERDUE_DAYS * 2.0) as i64);
        let scorer_beyond = ScenarioScorer::with_time(&tracker, beyond_max);
        let score_beyond = scorer_beyond.score_command("dd");

        // Both should give the same overdue factor (capped at 1.0)
        assert!(
            (score_max - score_beyond).abs() < 0.001,
            "Overdue factor should be capped: at_max={}, beyond={}",
            score_max,
            score_beyond
        );
    }
}
