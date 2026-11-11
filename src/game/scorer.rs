//! Scoring system for training scenarios
//!
//! This module provides the Scorer type for calculating performance metrics
//! and evaluating user solutions against optimal approaches.
//!
//! # Examples
//!
//! ```
//! use helix_trainer::game::Scorer;
//! use helix_trainer::config::ScoringConfig;
//!
//! // Direct score calculation
//! let score = Scorer::calculate_score(5, 5, 0, 100)?;
//! assert_eq!(score, 100); // Perfect solution
//!
//! // Using scenario configuration
//! let config = ScoringConfig {
//!     optimal_count: std::num::NonZeroUsize::new(5).unwrap(),
//!     max_points: 100,
//!     tolerance: 2,
//! };
//! let score = Scorer::score_with_config(&config, 7)?;
//! assert_eq!(score, 100); // Within tolerance
//! # Ok::<(), helix_trainer::security::SecurityError>(())
//! ```

use crate::config::ScoringConfig;
use crate::security::{self, SecurityError};

/// Calculates scores for training scenarios
///
/// Provides scoring algorithms for evaluating user solutions based on:
/// - Optimal action count
/// - Actual actions taken
/// - Tolerance for extra actions
/// - Maximum possible points
///
/// Supports multipliers for alternative solutions and aggregation
/// of multiple scores into totals and averages.
pub struct Scorer;

impl Scorer {
    /// Calculate score based on optimal vs actual action count
    ///
    /// Awards full points if the actual action count is within tolerance
    /// of the optimal count. Beyond tolerance, score is reduced proportionally
    /// based on the ratio of optimal to actual actions.
    ///
    /// # Arguments
    /// * `optimal_count` - Optimal number of actions to solve the scenario
    /// * `actual_count` - Actual number of actions taken by user
    /// * `tolerance` - Number of extra actions allowed without penalty
    /// * `max_points` - Maximum possible points for perfect solution
    ///
    /// # Returns
    /// Score from 0 to `max_points`
    ///
    /// # Errors
    /// Returns `SecurityError::ScoreOverflow` if:
    /// - `optimal_count` is 0
    /// - `actual_count` is 0
    /// - Score calculation would overflow
    ///
    /// # Examples
    /// ```
    /// use helix_trainer::game::Scorer;
    ///
    /// // Perfect solution: 100 points
    /// let score = Scorer::calculate_score(5, 5, 0, 100).unwrap();
    /// assert_eq!(score, 100);
    ///
    /// // Within tolerance: still 100 points
    /// let score = Scorer::calculate_score(5, 7, 2, 100).unwrap();
    /// assert_eq!(score, 100);
    ///
    /// // Beyond tolerance: reduced score
    /// let score = Scorer::calculate_score(5, 10, 0, 100).unwrap();
    /// assert_eq!(score, 50);
    /// ```
    pub fn calculate_score(
        optimal_count: usize,
        actual_count: usize,
        tolerance: usize,
        max_points: u32,
    ) -> Result<u32, SecurityError> {
        // Validate inputs
        if optimal_count == 0 {
            return Err(SecurityError::ScoreOverflow);
        }

        if actual_count == 0 {
            return Err(SecurityError::ScoreOverflow);
        }

        // If within tolerance, award full points
        if actual_count <= optimal_count.saturating_add(tolerance) {
            return Ok(max_points);
        }

        // Otherwise, calculate proportional score using safe arithmetic
        security::arithmetic::checked_score_calculation(optimal_count, actual_count, max_points)
    }

    /// Calculate score using scenario's scoring configuration
    ///
    /// Extracts the scoring parameters from a `ScoringConfig` and applies
    /// the standard scoring algorithm.
    ///
    /// # Errors
    /// Returns `SecurityError` if scoring validation fails
    ///
    /// # Examples
    /// ```
    /// use helix_trainer::game::Scorer;
    /// use helix_trainer::config::ScoringConfig;
    ///
    /// let config = ScoringConfig {
    ///     optimal_count: std::num::NonZeroUsize::new(5).unwrap(),
    ///     max_points: 100,
    ///     tolerance: 2,
    /// };
    ///
    /// let score = Scorer::score_with_config(&config, 7).unwrap();
    /// assert_eq!(score, 100); // Within tolerance
    /// ```
    pub fn score_with_config(
        config: &ScoringConfig,
        actual_count: usize,
    ) -> Result<u32, SecurityError> {
        Self::calculate_score(
            config.optimal_count.get(),
            actual_count,
            config.tolerance,
            config.max_points,
        )
    }

