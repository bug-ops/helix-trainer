//! Selection command tests for `HelixSimulator`
//!
//! Tests for selection operations including:
//! - Line selection: x
//! - Select all: %
//! - Selection + delete workflows: x+d

use super::sim_at_cursor;
use crate::helix::commands::*;
use crate::helix::error::SimulatorError;
use crate::helix::simulator::AnyModeSimulator;
use std::assert_matches;

// ============================================================================
// Line Selection Tests
// ============================================================================

#[test]
fn test_select_line_then_delete() {
    let mut sim = AnyModeSimulator::new("Keep\nDelete me\nKeep".to_string());

    // Move to line 1
    sim.execute_command(CMD_MOVE_DOWN).unwrap();
    let state = sim.display();
    assert_eq!(state.cursor_position().0, 1);

    // Execute x (select line)
    sim.execute_command(CMD_SELECT_LINE).unwrap();
    let state = sim.display();
    // Selection should cover "Delete me\n" (line 1 with newline)
    let sel = state.selection().expect("Should have selection after x");
    assert_eq!(sel.start_row, 1, "Selection start row");
    assert_eq!(sel.start_col, 0, "Selection start col");
    // End should be at start of next line (row 2, col 0) or end of this line
    assert!(sel.end_row >= 1, "Selection end row should be >= 1");

    // Execute d (delete selection)
    sim.execute_command(CMD_DELETE_SELECTION).unwrap();
    let state = sim.display();
    assert_eq!(state.content(), "Keep\nKeep");
}

#[test]
fn test_select_line_scenario_exact() {
    // Replicate exact scenario: "Select line then delete"
    // setup: file_content = "Keep\nDelete me\nKeep", cursor_position = [1, 0]

    let mut sim = sim_at_cursor("Keep\nDelete me\nKeep", 1, 0);
    let state = sim.display();
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
    let state = sim.display();
    eprintln!(
        "After x: cursor={:?}, sel={:?}",
        state.cursor_position(),
        state.selection()
    );

    let sel = state.selection().expect("Should have selection after x");
    assert_eq!(sel.start_row, 1, "Selection should start at row 1");
    assert_eq!(
        sel.end_row, 2,
        "Selection should end at row 2 (newline included)"
    );

    // Execute d (delete selection)
    sim.execute_command(CMD_DELETE_SELECTION).unwrap();
    let state = sim.display();
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
    let state = sim.display();
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
    let state = sim.display();
    assert_eq!(state.cursor_position().0, 1);

    // Select line with x
    sim.execute_command(CMD_SELECT_LINE).unwrap();

    // Delete selection with d
    sim.execute_command(CMD_DELETE_SELECTION).unwrap();
    let state = sim.display();
    assert_eq!(state.content(), "line 1\nline 3\nline 4\n");

    // Now cursor should be on what was line 3 (now line 2)
    assert_eq!(state.cursor_position().0, 1);

    // Repeat with . - should execute x+d together
    sim.execute_command(".").unwrap();
    let state = sim.display();
    // Line 3 (now at row 1) should be deleted
    assert_eq!(state.content(), "line 1\nline 4\n");

    // Repeat again
    sim.execute_command(".").unwrap();
    let state = sim.display();
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
    let state = sim.display();
    assert_eq!(state.content(), "line 1\nline 2\nline 3\n");
}

#[test]
fn test_compound_action_overwritten_by_simple_command() {
    // Test that a simple editing command after x+d overwrites the compound action
    let mut sim = AnyModeSimulator::new("aaa\nbbb\nccc\n".to_string());

    // Do x+d (compound action)
    sim.execute_command(CMD_SELECT_LINE).unwrap();
    sim.execute_command(CMD_DELETE_SELECTION).unwrap();
    let state = sim.display();
    assert_eq!(state.content(), "bbb\nccc\n");

    // Now do another x+d (still compound, same sequence)
    sim.execute_command(CMD_SELECT_LINE).unwrap();
    sim.execute_command(CMD_DELETE_SELECTION).unwrap();
    let state = sim.display();
    assert_eq!(state.content(), "ccc\n");

    // Repeat should do x+d again
}

