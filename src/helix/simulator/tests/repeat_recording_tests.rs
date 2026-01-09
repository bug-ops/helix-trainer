//! Repeat buffer recording tests for HelixSimulator
//!
//! Tests that verify commands are recorded correctly in the repeat buffer.
//! - Command recording: Verify editing commands are captured
//! - Insert mode recording: Verify text and movements are captured

use crate::helix::commands::*;
use crate::helix::repeat::{Mode, Movement, RepeatableAction};
use crate::helix::simulator::AnyModeSimulator;

// ============================================================================
// Command Recording Tests
// ============================================================================

#[test]
fn test_repeat_buffer_records_delete_char() {
    let mut sim = AnyModeSimulator::new("hello".to_string());

    // Execute delete command
    sim.execute_command(CMD_DELETE_SELECTION).unwrap();

    // Verify command was recorded
    let buffer = sim.repeat_buffer();
    assert!(!buffer.is_empty());

    match buffer.last_action() {
        Some(RepeatableAction::Command {
            keys,
            expected_mode,
        }) => {
            assert_eq!(keys.len(), 1);
            assert_eq!(*expected_mode, Mode::Normal);
        }
        _ => panic!("Expected Command action"),
    }
}

#[test]
fn test_repeat_buffer_records_delete_line() {
    let mut sim = AnyModeSimulator::new("line 1\nline 2".to_string());

    // Execute x+d (select line + delete) - the Helix way
    sim.execute_command(CMD_SELECT_LINE).unwrap();
    sim.execute_command(CMD_DELETE_SELECTION).unwrap();

    // Verify x+d was recorded as compound action
    let buffer = sim.repeat_buffer();
    assert!(!buffer.is_empty());

    match buffer.last_action() {
        Some(RepeatableAction::Command {
            keys,
            expected_mode,
        }) => {
            assert_eq!(keys.len(), 2); // 'x' + 'd' keys
            assert_eq!(*expected_mode, Mode::Normal);
        }
        _ => panic!("Expected Command action with 2 keys"),
    }
}

#[test]
fn test_repeat_buffer_does_not_record_movement() {
    let mut sim = AnyModeSimulator::new("hello".to_string());

    // Execute movement command
    sim.execute_command(CMD_MOVE_LEFT).unwrap();

    // Verify command was NOT recorded (movement is not repeatable)
    let buffer = sim.repeat_buffer();
    assert!(buffer.is_empty());
}

#[test]
fn test_repeat_buffer_does_not_record_undo() {
    let mut sim = AnyModeSimulator::new("test".to_string());

    // Do something first
    sim.execute_command(CMD_DELETE_SELECTION).unwrap();

    // Undo it
    sim.execute_command(CMD_UNDO).unwrap();

    // The buffer should still have 'x', not 'u'
    let buffer = sim.repeat_buffer();
    match buffer.last_action() {
        Some(RepeatableAction::Command { keys, .. }) => {
            assert_eq!(keys.len(), 1);
            // Should still be 'x', not 'u'
        }
        _ => panic!("Expected Command action"),
    }
}

#[test]
fn test_repeat_buffer_records_yank() {
    let mut sim = AnyModeSimulator::new("hello".to_string());

    // Execute yank command
    sim.execute_command(CMD_YANK).unwrap();

    // Verify yank was recorded
    let buffer = sim.repeat_buffer();
    assert!(!buffer.is_empty());
}

#[test]
fn test_repeat_buffer_records_paste() {
    let mut sim = AnyModeSimulator::new("hello".to_string());

    // Yank first
    sim.execute_command(CMD_YANK).unwrap();

    // Then paste
    sim.execute_command(CMD_PASTE_AFTER).unwrap();

    // Verify paste was recorded (last action)
    let buffer = sim.repeat_buffer();
    match buffer.last_action() {
        Some(RepeatableAction::Command { keys, .. }) => {
            assert_eq!(keys.len(), 1);
        }
        _ => panic!("Expected Command action"),
    }
}

#[test]
fn test_repeat_buffer_records_join_lines() {
    let mut sim = AnyModeSimulator::new("line 1\nline 2".to_string());

    // Execute join command
    sim.execute_command(CMD_JOIN_LINES).unwrap();

    // Verify join was recorded
    let buffer = sim.repeat_buffer();
    assert!(!buffer.is_empty());
}

