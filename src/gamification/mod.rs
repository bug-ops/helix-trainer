//! Gamification system for habit formation through daily engagement
//!
//! Implements daily quests, XP/leveling, streaks, and achievements

pub mod achievements;
pub mod profile;
pub mod quests;
pub mod storage;
pub mod streak;

pub use achievements::{Achievement, AchievementEngine, AchievementId, speed_time_ratio};
pub use profile::{UserProfile, XPCalculator, XPComponents};
pub use quests::{
    Quest, QuestDifficulty, QuestGeneration, QuestGenerator, QuestRegistrySlot,
    QuestTemplateRegistry, QuestTracker, QuestType,
};
pub use storage::{LockStatus, ProfileStorage, QuarantineError, QuarantineOutcome};
pub use streak::{StreakChange, StreakManager};

use thiserror::Error;

/// Errors raised by gamification state changes and storage.
#[derive(Error, Debug)]
pub enum GamificationError {
    /// No quest matches the requested id.
    #[error("Quest not found: {0}")]
    QuestNotFound(String),

    /// A level outside the valid range was requested.
    #[error("Invalid level: {0}")]
    InvalidLevel(u32),

    /// A freeze was requested but none is held.
    #[error("Streak freeze not available")]
    StreakFreezeUnavailable,

    /// A freeze was requested while there is no streak to protect.
    #[error("No active streak to protect with a freeze")]
    StreakFreezeNothingToProtect,

    /// The inactivity gap is outside what a freeze can cover.
    #[error(
        "Streak freeze cannot cover a gap of {days_since_active} days (coverable range is 2..={max_gap_days})"
    )]
    StreakFreezeGapOutOfRange {
        /// Days since the last practice day.
        days_since_active: i64,
        /// Largest gap a freeze can cover.
        max_gap_days: i64,
    },

    /// The achievement was unlocked earlier and cannot be unlocked twice.
    #[error("Achievement already unlocked: {0:?}")]
    AchievementAlreadyUnlocked(AchievementId),
}

/// Result alias with [`GamificationError`] as the error type.
pub type Result<T> = std::result::Result<T, GamificationError>;
