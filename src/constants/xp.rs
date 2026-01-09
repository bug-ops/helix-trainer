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
