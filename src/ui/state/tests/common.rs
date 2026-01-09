//! Common test utilities for UI state tests

use crate::config::Scenario;
use crate::testing::{ScenarioBuilder, test_app_state_with_scenarios};
use crate::ui::AppState;

/// Create a test scenario with default content
pub fn create_test_scenario() -> Scenario {
    ScenarioBuilder::new()
        .id("test_001")
        .description("A test scenario for UI testing")
        .hint("Use x to select line, then d to delete")
        .build()
}

/// Create a test app state with the given scenarios
pub fn create_test_app_state(scenarios: Vec<Scenario>) -> AppState {
    test_app_state_with_scenarios(scenarios)
}
