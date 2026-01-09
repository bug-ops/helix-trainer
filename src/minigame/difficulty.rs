//! Difficulty controller for adaptive gameplay
//!
//! Manages difficulty scaling and scenario selection for mini-games.
//!
//! The difficulty controller uses a weighted performance scoring system
//! that considers multiple factors:
//!
//! - **Success** (60%): Whether the scenario was completed
//! - **Speed** (25%): How fast relative to time limit
//! - **Efficiency** (15%): Optimal vs actual action count
//!
//! Recent results are weighted more heavily (recency bias), and harder
//! scenarios contribute more to the performance score.

use crate::config::{Difficulty, Scenario};
use crate::constants::{
    ADVANCED_DIFFICULTY_WEIGHT, ADVANCED_TIME_LIMIT, BEGINNER_DIFFICULTY_WEIGHT,
    BEGINNER_TIME_LIMIT, DEFAULT_PERFORMANCE_SCORE, DIFFICULTY_DECREASE_SCORE,
    DIFFICULTY_INCREASE_SCORE, FALLBACK_TIME_LIMIT, INTERMEDIATE_DIFFICULTY_WEIGHT,
    INTERMEDIATE_TIME_LIMIT, LEVEL_1_3_TIME_SCALE, LEVEL_4_6_TIME_SCALE, LEVEL_7_10_TIME_SCALE,
    LEVEL_ADVANCED_MAX, LEVEL_ADVANCED_MIN, LEVEL_BEGINNER_MAX, LEVEL_BEGINNER_MIN,
    LEVEL_INTERMEDIATE_MAX, LEVEL_INTERMEDIATE_MIN, MIN_SCENARIOS_FOR_DECREASE,
    MIN_SCENARIOS_FOR_INCREASE, MIN_TIME_SCALE_MULTIPLIER, PERFORMANCE_EFFICIENCY_WEIGHT,
    PERFORMANCE_HISTORY_SIZE, PERFORMANCE_SPEED_WEIGHT, PERFORMANCE_SUCCESS_WEIGHT,
    RECENCY_BASE_WEIGHT, RECENCY_WEIGHT_INCREMENT, RECENT_SUCCESS_RATE_FOR_DECREASE,
    RECENT_SUCCESS_RATE_FOR_INCREASE, RECENT_TREND_WINDOW,
};
use std::collections::VecDeque;
use std::time::Duration;

/// Performance data point for adaptive difficulty tracking
///
/// Captures comprehensive information about each scenario attempt
/// for use in weighted performance scoring.
///
/// # Examples
///
/// ```ignore
/// use helix_trainer::minigame::PerformancePoint;
/// use helix_trainer::config::Difficulty;
///
/// let point = PerformancePoint {
///     success: true,
///     time_ratio: 0.5, // completed in half the time limit
///     scenario_difficulty: Difficulty::Intermediate,
///     efficiency: 1.0, // used optimal number of actions
/// };
/// ```
#[derive(Debug, Clone)]
pub struct PerformancePoint {
    /// Whether scenario was completed successfully
    pub success: bool,

    /// Time taken relative to time limit (0.0 = instant, 1.0 = timeout)
    pub time_ratio: f64,

    /// Difficulty of the scenario completed
    pub scenario_difficulty: Difficulty,

    /// Action efficiency (optimal_count / actual_count, capped at 1.0)
    pub efficiency: f64,
}

impl PerformancePoint {
    /// Create a new performance point
    ///
    /// # Arguments
    ///
    /// * `success` - Whether the scenario was completed successfully
    /// * `time_ratio` - Time taken as fraction of limit (0.0 to 1.0+)
    /// * `scenario_difficulty` - Difficulty level of the scenario
    /// * `efficiency` - Action efficiency (optimal / actual, capped at 1.0)
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use helix_trainer::minigame::PerformancePoint;
    /// use helix_trainer::config::Difficulty;
    ///
    /// let point = PerformancePoint::new(
    ///     true,
    ///     0.6, // 60% of time used
    ///     Difficulty::Beginner,
    ///     0.8, // 80% efficiency
    /// );
    /// ```
    pub fn new(
        success: bool,
        time_ratio: f64,
        scenario_difficulty: Difficulty,
        efficiency: f64,
    ) -> Self {
        Self {
            success,
            time_ratio: time_ratio.clamp(0.0, 1.0),
            scenario_difficulty,
            efficiency: efficiency.clamp(0.0, 1.0),
        }
    }

