//! Gameplay-related constants
//!
//! Streaks, milestones, achievements, and other game mechanics.

use std::time::Duration;

// Mini-game scoring
/// Score milestone for earning an extra life
pub const EXTRA_LIFE_SCORE_MILESTONE: u64 = 5000;

/// Maximum time bonus multiplier (50% of base points)
pub const MAX_TIME_BONUS_MULTIPLIER: f64 = 0.5;

// Combo system
/// Bonus per combo level (10%)
pub const COMBO_BONUS_PER_LEVEL: f64 = 0.1;
/// Maximum combo bonus (50% at combo 5+)
pub const MAX_COMBO_BONUS: f64 = 0.5;

// Difficulty multipliers for scoring
/// Difficulty multiplier for Beginner scenarios
pub const DIFFICULTY_MULTIPLIER_BEGINNER: f64 = 1.0;
/// Difficulty multiplier for Intermediate scenarios
pub const DIFFICULTY_MULTIPLIER_INTERMEDIATE: f64 = 1.25;
/// Difficulty multiplier for Advanced scenarios
pub const DIFFICULTY_MULTIPLIER_ADVANCED: f64 = 1.5;

// Efficiency bonuses
/// Efficiency bonus for optimal action count (25%)
pub const OPTIMAL_EFFICIENCY_BONUS: f64 = 0.25;
/// Efficiency bonus for >80% efficiency (12.5%)
pub const MAX_EFFICIENCY_BONUS: f64 = 0.125;

// Streak milestones (in days)
/// Week streak milestone (7 days)
pub const STREAK_MILESTONE_WEEK: u32 = 7;
/// Month streak milestone (30 days)
pub const STREAK_MILESTONE_MONTH: u32 = 30;
/// Quarter streak milestone (90 days)
pub const STREAK_MILESTONE_QUARTER: u32 = 90;
/// Year streak milestone (365 days)
pub const STREAK_MILESTONE_YEAR: u32 = 365;

// Milestone XP bonuses
/// XP bonus for 7-day streak milestone
pub const MILESTONE_7_DAY_XP: u64 = 50;
/// XP bonus for 30-day streak milestone
pub const MILESTONE_30_DAY_XP: u64 = 200;
/// XP bonus for 90-day streak milestone
pub const MILESTONE_90_DAY_XP: u64 = 500;
/// XP bonus for 365-day streak milestone
pub const MILESTONE_365_DAY_XP: u64 = 2000;

// Challenge mode constants
/// Number of scenarios in daily challenge
pub const CHALLENGE_SCENARIO_COUNT: usize = 10;
/// Maximum attempts per day for challenge mode
pub const CHALLENGE_MAX_ATTEMPTS: u8 = 3;
/// Lives per challenge attempt
pub const CHALLENGE_LIVES_PER_ATTEMPT: u8 = 3;
/// Time limit per scenario in challenge mode (8 seconds)
pub const CHALLENGE_TIME_PER_SCENARIO: Duration = Duration::from_secs(8);

// Survival mode constants
/// Base time per scenario in survival mode (10 seconds)
pub const SURVIVAL_BASE_TIME: Duration = Duration::from_secs(10);
/// Minimum time per scenario in survival mode (3 seconds)
pub const SURVIVAL_MIN_TIME: Duration = Duration::from_secs(3);
/// Time decrease per level in survival mode (500ms)
pub const SURVIVAL_TIME_DECREASE_PER_LEVEL: Duration = Duration::from_millis(500);
