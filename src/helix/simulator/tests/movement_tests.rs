//! Movement command tests for HelixSimulator
//!
//! Tests for cursor movement commands including:
//! - Basic movement: h, l, j, k
//! - Word movement: w, b, e
//! - Line movement: gh, gl, gs
//! - Document navigation: gg, ge

use crate::helix::commands::*;
use crate::helix::simulator::AnyModeSimulator;

// ============================================================================
// Basic Movement Tests (h, l, j, k)
// ============================================================================

#[test]
fn test_move_right() {
    let mut sim = AnyModeSimulator::new("hello".to_string());

    sim.execute_command(CMD_MOVE_RIGHT).unwrap();
    let state = sim.get_state().unwrap();
    assert_eq!(state.cursor_position().col, 1);

    sim.execute_command(CMD_MOVE_RIGHT).unwrap();
    let state = sim.get_state().unwrap();
    assert_eq!(state.cursor_position().col, 2);
}

#[test]
fn test_move_left() {
    let mut sim = AnyModeSimulator::new("hello".to_string());

    // Move right twice
    sim.execute_command(CMD_MOVE_RIGHT).unwrap();
    sim.execute_command(CMD_MOVE_RIGHT).unwrap();

    // Move left once
    sim.execute_command(CMD_MOVE_LEFT).unwrap();
    let state = sim.get_state().unwrap();
    assert_eq!(state.cursor_position().col, 1);
}

#[test]
fn test_move_down_up() {
    let mut sim = AnyModeSimulator::new("line1\nline2\nline3\n".to_string());

    sim.execute_command(CMD_MOVE_DOWN).unwrap();
    let state = sim.get_state().unwrap();
    assert_eq!(state.cursor_position().row, 1);

    sim.execute_command(CMD_MOVE_DOWN).unwrap();
    let state = sim.get_state().unwrap();
    assert_eq!(state.cursor_position().row, 2);

    sim.execute_command(CMD_MOVE_UP).unwrap();
    let state = sim.get_state().unwrap();
    assert_eq!(state.cursor_position().row, 1);
}

// ============================================================================
// Word Movement Tests (w, b, e)
// ============================================================================

#[test]
fn test_word_movement() {
    let mut sim = AnyModeSimulator::new("hello world foo".to_string());

    // Move to next word
    sim.execute_command(CMD_MOVE_WORD_FORWARD).unwrap();
    let state = sim.get_state().unwrap();
    // 'w' extends selection from current position to start of next word
    // helix-core: anchor=0, head=6 (start of "world")
    // get_state() returns head position for cursor
    assert_eq!(state.cursor_position().col, 6); // Start of "world"

    // Move to next word again
    sim.execute_command(CMD_MOVE_WORD_FORWARD).unwrap();
    let state = sim.get_state().unwrap();
    assert_eq!(state.cursor_position().col, 12); // Start of "foo"
}

#[test]
fn test_move_word_boundary() {
    let mut sim = AnyModeSimulator::new("  spaced  words  ".to_string());

    sim.execute_command(CMD_MOVE_WORD_FORWARD).unwrap();
    let state = sim.get_state().unwrap();
    // Should move to first non-space character of next word
    assert!(state.cursor_position().col > 0);
}

#[test]
fn test_move_word_end() {
    let mut sim = AnyModeSimulator::new("hello world".to_string());

    sim.execute_command(CMD_MOVE_WORD_END).unwrap();
    let state = sim.get_state().unwrap();
    // Should be at end of "hello"
    assert!(state.cursor_position().col >= 4 && state.cursor_position().col <= 5);
}

#[test]
fn test_move_prev_word() {
    let mut sim = AnyModeSimulator::new("hello world foo".to_string());

    // Move to end of line first (use 'gl' in Helix, not '$')
    sim.execute_command(CMD_GOTO_LINE_END).unwrap();
    // Then move to previous word
    sim.execute_command(CMD_MOVE_WORD_BACKWARD).unwrap();

    let state = sim.get_state().unwrap();
    // Should have moved to start of "foo" (col 12)
    assert_eq!(state.cursor_position().col, 12);
}

// ============================================================================
// Line Movement Tests (gh, gl, gs)
// ============================================================================

#[test]
fn test_move_line_start() {
    let mut sim = AnyModeSimulator::new("hello\nworld\n".to_string());

    // Move to next line
    sim.execute_command(CMD_MOVE_DOWN).unwrap();
    // Move to end of line (use 'gl' in Helix, not '$')
    sim.execute_command(CMD_GOTO_LINE_END).unwrap();
    let state = sim.get_state().unwrap();
    // Cursor at end of "world" - which is position 4 or 5
    assert!(state.cursor_position().col >= 4);

    // Move to start of line (use 'gh' in Helix, not '0')
    sim.execute_command(CMD_GOTO_LINE_START).unwrap();
    let state = sim.get_state().unwrap();
    assert_eq!(state.cursor_position().col, 0);
}

#[test]
fn test_goto_first_nonwhitespace() {
    // Test gs command - go to first non-whitespace character
    let mut sim = AnyModeSimulator::new("    fn main() {".to_string());

    // Move cursor to middle of line first
    for _ in 0..10 {
        sim.execute_command(CMD_MOVE_RIGHT).unwrap();
    }
    assert_eq!(sim.get_state().unwrap().cursor_position().col, 10);

    // Execute gs command
    sim.execute_command(CMD_GOTO_FIRST_NONWHITESPACE).unwrap();
    let state = sim.get_state().unwrap();
    // Should be at position 4 (first 'f' in "fn")
    assert_eq!(state.cursor_position().col, 4);
}

// ============================================================================
// Document Navigation Tests (gg, ge)
// ============================================================================

#[test]
fn test_document_start() {
    let mut sim = AnyModeSimulator::new("line1\nline2\nline3\n".to_string());

    // Move somewhere else
    sim.execute_command(CMD_MOVE_DOWN).unwrap();
    let state = sim.get_state().unwrap();
    assert_eq!(state.cursor_position().row, 1);

    // Go back to start
    sim.execute_command(CMD_GOTO_FILE_START).unwrap();
    let state = sim.get_state().unwrap();
    assert_eq!(state.cursor_position().row, 0);
    assert_eq!(state.cursor_position().col, 0);
}

#[test]
fn test_goto_last_line_ge_command() {
    // Test 'ge' command (goto last line)
    let mut sim = AnyModeSimulator::new("Line 1\nLine 2\nLine 3\nLast line".to_string());

    // Cursor starts at (0, 0)
    let state = sim.get_state().unwrap();
    assert_eq!(state.cursor_position().row, 0);
    assert_eq!(state.cursor_position().col, 0);

    // Execute 'ge' to go to last line
    let result = sim.execute_command(CMD_GOTO_LAST_LINE);
    assert!(result.is_ok(), "ge command should succeed: {:?}", result);

    let state = sim.get_state().unwrap();
    assert_eq!(
        state.cursor_position().row,
        3,
        "Should be on last line (row 3)"
    );
    assert_eq!(
        state.cursor_position().col,
        0,
        "Cursor should be at start of line"
    );
}
