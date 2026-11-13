//! Repeat buffer recording tests for `HelixSimulator`
//!
//! Tests that verify commands are recorded correctly in the repeat buffer.
//! - Command recording: Verify editing commands are captured
//! - Insert mode recording: Verify text and movements are captured

use crate::grammar::commands::*;
use crate::helix::repeat::{InsertAction, InsertKey, Movement, RecordedInsert};
use crate::helix::simulator::AnyModeSimulator;

// ============================================================================
// Non-insert edits are never recorded (FR-066)
// ============================================================================

#[test]
fn test_repeat_buffer_ignores_non_insert_commands() {
    for (text, commands) in [
        ("hello", vec![CMD_DELETE_SELECTION]),
        (
            "line 1\nline 2",
            vec![CMD_SELECT_LINE, CMD_DELETE_SELECTION],
        ),
        ("hello", vec![CMD_YANK, CMD_PASTE_AFTER]),
        ("line 1\nline 2", vec![CMD_JOIN_LINES]),
        ("code", vec![CMD_INDENT]),
        ("hello", vec!["rx"]),
        ("hello", vec![CMD_MOVE_LEFT]),
        ("test", vec![CMD_DELETE_SELECTION, CMD_UNDO]),
    ] {
        let mut sim = AnyModeSimulator::new(text.to_string());
        for command in &commands {
            sim.execute_command(command).unwrap();
        }
        assert!(
            sim.repeat_buffer().is_empty(),
            "{commands:?} must not record a repeatable action"
        );
    }
}

// ============================================================================
// Insert Mode Recording Tests
// ============================================================================

#[test]
#[expect(
    clippy::assert_is_empty,
    reason = "assert!(x.is_empty()) reads better than the machine fix"
)]
fn test_insert_mode_recording_simple() {
    let mut sim = AnyModeSimulator::new("world".to_string());

    // Enter insert mode
    sim.execute_command(CMD_INSERT).unwrap();
    assert_eq!(sim.mode(), crate::helix::simulator::Mode::Insert);

    // Verify recording started
    assert!(sim.repeat_buffer().insert_recorder().is_recording());

    // Type text
    sim.execute_command(CMD_MOVE_LEFT).unwrap();
    sim.execute_command(CMD_INSERT).unwrap();

    // Exit insert mode
    sim.execute_command(CMD_ESCAPE).unwrap();
    assert_eq!(sim.mode(), crate::helix::simulator::Mode::Normal);

    // Verify insert sequence was recorded
    let buffer = sim.repeat_buffer();
    let Some(RecordedInsert { text, keys, .. }) = buffer.last_insert() else {
        panic!("expected a recorded Insert session");
    };
    assert_eq!(text, "hi");
    assert!(keys.is_empty());

    // Verify recording stopped
    assert!(!buffer.insert_recorder().is_recording());
}

#[test]
fn test_insert_mode_recording_with_movements() {
    let mut sim = AnyModeSimulator::new("test".to_string());

    // Enter insert mode
    sim.execute_command(CMD_INSERT).unwrap();

    // Type text with movements
    sim.execute_command(CMD_MOVE_LEFT).unwrap();
    sim.execute_command(CMD_INSERT).unwrap();
    sim.execute_command(CMD_ARROW_LEFT).unwrap();
    sim.execute_command(CMD_ARROW_LEFT).unwrap();
    sim.execute_command("!").unwrap();

    // Exit insert mode
    sim.execute_command(CMD_ESCAPE).unwrap();

    // Verify insert sequence with movements was recorded
    let buffer = sim.repeat_buffer();
    let Some(RecordedInsert { text, keys, .. }) = buffer.last_insert() else {
        panic!("expected a recorded Insert session");
    };
    assert_eq!(text, "hi!");
    let left = InsertAction::Move(Movement::Left);
    assert_eq!(
        keys,
        &[
            InsertKey {
                after_chars: 2,
                action: left
            },
            InsertKey {
                after_chars: 2,
                action: left
            },
        ]
    );
}

