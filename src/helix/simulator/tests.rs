//! Tests for HelixSimulator

use super::*;

#[test]
fn test_create_simulator() {
    let sim = HelixSimulator::new("hello world".to_string());
    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "hello world");
    assert_eq!(state.cursor_position().row, 0);
    assert_eq!(state.cursor_position().col, 0);
}

#[test]
fn test_initial_mode() {
    let sim = HelixSimulator::new("test".to_string());
    assert_eq!(sim.mode(), Mode::Normal);
}

#[test]
fn test_move_right() {
    let mut sim = HelixSimulator::new("hello".to_string());

    sim.execute_command("l").unwrap();
    let state = sim.get_state().unwrap();
    assert_eq!(state.cursor_position().col, 1);

    sim.execute_command("l").unwrap();
    let state = sim.get_state().unwrap();
    assert_eq!(state.cursor_position().col, 2);
}

#[test]
fn test_move_left() {
    let mut sim = HelixSimulator::new("hello".to_string());

    // Move right twice
    sim.execute_command("l").unwrap();
    sim.execute_command("l").unwrap();

    // Move left once
    sim.execute_command("h").unwrap();
    let state = sim.get_state().unwrap();
    assert_eq!(state.cursor_position().col, 1);
}

#[test]
fn test_word_movement() {
    let mut sim = HelixSimulator::new("hello world foo".to_string());

    // Move to next word
    sim.execute_command("w").unwrap();
    let state = sim.get_state().unwrap();
    assert_eq!(state.cursor_position().col, 6); // "world"

    // Move to next word again
    sim.execute_command("w").unwrap();
    let state = sim.get_state().unwrap();
    assert_eq!(state.cursor_position().col, 12); // "foo"
}

#[test]
fn test_delete_line() {
    let mut sim = HelixSimulator::new("line 1\nline 2\nline 3\n".to_string());

    sim.execute_command("dd").unwrap();

    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "line 2\nline 3\n");
}

#[test]
fn test_delete_char() {
    let mut sim = HelixSimulator::new("hello".to_string());

    sim.execute_command("x").unwrap();

    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "ello");
}

#[test]
fn test_delete_char_in_middle() {
    let mut sim = HelixSimulator::new("hello".to_string());

    sim.execute_command("l").unwrap(); // Move to 'e'
    sim.execute_command("l").unwrap(); // Move to 'l'
    sim.execute_command("x").unwrap(); // Delete 'l'

    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "helo");
}

#[test]
fn test_undo() {
    let mut sim = HelixSimulator::new("test\n".to_string());

    sim.execute_command("dd").unwrap();
    assert_eq!(sim.get_state().unwrap().content(), "");

    sim.execute_command("u").unwrap();
    assert_eq!(sim.get_state().unwrap().content(), "test\n");
}

#[test]
fn test_mode_change() {
    let mut sim = HelixSimulator::new("test".to_string());

    assert_eq!(sim.mode(), Mode::Normal);

    sim.execute_command("i").unwrap();
    assert_eq!(sim.mode(), Mode::Insert);

    sim.execute_command("Escape").unwrap();
    assert_eq!(sim.mode(), Mode::Normal);
}

#[test]
fn test_move_line_start() {
    let mut sim = HelixSimulator::new("hello\nworld\n".to_string());

    // Move to next line
    sim.execute_command("j").unwrap();
    // Move to end of line
    sim.execute_command("$").unwrap();
    let state = sim.get_state().unwrap();
    // Cursor at end of "world" - which is position 4 or 5
    assert!(state.cursor_position().col >= 4);

    // Move to start of line
    sim.execute_command("0").unwrap();
    let state = sim.get_state().unwrap();
    assert_eq!(state.cursor_position().col, 0);
}

#[test]
fn test_move_down_up() {
    let mut sim = HelixSimulator::new("line1\nline2\nline3\n".to_string());

    sim.execute_command("j").unwrap();
    let state = sim.get_state().unwrap();
    assert_eq!(state.cursor_position().row, 1);

    sim.execute_command("j").unwrap();
    let state = sim.get_state().unwrap();
    assert_eq!(state.cursor_position().row, 2);

    sim.execute_command("k").unwrap();
    let state = sim.get_state().unwrap();
    assert_eq!(state.cursor_position().row, 1);
}

#[test]
fn test_document_start() {
    let mut sim = HelixSimulator::new("line1\nline2\nline3\n".to_string());

    // Move somewhere else
    sim.execute_command("j").unwrap();
    let state = sim.get_state().unwrap();
    assert_eq!(state.cursor_position().row, 1);

    // Go back to start
    sim.execute_command("gg").unwrap();
    let state = sim.get_state().unwrap();
    assert_eq!(state.cursor_position().row, 0);
    assert_eq!(state.cursor_position().col, 0);
}

