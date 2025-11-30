//! Difficulty controller for adaptive gameplay
//!
//! Manages difficulty scaling and scenario selection for mini-games.

use crate::config::{Difficulty, Scenario};
use std::collections::VecDeque;
use std::time::Duration;

/// Maximum number of recent results to track for adaptive difficulty
const RECENT_RESULTS_WINDOW: usize = 10;

/// Difficulty controller for mini-game mode
///
/// Manages adaptive difficulty scaling based on player performance
/// and selects appropriate scenarios from the collection.
#[derive(Debug, Clone)]
pub struct DifficultyController {
    /// Current difficulty level (1-10)
    level: u8,

    /// Recent performance results for adaptive difficulty
    /// true = success, false = failure
    recent_results: VecDeque<bool>,
}

impl DifficultyController {
    /// Create a new difficulty controller starting at level 1
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use helix_trainer::minigame::DifficultyController;
    ///
    /// let controller = DifficultyController::new();
    /// assert_eq!(controller.current_level(), 1);
    /// ```
    pub fn new() -> Self {
        Self {
            level: 1,
            recent_results: VecDeque::with_capacity(RECENT_RESULTS_WINDOW),
        }
    }

    /// Get current difficulty level (1-10)
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use helix_trainer::minigame::DifficultyController;
    ///
    /// let controller = DifficultyController::new();
    /// assert_eq!(controller.current_level(), 1);
    /// ```
    pub fn current_level(&self) -> u8 {
        self.level
    }

    /// Get time limit for a scenario based on its difficulty and current level
    ///
    /// Base time limits by scenario difficulty:
    /// - Beginner: 10s
    /// - Intermediate: 8s
    /// - Advanced: 6s
    ///
    /// Time scaling by controller level:
    /// - Level 1-3: 100% of base time
    /// - Level 4-6: 90% of base time
    /// - Level 7-10: 80% of base time (minimum 50% of base)
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use helix_trainer::minigame::DifficultyController;
    /// use helix_trainer::config::{Scenario, Difficulty};
    ///
    /// let controller = DifficultyController::new();
    /// let scenario = create_beginner_scenario();
    /// let limit = controller.time_limit_for(&scenario);
    /// assert_eq!(limit.as_secs(), 10); // Beginner at level 1
    /// ```
    pub fn time_limit_for(&self, scenario: &Scenario) -> Duration {
        // Get base time from scenario metadata
        let base = if let Some(ref metadata) = scenario.metadata
            && let Some(difficulty) = metadata.difficulty
        {
            match difficulty {
                Difficulty::Beginner => Duration::from_secs(10),
                Difficulty::Intermediate => Duration::from_secs(8),
                Difficulty::Advanced => Duration::from_secs(6),
            }
        } else {
            // Fallback if no metadata: use medium time
            Duration::from_secs(8)
        };

        // Scale by current level (level 10 = 80% time, but not less than 50%)
        let scale = match self.level {
            1..=3 => 1.0,
            4..=6 => 0.9,
            7..=10 => 0.8,
            _ => 0.8, // Shouldn't happen, but safe fallback
        };

        let scaled_secs = base.as_secs_f64() * scale;
        let min_secs = base.as_secs_f64() * 0.5; // Never less than 50%

        Duration::from_secs_f64(scaled_secs.max(min_secs))
    }

