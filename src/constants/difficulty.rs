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

// Difficulty adjustment thresholds
/// Minimum results needed before adjusting difficulty
pub const MIN_RESULTS_FOR_DIFFICULTY_CHANGE: usize = 5;
/// Success rate threshold for increasing difficulty (90%)
pub const DIFFICULTY_INCREASE_THRESHOLD: f64 = 0.9;
/// Success rate threshold for decreasing difficulty (50%)
pub const DIFFICULTY_DECREASE_THRESHOLD: f64 = 0.5;
