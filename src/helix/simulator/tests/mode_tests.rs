//! Mode transition and insert mode tests for HelixSimulator
//!
//! Tests for mode changes and insert mode operations including:
//! - Mode transitions: i, a, I, A, Escape
//! - Insert mode text input
//! - Backspace in insert mode
//! - Arrow keys in insert mode

use crate::helix::commands::*;
use crate::helix::simulator::{AnyModeSimulator, Mode};

// ============================================================================
// Mode Transition Tests
// ============================================================================

#[test]
fn test_mode_change() {
    let mut sim = AnyModeSimulator::new("test".to_string());

    assert_eq!(sim.mode(), Mode::Normal);

    sim.execute_command(CMD_INSERT).unwrap();
    assert_eq!(sim.mode(), Mode::Insert);

    sim.execute_command(CMD_ESCAPE).unwrap();
    assert_eq!(sim.mode(), Mode::Normal);
}

#[test]
fn test_append_mode() {
    let mut sim = AnyModeSimulator::new("hello".to_string());

    // Cursor at start (position 0)
    assert_eq!(sim.state().unwrap().cursor_position().1, 0);

    // Press 'a' should move cursor one position right and enter insert mode
    sim.execute_command(CMD_APPEND).unwrap();

    let state = sim.state().unwrap();
    assert_eq!(sim.mode(), Mode::Insert);
    assert_eq!(state.cursor_position().1, 1); // Moved one right
}

#[test]
fn test_insert_at_line_start() {
    let mut sim = AnyModeSimulator::new("  hello world".to_string());

    // Move cursor to middle of line
    sim.execute_command(CMD_MOVE_WORD_FORWARD).unwrap();
    let state = sim.state().unwrap();
    assert!(state.cursor_position().1 > 0);

    // Press 'I' should move to start of line and enter insert mode
    sim.execute_command(CMD_INSERT_LINE_START).unwrap();

    let state = sim.state().unwrap();
    assert_eq!(sim.mode(), Mode::Insert);
    assert_eq!(state.cursor_position().1, 0);
}

#[test]
fn test_append_at_line_end() {
    let mut sim = AnyModeSimulator::new("hello world\nline2".to_string());

    // Cursor at start
    assert_eq!(sim.state().unwrap().cursor_position().1, 0);

    // Press 'A' should move to end of line and enter insert mode
    sim.execute_command(CMD_APPEND_LINE_END).unwrap();

    let state = sim.state().unwrap();
    assert_eq!(sim.mode(), Mode::Insert);
    assert_eq!(state.cursor_position().1, 11); // After "hello world"
    assert_eq!(state.cursor_position().0, 0); // Still on first line
}

// ============================================================================
// Insert Mode Text Input Tests
// ============================================================================

#[test]
fn test_insert_text_in_insert_mode() {
    let mut sim = AnyModeSimulator::new("hello".to_string());

    // Enter insert mode
    sim.execute_command(CMD_INSERT).unwrap();
    assert_eq!(sim.mode(), Mode::Insert);

    // Insert a character
    sim.execute_command("!").unwrap();

    let state = sim.state().unwrap();
    assert_eq!(state.content(), "!hello");
    assert_eq!(state.cursor_position().1, 1);
}

#[test]
fn test_append_at_line_end_and_insert() {
    let mut sim = AnyModeSimulator::new("hello".to_string());

    // Append at line end
    sim.execute_command(CMD_APPEND_LINE_END).unwrap();
    assert_eq!(sim.mode(), Mode::Insert);

    let cursor_pos = sim.state().unwrap().cursor_position().1;
    assert_eq!(cursor_pos, 5); // After 'hello'

    // Insert '!'
    sim.execute_command("!").unwrap();

    let state = sim.state().unwrap();
    assert_eq!(state.content(), "hello!");
    assert_eq!(state.cursor_position().1, 6);
}

#[test]
fn test_insert_multiple_chars() {
    let mut sim = AnyModeSimulator::new("".to_string());

    // Enter insert mode
    sim.execute_command(CMD_INSERT).unwrap();
    assert_eq!(sim.mode(), Mode::Insert);

    // Insert multiple characters
    sim.execute_command(CMD_APPEND).unwrap();
    sim.execute_command(CMD_MOVE_WORD_BACKWARD).unwrap();
    sim.execute_command(CMD_CHANGE).unwrap();

    let state = sim.state().unwrap();
    assert_eq!(state.content(), "abc");
    assert_eq!(state.cursor_position().1, 3);
}

// ============================================================================
// Backspace Tests
// ============================================================================

#[test]
fn test_backspace_in_insert_mode() {
    let mut sim = AnyModeSimulator::new("hello".to_string());

    // Enter insert mode at position 5
    // Note: Use CMD_GOTO_LINE_END (gl) instead of CMD_MOVE_LINE_END ($)
    // because '$' is NOT a line end command in Helix - use 'gl' for goto line end
    sim.execute_command(CMD_GOTO_LINE_END).unwrap(); // Move to end
    sim.execute_command(CMD_APPEND).unwrap(); // Append
    assert_eq!(sim.mode(), Mode::Insert);

    // Type some characters
    sim.execute_command("!").unwrap();
    sim.execute_command("!").unwrap();

    let state = sim.state().unwrap();
    assert_eq!(state.content(), "hello!!");
    assert_eq!(state.cursor_position().1, 7);

    // Backspace once
    sim.execute_command(CMD_BACKSPACE).unwrap();

    let state = sim.state().unwrap();
    assert_eq!(state.content(), "hello!");
    assert_eq!(state.cursor_position().1, 6);

    // Backspace again
    sim.execute_command(CMD_BACKSPACE).unwrap();

    let state = sim.state().unwrap();
    assert_eq!(state.content(), "hello");
    assert_eq!(state.cursor_position().1, 5);
}

#[test]
fn test_backspace_at_start() {
    let mut sim = AnyModeSimulator::new("test".to_string());

    // Enter insert mode at start
    sim.execute_command(CMD_INSERT).unwrap();
    assert_eq!(sim.mode(), Mode::Insert);

    // Backspace at position 0 should do nothing
    sim.execute_command(CMD_BACKSPACE).unwrap();

    let state = sim.state().unwrap();
    assert_eq!(state.content(), "test");
    assert_eq!(state.cursor_position().1, 0);
}

// ============================================================================
// Arrow Keys in Insert Mode Tests
// ============================================================================

#[test]
fn test_arrow_keys_in_insert_mode() {
    let mut sim = AnyModeSimulator::new("abc\ndef".to_string());

    // Enter insert mode
    sim.execute_command(CMD_INSERT).unwrap();
    assert_eq!(sim.mode(), Mode::Insert);

    // Test Right arrow
    sim.execute_command(CMD_ARROW_RIGHT).unwrap();
    assert_eq!(sim.state().unwrap().cursor_position().1, 1);

    // Test Left arrow
    sim.execute_command(CMD_ARROW_LEFT).unwrap();
    assert_eq!(sim.state().unwrap().cursor_position().1, 0);

    // Test Down arrow
    sim.execute_command(CMD_ARROW_DOWN).unwrap();
    assert_eq!(sim.state().unwrap().cursor_position().0, 1);

    // Test Up arrow
    sim.execute_command(CMD_ARROW_UP).unwrap();
    assert_eq!(sim.state().unwrap().cursor_position().0, 0);

    // Should still be in Insert mode
    assert_eq!(sim.mode(), Mode::Insert);
}
