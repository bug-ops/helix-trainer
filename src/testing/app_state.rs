//! Test AppState builder for creating test fixtures
//!
//! Provides utilities for creating `AppState` instances in tests.

use crate::config::Scenario;
use crate::gamification::{ProfileStorage, UserProfile};
use crate::learning::PerformanceTracker;
use crate::ui::state::AppState;

/// Create a test AppState with no scenarios
///
/// Uses default profile, storage, and tracker.
pub fn empty_test_app_state() -> AppState {
    test_app_state_with_scenarios(vec![])
}

/// Create a test AppState with the provided scenarios
///
/// Uses default profile, storage, and tracker.
pub fn test_app_state_with_scenarios(scenarios: Vec<Scenario>) -> AppState {
    let profile = UserProfile::new();
    let storage = ProfileStorage::for_test();
    let tracker = PerformanceTracker::new();
    AppState::new(scenarios, profile, storage, tracker)
}

/// Builder for creating test AppState with custom configuration
pub struct TestAppStateBuilder {
    scenarios: Vec<Scenario>,
    profile: Option<UserProfile>,
    storage: Option<ProfileStorage>,
    tracker: Option<PerformanceTracker>,
}

impl Default for TestAppStateBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl TestAppStateBuilder {
    /// Create a new builder with empty defaults
    pub fn new() -> Self {
        Self {
            scenarios: vec![],
            profile: None,
            storage: None,
            tracker: None,
        }
    }

    /// Add a scenario
    pub fn scenario(mut self, scenario: Scenario) -> Self {
        self.scenarios.push(scenario);
        self
    }

    /// Set all scenarios
    pub fn scenarios(mut self, scenarios: Vec<Scenario>) -> Self {
        self.scenarios = scenarios;
        self
    }

    /// Set a custom user profile
    pub fn profile(mut self, profile: UserProfile) -> Self {
        self.profile = Some(profile);
        self
    }

    /// Set a custom profile storage
    pub fn storage(mut self, storage: ProfileStorage) -> Self {
        self.storage = Some(storage);
        self
    }

    /// Set a custom performance tracker
    pub fn tracker(mut self, tracker: PerformanceTracker) -> Self {
        self.tracker = Some(tracker);
        self
    }

    /// Build the AppState
    pub fn build(self) -> AppState {
        let profile = self.profile.unwrap_or_default();
        let storage = self.storage.unwrap_or_else(ProfileStorage::for_test);
        let tracker = self.tracker.unwrap_or_default();
        AppState::new(self.scenarios, profile, storage, tracker)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::scenario::default_test_scenario;

    #[test]
    fn test_empty_state() {
        let state = empty_test_app_state();
        assert_eq!(state.game.scenario_collection.count(), 0);
    }

    #[test]
    fn test_state_with_scenarios() {
        let scenarios = vec![default_test_scenario()];
        let state = test_app_state_with_scenarios(scenarios);
        assert_eq!(state.game.scenario_collection.count(), 1);
    }

    #[test]
    fn test_builder() {
        let state = TestAppStateBuilder::new()
            .scenario(default_test_scenario())
            .scenario(default_test_scenario())
            .build();
        assert_eq!(state.game.scenario_collection.count(), 2);
    }
}