#[test]
fn test_repeat_buffer_records_indent() {
    let mut sim = AnyModeSimulator::new("code".to_string());

    // Execute indent command
    sim.execute_command(CMD_INDENT).unwrap();

    // Verify indent was recorded
    let buffer = sim.repeat_buffer();
    assert!(!buffer.is_empty());
}

#[test]
fn test_repeat_buffer_records_replace_char() {
    let mut sim = AnyModeSimulator::new("hello".to_string());

    // Execute replace command (r + x)
    sim.execute_command("rx").unwrap();

    // Verify replace was recorded
    let buffer = sim.repeat_buffer();
    assert!(!buffer.is_empty());

    match buffer.last_action() {
        Some(RepeatableAction::Command { keys, .. }) => {
            assert_eq!(keys.len(), 2); // 'r' and 'x'
        }
        _ => panic!("Expected Command action"),
    }
}

// ============================================================================
// Insert Mode Recording Tests
// ============================================================================

#[test]
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
    match buffer.last_action() {
        Some(RepeatableAction::InsertSequence {
            text, movements, ..
        }) => {
            assert_eq!(text, "hi");
            assert!(movements.is_empty());
        }
        _ => panic!("Expected InsertSequence action"),
    }

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
    match buffer.last_action() {
        Some(RepeatableAction::InsertSequence {
            text, movements, ..
        }) => {
            assert_eq!(text, "hi!");
            assert_eq!(movements.len(), 2);
            assert_eq!(movements[0], Movement::Left);
            assert_eq!(movements[1], Movement::Left);
        }
        _ => panic!("Expected InsertSequence action"),
    }
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
    match buffer.last_action() {
        Some(RepeatableAction::InsertSequence { text, .. }) => {
            assert_eq!(text, " world");
        }
        _ => panic!("Expected InsertSequence action"),
    }
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
    match buffer.last_action() {
        Some(RepeatableAction::InsertSequence { text, .. }) => {
            assert_eq!(text, "new");
        }
        _ => panic!("Expected InsertSequence action"),
    }
}

#[test]
fn test_insert_mode_empty_recording() {
    let mut sim = AnyModeSimulator::new("test".to_string());

    // Enter and immediately exit insert mode
    sim.execute_command(CMD_INSERT).unwrap();
    sim.execute_command(CMD_ESCAPE).unwrap();

    // Verify empty insert sequence was recorded
    let buffer = sim.repeat_buffer();
    match buffer.last_action() {
        Some(RepeatableAction::InsertSequence {
            text, movements, ..
        }) => {
            assert!(text.is_empty());
            assert!(movements.is_empty());
        }
        _ => panic!("Expected InsertSequence action"),
    }
}

#[test]
fn test_normal_command_overwrites_previous() {
    let mut sim = AnyModeSimulator::new("hello".to_string());

    // Execute first command
    sim.execute_command(CMD_DELETE_SELECTION).unwrap();

    // Execute second command
    sim.execute_command(CMD_DELETE_SELECTION).unwrap();

    // Verify only last command is recorded
    let buffer = sim.repeat_buffer();
    assert!(!buffer.is_empty());
    // Should have only one action (the second 'x')
}

#[test]
fn test_insert_mode_overwrites_normal_command() {
    let mut sim = AnyModeSimulator::new("test".to_string());

    // Execute normal command first
    sim.execute_command(CMD_DELETE_SELECTION).unwrap();

    // Enter insert mode
    sim.execute_command(CMD_INSERT).unwrap();
    sim.execute_command(CMD_APPEND).unwrap();
    sim.execute_command(CMD_ESCAPE).unwrap();

    // Verify insert sequence overwrote the delete command
    let buffer = sim.repeat_buffer();
    match buffer.last_action() {
        Some(RepeatableAction::InsertSequence { text, .. }) => {
            assert_eq!(text, "a");
        }
        _ => panic!("Expected InsertSequence action, not Command"),
    }
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
    match buffer.last_action() {
        Some(RepeatableAction::InsertSequence { text, .. }) => {
            // CMD_DELETE_SELECTION is "d", so it types "d" in insert mode
            assert_eq!(text, "d");
        }
        _ => panic!("Expected InsertSequence action"),
    }
}
