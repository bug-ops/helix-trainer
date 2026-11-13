//! Gamification system for habit formation through daily engagement
//!
//! Implements daily quests, XP/leveling, streaks, and achievements

#![cfg_attr(
    not(test),
    warn(
        clippy::arithmetic_side_effects,
        clippy::indexing_slicing,
        clippy::unwrap_used,
        clippy::expect_used
    )
)]

pub mod achievements;
pub mod challenge_progress;
pub mod profile;
pub mod quests;
pub mod storage;
pub mod streak;

pub use achievements::{Achievement, AchievementEngine, AchievementId, speed_time_ratio};
pub use challenge_progress::{ChallengeDay, ChallengeProgress};
pub use profile::{LevelProgress, UserProfile, XPCalculator, XPComponents};
pub use quests::{
    Quest, QuestDifficulty, QuestGeneration, QuestGenerator, QuestRegistrySlot, QuestStatus,
    QuestTemplateRegistry, QuestTracker, QuestType,
};
pub use storage::{LockStatus, PreviousCopy, ProfileStorage, QuarantineOutcome};
pub use streak::{StreakChange, StreakManager};

use thiserror::Error;

/// Errors raised by gamification state changes and storage.
#[derive(Error, Debug)]
pub enum GamificationError {
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
}

/// Result alias with [`GamificationError`] as the error type.
pub type Result<T> = std::result::Result<T, GamificationError>;