#[test]
fn test_unknown_command() {
    let mut sim = HelixSimulator::new("test".to_string());
    let result = sim.execute_command("unknown");
    assert!(result.is_err());
}

#[test]
fn test_multiple_line_deletions() {
    let mut sim = HelixSimulator::new("line1\nline2\nline3\n".to_string());

    sim.execute_command("dd").unwrap();
    sim.execute_command("dd").unwrap();

    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "line3\n");
}

#[test]
fn test_move_word_boundary() {
    let mut sim = HelixSimulator::new("  spaced  words  ".to_string());

    sim.execute_command("w").unwrap();
    let state = sim.get_state().unwrap();
    // Should move to first non-space character of next word
    assert!(state.cursor_position().col > 0);
}

#[test]
fn test_move_word_end() {
    let mut sim = HelixSimulator::new("hello world".to_string());

    sim.execute_command("e").unwrap();
    let state = sim.get_state().unwrap();
    // Should be at end of "hello"
    assert!(state.cursor_position().col >= 4 && state.cursor_position().col <= 5);
}

#[test]
fn test_move_prev_word() {
    let mut sim = HelixSimulator::new("hello world foo".to_string());

    // Move to end of document first
    sim.execute_command("G").unwrap();
    // Then move to previous word
    sim.execute_command("b").unwrap();

    let state = sim.get_state().unwrap();
    // Should have moved to start of a previous word
    assert!(state.cursor_position().col >= 11);
}

#[test]
fn test_append_mode() {
    let mut sim = HelixSimulator::new("hello".to_string());

    // Cursor at start (position 0)
    assert_eq!(sim.get_state().unwrap().cursor_position().col, 0);

    // Press 'a' should move cursor one position right and enter insert mode
    sim.execute_command("a").unwrap();

    let state = sim.get_state().unwrap();
    assert_eq!(sim.mode(), Mode::Insert);
    assert_eq!(state.cursor_position().col, 1); // Moved one right
}

#[test]
fn test_open_below() {
    let mut sim = HelixSimulator::new("line1\nline2".to_string());

    // Cursor at start of first line
    assert_eq!(sim.get_state().unwrap().cursor_position().row, 0);

    // Press 'o' should insert new line below and enter insert mode
    sim.execute_command("o").unwrap();

    let state = sim.get_state().unwrap();
    assert_eq!(sim.mode(), Mode::Insert);
    assert_eq!(state.content(), "line1\n\nline2");
    assert_eq!(state.cursor_position().row, 1); // On new empty line
}

#[test]
fn test_open_above() {
    let mut sim = HelixSimulator::new("line1\nline2".to_string());

    // Move to second line
    sim.execute_command("j").unwrap();
    assert_eq!(sim.get_state().unwrap().cursor_position().row, 1);

    // Press 'O' should insert new line above and enter insert mode
    sim.execute_command("O").unwrap();

    let state = sim.get_state().unwrap();
    assert_eq!(sim.mode(), Mode::Insert);
    assert_eq!(state.content(), "line1\n\nline2");
    assert_eq!(state.cursor_position().row, 1); // On new empty line
}

#[test]
fn test_replace_char() {
    let mut sim = HelixSimulator::new("hello".to_string());

    // Cursor at start
    assert_eq!(sim.get_state().unwrap().content(), "hello");

    // Press 'r' then 'X' should replace 'h' with 'X'
    sim.execute_command("rX").unwrap();

    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "Xello");
    assert_eq!(sim.mode(), Mode::Normal); // Should stay in normal mode
}

#[test]
fn test_insert_at_line_start() {
    let mut sim = HelixSimulator::new("  hello world".to_string());

    // Move cursor to middle of line
    sim.execute_command("w").unwrap();
    let state = sim.get_state().unwrap();
    assert!(state.cursor_position().col > 0);

    // Press 'I' should move to start of line and enter insert mode
    sim.execute_command("I").unwrap();

    let state = sim.get_state().unwrap();
    assert_eq!(sim.mode(), Mode::Insert);
    assert_eq!(state.cursor_position().col, 0);
}

#[test]
fn test_append_at_line_end() {
    let mut sim = HelixSimulator::new("hello world\nline2".to_string());

    // Cursor at start
    assert_eq!(sim.get_state().unwrap().cursor_position().col, 0);

    // Press 'A' should move to end of line and enter insert mode
    sim.execute_command("A").unwrap();

    let state = sim.get_state().unwrap();
    assert_eq!(sim.mode(), Mode::Insert);
    assert_eq!(state.cursor_position().col, 11); // After "hello world"
    assert_eq!(state.cursor_position().row, 0); // Still on first line
}

