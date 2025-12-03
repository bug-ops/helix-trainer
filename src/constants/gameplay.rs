//! Gameplay-related constants
//!
//! Streaks, milestones, achievements, and other game mechanics.

// Streak system
/// Number of quests completed to earn a streak freeze
pub const QUESTS_FOR_STREAK_FREEZE: u32 = 5;

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
