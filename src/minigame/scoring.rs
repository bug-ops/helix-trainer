//! Enhanced scoring system for mini-game mode
//!
//! Provides detailed score breakdown with combo tracking, difficulty multipliers,
//! time bonuses, and efficiency bonuses.

use crate::config::Difficulty;
use crate::constants::{
    COMBO_BONUS_PER_LEVEL, DIFFICULTY_MULTIPLIER_ADVANCED, DIFFICULTY_MULTIPLIER_BEGINNER,
    DIFFICULTY_MULTIPLIER_INTERMEDIATE, MAX_COMBO_BONUS, MAX_EFFICIENCY_BONUS,
    MAX_TIME_BONUS_MULTIPLIER, OPTIMAL_EFFICIENCY_BONUS,
};

/// Detailed score breakdown for UI display
///
/// Provides a complete breakdown of how the final score was calculated,
/// allowing UI to show each bonus component to the player.
#[derive(Debug, Clone, PartialEq)]
pub struct ScoreBreakdown {
    /// Base points from scenario difficulty
    pub base_points: u64,
    /// Bonus points for fast completion
    pub time_bonus: u64,
    /// Bonus points for efficient actions
    pub efficiency_bonus: u64,
    /// Bonus points from combo streak
    pub combo_bonus: u64,
    /// Multiplier from scenario difficulty (1.0, 1.25, or 1.5)
    pub difficulty_multiplier: f64,
    /// Multiplier from stats streak (applied separately in MiniGameStats)
    pub streak_multiplier: f64,
    /// Sum before multipliers applied
    pub subtotal: u64,
    /// Final total after all multipliers
    pub total: u64,
}

impl ScoreBreakdown {
    /// Create a zero breakdown (for failures)
    pub fn zero() -> Self {
        Self {
            base_points: 0,
            time_bonus: 0,
            efficiency_bonus: 0,
            combo_bonus: 0,
            difficulty_multiplier: 1.0,
            streak_multiplier: 1.0,
            subtotal: 0,
            total: 0,
        }
    }
}

/// Performance tier for combo tracking
///
/// Determines how the combo counter is affected by scenario completion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PerformanceTier {
    /// 100% efficiency and fast completion (< 50% time used)
    Perfect,
    /// >90% efficiency OR very fast (< 30% time used)
    Excellent,
    /// Completed successfully
    Good,
    /// Failed or timed out
    Failed,
}

impl PerformanceTier {
    /// Determine tier from performance metrics
    ///
    /// # Arguments
    ///
    /// * `success` - Whether scenario was completed
    /// * `efficiency` - Action efficiency (optimal / actual, 0.0 to 1.0)
    /// * `time_ratio` - Time used as fraction of limit (0.0 to 1.0)
    pub fn from_metrics(success: bool, efficiency: f64, time_ratio: f64) -> Self {
        if !success {
            return Self::Failed;
        }

        let is_optimal = efficiency >= 1.0;
        let is_fast = time_ratio < 0.5;
        let is_very_fast = time_ratio < 0.3;
        let is_efficient = efficiency >= 0.9;

        if is_optimal && is_fast {
            Self::Perfect
        } else if is_efficient || is_very_fast {
            Self::Excellent
        } else {
            Self::Good
        }
    }

    /// Check if this tier maintains combo
    pub fn maintains_combo(&self) -> bool {
        !matches!(self, Self::Failed)
    }

    /// Check if this tier increases combo
    pub fn increases_combo(&self) -> bool {
        matches!(self, Self::Perfect | Self::Excellent | Self::Good)
    }
}

/// Score calculator with combo tracking
///
/// Tracks combo state across scenarios and calculates detailed score breakdowns.
#[derive(Debug, Clone)]
pub struct ScoreCalculator {
    combo_count: u32,
    best_combo: u32,
}

