//! Spaced repetition learning system
//!
//! Implements FSRS algorithm for optimal review scheduling

#![cfg_attr(
    not(test),
    warn(
        clippy::arithmetic_side_effects,
        clippy::indexing_slicing,
        clippy::unwrap_used,
        clippy::expect_used
    )
)]

/// Aggregate learning statistics derived from the tracker.
pub mod analytics;
/// Per-command FSRS state and the tracker that updates it.
pub mod performance;
pub mod scenario_history;
/// Shared traits for progress tracking and tiered ranks.
pub mod scheduler;
/// Single urgency authority combining overdue ratio, difficulty and weakness.
pub mod session;
pub mod traits;
pub mod urgency;

pub use analytics::{Analytics, MasterySummary};
pub use performance::{
    AttemptOutcome, CardState, CommandPerformance, MasteryChange, MasteryLevel, PerformanceData,
    PerformanceTracker, Rating, ReferenceTime,
};
pub use scenario_history::{MasteryStats, ScenarioCompletion, ScenarioHistory, ScenarioMastery};
pub use scheduler::{ReviewItem, Scheduler};
pub use session::{ReviewOutcome, ReviewResult, ReviewSession, ReviewStep, SessionSummary};
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

    /// No performance record exists for the command.
    #[error("Command not found: {0}")]
    CommandNotFound(String),

    /// The FSRS scheduler rejected an operation; the `fsrs` error is the source.
    #[error("FSRS scheduler rejected the update")]
    FsrsError(#[source] fsrs::FSRSError),
}

/// Result alias with [`LearningError`] as the error type.
pub type Result<T> = std::result::Result<T, LearningError>;
