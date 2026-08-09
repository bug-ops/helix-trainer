//! Gamification system for habit formation through daily engagement
//!
//! Implements daily quests, XP/leveling, streaks, and achievements

pub mod achievements;
pub mod profile;
pub mod quests;
pub mod storage;
pub mod streak;

pub use achievements::{Achievement, AchievementEngine, AchievementId, speed_time_ratio};
pub use profile::{UserProfile, XPCalculator};
pub use quests::{
    Quest, QuestDifficulty, QuestGenerator, QuestTemplateRegistry, QuestTracker, QuestType,
};
pub use storage::ProfileStorage;
pub use streak::{StreakChange, StreakManager};

use thiserror::Error;

#[derive(Error, Debug)]
pub enum GamificationError {
    #[error("Quest not found: {0}")]
    QuestNotFound(String),

    #[error("Storage error: {0}")]
    StorageError(String),

    #[error("Invalid level: {0}")]
    InvalidLevel(u32),

    #[error("Streak freeze not available")]
    StreakFreezeUnavailable,

    #[error("Achievement already unlocked: {0:?}")]
    AchievementAlreadyUnlocked(AchievementId),
}

pub type Result<T> = std::result::Result<T, GamificationError>;