    /// Select next scenario from collection
    ///
    /// Selection algorithm:
    /// 1. Filter scenarios by appropriate difficulty range for current level
    /// 2. Randomly select from filtered candidates
    ///
    /// Difficulty mapping by level:
    /// - Level 1-3: Beginner only
    /// - Level 4-6: Beginner + Intermediate
    /// - Level 7-10: Intermediate + Advanced
    ///
    /// # Returns
    ///
    /// Returns Some(Scenario) if candidates exist, None if collection is empty
    /// or no scenarios match the current difficulty level.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use helix_trainer::minigame::DifficultyController;
    ///
    /// let mut controller = DifficultyController::new();
    /// let scenarios = load_scenarios();
    ///
    /// if let Some(scenario) = controller.next_scenario(&scenarios) {
    ///     println!("Selected: {}", scenario.name);
    /// }
    /// ```
    pub fn next_scenario(&mut self, scenarios: &[Scenario]) -> Option<Scenario> {
        if scenarios.is_empty() {
            return None;
        }

        // Determine difficulty range for current level
        let difficulties = self.available_difficulties();

        // Filter scenarios by difficulty
        let candidates: Vec<_> = scenarios
            .iter()
            .filter(|s| {
                if let Some(ref metadata) = s.metadata
                    && let Some(diff) = metadata.difficulty
                {
                    difficulties.contains(&diff)
                } else {
                    // Include scenarios without metadata at all levels
                    true
                }
            })
            .collect();

        if candidates.is_empty() {
            // Fallback: pick any random scenario
            use rand::Rng;
            let mut rng = rand::rng();
            let idx = rng.random_range(0..scenarios.len());
            return Some(scenarios[idx].clone());
        }

        // Random selection from candidates
        use rand::Rng;
        let mut rng = rand::rng();
        let idx = rng.random_range(0..candidates.len());
        Some(candidates[idx].clone())
    }

    /// Get available difficulties for current level
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use helix_trainer::minigame::DifficultyController;
    /// use helix_trainer::config::Difficulty;
    ///
    /// let controller = DifficultyController::new();
    /// let diffs = controller.available_difficulties();
    /// assert!(diffs.contains(&Difficulty::Beginner));
    /// ```
    pub fn available_difficulties(&self) -> Vec<Difficulty> {
        match self.level {
            1..=3 => vec![Difficulty::Beginner],
            4..=6 => vec![Difficulty::Beginner, Difficulty::Intermediate],
            7..=10 => vec![Difficulty::Intermediate, Difficulty::Advanced],
            _ => vec![Difficulty::Beginner], // Shouldn't happen
        }
    }

    /// Update difficulty based on scenario result
    ///
    /// Adaptive difficulty algorithm:
    /// - Track last 10 results
    /// - If success rate > 90% over 5+ attempts: increase level
    /// - If success rate < 50% over 5+ attempts: decrease level
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use helix_trainer::minigame::DifficultyController;
    ///
    /// let mut controller = DifficultyController::new();
    ///
    /// // Simulate high success rate
    /// for _ in 0..10 {
    ///     controller.update_after_result(true);
    /// }
    /// // Level should increase
    /// ```
    pub fn update_after_result(&mut self, success: bool) {
        // Add result to recent history
        self.recent_results.push_back(success);

        // Keep only last N results
        while self.recent_results.len() > RECENT_RESULTS_WINDOW {
            self.recent_results.pop_front();
        }

        // Need at least 5 results to adjust difficulty
        if self.recent_results.len() < 5 {
            return;
        }

        // Calculate success rate
        let successes = self.recent_results.iter().filter(|&&s| s).count();
        let total = self.recent_results.len();
        let rate = successes as f64 / total as f64;

        // Adjust difficulty based on performance
        if rate > 0.9 && self.level < 10 {
            self.increase_difficulty();
        } else if rate < 0.5 && self.level > 1 {
            self.decrease_difficulty();
        }
    }

    /// Increase difficulty level (max 10)
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use helix_trainer::minigame::DifficultyController;
    ///
    /// let mut controller = DifficultyController::new();
    /// controller.increase_difficulty();
    /// assert_eq!(controller.current_level(), 2);
    /// ```
    pub fn increase_difficulty(&mut self) {
        if self.level < 10 {
            self.level += 1;
            tracing::info!(new_level = self.level, "Difficulty increased");
        }
    }

    /// Decrease difficulty level (min 1)
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use helix_trainer::minigame::DifficultyController;
    ///
    /// let mut controller = DifficultyController::new();
    /// controller.level = 5;
    /// controller.decrease_difficulty();
    /// assert_eq!(controller.current_level(), 4);
    /// ```
    pub fn decrease_difficulty(&mut self) {
        if self.level > 1 {
            self.level -= 1;
            tracing::info!(new_level = self.level, "Difficulty decreased");
        }
    }