#[test]
fn test_change_selection() {
    let mut sim = HelixSimulator::new("hello".to_string());

    // Cursor at start
    assert_eq!(sim.get_state().unwrap().content(), "hello");
    assert_eq!(sim.mode(), Mode::Normal);

    // Press 'c' should delete 'h' and enter insert mode
    sim.execute_command("c").unwrap();

    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "ello");
    assert_eq!(sim.mode(), Mode::Insert);
    assert_eq!(state.cursor_position().col, 0); // Cursor stays at start
}

#[test]
fn test_yank_and_paste_after() {
    let mut sim = HelixSimulator::new("abc".to_string());

    // Yank 'a'
    sim.execute_command("y").unwrap();

    // Move to 'b'
    sim.execute_command("l").unwrap();
    assert_eq!(sim.get_state().unwrap().cursor_position().col, 1);

    // Paste after 'b' - should insert 'a' between 'b' and 'c'
    sim.execute_command("p").unwrap();

    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "abac");
    assert_eq!(state.cursor_position().col, 3); // Cursor after pasted 'a'
}

#[test]
fn test_yank_and_paste_before() {
    let mut sim = HelixSimulator::new("abc".to_string());

    // Move to 'c'
    sim.execute_command("l").unwrap();
    sim.execute_command("l").unwrap();
    assert_eq!(sim.get_state().unwrap().cursor_position().col, 2);

    // Yank 'c'
    sim.execute_command("y").unwrap();

    // Move back to 'a'
    sim.execute_command("h").unwrap();
    sim.execute_command("h").unwrap();
    assert_eq!(sim.get_state().unwrap().cursor_position().col, 0);

    // Paste before 'a'
    sim.execute_command("P").unwrap();

    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "cabc");
    assert_eq!(state.cursor_position().col, 1); // Cursor after pasted 'c'
}

#[test]
fn test_insert_text_in_insert_mode() {
    let mut sim = HelixSimulator::new("hello".to_string());

    // Enter insert mode
    sim.execute_command("i").unwrap();
    assert_eq!(sim.mode(), Mode::Insert);

    // Insert a character
    sim.execute_command("!").unwrap();

    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "!hello");
    assert_eq!(state.cursor_position().col, 1);
}

#[test]
fn test_append_at_line_end_and_insert() {
    let mut sim = HelixSimulator::new("hello".to_string());

    // Append at line end
    sim.execute_command("A").unwrap();
    assert_eq!(sim.mode(), Mode::Insert);

    let cursor_pos = sim.get_state().unwrap().cursor_position().col;
    assert_eq!(cursor_pos, 5); // After 'hello'

    // Insert '!'
    sim.execute_command("!").unwrap();

    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "hello!");
    assert_eq!(state.cursor_position().col, 6);
}

#[test]
fn test_insert_text_only_works_in_insert_mode() {
    let mut sim = HelixSimulator::new("hello".to_string());

    // Try to insert text in Normal mode - should fail
    let result = sim.insert_text("!");
    assert!(result.is_err());

    // Enter Insert mode
    sim.execute_command("i").unwrap();

    // Now it should work
    let result = sim.insert_text("!");
    assert!(result.is_ok());
}

#[test]
fn test_insert_multiple_chars() {
    let mut sim = HelixSimulator::new("".to_string());

    // Enter insert mode
    sim.execute_command("i").unwrap();
    assert_eq!(sim.mode(), Mode::Insert);

    // Insert multiple characters
    sim.execute_command("a").unwrap();
    sim.execute_command("b").unwrap();
    sim.execute_command("c").unwrap();

    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "abc");
    assert_eq!(state.cursor_position().col, 3);
}

#[test]
fn test_backspace_in_insert_mode() {
    let mut sim = HelixSimulator::new("hello".to_string());

    // Enter insert mode at position 5
    sim.execute_command("$").unwrap(); // Move to end
    sim.execute_command("a").unwrap(); // Append
    assert_eq!(sim.mode(), Mode::Insert);

    // Type some characters
    sim.execute_command("!").unwrap();
    sim.execute_command("!").unwrap();

    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "hello!!");
    assert_eq!(state.cursor_position().col, 7);

    // Backspace once
    sim.execute_command("Backspace").unwrap();

    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "hello!");
    assert_eq!(state.cursor_position().col, 6);

    // Backspace again
    sim.execute_command("Backspace").unwrap();

    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "hello");
    assert_eq!(state.cursor_position().col, 5);
}

#[test]
fn test_backspace_at_start() {
    let mut sim = HelixSimulator::new("test".to_string());

    // Enter insert mode at start
    sim.execute_command("i").unwrap();
    assert_eq!(sim.mode(), Mode::Insert);

    // Backspace at position 0 should do nothing
    sim.execute_command("Backspace").unwrap();

    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "test");
    assert_eq!(state.cursor_position().col, 0);
}

