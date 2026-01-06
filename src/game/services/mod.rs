//! Game service layer - orchestrates domain operations
//!
//! Separates business logic from UI handlers, providing:
//! - Scenario completion with XP calculation and mastery tracking
//! - Quest progress tracking across training and arcade modes

mod scenario_completion;

pub use scenario_completion::{CompletionResult, ScenarioCompletionService, XPComponents};
