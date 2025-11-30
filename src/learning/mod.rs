//! Spaced repetition learning system
//!
//! Implements FSRS algorithm for optimal review scheduling

pub mod analytics;
pub mod performance;
pub mod scenario_history;
pub mod scheduler;
pub mod session;
pub mod traits;

pub use analytics::{Analytics, MasterySummary};
pub use performance::{CardState, CommandPerformance, MasteryLevel, PerformanceTracker};
pub use scenario_history::{MasteryStats, ScenarioCompletion, ScenarioHistory, ScenarioMastery};
pub use scheduler::{ReviewItem, Scheduler};
pub use session::{ReviewResult, ReviewSession, SessionSummary};
pub use traits::{Modifier, ProgressTracker, ProgressionTier};

use thiserror::Error;

#[derive(Error, Debug)]
pub enum LearningError {
    #[error("Invalid stability: {0} (must be >= 0)")]
    InvalidStability(f64),

    #[error("Invalid difficulty: {0} (must be between 0 and 10)")]
    InvalidDifficulty(f64),

    #[error("Command not found: {0}")]
    CommandNotFound(String),

    #[error("FSRS error: {0}")]
    FsrsError(String),
}

pub type Result<T> = std::result::Result<T, LearningError>;