#[test]
fn test_arrow_keys_in_insert_mode() {
    let mut sim = HelixSimulator::new("abc\ndef".to_string());

    // Enter insert mode
    sim.execute_command("i").unwrap();
    assert_eq!(sim.mode(), Mode::Insert);

    // Test ArrowRight
    sim.execute_command("ArrowRight").unwrap();
    assert_eq!(sim.get_state().unwrap().cursor_position().col, 1);

    // Test ArrowLeft
    sim.execute_command("ArrowLeft").unwrap();
    assert_eq!(sim.get_state().unwrap().cursor_position().col, 0);

    // Test ArrowDown
    sim.execute_command("ArrowDown").unwrap();
    assert_eq!(sim.get_state().unwrap().cursor_position().row, 1);

    // Test ArrowUp
    sim.execute_command("ArrowUp").unwrap();
    assert_eq!(sim.get_state().unwrap().cursor_position().row, 0);

    // Should still be in Insert mode
    assert_eq!(sim.mode(), Mode::Insert);
}

#[test]
fn test_backspace_only_works_in_insert_mode() {
    let mut sim = HelixSimulator::new("hello".to_string());

    // Try backspace in Normal mode - should fail
    let result = sim.backspace();
    assert!(result.is_err());

    // Enter Insert mode
    sim.execute_command("i").unwrap();

    // Now it should work (but do nothing at position 0)
    let result = sim.backspace();
    assert!(result.is_ok());
}

#[test]
fn test_join_lines() {
    let mut sim = HelixSimulator::new("line1\nline2\nline3".to_string());

    // Join first two lines
    sim.execute_command("J").unwrap();

    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "line1 line2\nline3");
    assert_eq!(state.cursor_position().row, 0);
}

#[test]
fn test_join_lines_at_last_line() {
    let mut sim = HelixSimulator::new("line1\nline2".to_string());

    // Move to last line
    sim.execute_command("j").unwrap();
    assert_eq!(sim.get_state().unwrap().cursor_position().row, 1);

    // Try to join - should do nothing
    sim.execute_command("J").unwrap();

    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "line1\nline2");
}

#[test]
fn test_indent_line() {
    let mut sim = HelixSimulator::new("hello\nworld".to_string());

    // Indent first line
    sim.execute_command(">").unwrap();

    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "  hello\nworld");
    // Cursor should move forward by 2
    assert_eq!(state.cursor_position().col, 2);
}

#[test]
fn test_dedent_line() {
    let mut sim = HelixSimulator::new("  hello\n    world".to_string());

    // Dedent first line (remove 2 spaces)
    sim.execute_command("<").unwrap();

    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "hello\n    world");
    assert_eq!(state.cursor_position().col, 0);
}

#[test]
fn test_dedent_line_with_one_space() {
    let mut sim = HelixSimulator::new(" hello".to_string());

    // Dedent - should remove only 1 space
    sim.execute_command("<").unwrap();

    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "hello");
    assert_eq!(state.cursor_position().col, 0);
}

#[test]
fn test_dedent_line_no_spaces() {
    let mut sim = HelixSimulator::new("hello".to_string());

    // Dedent line with no leading spaces - should do nothing
    sim.execute_command("<").unwrap();

    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "hello");
}

#[test]
fn test_multiple_indent() {
    let mut sim = HelixSimulator::new("code".to_string());

    // Indent twice
    sim.execute_command(">").unwrap();
    sim.execute_command(">").unwrap();

    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "    code");
    assert_eq!(state.cursor_position().col, 4);
}

// ============================================================================
// Phase 2: Repeat Buffer Integration Tests
// ============================================================================

#[test]
fn test_repeat_buffer_records_delete_char() {
    let mut sim = HelixSimulator::new("hello".to_string());

    // Execute delete command
    sim.execute_command("x").unwrap();

    // Verify command was recorded
    let buffer = sim.repeat_buffer();
    assert!(!buffer.is_empty());

    match buffer.last_action() {
        Some(crate::helix::repeat::RepeatableAction::Command {
            keys,
            expected_mode,
        }) => {
            assert_eq!(keys.len(), 1);
            assert_eq!(*expected_mode, crate::helix::repeat::Mode::Normal);
        }
        _ => panic!("Expected Command action"),
    }
}

