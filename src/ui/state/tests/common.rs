//! Common test utilities for UI state tests

use crate::config::Scenario;
use crate::input::keymap::CanonicalKeys;
use crate::testing::{ScenarioBuilder, test_app_state_with_scenarios};
use crate::ui::{AppState, Message};

/// Build an `ExecuteCommand` message for a single physical key with no
/// keymap remap active - `keys` and `typed` are identical, matching every
/// keystroke these tests simulate.
pub fn exec(key: &'static str) -> Message {
    Message::ExecuteCommand {
        keys: CanonicalKeys::from_static(key),
        typed: std::borrow::Cow::Borrowed(key),
    }
}

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
