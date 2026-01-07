//! Scenario state initialization helpers
//!
//! Provides shared functionality for initializing scenario states,
//! reducing code duplication between training and arcade modes.

use crate::config::Scenario;
use crate::game::EditorState;
use crate::helix::AnyModeSimulator;
use crate::security::UserError;

/// Initialized scenario state ready for gameplay
///
/// Contains all the components needed to start a scenario:
/// - Initial editor state (starting position)
/// - Target editor state (goal to achieve)
/// - Current editor state (mutable during gameplay)
/// - Helix simulator for command execution
pub struct ScenarioState {
    /// The initial state when scenario starts
    pub initial_state: EditorState,
    /// The target state to achieve
    pub target_state: EditorState,
    /// Current state (clone of initial, modified during play)
    pub current_state: EditorState,
    /// Helix simulator for command execution
    pub simulator: AnyModeSimulator,
}

impl ScenarioState {
    /// Initialize scenario state from a scenario configuration
    ///
    /// Creates initial and target EditorState from scenario setup/target,
    /// and initializes the Helix simulator.
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
    /// assert_eq!(state.current_state, state.initial_state);
    /// ```
    pub fn from_scenario(scenario: &Scenario) -> Result<Self, UserError> {
        // Create initial state from scenario setup (with optional selection)
        let initial_state = EditorState::from_setup(
            &scenario.setup.file_content,
            [
                scenario.setup.cursor_position.0,
                scenario.setup.cursor_position.1,
            ],
            scenario.setup.selection,
        )
        .map_err(|_| UserError::ScenarioTooComplex)?;

        // Create target state with optional selection
        let target_state = EditorState::from_target(
            &scenario.target.file_content,
            [
                scenario.target.cursor_position.0,
                scenario.target.cursor_position.1,
            ],
            scenario.target.selection,
        )
        .map_err(|_| UserError::ScenarioTooComplex)?;

        // Clone initial state as current state
        let current_state = initial_state.clone();

        // Initialize Helix simulator from initial state
        let simulator = AnyModeSimulator::from_editor_state(&initial_state);

        Ok(Self {
            initial_state,
            target_state,
            current_state,
            simulator,
        })
    }

    /// Check if current state matches target state
    ///
    /// This is the unified completion check used by both modes.
    #[inline]
    pub fn is_completed(&self) -> bool {
        self.current_state.matches(&self.target_state)
    }

    /// Check if content matches target (ignoring cursor position)
    #[inline]
    pub fn content_matches(&self) -> bool {
        self.current_state.content_matches(&self.target_state)
    }

    /// Reset to initial state
    ///
    /// Restores current_state to initial_state and reinitializes simulator.
    pub fn reset(&mut self, scenario: &Scenario) {
        self.current_state = self.initial_state.clone();
        self.simulator = AnyModeSimulator::new(scenario.setup.file_content.clone());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{ScoringConfig, Setup, Solution, TargetState};

    fn create_test_scenario() -> Scenario {
        Scenario {
            id: "test".to_string(),
            name: "Test".to_string(),
            description: "Test scenario".to_string(),
            setup: Setup {
                file_content: "hello".to_string(),
                cursor_position: (0, 0),
                selection: None,
            },
            target: TargetState {
                file_content: "world".to_string(),
                cursor_position: (0, 0),
                selection: None,
            },
            solution: Solution {
                commands: vec!["ciw".to_string(), "world".to_string()],
                description: "Change word".to_string(),
            },
            alternatives: vec![],
            hints: vec![],
            scoring: ScoringConfig {
                optimal_count: 1,
                max_points: 100,
                tolerance: 0,
            },
            metadata: None,
        }
    }

    #[test]
    fn test_from_scenario_creates_states() {
        let scenario = create_test_scenario();
        let state = ScenarioState::from_scenario(&scenario).unwrap();

        assert_eq!(state.initial_state.content(), "hello");
        assert_eq!(state.target_state.content(), "world");
        assert_eq!(state.current_state.content(), "hello");
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