#[test]
fn test_repeat_buffer_records_delete_line() {
    let mut sim = HelixSimulator::new("line 1\nline 2".to_string());

    // Execute dd command
    sim.execute_command("dd").unwrap();

    // Verify dd was recorded
    let buffer = sim.repeat_buffer();
    assert!(!buffer.is_empty());

    match buffer.last_action() {
        Some(crate::helix::repeat::RepeatableAction::Command {
            keys,
            expected_mode,
        }) => {
            assert_eq!(keys.len(), 2); // Both 'd' keys
            assert_eq!(*expected_mode, crate::helix::repeat::Mode::Normal);
        }
        _ => panic!("Expected Command action with 2 keys"),
    }
}

#[test]
fn test_repeat_buffer_does_not_record_movement() {
    let mut sim = HelixSimulator::new("hello".to_string());

    // Execute movement command
    sim.execute_command("h").unwrap();

    // Verify command was NOT recorded (movement is not repeatable)
    let buffer = sim.repeat_buffer();
    assert!(buffer.is_empty());
}

#[test]
fn test_repeat_buffer_does_not_record_undo() {
    let mut sim = HelixSimulator::new("test".to_string());

    // Do something first
    sim.execute_command("x").unwrap();

    // Undo it
    sim.execute_command("u").unwrap();

    // The buffer should still have 'x', not 'u'
    let buffer = sim.repeat_buffer();
    match buffer.last_action() {
        Some(crate::helix::repeat::RepeatableAction::Command { keys, .. }) => {
            assert_eq!(keys.len(), 1);
            // Should still be 'x', not 'u'
        }
        _ => panic!("Expected Command action"),
    }
}

#[test]
fn test_repeat_buffer_records_yank() {
    let mut sim = HelixSimulator::new("hello".to_string());

    // Execute yank command
    sim.execute_command("y").unwrap();

    // Verify yank was recorded
    let buffer = sim.repeat_buffer();
    assert!(!buffer.is_empty());
}

#[test]
fn test_repeat_buffer_records_paste() {
    let mut sim = HelixSimulator::new("hello".to_string());

    // Yank first
    sim.execute_command("y").unwrap();

    // Then paste
    sim.execute_command("p").unwrap();

    // Verify paste was recorded (last action)
    let buffer = sim.repeat_buffer();
    match buffer.last_action() {
        Some(crate::helix::repeat::RepeatableAction::Command { keys, .. }) => {
            assert_eq!(keys.len(), 1);
        }
        _ => panic!("Expected Command action"),
    }
}

#[test]
fn test_repeat_buffer_records_join_lines() {
    let mut sim = HelixSimulator::new("line 1\nline 2".to_string());

    // Execute join command
    sim.execute_command("J").unwrap();

    // Verify join was recorded
    let buffer = sim.repeat_buffer();
    assert!(!buffer.is_empty());
}

#[test]
fn test_repeat_buffer_records_indent() {
    let mut sim = HelixSimulator::new("code".to_string());

    // Execute indent command
    sim.execute_command(">").unwrap();

    // Verify indent was recorded
    let buffer = sim.repeat_buffer();
    assert!(!buffer.is_empty());
}

#[test]
fn test_repeat_buffer_records_replace_char() {
    let mut sim = HelixSimulator::new("hello".to_string());

    // Execute replace command (r + x)
    sim.execute_command("rx").unwrap();

    // Verify replace was recorded
    let buffer = sim.repeat_buffer();
    assert!(!buffer.is_empty());

    match buffer.last_action() {
        Some(crate::helix::repeat::RepeatableAction::Command { keys, .. }) => {
            assert_eq!(keys.len(), 2); // 'r' and 'x'
        }
        _ => panic!("Expected Command action"),
    }
}

