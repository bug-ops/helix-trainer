//! Clipboard (yank/paste) tests for HelixSimulator
//!
//! Tests for yank and paste operations including:
//! - Yank: y
//! - Paste after: p
//! - Paste before: P

use crate::helix::commands::*;
use crate::helix::simulator::AnyModeSimulator;

// ============================================================================
// Yank and Paste After Tests
// ============================================================================

#[test]
fn test_yank_and_paste_after() {
    let mut sim = AnyModeSimulator::new("abc".to_string());

    // Yank 'a'
    sim.execute_command(CMD_YANK).unwrap();

    // Move to 'b'
    sim.execute_command(CMD_MOVE_RIGHT).unwrap();
    assert_eq!(sim.get_state().unwrap().cursor_position().col, 1);

    // Paste after 'b' - should insert 'a' between 'b' and 'c'
    sim.execute_command(CMD_PASTE_AFTER).unwrap();

    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "abac");
    // In Helix, cursor stays on last pasted character
    assert_eq!(state.cursor_position().col, 2); // Cursor on pasted 'a'
}

// ============================================================================
// Yank and Paste Before Tests
// ============================================================================

#[test]
fn test_yank_and_paste_before() {
    let mut sim = AnyModeSimulator::new("abc".to_string());

    // Move to 'c'
    sim.execute_command(CMD_MOVE_RIGHT).unwrap();
    sim.execute_command(CMD_MOVE_RIGHT).unwrap();
    assert_eq!(sim.get_state().unwrap().cursor_position().col, 2);

    // Yank 'c'
    sim.execute_command(CMD_YANK).unwrap();

    // Move back to 'a'
    sim.execute_command(CMD_MOVE_LEFT).unwrap();
    sim.execute_command(CMD_MOVE_LEFT).unwrap();
    assert_eq!(sim.get_state().unwrap().cursor_position().col, 0);

    // Paste before 'a'
    sim.execute_command(CMD_PASTE_BEFORE).unwrap();

    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "cabc");
    // In Helix, cursor stays on last pasted character
    assert_eq!(state.cursor_position().col, 0); // Cursor on pasted 'c'
}

// ============================================================================
// Paste Before Cursor Scenario Tests
// ============================================================================

#[test]
fn test_paste_before_cursor_scenario() {
    // Scenario: "Paste before cursor"
    // Setup: "xyz" with cursor at [0, 2] (on 'z')
    // Commands: y, h, P
    // Target: "xzyz" with cursor at [0, 1]

    let mut sim = AnyModeSimulator::new("xyz".to_string());

    // Move to position 2 ('z')
    sim.execute_command(CMD_MOVE_RIGHT).unwrap();
    sim.execute_command(CMD_MOVE_RIGHT).unwrap();

    let state = sim.get_state().unwrap();
    assert_eq!(
        state.cursor_position().col,
        2,
        "Should be at position 2 ('z')"
    );

    // Yank 'z'
    sim.execute_command(CMD_YANK).unwrap();

    // Check what was yanked
    let state = sim.get_state().unwrap();
    println!("After yank:");
    println!("  Cursor position: {:?}", state.cursor_position());

    // Move left to 'y'
    sim.execute_command(CMD_MOVE_LEFT).unwrap();

    let state = sim.get_state().unwrap();
    assert_eq!(
        state.cursor_position().col,
        1,
        "Should be at position 1 ('y')"
    );

    // Paste before
    sim.execute_command(CMD_PASTE_BEFORE).unwrap();

    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "xzyz", "Content should be 'xzyz'");
    // In Helix, cursor stays on last pasted character (position 1)
    assert_eq!(
        state.cursor_position().col,
        1,
        "Cursor should be at position 1 (on pasted 'z')"
    );
}

// ============================================================================
// Yank and Paste in Repeat Context
// ============================================================================

#[test]
fn test_repeat_yank_and_paste() {
    let mut sim = AnyModeSimulator::new("hello\nworld".to_string());

    // Yank first character
    sim.execute_command(CMD_YANK).unwrap();

    // Move down
    sim.execute_command(CMD_MOVE_DOWN).unwrap();

    // Paste
    sim.execute_command(CMD_PASTE_AFTER).unwrap();
    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "hello\nwhorld");

    // Repeat paste
    sim.execute_command(".").unwrap();
    let state = sim.get_state().unwrap();
    // The paste should repeat with the same clipboard content
    assert!(state.content().contains("hello"));
}
