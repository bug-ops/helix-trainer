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

// Quest XP rewards by difficulty
/// Default XP reward for easy quests
pub const QUEST_DEFAULT_XP_EASY: u64 = 25;
/// Default XP reward for medium quests
pub const QUEST_DEFAULT_XP_MEDIUM: u64 = 50;
/// Default XP reward for hard quests
pub const QUEST_DEFAULT_XP_HARD: u64 = 100;

// Quest type XP rewards (easy/medium/hard)
/// XP for easy scenario completion quest
pub const QUEST_XP_SCENARIO_EASY: u64 = 25;
/// XP for medium scenario completion quest
pub const QUEST_XP_SCENARIO_MEDIUM: u64 = 50;
/// XP for hard scenario completion quest
pub const QUEST_XP_SCENARIO_HARD: u64 = 100;

/// XP for easy command practice quest
pub const QUEST_XP_COMMAND_EASY: u64 = 30;
/// XP for medium command practice quest
pub const QUEST_XP_COMMAND_MEDIUM: u64 = 75;
/// XP for hard command practice quest
pub const QUEST_XP_COMMAND_HARD: u64 = 150;

/// XP for easy exploration quest
pub const QUEST_XP_EXPLORATION_EASY: u64 = 40;
/// XP for medium exploration quest
pub const QUEST_XP_EXPLORATION_MEDIUM: u64 = 80;
/// XP for hard exploration quest
pub const QUEST_XP_EXPLORATION_HARD: u64 = 160;

/// XP for speed run quest (medium only)
pub const QUEST_XP_SPEED_RUN: u64 = 100;

/// XP for time invested quest (hard)
pub const QUEST_XP_TIME_INVESTED: u64 = 200;