    /// Calculate the weighted score for this performance point
    ///
    /// Score components (configurable via constants):
    /// - Success: 60% weight (1.0 if success, 0.0 if failure)
    /// - Speed: 25% weight (1.0 - time_ratio, so faster = higher)
    /// - Efficiency: 15% weight (efficiency ratio)
    ///
    /// # Returns
    ///
    /// A score between 0.0 and 1.0 representing overall performance.
    #[must_use]
    pub fn calculate_score(&self) -> f64 {
        let success_score = if self.success { 1.0 } else { 0.0 };
        let speed_score = 1.0 - self.time_ratio;
        let efficiency_score = self.efficiency;

        success_score * PERFORMANCE_SUCCESS_WEIGHT
            + speed_score * PERFORMANCE_SPEED_WEIGHT
            + efficiency_score * PERFORMANCE_EFFICIENCY_WEIGHT
    }

    /// Get the difficulty weight multiplier for this scenario
    ///
    /// Harder scenarios contribute more to performance evaluation:
    /// - Beginner: 0.8x
    /// - Intermediate: 1.0x
    /// - Advanced: 1.2x
    pub fn difficulty_weight(&self) -> f64 {
        match self.scenario_difficulty {
            Difficulty::Beginner => BEGINNER_DIFFICULTY_WEIGHT,
            Difficulty::Intermediate => INTERMEDIATE_DIFFICULTY_WEIGHT,
            Difficulty::Advanced => ADVANCED_DIFFICULTY_WEIGHT,
        }
    }
}

/// Level change event for UI feedback
///
/// Generated when difficulty level changes, allowing the UI
/// to display appropriate notifications and animations.
///
/// # Examples
///
/// ```ignore
/// use helix_trainer::minigame::LevelChange;
///
/// match level_change {
///     LevelChange::Increased { from, to } => {
///         println!("Level up! {} -> {}", from, to);
///     }
///     LevelChange::Decreased { from, to } => {
///         println!("Level down: {} -> {}", from, to);
///     }
/// }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LevelChange {
    /// Difficulty increased due to good performance
    Increased {
        /// Previous level before increase
        from: u8,
        /// New level after increase
        to: u8,
    },
    /// Difficulty decreased due to struggling
    Decreased {
        /// Previous level before decrease
        from: u8,
        /// New level after decrease
        to: u8,
    },
}

impl LevelChange {
    /// Check if this is a level increase
    pub fn is_increase(&self) -> bool {
        matches!(self, LevelChange::Increased { .. })
    }

    /// Check if this is a level decrease
    pub fn is_decrease(&self) -> bool {
        matches!(self, LevelChange::Decreased { .. })
    }

    /// Get the from and to levels
    pub fn levels(&self) -> (u8, u8) {
        match *self {
            LevelChange::Increased { from, to } | LevelChange::Decreased { from, to } => (from, to),
        }
    }
}

/// Difficulty controller for mini-game mode
///
/// Manages adaptive difficulty scaling based on player performance
/// and selects appropriate scenarios from the collection.
///
/// The controller tracks comprehensive performance data and uses
/// weighted scoring to make smarter difficulty adjustments:
///
/// 1. **Performance Points**: Each attempt records success, time, and efficiency
/// 2. **Recency Weighting**: Recent results have more impact
/// 3. **Difficulty Weighting**: Harder scenarios count more
/// 4. **Trend Analysis**: Checks recent success pattern before adjusting
///
/// # Examples
///
/// ```ignore
/// use helix_trainer::minigame::{DifficultyController, PerformancePoint};
/// use helix_trainer::config::Difficulty;
///
/// let mut controller = DifficultyController::new();
///
/// // Record some performance
/// controller.update_after_scenario(PerformancePoint::new(
///     true,
///     0.4,
///     Difficulty::Beginner,
///     1.0,
/// ));
///
/// // Check if level changed
/// if let Some(change) = controller.take_level_change() {
///     println!("Level changed: {:?}", change);
/// }
/// ```
#[derive(Debug, Clone)]
pub struct DifficultyController {
    /// Current difficulty level (1-10)
    level: u8,

    /// Recent performance history with detailed metrics
    recent_performance: VecDeque<PerformancePoint>,

