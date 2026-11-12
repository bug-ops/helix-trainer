//! Typed reasons a quest template fails validation

use thiserror::Error;

/// Why a quest template was rejected by `QuestLoader`.
///
/// Wrapped in [`SecurityError::QuestTemplate`](crate::security::SecurityError::QuestTemplate);
/// the text is for logs, the user only sees the sanitized
/// [`UserError`](crate::security::UserError).
///
/// # Examples
///
/// ```
/// use helix_trainer::config::quests::QuestTemplateError;
///
/// let err = QuestTemplateError::XpRewardTooHigh { max: 50, actual: 60 };
/// assert_eq!(err.to_string(), "custom XP reward exceeds the maximum (max 50, got 60)");
/// ```
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum QuestTemplateError {
    /// The custom XP reward exceeds the cap.
    #[error("custom XP reward exceeds the maximum (max {max}, got {actual})")]
    XpRewardTooHigh {
        /// Maximum allowed reward.
        max: u32,
        /// Declared reward.
        actual: u32,
    },

    /// More required commands than allowed.
    #[error("too many required commands (max {max}, got {actual})")]
    TooManyRequiredCommands {
        /// Maximum allowed count.
        max: usize,
        /// Observed count.
        actual: usize,
    },

    /// More required scenarios than allowed.
    #[error("too many required scenarios (max {max}, got {actual})")]
    TooManyRequiredScenarios {
        /// Maximum allowed count.
        max: usize,
        /// Observed count.
        actual: usize,
    },

    /// `min_level` is above `max_level`.
    #[error("min_level {min} is greater than max_level {max}")]
    LevelRangeInverted {
        /// Declared lower bound.
        min: u32,
        /// Declared upper bound.
        max: u32,
    },

    /// The command name of a command-practice quest is too long.
    #[error("command name too long (max {max}, got {actual})")]
    CommandNameTooLong {
        /// Maximum allowed length in bytes.
        max: usize,
        /// Observed length in bytes.
        actual: usize,
    },

    /// The command-practice target is zero or above the cap.
    #[error("invalid command practice target {actual} (expected 1..={max})")]
    InvalidCommandPracticeTarget {
        /// Largest allowed target.
        max: u32,
        /// Declared target.
        actual: u32,
    },

    /// The scenario-completion target is zero or above the cap.
    #[error("invalid scenario completion target {actual} (expected 1..={max})")]
    InvalidScenarioCompletionTarget {
        /// Largest allowed target.
        max: u32,
        /// Declared target.
        actual: u32,
    },

    /// The speed-run time limit is zero or above the cap.
    #[error("invalid speed run time limit {actual}s (expected 1..={max})")]
    InvalidSpeedRunTimeLimit {
        /// Largest allowed limit in seconds.
        max: u64,
        /// Declared limit in seconds.
        actual: u64,
    },

    /// The time-invested target is zero or above the cap.
    #[error("invalid time invested target {actual} minutes (expected 1..={max})")]
    InvalidTimeInvestedTarget {
        /// Largest allowed target in minutes.
        max: u32,
        /// Declared target in minutes.
        actual: u32,
    },

    /// The exploration target is zero or above the cap.
    #[error("invalid exploration target {actual} (expected 1..={max})")]
    InvalidExplorationTarget {
        /// Largest allowed target.
        max: u32,
        /// Declared target.
        actual: u32,
    },
}