impl ScoreCalculator {
    /// Create a new score calculator
    pub fn new() -> Self {
        Self {
            combo_count: 0,
            best_combo: 0,
        }
    }

    /// Get current combo count
    pub fn combo_count(&self) -> u32 {
        self.combo_count
    }

    /// Get best combo achieved
    pub fn best_combo(&self) -> u32 {
        self.best_combo
    }

    /// Calculate score breakdown for a completed scenario
    ///
    /// # Arguments
    ///
    /// * `base_points` - Base points from scenario difficulty
    /// * `time_ratio` - Time used as fraction of limit (0.0 to 1.0, where 0 = instant)
    /// * `efficiency` - Action efficiency (optimal / actual, capped at 1.0)
    /// * `difficulty` - Scenario difficulty level
    /// * `streak_multiplier` - Current streak multiplier from MiniGameStats
    ///
    /// # Returns
    ///
    /// Detailed score breakdown with all bonus components.
    pub fn calculate(
        &mut self,
        base_points: u64,
        time_ratio: f64,
        efficiency: f64,
        difficulty: Difficulty,
        streak_multiplier: f64,
    ) -> ScoreBreakdown {
        let tier = PerformanceTier::from_metrics(true, efficiency, time_ratio);
        self.update_combo(tier);

        // Time bonus: faster = more points (up to MAX_TIME_BONUS_MULTIPLIER)
        let time_bonus_ratio = (1.0 - time_ratio).max(0.0);
        let time_bonus = (base_points as f64 * time_bonus_ratio * MAX_TIME_BONUS_MULTIPLIER) as u64;

        // Efficiency bonus: optimal = 25%, >80% = 12.5%
        let efficiency_bonus = if efficiency >= 1.0 {
            (base_points as f64 * OPTIMAL_EFFICIENCY_BONUS) as u64
        } else if efficiency >= 0.8 {
            (base_points as f64 * MAX_EFFICIENCY_BONUS) as u64
        } else {
            0
        };

        // Combo bonus: 10% per level, max 50%
        let combo_bonus_ratio =
            (self.combo_count as f64 * COMBO_BONUS_PER_LEVEL).min(MAX_COMBO_BONUS);
        let combo_bonus = (base_points as f64 * combo_bonus_ratio) as u64;

        // Difficulty multiplier
        let difficulty_multiplier = match difficulty {
            Difficulty::Beginner => DIFFICULTY_MULTIPLIER_BEGINNER,
            Difficulty::Intermediate => DIFFICULTY_MULTIPLIER_INTERMEDIATE,
            Difficulty::Advanced => DIFFICULTY_MULTIPLIER_ADVANCED,
        };

        // Calculate totals
        let subtotal = base_points + time_bonus + efficiency_bonus + combo_bonus;
        let total = (subtotal as f64 * difficulty_multiplier) as u64;

        ScoreBreakdown {
            base_points,
            time_bonus,
            efficiency_bonus,
            combo_bonus,
            difficulty_multiplier,
            streak_multiplier,
            subtotal,
            total,
        }
    }

    /// Calculate score for a failed scenario
    ///
    /// Resets combo and returns zero breakdown.
    pub fn calculate_failure(&mut self) -> ScoreBreakdown {
        self.update_combo(PerformanceTier::Failed);
        ScoreBreakdown::zero()
    }

    /// Update combo based on performance tier
    fn update_combo(&mut self, tier: PerformanceTier) {
        if tier.increases_combo() {
            self.combo_count = self.combo_count.saturating_add(1);
            if self.combo_count > self.best_combo {
                self.best_combo = self.combo_count;
            }
        } else {
            self.combo_count = 0;
        }
    }

    /// Reset combo counter
    pub fn reset_combo(&mut self) {
        self.combo_count = 0;
    }

    /// Reset all state
    pub fn reset(&mut self) {
        self.combo_count = 0;
        self.best_combo = 0;
    }
}