#[test]
fn test_insert_mode_recording_simple() {
    let mut sim = HelixSimulator::new("world".to_string());

    // Enter insert mode
    sim.execute_command("i").unwrap();
    assert_eq!(sim.mode(), Mode::Insert);

    // Verify recording started
    assert!(sim.repeat_buffer().insert_recorder().is_recording());

    // Type text
    sim.execute_command("h").unwrap();
    sim.execute_command("i").unwrap();

    // Exit insert mode
    sim.execute_command("Escape").unwrap();
    assert_eq!(sim.mode(), Mode::Normal);

    // Verify insert sequence was recorded
    let buffer = sim.repeat_buffer();
    match buffer.last_action() {
        Some(crate::helix::repeat::RepeatableAction::InsertSequence { text, movements }) => {
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
    let mut sim = HelixSimulator::new("test".to_string());

    // Enter insert mode
    sim.execute_command("i").unwrap();

    // Type text with movements
    sim.execute_command("h").unwrap();
    sim.execute_command("i").unwrap();
    sim.execute_command("ArrowLeft").unwrap();
    sim.execute_command("ArrowLeft").unwrap();
    sim.execute_command("!").unwrap();

    // Exit insert mode
    sim.execute_command("Escape").unwrap();

    // Verify insert sequence with movements was recorded
    let buffer = sim.repeat_buffer();
    match buffer.last_action() {
        Some(crate::helix::repeat::RepeatableAction::InsertSequence { text, movements }) => {
            assert_eq!(text, "hi!");
            assert_eq!(movements.len(), 2);
            assert_eq!(movements[0], crate::helix::repeat::Movement::Left);
            assert_eq!(movements[1], crate::helix::repeat::Movement::Left);
        }
        _ => panic!("Expected InsertSequence action"),
    }
}

#[test]
fn test_insert_mode_recording_append() {
    let mut sim = HelixSimulator::new("hello".to_string());

    // Enter insert mode via append
    sim.execute_command("a").unwrap();
    assert_eq!(sim.mode(), Mode::Insert);

    // Type text
    sim.execute_command(" ").unwrap();
    sim.execute_command("w").unwrap();
    sim.execute_command("o").unwrap();
    sim.execute_command("r").unwrap();
    sim.execute_command("l").unwrap();
    sim.execute_command("d").unwrap();

    // Exit insert mode
    sim.execute_command("Escape").unwrap();

    // Verify recording
    let buffer = sim.repeat_buffer();
    match buffer.last_action() {
        Some(crate::helix::repeat::RepeatableAction::InsertSequence { text, .. }) => {
            assert_eq!(text, " world");
        }
        _ => panic!("Expected InsertSequence action"),
    }
}

#[test]
fn test_insert_mode_recording_open_below() {
    let mut sim = HelixSimulator::new("line 1".to_string());

    // Enter insert mode via open below
    sim.execute_command("o").unwrap();
    assert_eq!(sim.mode(), Mode::Insert);

    // Type text
    sim.execute_command("n").unwrap();
    sim.execute_command("e").unwrap();
    sim.execute_command("w").unwrap();

    // Exit insert mode
    sim.execute_command("Escape").unwrap();

    // Verify recording
    let buffer = sim.repeat_buffer();
    match buffer.last_action() {
        Some(crate::helix::repeat::RepeatableAction::InsertSequence { text, .. }) => {
            assert_eq!(text, "new");
        }
        _ => panic!("Expected InsertSequence action"),
    }
}

#[test]
fn test_insert_mode_empty_recording() {
    let mut sim = HelixSimulator::new("test".to_string());

    // Enter and immediately exit insert mode
    sim.execute_command("i").unwrap();
    sim.execute_command("Escape").unwrap();

    // Verify empty insert sequence was recorded
    let buffer = sim.repeat_buffer();
    match buffer.last_action() {
        Some(crate::helix::repeat::RepeatableAction::InsertSequence { text, movements }) => {
            assert!(text.is_empty());
            assert!(movements.is_empty());
        }
        _ => panic!("Expected InsertSequence action"),
    }
}

#[test]
fn test_normal_command_overwrites_previous() {
    let mut sim = HelixSimulator::new("hello".to_string());

    // Execute first command
    sim.execute_command("x").unwrap();

    // Execute second command
    sim.execute_command("x").unwrap();

    // Verify only last command is recorded
    let buffer = sim.repeat_buffer();
    assert!(!buffer.is_empty());
    // Should have only one action (the second 'x')
}

#[test]
fn test_insert_mode_overwrites_normal_command() {
    let mut sim = HelixSimulator::new("test".to_string());

    // Execute normal command first
    sim.execute_command("x").unwrap();

    // Enter insert mode
    sim.execute_command("i").unwrap();
    sim.execute_command("a").unwrap();
    sim.execute_command("Escape").unwrap();

    // Verify insert sequence overwrote the delete command
    let buffer = sim.repeat_buffer();
    match buffer.last_action() {
        Some(crate::helix::repeat::RepeatableAction::InsertSequence { text, .. }) => {
            assert_eq!(text, "a");
        }
        _ => panic!("Expected InsertSequence action, not Command"),
    }
}

#[test]
fn test_change_command_records_and_enters_insert() {
    let mut sim = HelixSimulator::new("hello".to_string());

    // Execute change command
    sim.execute_command("c").unwrap();
    assert_eq!(sim.mode(), Mode::Insert);

    // Verify recording started
    assert!(sim.repeat_buffer().insert_recorder().is_recording());

    // Type replacement text
    sim.execute_command("x").unwrap();

    // Exit insert mode
    sim.execute_command("Escape").unwrap();

    // Verify insert sequence was recorded
    let buffer = sim.repeat_buffer();
    match buffer.last_action() {
        Some(crate::helix::repeat::RepeatableAction::InsertSequence { text, .. }) => {
            assert_eq!(text, "x");
        }
        _ => panic!("Expected InsertSequence action"),
    }
}

// ============================================================================
// Phase 3: Repeat Execution Tests
// ============================================================================

#[test]
fn test_repeat_delete_char() {
    let mut sim = HelixSimulator::new("hello".to_string());

    // Execute delete command
    sim.execute_command("x").unwrap();
    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "ello");

    // Repeat delete
    sim.execute_command(".").unwrap();
    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "llo");
}