    /// Get current success rate
    ///
    /// Returns success rate as a percentage (0.0 to 1.0).
    /// Returns None if no results recorded yet.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use helix_trainer::minigame::DifficultyController;
    ///
    /// let mut controller = DifficultyController::new();
    /// controller.update_after_result(true);
    /// controller.update_after_result(false);
    /// assert_eq!(controller.success_rate(), Some(0.5));
    /// ```
    pub fn success_rate(&self) -> Option<f64> {
        if self.recent_results.is_empty() {
            return None;
        }

        let successes = self.recent_results.iter().filter(|&&s| s).count();
        let total = self.recent_results.len();
        Some(successes as f64 / total as f64)
    }

    /// Reset difficulty to level 1
    ///
    /// Clears recent results and resets to starting difficulty.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use helix_trainer::minigame::DifficultyController;
    ///
    /// let mut controller = DifficultyController::new();
    /// controller.level = 5;
    /// controller.reset();
    /// assert_eq!(controller.current_level(), 1);
    /// ```
    pub fn reset(&mut self) {
        self.level = 1;
        self.recent_results.clear();
    }
}

impl Default for DifficultyController {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{
        Difficulty, Scenario, ScenarioMetadata, ScoringConfig, Setup, Solution, TargetState,
    };

    fn create_test_scenario(difficulty: Difficulty, id: &str) -> Scenario {
        Scenario {
            id: id.to_string(),
            name: format!("Test {}", id),
            description: "Test scenario".to_string(),
            setup: Setup {
                file_content: "test".to_string(),
                cursor_position: (0, 0),
            },
            target: TargetState {
                file_content: "test".to_string(),
                cursor_position: (0, 0),
                selection: None,
            },
            solution: Solution {
                commands: vec!["x".to_string()],
                description: "Delete".to_string(),
            },
            alternatives: vec![],
            hints: vec![],
            scoring: ScoringConfig {
                optimal_count: 1,
                max_points: 100,
                tolerance: 0,
            },
            metadata: Some(ScenarioMetadata {
                difficulty: Some(difficulty),
                ..Default::default()
            }),
        }
    }

    #[test]
    fn test_new_controller() {
        let controller = DifficultyController::new();
        assert_eq!(controller.current_level(), 1);
        assert_eq!(controller.success_rate(), None);
    }

    #[test]
    fn test_time_limit_by_difficulty() {
        let controller = DifficultyController::new();

        let beginner = create_test_scenario(Difficulty::Beginner, "b1");
        assert_eq!(controller.time_limit_for(&beginner).as_secs(), 10);

        let intermediate = create_test_scenario(Difficulty::Intermediate, "i1");
        assert_eq!(controller.time_limit_for(&intermediate).as_secs(), 8);

        let advanced = create_test_scenario(Difficulty::Advanced, "a1");
        assert_eq!(controller.time_limit_for(&advanced).as_secs(), 6);
    }

    #[test]
    fn test_time_limit_scales_with_level() {
        let mut controller = DifficultyController::new();
        let scenario = create_test_scenario(Difficulty::Beginner, "b1");

        // Level 1-3: 100%
        controller.level = 1;
        assert_eq!(controller.time_limit_for(&scenario).as_secs(), 10);

        controller.level = 3;
        assert_eq!(controller.time_limit_for(&scenario).as_secs(), 10);

        // Level 4-6: 90%
        controller.level = 4;
        assert_eq!(controller.time_limit_for(&scenario).as_secs(), 9);

        // Level 7-10: 80%
        controller.level = 7;
        assert_eq!(controller.time_limit_for(&scenario).as_secs(), 8);

        controller.level = 10;
        assert_eq!(controller.time_limit_for(&scenario).as_secs(), 8);
    }