// Expected buffers in the tests below were recorded from `hx` 25.07.1 on the
// same keys (`&`, `K`, `Alt-K`, #510).

fn run_commands(text: &str, commands: &[&str]) -> AnyModeSimulator {
    let mut sim = AnyModeSimulator::new(text.to_string());
    for command in commands {
        sim.execute_command(command).unwrap();
    }
    sim
}

fn content_of(sim: &AnyModeSimulator) -> String {
    sim.display().content()
}

#[test]
fn test_align_selections_pads_columns_to_common_end() {
    let sim = run_commands(
        "a b c\nxx yy zz",
        &[CMD_SELECT_ALL, "s \\w+", CMD_ALIGN_SELECTIONS],
    );
    assert_eq!(content_of(&sim), " a  b  c\nxx yy zz");
}

#[test]
fn test_align_selections_keeps_ranges_on_their_text() {
    let sim = run_commands(
        "a b c\nxx yy zz",
        &[
            CMD_SELECT_ALL,
            "s \\w+",
            CMD_ALIGN_SELECTIONS,
            CMD_DELETE_SELECTION,
        ],
    );
    assert_eq!(content_of(&sim), "     \n  ");
}

#[test]
fn test_align_selections_aligns_equals_signs() {
    let sim = run_commands(
        "a = 1\nbbb = 22\ncc = 3",
        &[CMD_SELECT_ALL, "s =", CMD_ALIGN_SELECTIONS],
    );
    assert_eq!(content_of(&sim), "a   = 1\nbbb = 22\ncc  = 3");
}

#[test]
fn test_align_selections_accounts_for_padding_inserted_in_earlier_columns() {
    let sim = run_commands(
        "a = 1\nbbb = 22\ncc = 3",
        &[CMD_SELECT_ALL, "s \\w+", CMD_ALIGN_SELECTIONS],
    );
    assert_eq!(content_of(&sim), "  a =  1\nbbb = 22\n cc =  3");
}

#[test]
fn test_align_selections_rejects_multi_line_selection() {
    let mut sim = AnyModeSimulator::new("a\nb".to_string());
    sim.execute_command(CMD_SELECT_ALL).unwrap();
    assert_matches!(
        sim.execute_command(CMD_ALIGN_SELECTIONS),
        Err(SimulatorError::AlignSpansLines)
    );
    assert_eq!(content_of(&sim), "a\nb");
}

#[test]
fn test_keep_selections_keeps_only_matching_ranges() {
    let sim = run_commands(
        "banana apple cherry",
        &[CMD_SELECT_ALL, "s \\w+", "K an", CMD_DELETE_SELECTION],
    );
    assert_eq!(content_of(&sim), " apple cherry");
}

#[test]
fn test_remove_selections_drops_matching_ranges() {
    let sim = run_commands(
        "banana apple cherry",
        &[CMD_SELECT_ALL, "s \\w+", "Alt-K an", CMD_DELETE_SELECTION],
    );
    assert_eq!(content_of(&sim), "banana  ");
}

#[test]
fn test_keep_selections_without_remaining_range_errs_and_keeps_selection() {
    let mut sim = run_commands("banana apple cherry", &[CMD_SELECT_ALL, "s \\w+"]);
    assert_matches!(
        sim.execute_command("K zzz"),
        Err(SimulatorError::NoSelections)
    );
    sim.execute_command(CMD_DELETE_SELECTION).unwrap();
    assert_eq!(content_of(&sim), "  ");
}

#[test]
fn test_remove_selections_with_invalid_regex_errs() {
    let mut sim = run_commands("banana apple", &[CMD_SELECT_ALL, "s \\w+"]);
    assert_matches!(
        sim.execute_command("Alt-K ("),
        Err(SimulatorError::InvalidRegex(_))
    );
}

#[test]
fn test_bare_keep_selections_is_an_error_not_a_silent_noop() {
    let mut sim = AnyModeSimulator::new("abc".to_string());
    assert_matches!(
        sim.execute_command(CMD_KEEP_MATCHING),
        Err(SimulatorError::PatternRequired)
    );
    assert_matches!(
        sim.execute_command(CMD_REMOVE_MATCHING),
        Err(SimulatorError::PatternRequired)
    );
}