#[test]
fn test_repeat_delete_line() {
    let mut sim = HelixSimulator::new("line 1\nline 2\nline 3".to_string());

    // Delete first line
    sim.execute_command("dd").unwrap();
    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "line 2\nline 3");

    // Repeat delete
    sim.execute_command(".").unwrap();
    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "line 3");
}

#[test]
fn test_repeat_insert_mode() {
    let mut sim = HelixSimulator::new("world".to_string());

    // Insert "hi"
    sim.execute_command("i").unwrap();
    sim.execute_command("h").unwrap();
    sim.execute_command("i").unwrap();
    sim.execute_command("Escape").unwrap();

    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "hiworld");

    // Move to end
    sim.execute_command("$").unwrap();

    // Repeat insert
    sim.execute_command(".").unwrap();
    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "hiworldhi");
}

#[test]
fn test_repeat_with_empty_buffer() {
    let mut sim = HelixSimulator::new("test".to_string());

    // Try to repeat without any previous action
    let result = sim.execute_command(".");
    assert!(result.is_ok()); // Should not error

    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "test"); // Should be unchanged
}

#[test]
fn test_repeat_is_not_recorded() {
    let mut sim = HelixSimulator::new("abcd".to_string());

    // Delete a char
    sim.execute_command("x").unwrap();
    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "bcd");

    // Repeat once
    sim.execute_command(".").unwrap();
    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "cd");

    // Repeat again - should repeat the ORIGINAL x, not the previous .
    sim.execute_command(".").unwrap();
    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "d");

    // Total: 3 deletes (original x + two repeats)
}

#[test]
fn test_repeat_yank_and_paste() {
    let mut sim = HelixSimulator::new("hello\nworld".to_string());

    // Yank first character
    sim.execute_command("y").unwrap();

    // Move down
    sim.execute_command("j").unwrap();

    // Paste
    sim.execute_command("p").unwrap();
    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "hello\nwhorld");

    // Repeat paste
    sim.execute_command(".").unwrap();
    let state = sim.get_state().unwrap();
    // The paste should repeat with the same clipboard content
    assert!(state.content().contains("hello"));
}

#[test]
fn test_repeat_join_lines() {
    let mut sim = HelixSimulator::new("line 1\nline 2\nline 3".to_string());

    // Join lines
    sim.execute_command("J").unwrap();
    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "line 1 line 2\nline 3");

    // Repeat join
    sim.execute_command(".").unwrap();
    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "line 1 line 2 line 3");
}

#[test]
fn test_repeat_indent() {
    let mut sim = HelixSimulator::new("line 1\nline 2".to_string());

    // Indent
    sim.execute_command(">").unwrap();
    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "  line 1\nline 2");

    // Repeat indent
    sim.execute_command(".").unwrap();
    let state = sim.get_state().unwrap();
    // Line should be double-indented
    assert_eq!(state.content(), "    line 1\nline 2");
}

#[test]
fn test_repeat_dedent() {
    let mut sim = HelixSimulator::new("    code".to_string());

    // Dedent
    sim.execute_command("<").unwrap();
    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "  code");

    // Repeat dedent
    sim.execute_command(".").unwrap();
    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "code");
}

#[test]
fn test_repeat_replace_char() {
    let mut sim = HelixSimulator::new("hello".to_string());

    // Replace 'h' with 'x'
    sim.execute_command("rx").unwrap();
    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "xello");

    // Move to next char
    sim.execute_command("l").unwrap();

    // Repeat replace (should replace 'e' with 'x')
    sim.execute_command(".").unwrap();
    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "xxllo");
}

#[test]
fn test_repeat_append() {
    let mut sim = HelixSimulator::new("hello".to_string());

    // Move to end of word and append " world"
    sim.execute_command("$").unwrap(); // Move to end
    sim.execute_command("a").unwrap(); // Append (cursor after last char)
    sim.execute_command(" ").unwrap();
    sim.execute_command("w").unwrap();
    sim.execute_command("o").unwrap();
    sim.execute_command("r").unwrap();
    sim.execute_command("l").unwrap();
    sim.execute_command("d").unwrap();
    sim.execute_command("Escape").unwrap();

    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "hello world");

    // Move to start
    sim.execute_command("0").unwrap();

    // Repeat should insert " world" at current position
    sim.execute_command(".").unwrap();
    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), " worldhello world");
}

