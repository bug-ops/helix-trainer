//! Scenario state initialization helpers
//!
//! Provides shared functionality for initializing scenario states,
//! reducing code duplication between training and arcade modes.

use crate::config::{CursorSpec, DEFAULT_LANGUAGE, Pos, Scenario};
use crate::helix::{AnyModeSimulator, EditorSnapshot};

fn snapshot_of(content: String, cursor: &CursorSpec) -> EditorSnapshot {
    let point = |p: &Pos| [p.row(), p.col()];
    match cursor {
        CursorSpec::Point(p) => EditorSnapshot::from_row_col(content, p.row(), p.col()),
        CursorSpec::Range(r) => EditorSnapshot::from_row_col_selection(
            content,
            [
                r.anchor().row(),
                r.anchor().col(),
                r.head().row(),
                r.head().col(),
            ],
        ),
        CursorSpec::Points(ps) => {
            let points: Vec<[usize; 2]> = ps.as_slice().iter().map(point).collect();
            EditorSnapshot::from_multi_cursor(content, &points)
        }
        CursorSpec::Ranges(rs) => {
            let ranges: Vec<[usize; 4]> = rs
                .as_slice()
                .iter()
                .map(|r| {
                    let [ar, ac] = point(&r.anchor());
                    let [hr, hc] = point(&r.head());
                    [ar, ac, hr, hc]
                })
                .collect();
            EditorSnapshot::from_multi_selection(content, &ranges)
        }
    }
}

/// Initialized scenario state ready for gameplay
///
/// Contains the Helix simulator (source of truth) and target snapshot
/// for efficient completion checking. All state queries go through
/// the simulator's `EditorDisplay` facade.
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
    /// The initialized state, ready for gameplay.
    ///
    /// # Examples
    ///
    /// ```
    /// use helix_trainer::config::ScenarioLoader;
    /// use helix_trainer::game::ScenarioState;
    ///
    /// let scenarios = ScenarioLoader::new().load_from_embedded("en").expect("embedded scenarios load");
    /// let state = ScenarioState::from_scenario(&scenarios[0]);
    /// assert!(!state.is_completed());
    /// ```
    pub fn from_scenario(scenario: &Scenario) -> Self {
        let initial_snapshot =
            snapshot_of(scenario.setup.file_content.clone(), &scenario.setup.cursor);
        let target_snapshot = snapshot_of(
            scenario.target.file_content.clone(),
            &scenario.target.cursor,
        );

        // Initialize Helix simulator from initial snapshot
        let simulator = AnyModeSimulator::from_snapshot(&initial_snapshot).with_language(
            scenario
                .setup
                .language
                .as_deref()
                .unwrap_or(DEFAULT_LANGUAGE),
        );

        Self {
            simulator,
            target_snapshot,
        }
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
    use crate::config::CursorRange;
    use crate::testing::{ScenarioBuilder, point, selection};

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
        let state = ScenarioState::from_scenario(&scenario);

        // Verify simulator has initial content
        assert_eq!(state.simulator.display().content(), "hello");
        // Verify target snapshot has target content
        assert_eq!(state.target_snapshot.content, "world");
    }

    #[test]
    fn test_is_completed_false_initially() {
        let scenario = create_test_scenario();
        let state = ScenarioState::from_scenario(&scenario);

        assert!(!state.is_completed());
    }

    #[test]
    fn test_content_matches() {
        let scenario = create_test_scenario();
        let state = ScenarioState::from_scenario(&scenario);

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
        let mut state = ScenarioState::from_scenario(&scenario);

        state.simulator.execute_command("o").unwrap();
        assert_eq!(
            state.simulator.display().content(),
            "fn add(a: i32, b: i32) -> i32 {\n    \n    a + b\n}"
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

    fn pos(row: usize, col: usize) -> Pos {
        Pos::try_from([row, col]).unwrap()
    }

    #[test]
    fn test_snapshot_of_each_cursor_form() {
        let content = "hello world\nsecond";

        let at = snapshot_of(content.to_string(), &point(0, 2));
        assert_eq!(at.selections.len(), 1);
        assert_eq!(at.cursor_offset(), 2);

        let range = snapshot_of(content.to_string(), &selection((0, 0), (0, 5)));
        assert_eq!(
            (range.selections[0].anchor, range.selections[0].head),
            (0, 5)
        );

        let points = CursorSpec::Points(vec![pos(0, 1), pos(1, 1)].try_into().unwrap());
        let snap = snapshot_of(content.to_string(), &points);
        assert_eq!(snap.selections.len(), 2);
        assert_eq!(snap.selections[1].head, 13);

        let ranges = CursorSpec::Ranges(
            vec![
                CursorRange::try_from([0, 0, 0, 5]).unwrap(),
                CursorRange::try_from([0, 6, 0, 11]).unwrap(),
            ]
            .try_into()
            .unwrap(),
        );
        let snap = snapshot_of(content.to_string(), &ranges);
        assert_eq!(snap.selections.len(), 2);
        assert_eq!(
            (snap.selections[1].anchor, snap.selections[1].head),
            (6, 11)
        );
    }
}