impl Default for ScoreCalculator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod performance_tier_tests {
        use super::*;

        #[test]
        fn test_perfect_tier() {
            let tier = PerformanceTier::from_metrics(true, 1.0, 0.4);
            assert_eq!(tier, PerformanceTier::Perfect);
        }

        #[test]
        fn test_excellent_tier_efficient() {
            let tier = PerformanceTier::from_metrics(true, 0.95, 0.6);
            assert_eq!(tier, PerformanceTier::Excellent);
        }

        #[test]
        fn test_excellent_tier_fast() {
            let tier = PerformanceTier::from_metrics(true, 0.5, 0.25);
            assert_eq!(tier, PerformanceTier::Excellent);
        }

        #[test]
        fn test_good_tier() {
            let tier = PerformanceTier::from_metrics(true, 0.7, 0.7);
            assert_eq!(tier, PerformanceTier::Good);
        }

        #[test]
        fn test_failed_tier() {
            let tier = PerformanceTier::from_metrics(false, 0.0, 1.0);
            assert_eq!(tier, PerformanceTier::Failed);
        }

        #[test]
        fn test_maintains_combo() {
            assert!(PerformanceTier::Perfect.maintains_combo());
            assert!(PerformanceTier::Excellent.maintains_combo());
            assert!(PerformanceTier::Good.maintains_combo());
            assert!(!PerformanceTier::Failed.maintains_combo());
        }