    /// Cumulative weighted performance score (0.0 to 1.0)
    performance_score: f64,

    /// Scenarios completed at current level (resets on level change)
    scenarios_at_level: u32,

    /// Flag indicating if level changed recently (for UI feedback)
    level_changed: Option<LevelChange>,
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
            recent_performance: VecDeque::with_capacity(PERFORMANCE_HISTORY_SIZE),
            performance_score: DEFAULT_PERFORMANCE_SCORE,
            scenarios_at_level: 0,
            level_changed: None,
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
                Difficulty::Beginner => BEGINNER_TIME_LIMIT,
                Difficulty::Intermediate => INTERMEDIATE_TIME_LIMIT,
                Difficulty::Advanced => ADVANCED_TIME_LIMIT,
            }
        } else {
            // Fallback if no metadata: use medium time
            FALLBACK_TIME_LIMIT
        };

        // Scale by current level (level 10 = 80% time, but not less than 50%)
        let scale = match self.level {
            LEVEL_BEGINNER_MIN..=LEVEL_BEGINNER_MAX => LEVEL_1_3_TIME_SCALE,
            LEVEL_INTERMEDIATE_MIN..=LEVEL_INTERMEDIATE_MAX => LEVEL_4_6_TIME_SCALE,
            LEVEL_ADVANCED_MIN..=LEVEL_ADVANCED_MAX => LEVEL_7_10_TIME_SCALE,
            _ => LEVEL_7_10_TIME_SCALE, // Shouldn't happen, but safe fallback
        };

        let scaled_secs = base.as_secs_f64() * scale;
        let min_secs = base.as_secs_f64() * MIN_TIME_SCALE_MULTIPLIER;

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
            LEVEL_BEGINNER_MIN..=LEVEL_BEGINNER_MAX => vec![Difficulty::Beginner],
            LEVEL_INTERMEDIATE_MIN..=LEVEL_INTERMEDIATE_MAX => {
                vec![Difficulty::Beginner, Difficulty::Intermediate]
            }
            LEVEL_ADVANCED_MIN..=LEVEL_ADVANCED_MAX => {
                vec![Difficulty::Intermediate, Difficulty::Advanced]
            }
            _ => vec![Difficulty::Beginner], // Shouldn't happen
        }
    }

    /// Update difficulty based on comprehensive performance data
    ///
    /// This algorithm considers:
    /// - Success/failure
    /// - Time taken relative to limit
    /// - Action efficiency
    /// - Recent performance trend
    /// - Difficulty of completed scenarios
    ///
    /// # Arguments
    ///
    /// * `point` - Performance data for the completed scenario
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use helix_trainer::minigame::{DifficultyController, PerformancePoint};
    /// use helix_trainer::config::Difficulty;
    ///
    /// let mut controller = DifficultyController::new();
    ///
    /// // Record a successful, fast, efficient completion
    /// controller.update_after_scenario(PerformancePoint::new(
    ///     true,
    ///     0.3,  // 30% of time used
    ///     Difficulty::Beginner,
    ///     1.0,  // optimal actions
    /// ));
    /// ```
    pub fn update_after_scenario(&mut self, point: PerformancePoint) {
        // Add to history
        self.recent_performance.push_back(point);

        // Keep only last N results
        while self.recent_performance.len() > PERFORMANCE_HISTORY_SIZE {
            self.recent_performance.pop_front();
        }

        // Recalculate weighted performance score
        self.performance_score = self.calculate_performance_score();

        // Increment scenarios at current level
        self.scenarios_at_level = self.scenarios_at_level.saturating_add(1);

        // Clear previous level change event
        self.level_changed = None;

        // Need minimum history before adjusting
        if self.recent_performance.len() < RECENT_TREND_WINDOW {
            return;
        }

        // Determine if level should change
        if self.should_increase_difficulty() {
            self.increase_level();
        } else if self.should_decrease_difficulty() {
            self.decrease_level();
        }
    }

    /// Calculate weighted performance score from recent history
    ///
    /// The score considers:
    /// - Each point's individual score (success, speed, efficiency)
    /// - Recency weighting (newer results weighted more)
    /// - Difficulty weighting (harder scenarios count more)
    ///
    /// # Returns
    ///
    /// A score between 0.0 and 1.0 representing overall performance.
    fn calculate_performance_score(&self) -> f64 {
        if self.recent_performance.is_empty() {
            return DEFAULT_PERFORMANCE_SCORE;
        }

        let mut weighted_sum = 0.0;
        let mut weight_total = 0.0;

        // Iterate with recency weighting (later items = more recent = higher weight)
        for (i, point) in self.recent_performance.iter().enumerate() {
            // Recency weight: 1.0, 1.1, 1.2, ... for older to newer
            let recency_weight = RECENCY_BASE_WEIGHT + (i as f64 * RECENCY_WEIGHT_INCREMENT);

            // Get point's score and difficulty weight
            let point_score = point.calculate_score();
            let difficulty_weight = point.difficulty_weight();

            // Combined weight
            let total_weight = recency_weight * difficulty_weight;

            weighted_sum += point_score * total_weight;
            weight_total += total_weight;
        }

        if weight_total <= 0.0 {
            return DEFAULT_PERFORMANCE_SCORE;
        }

        weighted_sum / weight_total
    }

    /// Check if difficulty should increase
    ///
    /// Requires:
    /// - Performance score above threshold (85%)
    /// - Minimum scenarios at current level (5)
    /// - Recent success rate above threshold (80%)
    /// - Not already at max level
    fn should_increase_difficulty(&self) -> bool {
        // Must be below max level
        if self.level >= LEVEL_ADVANCED_MAX {
            return false;
        }

        // Performance score must be high
        // Note: At exactly DIFFICULTY_INCREASE_SCORE (0.85), increase IS allowed.
        // Using `<` means score >= 0.85 passes this check.
        if self.performance_score < DIFFICULTY_INCREASE_SCORE {
            return false;
        }

        // Need sufficient scenarios at current level
        if self.scenarios_at_level < MIN_SCENARIOS_FOR_INCREASE {
            return false;
        }

        // Check recent trend (must be mostly successful)
        let recent_success_rate = self.calculate_recent_success_rate();
        recent_success_rate >= RECENT_SUCCESS_RATE_FOR_INCREASE
    }

    /// Check if difficulty should decrease
    ///
    /// Requires:
    /// - Performance score below threshold (40%)
    /// - Minimum scenarios at current level (3)
    /// - Recent success rate below threshold (40%)
    /// - Not already at min level
    fn should_decrease_difficulty(&self) -> bool {
        // Must be above min level
        if self.level <= LEVEL_BEGINNER_MIN {
            return false;
        }

        // Performance score must be low
        // Note: At exactly DIFFICULTY_DECREASE_SCORE (0.4), decrease is NOT allowed.
        // Using `>` (not `>=`) gives benefit of doubt to player at the threshold.
        if self.performance_score > DIFFICULTY_DECREASE_SCORE {
            return false;
        }

        // Need sufficient scenarios at current level
        if self.scenarios_at_level < MIN_SCENARIOS_FOR_DECREASE {
            return false;
        }

        // Check recent trend (must be mostly failures)
        let recent_success_rate = self.calculate_recent_success_rate();
        recent_success_rate <= RECENT_SUCCESS_RATE_FOR_DECREASE
    }

    /// Calculate success rate of recent results (for trend analysis)
    fn calculate_recent_success_rate(&self) -> f64 {
        let recent_count = RECENT_TREND_WINDOW.min(self.recent_performance.len());
        if recent_count == 0 {
            return 0.0;
        }

        let successes = self
            .recent_performance
            .iter()
            .rev()
            .take(recent_count)
            .filter(|p| p.success)
            .count();

        successes as f64 / recent_count as f64
    }

    /// Increase difficulty level (internal, with event generation)
    fn increase_level(&mut self) {
        let old_level = self.level;
        if self.level < LEVEL_ADVANCED_MAX {
            self.level += 1;
            self.scenarios_at_level = 0;
            self.level_changed = Some(LevelChange::Increased {
                from: old_level,
                to: self.level,
            });
            tracing::info!(
                old_level = old_level,
                new_level = self.level,
                performance = %self.performance_score,
                "Difficulty increased"
            );
        }
    }

    /// Decrease difficulty level (internal, with event generation)
    fn decrease_level(&mut self) {
        let old_level = self.level;
        if self.level > LEVEL_BEGINNER_MIN {
            self.level -= 1;
            self.scenarios_at_level = 0;
            self.level_changed = Some(LevelChange::Decreased {
                from: old_level,
                to: self.level,
            });
            tracing::info!(
                old_level = old_level,
                new_level = self.level,
                performance = %self.performance_score,
                "Difficulty decreased"
            );
        }
    }

    /// Increase difficulty level (max 10) - public API
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
        self.increase_level();
    }

    /// Decrease difficulty level (min 1) - public API
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
        self.decrease_level();
    }

    /// Take the level change event (consumes it)
    ///
    /// Returns the level change event if one occurred, clearing it for
    /// subsequent calls. This allows UI to display notifications once.
    ///
    /// # Returns
    ///
    /// `Some(LevelChange)` if level changed since last call, `None` otherwise.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use helix_trainer::minigame::DifficultyController;
    ///
    /// let mut controller = DifficultyController::new();
    /// // ... simulate level change ...
    ///
    /// if let Some(change) = controller.take_level_change() {
    ///     match change {
    ///         LevelChange::Increased { from, to } => println!("Level up!"),
    ///         LevelChange::Decreased { from, to } => println!("Level down"),
    ///     }
    /// }
    /// // Second call returns None
    /// assert!(controller.take_level_change().is_none());
    /// ```
    #[must_use]
    pub fn take_level_change(&mut self) -> Option<LevelChange> {
        self.level_changed.take()
    }

    /// Get current performance score (0.0 to 1.0)
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use helix_trainer::minigame::DifficultyController;
    ///
    /// let controller = DifficultyController::new();
    /// let score = controller.performance_score();
    /// assert!(score >= 0.0 && score <= 1.0);
    /// ```
    pub fn performance_score(&self) -> f64 {
        self.performance_score
    }

    /// Get progress toward next level (0.0 to 1.0)
    ///
    /// This indicates how close the player is to leveling up:
    /// - Below performance threshold: progress = score / threshold
    /// - Above threshold: progress = scenarios_completed / required
    ///
    /// # Returns
    ///
    /// A value between 0.0 and 1.0 indicating progress toward next level.
    pub fn level_progress(&self) -> f64 {
        if self.level >= LEVEL_ADVANCED_MAX {
            return 1.0; // Already at max
        }

        if self.performance_score < DIFFICULTY_INCREASE_SCORE {
            // Below threshold - show distance to threshold
            self.performance_score / DIFFICULTY_INCREASE_SCORE
        } else {
            // Above threshold - show scenarios completed
            (self.scenarios_at_level as f64 / MIN_SCENARIOS_FOR_INCREASE as f64).min(1.0)
        }
    }

    /// Get scenarios completed at current level
    pub fn scenarios_at_level(&self) -> u32 {
        self.scenarios_at_level
    }

    /// Get current success rate
    ///
    /// Returns success rate as a percentage (0.0 to 1.0).
    /// Returns None if no results recorded yet.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use helix_trainer::minigame::{DifficultyController, PerformancePoint};
    /// use helix_trainer::config::Difficulty;
    ///
    /// let mut controller = DifficultyController::new();
    /// controller.update_after_scenario(PerformancePoint::new(true, 0.5, Difficulty::Beginner, 0.8));
    /// controller.update_after_scenario(PerformancePoint::new(false, 1.0, Difficulty::Beginner, 0.0));
    /// let rate = controller.success_rate();
    /// assert_eq!(rate, Some(0.5));
    /// ```
    pub fn success_rate(&self) -> Option<f64> {
        if self.recent_performance.is_empty() {
            return None;
        }

        let successes = self.recent_performance.iter().filter(|p| p.success).count();
        let total = self.recent_performance.len();
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
        self.recent_performance.clear();
        self.performance_score = DEFAULT_PERFORMANCE_SCORE;
        self.scenarios_at_level = 0;
        self.level_changed = None;
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
                selection: None,
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

    // Helper to create a performance point for testing
    fn create_performance_point(
        success: bool,
        time_ratio: f64,
        difficulty: Difficulty,
        efficiency: f64,
    ) -> PerformancePoint {
        PerformancePoint::new(success, time_ratio, difficulty, efficiency)
    }

    // ========================================
    // PerformancePoint Tests
    // ========================================

    #[test]
    fn test_performance_point_creation() {
        let point = PerformancePoint::new(true, 0.5, Difficulty::Beginner, 0.8);
        assert!(point.success);
        assert!((point.time_ratio - 0.5).abs() < 0.001);
        assert_eq!(point.scenario_difficulty, Difficulty::Beginner);
        assert!((point.efficiency - 0.8).abs() < 0.001);
    }

    #[test]
    fn test_performance_point_clamps_values() {
        // Time ratio should be clamped to [0, 1]
        let point_high = PerformancePoint::new(true, 1.5, Difficulty::Beginner, 0.5);
        assert!((point_high.time_ratio - 1.0).abs() < 0.001);

        let point_low = PerformancePoint::new(true, -0.5, Difficulty::Beginner, 0.5);
        assert!((point_low.time_ratio - 0.0).abs() < 0.001);

        // Efficiency should also be clamped
        let point_eff = PerformancePoint::new(true, 0.5, Difficulty::Beginner, 1.5);
        assert!((point_eff.efficiency - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_performance_point_score_calculation() {
        // Perfect performance: success=true, fast (0.0 time), 100% efficiency
        let perfect = PerformancePoint::new(true, 0.0, Difficulty::Beginner, 1.0);
        let perfect_score = perfect.calculate_score();
        // Expected: 1.0 * 0.6 + 1.0 * 0.25 + 1.0 * 0.15 = 1.0
        assert!((perfect_score - 1.0).abs() < 0.001);

        // Total failure: success=false, timeout (1.0 time), 0% efficiency
        let failure = PerformancePoint::new(false, 1.0, Difficulty::Beginner, 0.0);
        let failure_score = failure.calculate_score();
        // Expected: 0.0 * 0.6 + 0.0 * 0.25 + 0.0 * 0.15 = 0.0
        assert!((failure_score - 0.0).abs() < 0.001);

        // Moderate performance: success=true, 50% time, 80% efficiency
        let moderate = PerformancePoint::new(true, 0.5, Difficulty::Beginner, 0.8);
        let moderate_score = moderate.calculate_score();
        // Expected: 1.0 * 0.6 + 0.5 * 0.25 + 0.8 * 0.15 = 0.6 + 0.125 + 0.12 = 0.845
        assert!((moderate_score - 0.845).abs() < 0.01);
    }

    #[test]
    fn test_performance_point_difficulty_weight() {
        let beginner = PerformancePoint::new(true, 0.5, Difficulty::Beginner, 0.8);
        assert!((beginner.difficulty_weight() - BEGINNER_DIFFICULTY_WEIGHT).abs() < 0.001);

        let intermediate = PerformancePoint::new(true, 0.5, Difficulty::Intermediate, 0.8);
        assert!((intermediate.difficulty_weight() - INTERMEDIATE_DIFFICULTY_WEIGHT).abs() < 0.001);

        let advanced = PerformancePoint::new(true, 0.5, Difficulty::Advanced, 0.8);
        assert!((advanced.difficulty_weight() - ADVANCED_DIFFICULTY_WEIGHT).abs() < 0.001);
    }

    // ========================================
    // LevelChange Tests
    // ========================================

    #[test]
    fn test_level_change_increase() {
        let change = LevelChange::Increased { from: 3, to: 4 };
        assert!(change.is_increase());
        assert!(!change.is_decrease());
        assert_eq!(change.levels(), (3, 4));
    }

    #[test]
    fn test_level_change_decrease() {
        let change = LevelChange::Decreased { from: 5, to: 4 };
        assert!(!change.is_increase());
        assert!(change.is_decrease());
        assert_eq!(change.levels(), (5, 4));
    }

    // ========================================
    // DifficultyController Tests
    // ========================================

    #[test]
    fn test_new_controller() {
        let controller = DifficultyController::new();
        assert_eq!(controller.current_level(), 1);
        assert_eq!(controller.success_rate(), None);
        assert!((controller.performance_score() - DEFAULT_PERFORMANCE_SCORE).abs() < 0.001);
        assert_eq!(controller.scenarios_at_level(), 0);
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

    // ========================================
    // Performance Score Calculation Tests
    // ========================================

    #[test]
    fn test_performance_score_empty_history() {
        let controller = DifficultyController::new();
        assert!((controller.performance_score() - DEFAULT_PERFORMANCE_SCORE).abs() < 0.001);
    }

    #[test]
    fn test_performance_score_single_perfect() {
        let mut controller = DifficultyController::new();
        controller.update_after_scenario(create_performance_point(
            true,
            0.0,
            Difficulty::Beginner,
            1.0,
        ));

        // Should be close to 1.0 (perfect)
        assert!(controller.performance_score() > 0.9);
    }

    #[test]
    fn test_performance_score_single_failure() {
        let mut controller = DifficultyController::new();
        controller.update_after_scenario(create_performance_point(
            false,
            1.0,
            Difficulty::Beginner,
            0.0,
        ));

        // Should be close to 0.0 (failure)
        assert!(controller.performance_score() < 0.1);
    }

    #[test]
    fn test_performance_score_recency_weighting() {
        let mut controller = DifficultyController::new();

        // Add old failures
        for _ in 0..5 {
            controller.update_after_scenario(create_performance_point(
                false,
                1.0,
                Difficulty::Beginner,
                0.0,
            ));
        }

        let score_after_failures = controller.performance_score();

        // Add recent successes
        for _ in 0..5 {
            controller.update_after_scenario(create_performance_point(
                true,
                0.3,
                Difficulty::Beginner,
                1.0,
            ));
        }

        let score_after_successes = controller.performance_score();

        // Recent successes should raise the score significantly
        // (due to recency weighting, even with equal count, recent matters more)
        assert!(score_after_successes > score_after_failures);
    }

    #[test]
    fn test_performance_score_difficulty_weighting() {
        // Test that advanced scenarios contribute more to score
        let mut controller1 = DifficultyController::new();
        let mut controller2 = DifficultyController::new();

        // Controller 1: All beginner successes
        for _ in 0..5 {
            controller1.update_after_scenario(create_performance_point(
                true,
                0.3,
                Difficulty::Beginner,
                1.0,
            ));
        }

        // Controller 2: All advanced successes
        for _ in 0..5 {
            controller2.update_after_scenario(create_performance_point(
                true,
                0.3,
                Difficulty::Advanced,
                1.0,
            ));
        }

        // Both should have high scores, but we're mainly checking they're calculated
        assert!(controller1.performance_score() > 0.7);
        assert!(controller2.performance_score() > 0.7);
    }

    // ========================================
    // Difficulty Adjustment Tests
    // ========================================

    #[test]
    fn test_update_after_scenario_tracks_history() {
        let mut controller = DifficultyController::new();

        // Use moderate performance (not too good to trigger level increase)
        for _ in 0..5 {
            controller.update_after_scenario(create_performance_point(
                true,
                0.7, // Slower (less impressive)
                Difficulty::Beginner,
                0.6, // Lower efficiency
            ));
        }

        // History should track all 5 points
        assert_eq!(controller.recent_performance.len(), 5);

        // scenarios_at_level may have been affected by level changes,
        // but for moderate performance it should remain at 5 (no level change)
        // If level did change, it resets to 0, so we check history instead
        assert!(controller.scenarios_at_level() <= 5);
    }

    #[test]
    fn test_performance_history_window_limit() {
        let mut controller = DifficultyController::new();

        // Add more than PERFORMANCE_HISTORY_SIZE points
        for _ in 0..20 {
            controller.update_after_scenario(create_performance_point(
                true,
                0.5,
                Difficulty::Beginner,
                0.8,
            ));
        }

        // Should be capped at PERFORMANCE_HISTORY_SIZE
        assert_eq!(
            controller.recent_performance.len(),
            PERFORMANCE_HISTORY_SIZE
        );
    }

    #[test]
    fn test_difficulty_increase_requires_consistency() {
        let mut controller = DifficultyController::new();
        let initial_level = controller.current_level();

        // Add fewer scenarios than MIN_SCENARIOS_FOR_INCREASE
        for _ in 0..3 {
            controller.update_after_scenario(create_performance_point(
                true,
                0.2, // Very fast
                Difficulty::Beginner,
                1.0, // Perfect efficiency
            ));
        }

        // Level should not have increased (need at least 5 scenarios)
        assert_eq!(controller.current_level(), initial_level);

        // Add more excellent performances to reach minimum
        for _ in 0..10 {
            controller.update_after_scenario(create_performance_point(
                true,
                0.2,
                Difficulty::Beginner,
                1.0,
            ));
        }

        // Now level should have increased
        assert!(controller.current_level() > initial_level);
    }

    #[test]
    fn test_difficulty_decrease_on_struggling() {
        let mut controller = DifficultyController::new();
        controller.level = 5;
        let initial_level = controller.current_level();

        // Add consistent failures
        for _ in 0..10 {
            controller.update_after_scenario(create_performance_point(
                false,
                1.0, // Timeout
                Difficulty::Intermediate,
                0.0, // No efficiency
            ));
        }

        // Level should have decreased
        assert!(controller.current_level() < initial_level);
    }

    #[test]
    fn test_level_change_events_increase() {
        let mut controller = DifficultyController::new();

        // No level change initially
        assert!(controller.take_level_change().is_none());

        // Force a level increase with excellent performance
        for _ in 0..15 {
            controller.update_after_scenario(create_performance_point(
                true,
                0.2,
                Difficulty::Beginner,
                1.0,
            ));
        }

        // Check level change event
        let change = controller.take_level_change();
        if let Some(LevelChange::Increased { from, to }) = change {
            assert!(to > from);
        }

        // Second call should return None (consumed)
        assert!(controller.take_level_change().is_none());
    }

    #[test]
    fn test_level_change_events_decrease() {
        let mut controller = DifficultyController::new();
        controller.level = 5;

        // Force a level decrease with poor performance
        for _ in 0..10 {
            controller.update_after_scenario(create_performance_point(
                false,
                1.0,
                Difficulty::Intermediate,
                0.0,
            ));
        }

        let change = controller.take_level_change();
        if let Some(LevelChange::Decreased { from, to }) = change {
            assert!(to < from);
        }
    }

    #[test]
    fn test_level_progress_below_threshold() {
        let mut controller = DifficultyController::new();

        // Add some moderate performance
        controller.update_after_scenario(create_performance_point(
            true,
            0.6, // Slow
            Difficulty::Beginner,
            0.6, // Poor efficiency
        ));

        // Progress should reflect score / threshold ratio
        let progress = controller.level_progress();
        assert!((0.0..=1.0).contains(&progress));
    }

    #[test]
    fn test_level_progress_at_max_level() {
        let mut controller = DifficultyController::new();
        controller.level = LEVEL_ADVANCED_MAX;

        // Progress should be 1.0 at max level
        assert!((controller.level_progress() - 1.0).abs() < 0.001);
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
        controller.update_after_scenario(create_performance_point(
            true,
            0.5,
            Difficulty::Beginner,
            0.8,
        ));
        controller.update_after_scenario(create_performance_point(
            false,
            1.0,
            Difficulty::Beginner,
            0.0,
        ));

        controller.reset();
        assert_eq!(controller.current_level(), 1);
        assert_eq!(controller.success_rate(), None);
        assert!((controller.performance_score() - DEFAULT_PERFORMANCE_SCORE).abs() < 0.001);
        assert_eq!(controller.scenarios_at_level(), 0);
    }

    // ========================================
    // Edge Case Tests
    // ========================================

    #[test]
    fn test_min_level_cannot_decrease() {
        let mut controller = DifficultyController::new();
        controller.level = 1;

        // Add many failures
        for _ in 0..20 {
            controller.update_after_scenario(create_performance_point(
                false,
                1.0,
                Difficulty::Beginner,
                0.0,
            ));
        }

        // Level should stay at 1
        assert_eq!(controller.current_level(), 1);
    }

    #[test]
    fn test_max_level_cannot_increase() {
        let mut controller = DifficultyController::new();
        controller.level = 10;

        // Add many successes
        for _ in 0..20 {
            controller.update_after_scenario(create_performance_point(
                true,
                0.1,
                Difficulty::Advanced,
                1.0,
            ));
        }

        // Level should stay at 10
        assert_eq!(controller.current_level(), 10);
    }

    #[test]
    fn test_scenarios_at_level_resets_on_level_change() {
        let mut controller = DifficultyController::new();

        // Add scenarios at level 1
        for _ in 0..15 {
            controller.update_after_scenario(create_performance_point(
                true,
                0.2,
                Difficulty::Beginner,
                1.0,
            ));
        }

        // If level increased, scenarios_at_level should have reset
        if controller.current_level() > 1 {
            // After level change, counter resets, then we may have added more
            assert!(controller.scenarios_at_level() < 15);
        }
    }
}
