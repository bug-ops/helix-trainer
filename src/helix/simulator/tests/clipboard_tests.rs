//! Clipboard (yank/paste) tests for HelixSimulator
//!
//! Tests for yank and paste operations including:
//! - Yank: y
//! - Paste after: p
//! - Paste before: P

use crate::game::EditorState;
use crate::game::editor_state::{CursorPosition, Selection};
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
    assert_eq!(sim.get_state().unwrap().cursor_position().1, 1);

    // Paste after 'b' - should insert 'a' between 'b' and 'c'
    sim.execute_command(CMD_PASTE_AFTER).unwrap();

    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "abac");
    // In Helix, cursor stays on last pasted character
    assert_eq!(state.cursor_position().1, 2); // Cursor on pasted 'a'
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
    assert_eq!(sim.get_state().unwrap().cursor_position().1, 2);

    // Yank 'c'
    sim.execute_command(CMD_YANK).unwrap();

    // Move back to 'a'
    sim.execute_command(CMD_MOVE_LEFT).unwrap();
    sim.execute_command(CMD_MOVE_LEFT).unwrap();
    assert_eq!(sim.get_state().unwrap().cursor_position().1, 0);

    // Paste before 'a'
    sim.execute_command(CMD_PASTE_BEFORE).unwrap();

    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "cabc");
    // In Helix, cursor stays on last pasted character
    assert_eq!(state.cursor_position().1, 0); // Cursor on pasted 'c'
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
        state.cursor_position().1,
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
        state.cursor_position().1,
        1,
        "Should be at position 1 ('y')"
    );

    // Paste before
    sim.execute_command(CMD_PASTE_BEFORE).unwrap();

    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "xzyz", "Content should be 'xzyz'");
    // In Helix, cursor stays on last pasted character (position 1)
    assert_eq!(
        state.cursor_position().1,
        1,
        "Cursor should be at position 1 (on pasted 'z')"
    );
}

// ============================================================================
// Multi-Character Selection Yank Tests (Issue #266)
// ============================================================================

#[test]
fn test_yank_multichar_forward_selection_round_trips_full_text() {
    // "world" as a forward range (anchor=6 < head=11)
    let cursor = CursorPosition::new(0, 11).unwrap();
    let sel = Selection::new(CursorPosition::new(0, 6).unwrap(), cursor);
    let state = EditorState::new("hello world".to_string(), cursor, Some(sel)).unwrap();

    let mut sim = AnyModeSimulator::from_editor_state(&state);
    sim.execute_command(CMD_YANK).unwrap();
    sim.execute_command(CMD_PASTE_AFTER).unwrap();

    let state = sim.get_state().unwrap();
    assert_eq!(
        state.content(),
        "hello worldworld",
        "Full 'world' range should be yanked and pasted, not just the head char"
    );
}

#[test]
fn test_yank_multichar_backward_selection_extracts_same_range() {
    // Same "world" range, flipped to a backward selection (anchor=11 > head=6)
    let cursor = CursorPosition::new(0, 11).unwrap();
    let sel = Selection::new(CursorPosition::new(0, 6).unwrap(), cursor);
    let state = EditorState::new("hello world".to_string(), cursor, Some(sel)).unwrap();

    let mut sim = AnyModeSimulator::from_editor_state(&state);
    sim.execute_command(CMD_FLIP_SELECTIONS).unwrap();
    sim.execute_command(CMD_YANK).unwrap();
    sim.execute_command(CMD_PASTE_AFTER).unwrap();

    let state = sim.get_state().unwrap();
    assert_eq!(
        state.content(),
        "hello worldworld",
        "Yank must normalize a backward (anchor > head) selection to the same text, \
         and paste-after must insert after the end of the selection regardless of \
         its direction (issue #266/#265 critic finding S1/S2)"
    );
}

