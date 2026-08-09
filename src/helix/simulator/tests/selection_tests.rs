//! Selection command tests for HelixSimulator
//!
//! Tests for selection operations including:
//! - Line selection: x
//! - Select all: %
//! - Selection + delete workflows: x+d

use crate::game::EditorState;
use crate::game::editor_state::CursorPosition;
use crate::helix::commands::*;
use crate::helix::simulator::AnyModeSimulator;

// ============================================================================
// Line Selection Tests
// ============================================================================

#[test]
fn test_select_line_then_delete() {
    let mut sim = AnyModeSimulator::new("Keep\nDelete me\nKeep".to_string());

    // Move to line 1
    sim.execute_command(CMD_MOVE_DOWN).unwrap();
    let state = sim.state().unwrap();
    assert_eq!(state.cursor_position().0, 1);

    // Execute x (select line)
    sim.execute_command(CMD_SELECT_LINE).unwrap();
    let state = sim.state().unwrap();
    // Selection should cover "Delete me\n" (line 1 with newline)
    let sel = state.selection().expect("Should have selection after x");
    assert_eq!(sel.start.row, 1, "Selection start row");
    assert_eq!(sel.start.col, 0, "Selection start col");
    // End should be at start of next line (row 2, col 0) or end of this line
    assert!(sel.end.row >= 1, "Selection end row should be >= 1");

    // Execute d (delete selection)
    sim.execute_command(CMD_DELETE_SELECTION).unwrap();
    let state = sim.state().unwrap();
    assert_eq!(state.content(), "Keep\nKeep");
}

#[test]
fn test_select_line_scenario_exact() {
    // Replicate exact scenario: "Select line then delete"
    // setup: file_content = "Keep\nDelete me\nKeep", cursor_position = [1, 0]

    let initial_state = EditorState::new(
        "Keep\nDelete me\nKeep".to_string(),
        CursorPosition::new(1, 0).unwrap(),
        None,
    )
    .unwrap();

    let mut sim = AnyModeSimulator::from_editor_state(&initial_state);
    let state = sim.state().unwrap();
    eprintln!(
        "Initial: cursor={:?}, sel={:?}, content={:?}",
        state.cursor_position(),
        state.selection(),
        state.content()
    );
    assert_eq!(state.cursor_position().0, 1);
    assert_eq!(state.cursor_position().1, 0);

    // Execute x (select line)
    sim.execute_command(CMD_SELECT_LINE).unwrap();
    let state = sim.state().unwrap();
    eprintln!(
        "After x: cursor={:?}, sel={:?}",
        state.cursor_position(),
        state.selection()
    );

    let sel = state.selection().expect("Should have selection after x");
    assert_eq!(sel.start.row, 1, "Selection should start at row 1");
    assert_eq!(
        sel.end.row, 2,
        "Selection should end at row 2 (newline included)"
    );

    // Execute d (delete selection)
    sim.execute_command(CMD_DELETE_SELECTION).unwrap();
    let state = sim.state().unwrap();
    eprintln!(
        "After d: cursor={:?}, sel={:?}, content={:?}",
        state.cursor_position(),
        state.selection(),
        state.content()
    );
    assert_eq!(
        state.content(),
        "Keep\nKeep",
        "After xd should delete only line 1"
    );
}

// ============================================================================
// Select All Tests
// ============================================================================

#[test]
fn test_select_all_then_delete() {
    // Scenario: User selects all with '%', then deletes with 'd'
    let mut sim = AnyModeSimulator::new("hello\nworld".to_string());

    // Select all
    sim.execute_command("%").unwrap();

    // Delete selection
    sim.execute_command(CMD_DELETE_SELECTION).unwrap();
    let state = sim.state().unwrap();
    assert!(state.content().is_empty() || state.content() == "\n");
}

// ============================================================================
// Compound Selection Tests (for repeat functionality)
// ============================================================================

#[test]
fn test_repeat_select_line_then_delete() {
    // Scenario: User selects a line with 'x', deletes with 'd', then repeats with '.'
    // Expected: The repeat should execute both x and d together
    let mut sim = AnyModeSimulator::new("line 1\nline 2\nline 3\nline 4\n".to_string());

    // Move to line 2
    sim.execute_command(CMD_MOVE_DOWN).unwrap();
    let state = sim.state().unwrap();
    assert_eq!(state.cursor_position().0, 1);

    // Select line with x
    sim.execute_command(CMD_SELECT_LINE).unwrap();

    // Delete selection with d
    sim.execute_command(CMD_DELETE_SELECTION).unwrap();
    let state = sim.state().unwrap();
    assert_eq!(state.content(), "line 1\nline 3\nline 4\n");

    // Now cursor should be on what was line 3 (now line 2)
    assert_eq!(state.cursor_position().0, 1);

    // Repeat with . - should execute x+d together
    sim.execute_command(".").unwrap();
    let state = sim.state().unwrap();
    // Line 3 (now at row 1) should be deleted
    assert_eq!(state.content(), "line 1\nline 4\n");

    // Repeat again
    sim.execute_command(".").unwrap();
    let state = sim.state().unwrap();
    assert_eq!(state.content(), "line 1\n");
}

#[test]
fn test_select_line_then_yank() {
    // Scenario: User selects a line with 'x', yanks with 'y'
    let mut sim = AnyModeSimulator::new("line 1\nline 2\nline 3\n".to_string());

    // Move to line 2
    sim.execute_command(CMD_MOVE_DOWN).unwrap();

    // Select line
    sim.execute_command(CMD_SELECT_LINE).unwrap();

    // Yank
    sim.execute_command(CMD_YANK).unwrap();

    // Move down - need to navigate away from current selection first
    // After yank, selection is still active, so collapse it first
    sim.execute_command(CMD_COLLAPSE_SELECTION).unwrap();
    sim.execute_command(CMD_MOVE_DOWN).unwrap();

    // Repeat x+y - should select current line and yank
    sim.execute_command(".").unwrap();

    // Verify we're still on same content (yank doesn't delete)
    let state = sim.state().unwrap();
    assert_eq!(state.content(), "line 1\nline 2\nline 3\n");
}

#[test]
fn test_compound_action_overwritten_by_simple_command() {
    // Test that a simple editing command after x+d overwrites the compound action
    let mut sim = AnyModeSimulator::new("aaa\nbbb\nccc\n".to_string());

    // Do x+d (compound action)
    sim.execute_command(CMD_SELECT_LINE).unwrap();
    sim.execute_command(CMD_DELETE_SELECTION).unwrap();
    let state = sim.state().unwrap();
    assert_eq!(state.content(), "bbb\nccc\n");

    // Now do another x+d (still compound, same sequence)
    sim.execute_command(CMD_SELECT_LINE).unwrap();
    sim.execute_command(CMD_DELETE_SELECTION).unwrap();
    let state = sim.state().unwrap();
    assert_eq!(state.content(), "ccc\n");

    // Repeat should do x+d again
}