        #[test]
        fn test_increases_combo() {
            assert!(PerformanceTier::Perfect.increases_combo());
            assert!(PerformanceTier::Excellent.increases_combo());
            assert!(PerformanceTier::Good.increases_combo());
            assert!(!PerformanceTier::Failed.increases_combo());
        }
    }

    mod score_calculator_tests {
        use super::*;

        #[test]
        fn test_new_calculator() {
            let calc = ScoreCalculator::new();
            assert_eq!(calc.combo_count(), 0);
            assert_eq!(calc.best_combo(), 0);
        }

        #[test]
        fn test_base_points_only() {
            let mut calc = ScoreCalculator::new();
            let breakdown = calc.calculate(100, 1.0, 0.5, Difficulty::Beginner, 1.0);

            assert_eq!(breakdown.base_points, 100);
            assert_eq!(breakdown.time_bonus, 0);
            assert_eq!(breakdown.efficiency_bonus, 0);
            assert_eq!(breakdown.combo_bonus, 10);
            assert!((breakdown.difficulty_multiplier - 1.0).abs() < 0.001);
        }

        #[test]
        fn test_time_bonus() {
            let mut calc = ScoreCalculator::new();
            let breakdown = calc.calculate(100, 0.0, 0.5, Difficulty::Beginner, 1.0);

            assert_eq!(breakdown.time_bonus, 50);
        }

        #[test]
        fn test_time_bonus_partial() {
            let mut calc = ScoreCalculator::new();
            let breakdown = calc.calculate(100, 0.5, 0.5, Difficulty::Beginner, 1.0);

            assert_eq!(breakdown.time_bonus, 25);
        }

        #[test]
        fn test_efficiency_bonus_optimal() {
            let mut calc = ScoreCalculator::new();
            let breakdown = calc.calculate(100, 0.8, 1.0, Difficulty::Beginner, 1.0);

            assert_eq!(breakdown.efficiency_bonus, 25);
        }

        #[test]
        fn test_efficiency_bonus_good() {
            let mut calc = ScoreCalculator::new();
            let breakdown = calc.calculate(100, 0.8, 0.85, Difficulty::Beginner, 1.0);

            assert_eq!(breakdown.efficiency_bonus, 12);
        }

        #[test]
        fn test_efficiency_bonus_poor() {
            let mut calc = ScoreCalculator::new();
            let breakdown = calc.calculate(100, 0.8, 0.5, Difficulty::Beginner, 1.0);

            assert_eq!(breakdown.efficiency_bonus, 0);
        }

        #[test]
        fn test_combo_building() {
            let mut calc = ScoreCalculator::new();

            calc.calculate(100, 0.5, 0.8, Difficulty::Beginner, 1.0);
            assert_eq!(calc.combo_count(), 1);

            calc.calculate(100, 0.5, 0.8, Difficulty::Beginner, 1.0);
            assert_eq!(calc.combo_count(), 2);

            calc.calculate(100, 0.5, 0.8, Difficulty::Beginner, 1.0);
            assert_eq!(calc.combo_count(), 3);
        }

        #[test]
        fn test_combo_bonus_calculation() {
            let mut calc = ScoreCalculator::new();

            for _ in 0..3 {
                calc.calculate(100, 0.5, 0.8, Difficulty::Beginner, 1.0);
            }

            let breakdown = calc.calculate(100, 0.5, 0.8, Difficulty::Beginner, 1.0);
            assert_eq!(breakdown.combo_bonus, 40);
        }

        #[test]
        fn test_combo_bonus_max() {
            let mut calc = ScoreCalculator::new();

            for _ in 0..10 {
                calc.calculate(100, 0.5, 0.8, Difficulty::Beginner, 1.0);
            }

            let breakdown = calc.calculate(100, 0.5, 0.8, Difficulty::Beginner, 1.0);
            assert_eq!(breakdown.combo_bonus, 50);
        }

        #[test]
        fn test_combo_reset_on_failure() {
            let mut calc = ScoreCalculator::new();

            for _ in 0..5 {
                calc.calculate(100, 0.5, 0.8, Difficulty::Beginner, 1.0);
            }
            assert_eq!(calc.combo_count(), 5);

            calc.calculate_failure();
            assert_eq!(calc.combo_count(), 0);
        }

        #[test]
        fn test_best_combo_tracking() {
            let mut calc = ScoreCalculator::new();

            for _ in 0..5 {
                calc.calculate(100, 0.5, 0.8, Difficulty::Beginner, 1.0);
            }
            assert_eq!(calc.best_combo(), 5);

            calc.calculate_failure();
            assert_eq!(calc.best_combo(), 5);

            for _ in 0..3 {
                calc.calculate(100, 0.5, 0.8, Difficulty::Beginner, 1.0);
            }
            assert_eq!(calc.best_combo(), 5);

            for _ in 0..3 {
                calc.calculate(100, 0.5, 0.8, Difficulty::Beginner, 1.0);
            }
            assert_eq!(calc.best_combo(), 6);
        }

        #[test]
        fn test_difficulty_multiplier_beginner() {
            let mut calc = ScoreCalculator::new();
            let breakdown = calc.calculate(100, 0.8, 0.5, Difficulty::Beginner, 1.0);

            assert!((breakdown.difficulty_multiplier - 1.0).abs() < 0.001);
        }

        #[test]
        fn test_difficulty_multiplier_intermediate() {
            let mut calc = ScoreCalculator::new();
            let breakdown = calc.calculate(100, 0.8, 0.5, Difficulty::Intermediate, 1.0);

            assert!((breakdown.difficulty_multiplier - 1.25).abs() < 0.001);
        }

        #[test]
        fn test_difficulty_multiplier_advanced() {
            let mut calc = ScoreCalculator::new();
            let breakdown = calc.calculate(100, 0.8, 0.5, Difficulty::Advanced, 1.0);

            assert!((breakdown.difficulty_multiplier - 1.5).abs() < 0.001);
        }

        #[test]
        fn test_subtotal_calculation() {
            let mut calc = ScoreCalculator::new();
            let breakdown = calc.calculate(100, 0.0, 1.0, Difficulty::Beginner, 1.0);

            let expected_subtotal = breakdown.base_points
                + breakdown.time_bonus
                + breakdown.efficiency_bonus
                + breakdown.combo_bonus;
            assert_eq!(breakdown.subtotal, expected_subtotal);
        }

        #[test]
        fn test_total_with_difficulty_multiplier() {
            let mut calc = ScoreCalculator::new();
            let breakdown = calc.calculate(100, 0.8, 0.5, Difficulty::Advanced, 1.0);

            let expected_total = (breakdown.subtotal as f64 * 1.5) as u64;
            assert_eq!(breakdown.total, expected_total);
        }

        #[test]
        fn test_failure_returns_zero() {
            let mut calc = ScoreCalculator::new();
            let breakdown = calc.calculate_failure();

            assert_eq!(breakdown, ScoreBreakdown::zero());
        }

        #[test]
        fn test_reset() {
            let mut calc = ScoreCalculator::new();

            for _ in 0..5 {
                calc.calculate(100, 0.5, 0.8, Difficulty::Beginner, 1.0);
            }

            calc.reset();
            assert_eq!(calc.combo_count(), 0);
            assert_eq!(calc.best_combo(), 0);
        }

        #[test]
        fn test_reset_combo_only() {
            let mut calc = ScoreCalculator::new();

            for _ in 0..5 {
                calc.calculate(100, 0.5, 0.8, Difficulty::Beginner, 1.0);
            }

            calc.reset_combo();
            assert_eq!(calc.combo_count(), 0);
            assert_eq!(calc.best_combo(), 5);
        }
    }

    mod edge_case_tests {
        use super::*;

        #[test]
        fn test_zero_base_points() {
            let mut calc = ScoreCalculator::new();
            let breakdown = calc.calculate(0, 0.5, 0.8, Difficulty::Beginner, 1.0);

            assert_eq!(breakdown.base_points, 0);
            assert_eq!(breakdown.time_bonus, 0);
            assert_eq!(breakdown.efficiency_bonus, 0);
            assert_eq!(breakdown.combo_bonus, 0);
            assert_eq!(breakdown.total, 0);
        }

        #[test]
        fn test_time_ratio_clamped() {
            let mut calc = ScoreCalculator::new();
            let breakdown = calc.calculate(100, 1.5, 0.5, Difficulty::Beginner, 1.0);

            assert_eq!(breakdown.time_bonus, 0);
        }

        #[test]
        fn test_efficiency_exactly_one() {
            let mut calc = ScoreCalculator::new();
            let breakdown = calc.calculate(100, 0.5, 1.0, Difficulty::Beginner, 1.0);

            assert_eq!(breakdown.efficiency_bonus, 25);
        }

        #[test]
        fn test_efficiency_exactly_point_eight() {
            let mut calc = ScoreCalculator::new();
            let breakdown = calc.calculate(100, 0.5, 0.8, Difficulty::Beginner, 1.0);

            assert_eq!(breakdown.efficiency_bonus, 12);
        }

        #[test]
        fn test_efficiency_just_below_point_eight() {
            let mut calc = ScoreCalculator::new();
            let breakdown = calc.calculate(100, 0.5, 0.79, Difficulty::Beginner, 1.0);

            assert_eq!(breakdown.efficiency_bonus, 0);
        }

        #[test]
        fn test_perfect_scenario() {
            let mut calc = ScoreCalculator::new();

            for _ in 0..5 {
                calc.calculate(100, 0.3, 1.0, Difficulty::Beginner, 1.0);
            }

            let breakdown = calc.calculate(100, 0.0, 1.0, Difficulty::Advanced, 2.0);

            assert_eq!(breakdown.base_points, 100);
            assert_eq!(breakdown.time_bonus, 50);
            assert_eq!(breakdown.efficiency_bonus, 25);
            assert_eq!(breakdown.combo_bonus, 50);
            assert!((breakdown.difficulty_multiplier - 1.5).abs() < 0.001);
            assert_eq!(breakdown.subtotal, 225);
            assert_eq!(breakdown.total, 337);
        }
    }
}