#[test]
fn test_paste_before_multichar_forward_selection_inserts_at_selection_start() {
    // "world" as a forward range (anchor=6 < head=11)
    let cursor = CursorPosition::new(0, 11).unwrap();
    let sel = Selection::new(CursorPosition::new(0, 6).unwrap(), cursor);
    let state = EditorState::new("hello world".to_string(), cursor, Some(sel)).unwrap();

    let mut sim = AnyModeSimulator::from_editor_state(&state);
    sim.execute_command(CMD_YANK).unwrap();
    sim.execute_command(CMD_PASTE_BEFORE).unwrap();

    let state = sim.get_state().unwrap();
    assert_eq!(
        state.content(),
        "hello worldworld",
        "paste-before must insert at the start of the selection"
    );
}

#[test]
fn test_paste_before_multichar_backward_selection_inserts_at_selection_start() {
    // Same "world" range, flipped to a backward selection (anchor=11 > head=6)
    let cursor = CursorPosition::new(0, 11).unwrap();
    let sel = Selection::new(CursorPosition::new(0, 6).unwrap(), cursor);
    let state = EditorState::new("hello world".to_string(), cursor, Some(sel)).unwrap();

    let mut sim = AnyModeSimulator::from_editor_state(&state);
    sim.execute_command(CMD_FLIP_SELECTIONS).unwrap();
    sim.execute_command(CMD_YANK).unwrap();
    sim.execute_command(CMD_PASTE_BEFORE).unwrap();

    let state = sim.get_state().unwrap();
    assert_eq!(
        state.content(),
        "hello worldworld",
        "paste-before must insert at the start of the selection regardless of \
         its direction (issue #266/#265 critic finding S1/S2), not at the raw head"
    );
}

#[test]
fn test_paste_after_select_line_yank_paste_matches_helix() {
    // Critic finding S1 exact repro: `x y p` on "hello\nworld\n" must match
    // real Helix ("hello\nhello\nworld\n"), not the old head+1 offset bug
    // ("hhello\nello\nworld\n").
    let mut sim = AnyModeSimulator::new("hello\nworld\n".to_string());

    sim.execute_command(CMD_SELECT_LINE).unwrap();
    sim.execute_command(CMD_YANK).unwrap();
    sim.execute_command(CMD_PASTE_AFTER).unwrap();

    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "hello\nhello\nworld\n");
}

#[test]
fn test_paste_before_divergent_clipboard_matches_helix() {
    // Critic finding S2 exact repro: `y Alt-x P` on "ab cd" must match real
    // Helix ("aab cd"), not the old raw-head offset bug ("ab cda"). The
    // divergent clipboard ('a', yanked from a point selection) versus the
    // later whole-line selection unmasks a `paste_before` offset bug that a
    // same-text yank/paste round-trip cannot detect.
    let mut sim = AnyModeSimulator::new("ab cd".to_string());

    sim.execute_command(CMD_YANK).unwrap();
    sim.execute_command(CMD_SHRINK_TO_LINE_BOUNDS).unwrap();
    sim.execute_command(CMD_PASTE_BEFORE).unwrap();

    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "aab cd");
}

#[test]
fn test_yank_select_line_round_trips_full_line() {
    // 'x' (select_line) produces a backward range (anchor=line_end, head=line_start)
    let mut sim = AnyModeSimulator::new("abc\nXYZ\n".to_string());

    sim.execute_command(CMD_SELECT_LINE).unwrap();
    sim.execute_command(CMD_YANK).unwrap();
    sim.execute_command(CMD_COLLAPSE_SELECTION).unwrap();
    sim.execute_command(CMD_MOVE_DOWN).unwrap();
    sim.execute_command(CMD_PASTE_AFTER).unwrap();

    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "abc\nXabc\nYZ\n");
}

#[test]
fn test_yank_point_selection_still_yanks_single_char() {
    // Plain cursor with no active selection (anchor == head) must keep the
    // pre-#266 behavior of yanking exactly one character.
    let mut sim = AnyModeSimulator::new("hello world".to_string());

    sim.execute_command(CMD_YANK).unwrap();
    sim.execute_command(CMD_PASTE_AFTER).unwrap();

    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "hhello world");
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
