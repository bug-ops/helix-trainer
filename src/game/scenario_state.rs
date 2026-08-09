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
            scenario.setup.cursor.cursor_position,
            scenario.setup.cursor.selection,
            scenario.setup.cursor.cursors.as_deref(),
            scenario.setup.cursor.selections.as_deref(),
        );

        // Create target snapshot for completion checking
        let target_snapshot = EditorSnapshot::from_scenario_config(
            scenario.target.file_content.clone(),
            scenario.target.cursor.cursor_position,
            scenario.target.cursor.selection,
            scenario.target.cursor.cursors.as_deref(),
            scenario.target.cursor.selections.as_deref(),
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

    /// Regression test for #283: entering Insert mode alone must not
    /// register as completion, even when the target content is already
    /// reachable mid-Insert (e.g. `o` opens a blank line matching the
    /// target before `Escape` returns to Normal mode).
    #[test]
    fn test_insert_mode_entry_alone_does_not_complete() {
        let scenario = ScenarioBuilder::new()
            .id("open_below_001")
            .name("Insert line below")
            .setup_content("fn add(a: i32, b: i32) -> i32 {\n    a + b\n}")
            .setup_cursor(0, 0)
            .target_content("fn add(a: i32, b: i32) -> i32 {\n\n    a + b\n}")
            .target_cursor(1, 0)
            .commands(vec!["o", "Escape"])
            .command_description("Press 'o' to open line below, then Escape")
            .optimal_count(2)
            .build();
        let mut state = ScenarioState::from_scenario(&scenario).unwrap();

        state.simulator.execute_command("o").unwrap();
        assert_eq!(
            state.simulator.display().content(),
            scenario.target.file_content
        );
        assert!(
            !state.is_completed(),
            "scenario must not complete while still in Insert mode"
        );

        state.simulator.execute_command("Escape").unwrap();
        assert!(
            state.is_completed(),
            "scenario must complete once back in Normal mode"
        );
    }
}