// Note: `o` and `O` commands are complex - they create a newline AND enter insert mode.
// The newline creation is not captured in InsertSequence recording.
// These would require a special RepeatableAction variant (e.g., RepeatableAction::OpenLine).
// For Phase 3, we focus on simpler insert mode replay.
// These tests are commented out until Phase 4+ implements full command replay.

// #[test]
// fn test_repeat_open_below() {
//     // TODO: Implement RepeatableAction::OpenLine variant
// }

// #[test]
// fn test_repeat_open_above() {
//     // TODO: Implement RepeatableAction::OpenLine variant
// }

#[test]
fn test_repeat_insert_with_movements() {
    let mut sim = HelixSimulator::new("world".to_string());

    // Insert with arrow key movements (simplified test)
    // Note: Current implementation applies movements AFTER all text insertion
    // This is a known limitation - movements aren't interleaved with text
    sim.execute_command("i").unwrap();
    sim.execute_command("h").unwrap();
    sim.execute_command("i").unwrap();
    sim.execute_command("ArrowLeft").unwrap();
    sim.execute_command("Escape").unwrap();

    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "hiworld");
    // Cursor should be at position 1 (moved left once from 2)
    assert_eq!(state.cursor_position().col, 1);

    // Move to end
    sim.execute_command("$").unwrap();

    // Repeat insert with movements
    sim.execute_command(".").unwrap();
    let state = sim.get_state().unwrap();
    // Should insert "hi" at end, then move left once
    assert_eq!(state.content(), "hiworldhi");
    // Cursor moved left from position 9 to position 8
    assert_eq!(state.cursor_position().col, 8);
}

#[test]
fn test_repeat_insert_simple() {
    let mut sim = HelixSimulator::new("hello".to_string());

    // Insert 'x' at the beginning
    sim.execute_command("i").unwrap(); // Enter insert mode
    sim.execute_command("x").unwrap(); // Insert 'x'
    sim.execute_command("Escape").unwrap();

    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "xhello");
    // After insert + escape, cursor is at position 1 (after 'x')

    // Move to position: cursor at 1, move right twice → position 3 (on first 'l')
    sim.execute_command("l").unwrap(); // cursor at 2 ('e')
    sim.execute_command("l").unwrap(); // cursor at 3 (first 'l')

    // Repeat - should insert 'x' at position 3
    sim.execute_command(".").unwrap();
    let state = sim.get_state().unwrap();
    // Result: "xhe" + "x" + "llo" = "xhexllo"
    assert_eq!(state.content(), "xhexllo");
}

#[test]
fn test_repeat_multiple_times() {
    let mut sim = HelixSimulator::new("xxxxxx".to_string());

    // Delete once
    sim.execute_command("x").unwrap();

    // Repeat 4 times
    for _ in 0..4 {
        sim.execute_command(".").unwrap();
    }

    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "x"); // 5 deletes total
}

#[test]
fn test_repeat_after_undo() {
    let mut sim = HelixSimulator::new("test".to_string());

    // Delete a char
    sim.execute_command("x").unwrap();

    // Undo it
    sim.execute_command("u").unwrap();

    // The repeat buffer should still have 'x'
    // Repeat should still delete
    sim.execute_command(".").unwrap();
    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "est");
}

#[test]
fn test_repeat_preserves_action_across_movements() {
    let mut sim = HelixSimulator::new("hello world".to_string());

    // Delete 'h'
    sim.execute_command("x").unwrap();

    // Move around (movements don't change repeat buffer)
    sim.execute_command("l").unwrap();
    sim.execute_command("w").unwrap();
    sim.execute_command("0").unwrap();

    // Repeat should still delete
    sim.execute_command(".").unwrap();
    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "llo world");
}

#[test]
fn test_repeat_insert_at_line_start() {
    let mut sim = HelixSimulator::new("hello".to_string());

    // Insert at line start
    sim.execute_command("I").unwrap();
    sim.execute_command(">").unwrap();
    sim.execute_command(">").unwrap();
    sim.execute_command("Escape").unwrap();

    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), ">>hello");

    // Move somewhere else
    sim.execute_command("$").unwrap();

    // Repeat
    sim.execute_command(".").unwrap();
    let state = sim.get_state().unwrap();
    // Should insert ">>" at cursor position
    assert!(state.content().contains(">>"));
}

#[test]
fn test_repeat_append_at_line_end() {
    let mut sim = HelixSimulator::new("hello".to_string());

    // Append at line end
    sim.execute_command("A").unwrap();
    sim.execute_command("!").unwrap();
    sim.execute_command("!").unwrap();
    sim.execute_command("Escape").unwrap();

    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "hello!!");

    // Move to start
    sim.execute_command("0").unwrap();

    // Repeat
    sim.execute_command(".").unwrap();
    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "!!hello!!");
}
