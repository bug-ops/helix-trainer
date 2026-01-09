//! Test utilities module
//!
//! This module is only compiled in test builds and provides shared utilities
//! for creating test fixtures across the codebase.
//!
//! # Example
//!
//! ```rust,no_run
//! use helix_trainer::testing::{ScenarioBuilder, default_test_scenario, empty_test_app_state};
//! use helix_trainer::config::Difficulty;
//!
//! // Create a simple test scenario
//! let scenario = default_test_scenario();
//!
//! // Create a scenario with custom configuration
//! let scenario = ScenarioBuilder::new()
//!     .id("my_test")
//!     .difficulty(Difficulty::Beginner)
//!     .build();
//!
//! // Create an empty app state for testing
//! let state = empty_test_app_state();
//! ```

pub mod app_state;
pub mod scenario;

// Re-export commonly used items
pub use app_state::{TestAppStateBuilder, empty_test_app_state, test_app_state_with_scenarios};
pub use scenario::{
    ScenarioBuilder, default_test_scenario, test_scenario_with_difficulty, test_scenario_with_id,
};