    /// Get performance rating based on score percentage
    ///
    /// Maps a numeric score to a human-readable performance level.
    /// Useful for UI display and user feedback.
    ///
    /// # Arguments
    /// * `score` - Points earned
    /// * `max_points` - Maximum possible points
    ///
    /// # Returns
    /// Performance rating from Perfect to Poor
    ///
    /// # Examples
    /// ```
    /// use helix_trainer::game::{Scorer, PerformanceRating};
    ///
    /// assert_eq!(Scorer::rating(100, 100), PerformanceRating::Perfect);
    /// assert_eq!(Scorer::rating(95, 100), PerformanceRating::Excellent);
    /// assert_eq!(Scorer::rating(75, 100), PerformanceRating::Good);
    /// assert_eq!(Scorer::rating(30, 100), PerformanceRating::Poor);
    /// ```
    pub fn rating(score: u32, max_points: u32) -> PerformanceRating {
        if max_points == 0 {
            return PerformanceRating::Poor;
        }

        let ratio = f64::from(score) / f64::from(max_points);

        match crate::numeric::floor_percent(ratio) {
            100 => PerformanceRating::Perfect,
            80..=99 => PerformanceRating::Excellent,
            60..=79 => PerformanceRating::Good,
            40..=59 => PerformanceRating::Fair,
            _ => PerformanceRating::Poor,
        }
    }

    /// Calculate total session score from multiple scenario scores
    ///
    /// Sums up scores from multiple completed scenarios with overflow protection.
    ///
    /// # Errors
    /// Returns `SecurityError::ScoreOverflow` if sum would overflow u32
    ///
    /// # Examples
    /// ```
    /// use helix_trainer::game::Scorer;
    ///
    /// let scores = vec![100, 85, 90];
    /// let total = Scorer::calculate_total_score(&scores).unwrap();
    /// assert_eq!(total, 275);
    /// ```
    pub fn calculate_total_score(scores: &[u32]) -> Result<u32, SecurityError> {
        let mut total: u32 = 0;

        for &score in scores {
            total = security::arithmetic::checked_score_add(total, score)?;
        }

        Ok(total)
    }
}

/// Performance rating levels
///
/// Categorizes performance into five levels based on percentage of maximum points.
/// Useful for displaying user feedback and progress in the UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PerformanceRating {
    /// 100% - Perfect execution with no suboptimal moves
    Perfect,
    /// 80-99% - Excellent performance with minimal deviations
    Excellent,
    /// 60-79% - Good performance with some room for improvement
    Good,
    /// 40-59% - Fair performance, basic understanding shown
    Fair,
    /// <40% - Needs improvement, significant deviations from optimal
    Poor,
}

