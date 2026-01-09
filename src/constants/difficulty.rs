//! Difficulty scaling constants
//!
//! Time limits, thresholds, and scaling factors for difficulty system.

use std::time::Duration;

// Time limits by difficulty
/// Time limit for beginner difficulty (10 seconds)
pub const BEGINNER_TIME_LIMIT: Duration = Duration::from_secs(10);
/// Time limit for intermediate difficulty (8 seconds)
pub const INTERMEDIATE_TIME_LIMIT: Duration = Duration::from_secs(8);
/// Time limit for advanced difficulty (6 seconds)
pub const ADVANCED_TIME_LIMIT: Duration = Duration::from_secs(6);
/// Fallback time limit when difficulty unknown (8 seconds)
pub const FALLBACK_TIME_LIMIT: Duration = Duration::from_secs(8);

// Time scaling by level
/// Time scale for levels 1-3 (100%)
pub const LEVEL_1_3_TIME_SCALE: f64 = 1.0;
/// Time scale for levels 4-6 (90%)
pub const LEVEL_4_6_TIME_SCALE: f64 = 0.9;
/// Time scale for levels 7-10 (80%)
pub const LEVEL_7_10_TIME_SCALE: f64 = 0.8;
/// Minimum time scale multiplier (50%)
pub const MIN_TIME_SCALE_MULTIPLIER: f64 = 0.5;

// Difficulty level ranges
/// Beginner level range minimum
pub const LEVEL_BEGINNER_MIN: u8 = 1;
/// Beginner level range maximum
pub const LEVEL_BEGINNER_MAX: u8 = 3;
/// Intermediate level range minimum
pub const LEVEL_INTERMEDIATE_MIN: u8 = 4;
/// Intermediate level range maximum
pub const LEVEL_INTERMEDIATE_MAX: u8 = 6;
/// Advanced level range minimum
pub const LEVEL_ADVANCED_MIN: u8 = 7;
/// Advanced level range maximum
pub const LEVEL_ADVANCED_MAX: u8 = 10;

// Performance scoring weights
/// Weight for success component in performance calculation (60%)
pub const PERFORMANCE_SUCCESS_WEIGHT: f64 = 0.6;
/// Weight for speed component in performance calculation (25%)
pub const PERFORMANCE_SPEED_WEIGHT: f64 = 0.25;
/// Weight for efficiency component in performance calculation (15%)
pub const PERFORMANCE_EFFICIENCY_WEIGHT: f64 = 0.15;

// Difficulty adjustment thresholds
/// Performance score threshold for increasing difficulty (85%)
pub const DIFFICULTY_INCREASE_SCORE: f64 = 0.85;
/// Performance score threshold for decreasing difficulty (40%)
pub const DIFFICULTY_DECREASE_SCORE: f64 = 0.4;
/// Minimum scenarios completed at current level before allowing increase
pub const MIN_SCENARIOS_FOR_INCREASE: u32 = 5;
/// Minimum scenarios completed at current level before allowing decrease
pub const MIN_SCENARIOS_FOR_DECREASE: u32 = 3;
/// Recent success rate threshold for difficulty increase (80%)
pub const RECENT_SUCCESS_RATE_FOR_INCREASE: f64 = 0.8;
/// Recent success rate threshold for difficulty decrease (40%)
pub const RECENT_SUCCESS_RATE_FOR_DECREASE: f64 = 0.4;
/// Number of recent results to check for trend analysis
pub const RECENT_TREND_WINDOW: usize = 5;

// Performance history window
/// Maximum number of performance points to track
pub const PERFORMANCE_HISTORY_SIZE: usize = 15;

// Difficulty weights for performance scoring
/// Weight multiplier for beginner difficulty scenarios
pub const BEGINNER_DIFFICULTY_WEIGHT: f64 = 0.8;
/// Weight multiplier for intermediate difficulty scenarios
pub const INTERMEDIATE_DIFFICULTY_WEIGHT: f64 = 1.0;
/// Weight multiplier for advanced difficulty scenarios
pub const ADVANCED_DIFFICULTY_WEIGHT: f64 = 1.2;

// Recency weighting
/// Base weight for recency calculation
pub const RECENCY_BASE_WEIGHT: f64 = 1.0;
/// Increment per position for recency weighting (more recent = higher weight)
pub const RECENCY_WEIGHT_INCREMENT: f64 = 0.1;

// Default performance score when no history exists
/// Neutral starting performance score (50%)
pub const DEFAULT_PERFORMANCE_SCORE: f64 = 0.5;
