//! XP (Experience Points) calculation constants
//!
//! All values related to XP rewards, leveling formulas, and progression.

// Leveling formula
/// Base coefficient for XP leveling formula
pub const XP_LEVEL_FORMULA_BASE: f64 = 100.0;
/// Power exponent for XP leveling formula
pub const XP_LEVEL_FORMULA_EXPONENT: f64 = 1.5;
/// Maximum player level for calculations
pub const MAX_PLAYER_LEVEL: u32 = 100;

// Scenario XP rewards
/// Base XP per 100 points for regular scenarios
pub const SCENARIO_BASE_XP_PER_100_POINTS: u64 = 50;
/// Divisor of the base XP that yields the perfect-score bonus (`base / 5`)
pub const SCENARIO_PERFECT_BONUS_DIVISOR: u64 = 5;
/// Flat bonus XP for the first scenario completed in a day
pub const SCENARIO_FIRST_TODAY_BONUS_XP: u64 = 10;

// Review session XP rewards
/// XP per command completed in a review session
pub const REVIEW_XP_PER_COMMAND: u64 = 10;
/// XP a review session earns at a 100% success rate
pub const REVIEW_SUCCESS_BONUS_MAX_XP: f64 = 20.0;

// Mini-game XP rewards
/// Base XP per 100 points in mini-game
pub const MINIGAME_XP_PER_100_POINTS: u64 = 100;
/// Bonus XP per difficulty level in mini-game
pub const MINIGAME_LEVEL_BONUS_XP: u64 = 10;
/// Divisor for streak bonus calculation in mini-game
pub const MINIGAME_STREAK_BONUS_DIVISOR: u64 = 5;
/// Maximum streak bonus XP in mini-game
pub const MINIGAME_STREAK_BONUS_MAX_XP: u64 = 15;
/// Base scenario XP in arcade mode
pub const MINIGAME_SCENARIO_BASE_XP: u64 = 15;
/// XP multiplier per streak level in arcade mode
pub const MINIGAME_STREAK_XP_MULTIPLIER: u64 = 2;

// Scenario XP multipliers
/// XP multiplier for a scenario still being learned
pub const MASTERY_FACTOR_LEARNING: f64 = 1.0;
/// XP multiplier for a proficient scenario
pub const MASTERY_FACTOR_PROFICIENT: f64 = 0.5;
/// XP multiplier for a mastered scenario
pub const MASTERY_FACTOR_MASTERED: f64 = 0.2;
/// XP multiplier for the first attempt of a scenario today
pub const REPEAT_PENALTY_NONE: f64 = 1.0;
/// XP multiplier for the second and third attempt of a scenario today
pub const REPEAT_PENALTY_LIGHT: f64 = 0.7;
/// XP multiplier for the fourth and later attempts of a scenario today
pub const REPEAT_PENALTY_HEAVY: f64 = 0.3;