impl PerformanceRating {
    /// Get a human-readable description of the rating
    ///
    /// # Examples
    ///
    /// ```
    /// use helix_trainer::game::PerformanceRating;
    ///
    /// assert_eq!(
    ///     PerformanceRating::Perfect.description(),
    ///     "Perfect! Optimal solution."
    /// );
    /// assert_eq!(
    ///     PerformanceRating::Good.description(),
    ///     "Good job!"
    /// );
    /// ```
    pub fn description(&self) -> &'static str {
        match self {
            Self::Perfect => "Perfect! Optimal solution.",
            Self::Excellent => "Excellent work!",
            Self::Good => "Good job!",
            Self::Fair => "Fair attempt.",
            Self::Poor => "Keep practicing!",
        }
    }

    /// Get emoji representation for terminal UI display
    ///
    /// Returns a single emoji character that visually represents
    /// the performance level.
    ///
    /// # Examples
    ///
    /// ```
    /// use helix_trainer::game::PerformanceRating;
    ///
    /// assert_eq!(PerformanceRating::Perfect.emoji(), "⭐");
    /// assert_eq!(PerformanceRating::Good.emoji(), "✨");
    /// assert_eq!(PerformanceRating::Poor.emoji(), "🔸");
    /// ```
    pub fn emoji(&self) -> &'static str {
        match self {
            Self::Perfect => "⭐",
            Self::Excellent => "🌟",
            Self::Good => "✨",
            Self::Fair => "💫",
            Self::Poor => "🔸",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ScoringConfig;

    #[test]
    fn test_perfect_score() {
        let score = Scorer::calculate_score(5, 5, 0, 100).unwrap();
        assert_eq!(score, 100);
    }

    #[test]
    fn test_within_tolerance() {
        let score = Scorer::calculate_score(5, 7, 2, 100).unwrap();
        assert_eq!(score, 100);
    }

    #[test]
    fn test_beyond_tolerance() {
        let score = Scorer::calculate_score(5, 10, 0, 100).unwrap();
        assert_eq!(score, 50);
    }

    #[test]
    fn test_zero_optimal_count() {
        let result = Scorer::calculate_score(0, 5, 0, 100);
        assert!(result.is_err());
    }

    #[test]
    fn test_zero_actual_count() {
        let result = Scorer::calculate_score(5, 0, 0, 100);
        assert!(result.is_err());
    }

    #[test]
    fn test_score_with_config() {
        let config = ScoringConfig {
            optimal_count: std::num::NonZeroUsize::new(5).unwrap(),
            max_points: 100,
            tolerance: 2,
        };

        let score = Scorer::score_with_config(&config, 7).unwrap();
        assert_eq!(score, 100);
    }

    #[test]
    fn test_rating_perfect() {
        assert_eq!(Scorer::rating(100, 100), PerformanceRating::Perfect);
    }

    #[test]
    fn test_rating_excellent() {
        assert_eq!(Scorer::rating(95, 100), PerformanceRating::Excellent);
        // Verify Excellent is reachable with realistic scenario data (optimal <= 7)
        // optimal=7, actual=8 → score = floor(7/8 * 100) = 87 → Excellent
        let score = Scorer::calculate_score(7, 8, 0, 100).unwrap();
        assert_eq!(Scorer::rating(score, 100), PerformanceRating::Excellent);
        // optimal=4, actual=5 → score = floor(4/5 * 100) = 80 → Excellent
        let score = Scorer::calculate_score(4, 5, 0, 100).unwrap();
        assert_eq!(Scorer::rating(score, 100), PerformanceRating::Excellent);
    }

    #[test]
    fn test_rating_good() {
        assert_eq!(Scorer::rating(75, 100), PerformanceRating::Good);
    }

    #[test]
    fn test_rating_fair() {
        assert_eq!(Scorer::rating(50, 100), PerformanceRating::Fair);
    }

    #[test]
    fn test_rating_poor() {
        assert_eq!(Scorer::rating(30, 100), PerformanceRating::Poor);
    }

    #[test]
    fn test_rating_zero_max_points() {
        assert_eq!(Scorer::rating(0, 0), PerformanceRating::Poor);
    }

    #[test]
    fn test_calculate_total_score() {
        let scores = vec![100, 85, 90];
        let total = Scorer::calculate_total_score(&scores).unwrap();
        assert_eq!(total, 275);
    }

    #[test]
    fn test_performance_rating_descriptions() {
        assert_eq!(
            PerformanceRating::Perfect.description(),
            "Perfect! Optimal solution."
        );
        assert_eq!(
            PerformanceRating::Excellent.description(),
            "Excellent work!"
        );
        assert_eq!(PerformanceRating::Good.description(), "Good job!");
        assert_eq!(PerformanceRating::Fair.description(), "Fair attempt.");
        assert_eq!(PerformanceRating::Poor.description(), "Keep practicing!");
    }

    #[test]
    fn test_performance_rating_emojis() {
        assert_eq!(PerformanceRating::Perfect.emoji(), "⭐");
        assert_eq!(PerformanceRating::Excellent.emoji(), "🌟");
        assert_eq!(PerformanceRating::Good.emoji(), "✨");
        assert_eq!(PerformanceRating::Fair.emoji(), "💫");
        assert_eq!(PerformanceRating::Poor.emoji(), "🔸");
    }

    #[test]
    fn test_saturating_tolerance() {
        // Ensure tolerance addition doesn't overflow
        let score = Scorer::calculate_score(5, 10, usize::MAX, 100).unwrap();
        assert_eq!(score, 100); // Within saturating tolerance
    }

    #[test]
    fn test_total_score_overflow_prevention() {
        // Test that overflow is properly prevented
        let scores = vec![u32::MAX, u32::MAX];
        let result = Scorer::calculate_total_score(&scores);
        assert!(result.is_err());
    }

    #[test]
    fn test_actual_less_than_optimal() {
        // When actual < optimal, score should be capped at max_points
        let score = Scorer::calculate_score(10, 5, 0, 100).unwrap();
        assert_eq!(score, 100);
    }

    #[test]
    fn test_score_with_different_max_points() {
        let score = Scorer::calculate_score(5, 10, 0, 50).unwrap();
        assert_eq!(score, 25);

        let score = Scorer::calculate_score(5, 10, 0, 200).unwrap();
        assert_eq!(score, 100);
    }
}
