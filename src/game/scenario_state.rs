//! Scenario state initialization helpers
//!
//! Provides shared functionality for initializing scenario states,
//! reducing code duplication between training and arcade modes.

use crate::config::Scenario;
use crate::helix::{AnyModeSimulator, EditorSnapshot};
use crate::security::UserError;

/// Initialized scenario state ready for gameplay
///
/// Contains the Helix simulator (source of truth) and target snapshot
/// for efficient completion checking. All state queries go through
/// the simulator's EditorDisplay facade.
pub struct ScenarioState {
    /// Helix simulator for command execution (source of truth)
    pub simulator: AnyModeSimulator,
    /// Target as snapshot for efficient completion checking
    pub target_snapshot: EditorSnapshot,
}

impl ScenarioState {
    /// Initialize scenario state from a scenario configuration
    ///
    /// Creates the Helix simulator from scenario setup and target snapshot
    /// for completion checking.
    ///
    /// # Arguments
    /// * `scenario` - The scenario configuration to initialize from
    ///
    /// # Returns
    /// * `Ok(ScenarioState)` - Initialized state ready for gameplay
    /// * `Err(UserError::ScenarioTooComplex)` - If state creation fails
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use helix_trainer::game::ScenarioState;
    ///
    /// let state = ScenarioState::from_scenario(&scenario)?;
    /// assert!(!state.is_completed());
    /// ```
    pub fn from_scenario(scenario: &Scenario) -> Result<Self, UserError> {
        // Create initial snapshot from scenario setup
        let initial_snapshot = EditorSnapshot::from_scenario_config(
            scenario.setup.file_content.clone(),
            scenario.setup.cursor_position,
            scenario.setup.selection,
            scenario.setup.cursors.as_deref(),
            scenario.setup.selections.as_deref(),
        );

        // Create target snapshot for completion checking
        let target_snapshot = EditorSnapshot::from_scenario_config(
            scenario.target.file_content.clone(),
            scenario.target.cursor_position,
            scenario.target.selection,
            scenario.target.cursors.as_deref(),
            scenario.target.selections.as_deref(),
        );

        // Initialize Helix simulator from initial snapshot
        let simulator = AnyModeSimulator::from_snapshot(&initial_snapshot);

        Ok(Self {
            simulator,
            target_snapshot,
        })
    }

    /// Check if current state matches target state
    ///
    /// Uses `HelixSimulator::matches_snapshot()` for efficient comparison
    /// directly against helix-core primitives.
    #[inline]
    pub fn is_completed(&self) -> bool {
        self.simulator.matches_snapshot(&self.target_snapshot)
    }

    /// Check if content matches target (ignoring cursor position)
    #[inline]
    pub fn content_matches(&self) -> bool {
        let current_snapshot = self.simulator.to_snapshot();
        current_snapshot.content_matches(&self.target_snapshot)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::ScenarioBuilder;

    fn create_test_scenario() -> Scenario {
        ScenarioBuilder::new()
            .id("test")
            .name("Test")
            .description("Test scenario")
            .setup_content("hello")
            .target_content("world")
            .commands(vec!["ciw", "world"])
            .command_description("Change word")
            .optimal_count(1)
            .build()
    }

    #[test]
    fn test_from_scenario_creates_states() {
        let scenario = create_test_scenario();
        let state = ScenarioState::from_scenario(&scenario).unwrap();

        // Verify simulator has initial content
        assert_eq!(state.simulator.display().content(), "hello");
        // Verify target snapshot has target content
        assert_eq!(state.target_snapshot.content, "world");
    }

    #[test]
    fn test_is_completed_false_initially() {
        let scenario = create_test_scenario();
        let state = ScenarioState::from_scenario(&scenario).unwrap();

        assert!(!state.is_completed());
    }

    #[test]
    fn test_content_matches() {
        let scenario = create_test_scenario();
        let state = ScenarioState::from_scenario(&scenario).unwrap();

        assert!(!state.content_matches());
    }
}
