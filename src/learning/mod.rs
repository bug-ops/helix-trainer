//! Spaced repetition learning system
//!
//! Implements FSRS algorithm for optimal review scheduling

/// Aggregate learning statistics derived from the tracker.
pub mod analytics;
/// Per-command FSRS state and the tracker that updates it.
pub mod performance;
pub mod reviewable;
pub mod scenario_history;
/// Shared traits for progress tracking and tiered ranks.
pub mod scheduler;
/// Single urgency authority combining overdue ratio, difficulty and weakness.
pub mod session;
pub mod traits;
pub mod urgency;

pub use analytics::{Analytics, MasterySummary};
pub use performance::{
    CardState, CommandPerformance, MasteryLevel, PerformanceData, PerformanceTracker,
};
pub use reviewable::ReviewableCommand;
pub use scenario_history::{
    MasteryStats, ScenarioCompletion, ScenarioHistory, ScenarioMastery, is_valid_scenario_id,
};
pub use scheduler::{ReviewItem, Scheduler};
pub use session::{ReviewResult, ReviewSession, SessionSummary};
pub use traits::{Modifier, ProgressTracker, ProgressionTier};
pub use urgency::{Urgency, command_urgency};

use thiserror::Error;

/// Errors raised by the learning subsystem.
#[derive(Error, Debug)]
pub enum LearningError {
    /// Stability is negative.
    #[error("Invalid stability: {0} (must be >= 0)")]
    InvalidStability(f64),

    /// Difficulty is outside the FSRS range of 0 to 10.
    #[error("Invalid difficulty: {0} (must be between 0 and 10)")]
    InvalidDifficulty(f64),

    /// Persisted successes exceed attempts, so the record is corrupt.
    #[error("Invalid counters: {successes} successes exceed {attempts} attempts")]
    InvalidCounters {
        /// Recorded attempts.
        attempts: u32,
        /// Recorded successes.
        successes: u32,
    },

    /// The review session has no more items.
    #[error("Review session is already complete")]
    SessionComplete,

    /// No performance record exists for the command.
    #[error("Command not found: {0}")]
    CommandNotFound(String),

    /// The FSRS scheduler rejected an operation.
    #[error("FSRS error: {0}")]
    FsrsError(String),
}

/// Result alias with [`LearningError`] as the error type.
pub type Result<T> = std::result::Result<T, LearningError>;