#[test]
fn test_insert_mode_recording_append() {
    let mut sim = AnyModeSimulator::new("hello".to_string());

    // Enter insert mode via append
    sim.execute_command(CMD_APPEND).unwrap();
    assert_eq!(sim.mode(), crate::helix::simulator::Mode::Insert);

    // Type text
    sim.execute_command(" ").unwrap();
    sim.execute_command(CMD_MOVE_WORD_FORWARD).unwrap();
    sim.execute_command(CMD_OPEN_BELOW).unwrap();
    sim.execute_command(CMD_REPLACE).unwrap();
    sim.execute_command(CMD_MOVE_RIGHT).unwrap();
    sim.execute_command("d").unwrap();

    // Exit insert mode
    sim.execute_command(CMD_ESCAPE).unwrap();

    // Verify recording
    let buffer = sim.repeat_buffer();
    let Some(RecordedInsert { text, .. }) = buffer.last_insert() else {
        panic!("expected a recorded Insert session");
    };
    assert_eq!(text, " world");
}

#[test]
fn test_insert_mode_recording_open_below() {
    let mut sim = AnyModeSimulator::new("line 1".to_string());

    // Enter insert mode via open below
    sim.execute_command(CMD_OPEN_BELOW).unwrap();
    assert_eq!(sim.mode(), crate::helix::simulator::Mode::Insert);

    // Type text
    sim.execute_command("n").unwrap();
    sim.execute_command(CMD_MOVE_WORD_END).unwrap();
    sim.execute_command(CMD_MOVE_WORD_FORWARD).unwrap();

    // Exit insert mode
    sim.execute_command(CMD_ESCAPE).unwrap();

    // Verify recording
    let buffer = sim.repeat_buffer();
    let Some(RecordedInsert { text, .. }) = buffer.last_insert() else {
        panic!("expected a recorded Insert session");
    };
    assert_eq!(text, "new");
}

#[test]
#[expect(
    clippy::assert_is_empty,
    reason = "assert!(x.is_empty()) reads better than the machine fix"
)]
fn test_insert_mode_empty_recording() {
    let mut sim = AnyModeSimulator::new("test".to_string());

    // Enter and immediately exit insert mode
    sim.execute_command(CMD_INSERT).unwrap();
    sim.execute_command(CMD_ESCAPE).unwrap();

    // Verify empty insert sequence was recorded
    let buffer = sim.repeat_buffer();
    let Some(RecordedInsert { text, keys, .. }) = buffer.last_insert() else {
        panic!("expected a recorded Insert session");
    };
    assert!(text.is_empty());
    assert!(keys.is_empty());
}

#[test]
fn test_non_insert_edit_keeps_the_earlier_insert_session() {
    let mut sim = AnyModeSimulator::new("test".to_string());
    sim.execute_command(CMD_INSERT).unwrap();
    sim.execute_command("Z").unwrap();
    sim.execute_command(CMD_ESCAPE).unwrap();

    sim.execute_command(CMD_SELECT_LINE).unwrap();
    sim.execute_command(CMD_DELETE_SELECTION).unwrap();

    let Some(RecordedInsert { text, .. }) = sim.repeat_buffer().last_insert() else {
        panic!("expected a recorded Insert session");
    };
    assert_eq!(text, "Z");
}

#[test]
fn test_insert_mode_after_normal_command_records_the_insert() {
    let mut sim = AnyModeSimulator::new("test".to_string());

    // Execute normal command first
    sim.execute_command(CMD_DELETE_SELECTION).unwrap();

    // Enter insert mode
    sim.execute_command(CMD_INSERT).unwrap();
    sim.execute_command(CMD_APPEND).unwrap();
    sim.execute_command(CMD_ESCAPE).unwrap();

    // Verify the insert session is what was recorded
    let buffer = sim.repeat_buffer();
    let Some(RecordedInsert { text, .. }) = buffer.last_insert() else {
        panic!("expected a recorded Insert session");
    };
    assert_eq!(text, "a");
}

#[test]
fn test_change_command_records_and_enters_insert() {
    let mut sim = AnyModeSimulator::new("hello".to_string());

    // Execute change command
    sim.execute_command(CMD_CHANGE).unwrap();
    assert_eq!(sim.mode(), crate::helix::simulator::Mode::Insert);

    // Verify recording started
    assert!(sim.repeat_buffer().insert_recorder().is_recording());

    // Type replacement text
    sim.execute_command(CMD_DELETE_SELECTION).unwrap();

    // Exit insert mode
    sim.execute_command(CMD_ESCAPE).unwrap();

    // Verify insert sequence was recorded
    let buffer = sim.repeat_buffer();
    let Some(RecordedInsert { text, .. }) = buffer.last_insert() else {
        panic!("expected a recorded Insert session");
    };
    // CMD_DELETE_SELECTION is "d", so it types "d" in insert mode
    assert_eq!(text, "d");
}