    #[test]
    fn test_available_difficulties_by_level() {
        let mut controller = DifficultyController::new();

        controller.level = 1;
        assert_eq!(
            controller.available_difficulties(),
            vec![Difficulty::Beginner]
        );

        controller.level = 3;
        assert_eq!(
            controller.available_difficulties(),
            vec![Difficulty::Beginner]
        );

        controller.level = 4;
        assert_eq!(
            controller.available_difficulties(),
            vec![Difficulty::Beginner, Difficulty::Intermediate]
        );

        controller.level = 6;
        assert_eq!(
            controller.available_difficulties(),
            vec![Difficulty::Beginner, Difficulty::Intermediate]
        );

        controller.level = 7;
        assert_eq!(
            controller.available_difficulties(),
            vec![Difficulty::Intermediate, Difficulty::Advanced]
        );

        controller.level = 10;
        assert_eq!(
            controller.available_difficulties(),
            vec![Difficulty::Intermediate, Difficulty::Advanced]
        );
    }

    #[test]
    fn test_next_scenario_filters_by_difficulty() {
        let mut controller = DifficultyController::new();
        let scenarios = vec![
            create_test_scenario(Difficulty::Beginner, "b1"),
            create_test_scenario(Difficulty::Intermediate, "i1"),
            create_test_scenario(Difficulty::Advanced, "a1"),
        ];

        // Level 1-3: only beginner
        controller.level = 1;
        for _ in 0..10 {
            let selected = controller.next_scenario(&scenarios).unwrap();
            if let Some(ref metadata) = selected.metadata {
                assert_eq!(metadata.difficulty, Some(Difficulty::Beginner));
            }
        }

        // Level 7-10: intermediate and advanced
        controller.level = 7;
        let mut found_intermediate = false;
        let mut found_advanced = false;

        for _ in 0..50 {
            let selected = controller.next_scenario(&scenarios).unwrap();
            if let Some(ref metadata) = selected.metadata
                && let Some(diff) = metadata.difficulty
            {
                match diff {
                    Difficulty::Intermediate => found_intermediate = true,
                    Difficulty::Advanced => found_advanced = true,
                    Difficulty::Beginner => panic!("Should not select beginner at level 7"),
                }
            }
        }

        assert!(found_intermediate);
        assert!(found_advanced);
    }

    #[test]
    fn test_update_difficulty_increase() {
        let mut controller = DifficultyController::new();

        // Simulate 10 successes (>90% success rate)
        for _ in 0..10 {
            controller.update_after_result(true);
        }

        assert!(controller.current_level() > 1);
    }

    #[test]
    fn test_update_difficulty_decrease() {
        let mut controller = DifficultyController::new();
        controller.level = 5;

        // Simulate 10 failures (<50% success rate)
        for _ in 0..10 {
            controller.update_after_result(false);
        }

        assert!(controller.current_level() < 5);
    }

    #[test]
    fn test_success_rate_calculation() {
        let mut controller = DifficultyController::new();

        controller.update_after_result(true);
        controller.update_after_result(true);
        controller.update_after_result(false);
        controller.update_after_result(true);

        let rate = controller.success_rate().unwrap();
        assert!((rate - 0.75).abs() < 0.01);
    }

    #[test]
    fn test_difficulty_bounds() {
        let mut controller = DifficultyController::new();

        // Can't go below 1
        controller.decrease_difficulty();
        assert_eq!(controller.current_level(), 1);

        // Can't go above 10
        controller.level = 10;
        controller.increase_difficulty();
        assert_eq!(controller.current_level(), 10);
    }

    #[test]
    fn test_reset() {
        let mut controller = DifficultyController::new();
        controller.level = 5;
        controller.update_after_result(true);
        controller.update_after_result(false);

        controller.reset();
        assert_eq!(controller.current_level(), 1);
        assert_eq!(controller.success_rate(), None);
    }

    #[test]
    fn test_recent_results_window() {
        let mut controller = DifficultyController::new();

        // Add more than window size
        for _ in 0..15 {
            controller.update_after_result(true);
        }

        // Should only keep last 10
        assert_eq!(controller.recent_results.len(), RECENT_RESULTS_WINDOW);
    }
}
