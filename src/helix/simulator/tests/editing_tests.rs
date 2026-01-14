//! Editing command tests for HelixSimulator
//!
//! Tests for text modification commands including:
//! - Delete operations: d, x+d
//! - Undo/redo: u
//! - Line operations: o, O, J
//! - Indentation: >, <
//! - Character replacement: r
//! - Change command: c

use crate::helix::commands::*;
use crate::helix::simulator::{AnyModeSimulator, Mode};

// ============================================================================
// Delete Tests
// ============================================================================

#[test]
fn test_delete_char() {
    let mut sim = AnyModeSimulator::new("hello".to_string());

    sim.execute_command(CMD_DELETE_SELECTION).unwrap();

    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "ello");
}

#[test]
fn test_delete_char_in_middle() {
    let mut sim = AnyModeSimulator::new("hello".to_string());

    sim.execute_command(CMD_MOVE_RIGHT).unwrap(); // Move to 'e'
    sim.execute_command(CMD_MOVE_RIGHT).unwrap(); // Move to 'l'
    sim.execute_command(CMD_DELETE_SELECTION).unwrap(); // Delete 'l'

    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "helo");
}

#[test]
fn test_multiple_line_deletions() {
    let mut sim = AnyModeSimulator::new("line1\nline2\nline3\n".to_string());

    // Delete first line with x+d
    sim.execute_command(CMD_SELECT_LINE).unwrap();
    sim.execute_command(CMD_DELETE_SELECTION).unwrap();
    // Delete second line with x+d
    sim.execute_command(CMD_SELECT_LINE).unwrap();
    sim.execute_command(CMD_DELETE_SELECTION).unwrap();

    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "line3\n");
}

// ============================================================================
// Undo Tests
// ============================================================================

#[test]
fn test_undo() {
    let mut sim = AnyModeSimulator::new("test\n".to_string());

    // Delete line using x (select) + d (delete)
    sim.execute_command(CMD_SELECT_LINE).unwrap();
    sim.execute_command(CMD_DELETE_SELECTION).unwrap();
    assert_eq!(sim.get_state().unwrap().content(), "");

    sim.execute_command(CMD_UNDO).unwrap();
    assert_eq!(sim.get_state().unwrap().content(), "test\n");
}

// ============================================================================
// Open Line Tests (o, O)
// ============================================================================

#[test]
fn test_open_below() {
    let mut sim = AnyModeSimulator::new("line1\nline2".to_string());

    // Cursor at start of first line
    assert_eq!(sim.get_state().unwrap().cursor_position().0, 0);

    // Press 'o' should insert new line below and enter insert mode
    sim.execute_command(CMD_OPEN_BELOW).unwrap();

    let state = sim.get_state().unwrap();
    assert_eq!(sim.mode(), Mode::Insert);
    assert_eq!(state.content(), "line1\n\nline2");
    assert_eq!(state.cursor_position().0, 1); // On new empty line
}

#[test]
fn test_open_above() {
    let mut sim = AnyModeSimulator::new("line1\nline2".to_string());

    // Move to second line
    sim.execute_command(CMD_MOVE_DOWN).unwrap();
    assert_eq!(sim.get_state().unwrap().cursor_position().0, 1);

    // Press 'O' should insert new line above and enter insert mode
    sim.execute_command(CMD_OPEN_ABOVE).unwrap();

    let state = sim.get_state().unwrap();
    assert_eq!(sim.mode(), Mode::Insert);
    assert_eq!(state.content(), "line1\n\nline2");
    assert_eq!(state.cursor_position().0, 1); // On new empty line
}

// ============================================================================
// Join Lines Tests
// ============================================================================

#[test]
fn test_join_lines() {
    let mut sim = AnyModeSimulator::new("line1\nline2\nline3".to_string());

    // Join first two lines
    sim.execute_command(CMD_JOIN_LINES).unwrap();

    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "line1 line2\nline3");
    assert_eq!(state.cursor_position().0, 0);
}

#[test]
fn test_join_lines_at_last_line() {
    let mut sim = AnyModeSimulator::new("line1\nline2".to_string());

    // Move to last line
    sim.execute_command(CMD_MOVE_DOWN).unwrap();
    assert_eq!(sim.get_state().unwrap().cursor_position().0, 1);

    // Try to join - should do nothing
    sim.execute_command(CMD_JOIN_LINES).unwrap();

    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "line1\nline2");
}

// ============================================================================
// Indentation Tests
// ============================================================================

#[test]
fn test_indent_line() {
    let mut sim = AnyModeSimulator::new("hello\nworld".to_string());

    // Indent first line
    sim.execute_command(CMD_INDENT).unwrap();

    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "  hello\nworld");
    // Cursor should move forward by 2
    assert_eq!(state.cursor_position().1, 2);
}

#[test]
fn test_dedent_line() {
    let mut sim = AnyModeSimulator::new("  hello\n    world".to_string());

    // Dedent first line (remove 2 spaces)
    sim.execute_command(CMD_DEDENT).unwrap();

    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "hello\n    world");
    assert_eq!(state.cursor_position().1, 0);
}

#[test]
fn test_dedent_line_with_one_space() {
    let mut sim = AnyModeSimulator::new(" hello".to_string());

    // Dedent - should remove only 1 space
    sim.execute_command(CMD_DEDENT).unwrap();

    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "hello");
    assert_eq!(state.cursor_position().1, 0);
}

#[test]
fn test_dedent_line_no_spaces() {
    let mut sim = AnyModeSimulator::new("hello".to_string());

    // Dedent line with no leading spaces - should do nothing
    sim.execute_command(CMD_DEDENT).unwrap();

    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "hello");
}

#[test]
fn test_multiple_indent() {
    let mut sim = AnyModeSimulator::new("code".to_string());

    // Indent twice
    sim.execute_command(CMD_INDENT).unwrap();
    sim.execute_command(CMD_INDENT).unwrap();

    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "    code");
    assert_eq!(state.cursor_position().1, 4);
}

// ============================================================================
// Replace Character Tests
// ============================================================================

#[test]
fn test_replace_char() {
    let mut sim = AnyModeSimulator::new("hello".to_string());

    // Cursor at start
    assert_eq!(sim.get_state().unwrap().content(), "hello");

    // Press 'r' then 'X' should replace 'h' with 'X'
    sim.execute_command("rX").unwrap();

    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "Xello");
    assert_eq!(sim.mode(), Mode::Normal); // Should stay in normal mode
}

// ============================================================================
// Change Command Tests
// ============================================================================

#[test]
fn test_change_selection() {
    let mut sim = AnyModeSimulator::new("hello".to_string());

    // Cursor at start
    assert_eq!(sim.get_state().unwrap().content(), "hello");
    assert_eq!(sim.mode(), Mode::Normal);

    // Press 'c' should delete 'h' and enter insert mode
    sim.execute_command(CMD_CHANGE).unwrap();

    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "ello");
    assert_eq!(sim.mode(), Mode::Insert);
    assert_eq!(state.cursor_position().1, 0); // Cursor stays at start
}
