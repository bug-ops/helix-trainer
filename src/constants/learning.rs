//! FSRS (Free Spaced Repetition Scheduler) constants
//!
//! Scoring weights and thresholds for FSRS-based scenario selection in Arcade mode.
//! These constants control how the system prioritizes scenarios based on the
//! learner's command mastery tracked by the FSRS algorithm.

// Scenario scoring weights
// These weights determine how different factors contribute to scenario priority.
// Sum of OVERDUE + WEAKNESS should equal 1.0 minus NOVELTY for balanced scoring.

/// Weight for overdue commands in scenario scoring (40%)
///
/// Higher values increase priority for commands past their scheduled review date.
/// Commands overdue by `FSRS_MAX_OVERDUE_DAYS` receive full weight.
pub const FSRS_OVERDUE_WEIGHT: f64 = 0.4;

/// Weight for weak commands (low success rate) in scenario scoring (40%)
///
/// Higher values increase priority for commands with poor historical performance.
/// Weight is scaled by `(1.0 - success_rate)`, so 0% success = full weight.
pub const FSRS_WEAKNESS_WEIGHT: f64 = 0.4;

/// Weight for novel (never practiced) commands (20%)
///
/// Lower than overdue/weakness to prioritize review of known commands
/// over introduction of new material, following spaced repetition principles.
pub const FSRS_NOVELTY_WEIGHT: f64 = 0.2;

// Selection parameters

/// Base weight ensuring all scenarios remain selectable (10%)
///
/// Added to every scenario's FSRS score to prevent mastered scenarios
/// from having zero selection probability. Ensures variety in practice.
pub const FSRS_BASE_WEIGHT: f64 = 0.1;

/// Days overdue for maximum urgency score (7 days)
///
/// Commands overdue by this many days or more receive the maximum
/// overdue factor of 1.0. Earlier overdue commands are scaled linearly.
pub const FSRS_MAX_OVERDUE_DAYS: f64 = 7.0;
