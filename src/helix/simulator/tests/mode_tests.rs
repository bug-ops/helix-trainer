//! Mode transition and insert mode tests for `HelixSimulator`
//!
//! Tests for mode changes and insert mode operations including:
//! - Mode transitions: i, a, I, A, Escape
//! - Insert mode text input
//! - Backspace in insert mode
//! - Arrow keys in insert mode

use crate::grammar::commands::*;
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
    assert_eq!(sim.display().cursor_position().col(), 0);

    // Press 'a' should move cursor one position right and enter insert mode
    sim.execute_command(CMD_APPEND).unwrap();

    let state = sim.display();
    assert_eq!(sim.mode(), Mode::Insert);
    assert_eq!(state.cursor_position().col(), 1); // Moved one right
}

#[test]
fn test_insert_at_line_start() {
    let mut sim = AnyModeSimulator::new("  hello world".to_string());

    // Move cursor to middle of line
    sim.execute_command(CMD_MOVE_WORD_FORWARD).unwrap();
    let state = sim.display();
    assert!(state.cursor_position().col() > 0);

    // Press 'I' should move to the first non-blank character and enter insert mode
    sim.execute_command(CMD_INSERT_LINE_START).unwrap();

    let state = sim.display();
    assert_eq!(sim.mode(), Mode::Insert);
    assert_eq!(state.cursor_position().col(), 2);
}

#[test]
fn test_append_at_line_end() {
    let mut sim = AnyModeSimulator::new("hello world\nline2".to_string());

    // Cursor at start
    assert_eq!(sim.display().cursor_position().col(), 0);

    // Press 'A' should move to end of line and enter insert mode
    sim.execute_command(CMD_APPEND_LINE_END).unwrap();

    let state = sim.display();
    assert_eq!(sim.mode(), Mode::Insert);
    assert_eq!(state.cursor_position().col(), 11); // After "hello world"
    assert_eq!(state.cursor_position().row(), 0); // Still on first line
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

    let state = sim.display();
    assert_eq!(state.content(), "!hello");
    assert_eq!(state.cursor_position().col(), 1);
}

#[test]
fn test_append_at_line_end_and_insert() {
    let mut sim = AnyModeSimulator::new("hello".to_string());

    // Append at line end
    sim.execute_command(CMD_APPEND_LINE_END).unwrap();
    assert_eq!(sim.mode(), Mode::Insert);

    let cursor_pos = sim.display().cursor_position().col();
    assert_eq!(cursor_pos, 5); // After 'hello'

    // Insert '!'
    sim.execute_command("!").unwrap();

    let state = sim.display();
    assert_eq!(state.content(), "hello!");
    assert_eq!(state.cursor_position().col(), 6);
}

#[test]
fn test_insert_multiple_chars() {
    let mut sim = AnyModeSimulator::new(String::new());

    // Enter insert mode
    sim.execute_command(CMD_INSERT).unwrap();
    assert_eq!(sim.mode(), Mode::Insert);

    // Insert multiple characters
    sim.execute_command(CMD_APPEND).unwrap();
    sim.execute_command(CMD_MOVE_WORD_BACKWARD).unwrap();
    sim.execute_command(CMD_CHANGE).unwrap();

    let state = sim.display();
    assert_eq!(state.content(), "abc");
    assert_eq!(state.cursor_position().col(), 3);
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

    let state = sim.display();
    assert_eq!(state.content(), "hello!!");
    assert_eq!(state.cursor_position().col(), 7);

    // Backspace once
    sim.execute_command(CMD_BACKSPACE).unwrap();

    let state = sim.display();
    assert_eq!(state.content(), "hello!");
    assert_eq!(state.cursor_position().col(), 6);

    // Backspace again
    sim.execute_command(CMD_BACKSPACE).unwrap();

    let state = sim.display();
    assert_eq!(state.content(), "hello");
    assert_eq!(state.cursor_position().col(), 5);
}

#[test]
fn test_backspace_at_start() {
    let mut sim = AnyModeSimulator::new("test".to_string());

    // Enter insert mode at start
    sim.execute_command(CMD_INSERT).unwrap();
    assert_eq!(sim.mode(), Mode::Insert);

    // Backspace at position 0 should do nothing
    sim.execute_command(CMD_BACKSPACE).unwrap();

    let state = sim.display();
    assert_eq!(state.content(), "test");
    assert_eq!(state.cursor_position().col(), 0);
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
    assert_eq!(sim.display().cursor_position().col(), 1);

    // Test Left arrow
    sim.execute_command(CMD_ARROW_LEFT).unwrap();
    assert_eq!(sim.display().cursor_position().col(), 0);

    // Test Down arrow
    sim.execute_command(CMD_ARROW_DOWN).unwrap();
    assert_eq!(sim.display().cursor_position().row(), 1);

    // Test Up arrow
    sim.execute_command(CMD_ARROW_UP).unwrap();
    assert_eq!(sim.display().cursor_position().row(), 0);

    // Should still be in Insert mode
    assert_eq!(sim.mode(), Mode::Insert);
}

#[test]
fn test_typed_insert_input_matches_the_string_path() {
    use crate::helix::InsertInput;

    let keys = ["hi", CMD_ARROW_LEFT, "x", CMD_BACKSPACE, CMD_ESCAPE];
    let mut by_string = AnyModeSimulator::new("ab".to_string());
    let mut typed = AnyModeSimulator::new("ab".to_string());
    by_string.execute_command(CMD_INSERT).unwrap();
    typed.execute_command(CMD_INSERT).unwrap();

    for key in keys {
        by_string.execute_command(key).unwrap();
        typed.execute_insert(&InsertInput::parse(key)).unwrap();
        assert_eq!(typed.to_snapshot(), by_string.to_snapshot(), "{key}");
        assert_eq!(typed.mode(), by_string.mode(), "{key}");
    }
    assert_eq!(typed.mode(), Mode::Normal);
}

#[test]
fn test_typed_insert_input_in_normal_mode_runs_the_named_command() {
    use crate::helix::InsertInput;

    let mut sim = AnyModeSimulator::new("ab".to_string());
    sim.execute_insert(&InsertInput::parse(CMD_INSERT)).unwrap();
    assert_eq!(sim.mode(), Mode::Insert);
}
