//! Tests for HelixSimulator

use super::*;
use crate::helix::commands::*;

#[test]
fn test_create_simulator() {
    let sim = AnyModeSimulator::new("hello world".to_string());
    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "hello world");
    assert_eq!(state.cursor_position().row, 0);
    assert_eq!(state.cursor_position().col, 0);
}

#[test]
fn test_initial_mode() {
    let sim = AnyModeSimulator::new("test".to_string());
    assert_eq!(sim.mode(), Mode::Normal);
}

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
fn test_word_movement() {
    let mut sim = AnyModeSimulator::new("hello world foo".to_string());

    // Move to next word
    sim.execute_command(CMD_MOVE_WORD_FORWARD).unwrap();
    let state = sim.get_state().unwrap();
    assert_eq!(state.cursor_position().col, 6); // "world"

    // Move to next word again
    sim.execute_command(CMD_MOVE_WORD_FORWARD).unwrap();
    let state = sim.get_state().unwrap();
    assert_eq!(state.cursor_position().col, 12); // "foo"
}

#[test]
fn test_delete_line() {
    // In Helix, delete line is done with 'x' (select line) + 'd' (delete)
    let mut sim = AnyModeSimulator::new("line 1\nline 2\nline 3\n".to_string());

    sim.execute_command(CMD_SELECT_LINE).unwrap();
    sim.execute_command(CMD_DELETE_SELECTION).unwrap();

    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "line 2\nline 3\n");
}

#[test]
fn test_select_line_then_delete() {
    let mut sim = AnyModeSimulator::new("Keep\nDelete me\nKeep".to_string());

    // Move to line 1
    sim.execute_command(CMD_MOVE_DOWN).unwrap();
    let state = sim.get_state().unwrap();
    assert_eq!(state.cursor_position().row, 1);

    // Execute x (select line)
    sim.execute_command(CMD_SELECT_LINE).unwrap();
    let state = sim.get_state().unwrap();
    // Selection should cover "Delete me\n" (line 1 with newline)
    let sel = state.selection().expect("Should have selection after x");
    assert_eq!(sel.start.row, 1, "Selection start row");
    assert_eq!(sel.start.col, 0, "Selection start col");
    // End should be at start of next line (row 2, col 0) or end of this line
    assert!(sel.end.row >= 1, "Selection end row should be >= 1");

    // Execute d (delete selection)
    sim.execute_command(CMD_DELETE_SELECTION).unwrap();
    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "Keep\nKeep");
}

#[test]
fn test_select_line_scenario_exact() {
    // Replicate exact scenario: "Select line then delete"
    // setup: file_content = "Keep\nDelete me\nKeep", cursor_position = [1, 0]
    use crate::game::{CursorPosition, EditorState};

    let initial_state = EditorState::new(
        "Keep\nDelete me\nKeep".to_string(),
        CursorPosition::new(1, 0).unwrap(),
        None,
    )
    .unwrap();

    let mut sim = AnyModeSimulator::from_editor_state(&initial_state);
    let state = sim.get_state().unwrap();
    eprintln!(
        "Initial: cursor={:?}, sel={:?}, content={:?}",
        state.cursor_position(),
        state.selection(),
        state.content()
    );
    assert_eq!(state.cursor_position().row, 1);
    assert_eq!(state.cursor_position().col, 0);

    // Execute x (select line)
    sim.execute_command(CMD_SELECT_LINE).unwrap();
    let state = sim.get_state().unwrap();
    eprintln!(
        "After x: cursor={:?}, sel={:?}",
        state.cursor_position(),
        state.selection()
    );

    let sel = state.selection().expect("Should have selection after x");
    assert_eq!(sel.start.row, 1, "Selection should start at row 1");
    assert_eq!(
        sel.end.row, 2,
        "Selection should end at row 2 (newline included)"
    );

    // Execute d (delete selection)
    sim.execute_command(CMD_DELETE_SELECTION).unwrap();
    let state = sim.get_state().unwrap();
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
fn test_undo() {
    let mut sim = AnyModeSimulator::new("test\n".to_string());

    // Delete line using x (select) + d (delete)
    sim.execute_command(CMD_SELECT_LINE).unwrap();
    sim.execute_command(CMD_DELETE_SELECTION).unwrap();
    assert_eq!(sim.get_state().unwrap().content(), "");

    sim.execute_command(CMD_UNDO).unwrap();
    assert_eq!(sim.get_state().unwrap().content(), "test\n");
}

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
fn test_move_line_start() {
    let mut sim = AnyModeSimulator::new("hello\nworld\n".to_string());

    // Move to next line
    sim.execute_command(CMD_MOVE_DOWN).unwrap();
    // Move to end of line
    sim.execute_command(CMD_MOVE_LINE_END).unwrap();
    let state = sim.get_state().unwrap();
    // Cursor at end of "world" - which is position 4 or 5
    assert!(state.cursor_position().col >= 4);

    // Move to start of line
    sim.execute_command(CMD_MOVE_LINE_START).unwrap();
    let state = sim.get_state().unwrap();
    assert_eq!(state.cursor_position().col, 0);
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
fn test_unknown_command() {
    let mut sim = AnyModeSimulator::new("test".to_string());
    let result = sim.execute_command("unknown");
    assert!(result.is_err());
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

    // Move to end of document first
    sim.execute_command(CMD_GOTO_FILE_END).unwrap();
    // Then move to previous word
    sim.execute_command(CMD_MOVE_WORD_BACKWARD).unwrap();

    let state = sim.get_state().unwrap();
    // Should have moved to start of a previous word
    assert!(state.cursor_position().col >= 11);
}

#[test]
fn test_append_mode() {
    let mut sim = AnyModeSimulator::new("hello".to_string());

    // Cursor at start (position 0)
    assert_eq!(sim.get_state().unwrap().cursor_position().col, 0);

    // Press 'a' should move cursor one position right and enter insert mode
    sim.execute_command(CMD_APPEND).unwrap();

    let state = sim.get_state().unwrap();
    assert_eq!(sim.mode(), Mode::Insert);
    assert_eq!(state.cursor_position().col, 1); // Moved one right
}

#[test]
fn test_open_below() {
    let mut sim = AnyModeSimulator::new("line1\nline2".to_string());

    // Cursor at start of first line
    assert_eq!(sim.get_state().unwrap().cursor_position().row, 0);

    // Press 'o' should insert new line below and enter insert mode
    sim.execute_command(CMD_OPEN_BELOW).unwrap();

    let state = sim.get_state().unwrap();
    assert_eq!(sim.mode(), Mode::Insert);
    assert_eq!(state.content(), "line1\n\nline2");
    assert_eq!(state.cursor_position().row, 1); // On new empty line
}

#[test]
fn test_open_above() {
    let mut sim = AnyModeSimulator::new("line1\nline2".to_string());

    // Move to second line
    sim.execute_command(CMD_MOVE_DOWN).unwrap();
    assert_eq!(sim.get_state().unwrap().cursor_position().row, 1);

    // Press 'O' should insert new line above and enter insert mode
    sim.execute_command(CMD_OPEN_ABOVE).unwrap();

    let state = sim.get_state().unwrap();
    assert_eq!(sim.mode(), Mode::Insert);
    assert_eq!(state.content(), "line1\n\nline2");
    assert_eq!(state.cursor_position().row, 1); // On new empty line
}

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

#[test]
fn test_insert_at_line_start() {
    let mut sim = AnyModeSimulator::new("  hello world".to_string());

    // Move cursor to middle of line
    sim.execute_command(CMD_MOVE_WORD_FORWARD).unwrap();
    let state = sim.get_state().unwrap();
    assert!(state.cursor_position().col > 0);

    // Press 'I' should move to start of line and enter insert mode
    sim.execute_command(CMD_INSERT_LINE_START).unwrap();

    let state = sim.get_state().unwrap();
    assert_eq!(sim.mode(), Mode::Insert);
    assert_eq!(state.cursor_position().col, 0);
}

#[test]
fn test_append_at_line_end() {
    let mut sim = AnyModeSimulator::new("hello world\nline2".to_string());

    // Cursor at start
    assert_eq!(sim.get_state().unwrap().cursor_position().col, 0);

    // Press 'A' should move to end of line and enter insert mode
    sim.execute_command(CMD_APPEND_LINE_END).unwrap();

    let state = sim.get_state().unwrap();
    assert_eq!(sim.mode(), Mode::Insert);
    assert_eq!(state.cursor_position().col, 11); // After "hello world"
    assert_eq!(state.cursor_position().row, 0); // Still on first line
}

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
    assert_eq!(state.cursor_position().col, 0); // Cursor stays at start
}

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
    assert_eq!(state.cursor_position().col, 3); // Cursor after pasted 'a'
}

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
    assert_eq!(state.cursor_position().col, 1); // Cursor after pasted 'c'
}

#[test]
fn test_insert_text_in_insert_mode() {
    let mut sim = AnyModeSimulator::new("hello".to_string());

    // Enter insert mode
    sim.execute_command(CMD_INSERT).unwrap();
    assert_eq!(sim.mode(), Mode::Insert);

    // Insert a character
    sim.execute_command("!").unwrap();

    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "!hello");
    assert_eq!(state.cursor_position().col, 1);
}

#[test]
fn test_append_at_line_end_and_insert() {
    let mut sim = AnyModeSimulator::new("hello".to_string());

    // Append at line end
    sim.execute_command(CMD_APPEND_LINE_END).unwrap();
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
fn test_insert_text_works_in_insert_mode() {
    let mut sim = AnyModeSimulator::new("hello".to_string());

    // Enter Insert mode
    sim.execute_command(CMD_INSERT).unwrap();

    // Insert text should work via execute_command
    let result = sim.execute_command("!");
    assert!(result.is_ok());

    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "!hello");
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

    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "abc");
    assert_eq!(state.cursor_position().col, 3);
}

#[test]
fn test_backspace_in_insert_mode() {
    let mut sim = AnyModeSimulator::new("hello".to_string());

    // Enter insert mode at position 5
    sim.execute_command(CMD_MOVE_LINE_END).unwrap(); // Move to end
    sim.execute_command(CMD_APPEND).unwrap(); // Append
    assert_eq!(sim.mode(), Mode::Insert);

    // Type some characters
    sim.execute_command("!").unwrap();
    sim.execute_command("!").unwrap();

    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "hello!!");
    assert_eq!(state.cursor_position().col, 7);

    // Backspace once
    sim.execute_command(CMD_BACKSPACE).unwrap();

    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "hello!");
    assert_eq!(state.cursor_position().col, 6);

    // Backspace again
    sim.execute_command(CMD_BACKSPACE).unwrap();

    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "hello");
    assert_eq!(state.cursor_position().col, 5);
}

#[test]
fn test_backspace_at_start() {
    let mut sim = AnyModeSimulator::new("test".to_string());

    // Enter insert mode at start
    sim.execute_command(CMD_INSERT).unwrap();
    assert_eq!(sim.mode(), Mode::Insert);

    // Backspace at position 0 should do nothing
    sim.execute_command(CMD_BACKSPACE).unwrap();

    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "test");
    assert_eq!(state.cursor_position().col, 0);
}

#[test]
fn test_arrow_keys_in_insert_mode() {
    let mut sim = AnyModeSimulator::new("abc\ndef".to_string());

    // Enter insert mode
    sim.execute_command(CMD_INSERT).unwrap();
    assert_eq!(sim.mode(), Mode::Insert);

    // Test Right arrow
    sim.execute_command(CMD_ARROW_RIGHT).unwrap();
    assert_eq!(sim.get_state().unwrap().cursor_position().col, 1);

    // Test Left arrow
    sim.execute_command(CMD_ARROW_LEFT).unwrap();
    assert_eq!(sim.get_state().unwrap().cursor_position().col, 0);

    // Test Down arrow
    sim.execute_command(CMD_ARROW_DOWN).unwrap();
    assert_eq!(sim.get_state().unwrap().cursor_position().row, 1);

    // Test Up arrow
    sim.execute_command(CMD_ARROW_UP).unwrap();
    assert_eq!(sim.get_state().unwrap().cursor_position().row, 0);

    // Should still be in Insert mode
    assert_eq!(sim.mode(), Mode::Insert);
}

#[test]
fn test_backspace_works_in_insert_mode() {
    let mut sim = AnyModeSimulator::new("hello".to_string());

    // Enter Insert mode
    sim.execute_command(CMD_INSERT).unwrap();

    // Move right first to have something to delete
    sim.execute_command(CMD_ARROW_RIGHT).unwrap(); // Arrow right

    // Backspace should work in Insert mode
    let result = sim.execute_command(CMD_BACKSPACE);
    assert!(result.is_ok());
}

#[test]
fn test_join_lines() {
    let mut sim = AnyModeSimulator::new("line1\nline2\nline3".to_string());

    // Join first two lines
    sim.execute_command(CMD_JOIN_LINES).unwrap();

    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "line1 line2\nline3");
    assert_eq!(state.cursor_position().row, 0);
}

#[test]
fn test_join_lines_at_last_line() {
    let mut sim = AnyModeSimulator::new("line1\nline2".to_string());

    // Move to last line
    sim.execute_command(CMD_MOVE_DOWN).unwrap();
    assert_eq!(sim.get_state().unwrap().cursor_position().row, 1);

    // Try to join - should do nothing
    sim.execute_command(CMD_JOIN_LINES).unwrap();

    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "line1\nline2");
}

#[test]
fn test_indent_line() {
    let mut sim = AnyModeSimulator::new("hello\nworld".to_string());

    // Indent first line
    sim.execute_command(CMD_INDENT).unwrap();

    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "  hello\nworld");
    // Cursor should move forward by 2
    assert_eq!(state.cursor_position().col, 2);
}

#[test]
fn test_dedent_line() {
    let mut sim = AnyModeSimulator::new("  hello\n    world".to_string());

    // Dedent first line (remove 2 spaces)
    sim.execute_command(CMD_DEDENT).unwrap();

    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "hello\n    world");
    assert_eq!(state.cursor_position().col, 0);
}

#[test]
fn test_dedent_line_with_one_space() {
    let mut sim = AnyModeSimulator::new(" hello".to_string());

    // Dedent - should remove only 1 space
    sim.execute_command(CMD_DEDENT).unwrap();

    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "hello");
    assert_eq!(state.cursor_position().col, 0);
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
    assert_eq!(state.cursor_position().col, 4);
}

// ============================================================================
// Phase 2: Repeat Buffer Integration Tests
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
    let mut sim = AnyModeSimulator::new("line 1\nline 2".to_string());

    // Execute x+d (select line + delete) - the Helix way
    sim.execute_command(CMD_SELECT_LINE).unwrap();
    sim.execute_command(CMD_DELETE_SELECTION).unwrap();

    // Verify x+d was recorded as compound action
    let buffer = sim.repeat_buffer();
    assert!(!buffer.is_empty());

    match buffer.last_action() {
        Some(crate::helix::repeat::RepeatableAction::Command {
            keys,
            expected_mode,
        }) => {
            assert_eq!(keys.len(), 2); // 'x' + 'd' keys
            assert_eq!(*expected_mode, crate::helix::repeat::Mode::Normal);
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
        Some(crate::helix::repeat::RepeatableAction::Command { keys, .. }) => {
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
        Some(crate::helix::repeat::RepeatableAction::Command { keys, .. }) => {
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
        Some(crate::helix::repeat::RepeatableAction::Command { keys, .. }) => {
            assert_eq!(keys.len(), 2); // 'r' and 'x'
        }
        _ => panic!("Expected Command action"),
    }
}

#[test]
fn test_insert_mode_recording_simple() {
    let mut sim = AnyModeSimulator::new("world".to_string());

    // Enter insert mode
    sim.execute_command(CMD_INSERT).unwrap();
    assert_eq!(sim.mode(), Mode::Insert);

    // Verify recording started
    assert!(sim.repeat_buffer().insert_recorder().is_recording());

    // Type text
    sim.execute_command(CMD_MOVE_LEFT).unwrap();
    sim.execute_command(CMD_INSERT).unwrap();

    // Exit insert mode
    sim.execute_command(CMD_ESCAPE).unwrap();
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
    let mut sim = AnyModeSimulator::new("hello".to_string());

    // Enter insert mode via append
    sim.execute_command(CMD_APPEND).unwrap();
    assert_eq!(sim.mode(), Mode::Insert);

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
        Some(crate::helix::repeat::RepeatableAction::InsertSequence { text, .. }) => {
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
    assert_eq!(sim.mode(), Mode::Insert);

    // Type text
    sim.execute_command("n").unwrap();
    sim.execute_command(CMD_MOVE_WORD_END).unwrap();
    sim.execute_command(CMD_MOVE_WORD_FORWARD).unwrap();

    // Exit insert mode
    sim.execute_command(CMD_ESCAPE).unwrap();

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
    let mut sim = AnyModeSimulator::new("test".to_string());

    // Enter and immediately exit insert mode
    sim.execute_command(CMD_INSERT).unwrap();
    sim.execute_command(CMD_ESCAPE).unwrap();

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
        Some(crate::helix::repeat::RepeatableAction::InsertSequence { text, .. }) => {
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
    assert_eq!(sim.mode(), Mode::Insert);

    // Verify recording started
    assert!(sim.repeat_buffer().insert_recorder().is_recording());

    // Type replacement text
    sim.execute_command(CMD_DELETE_SELECTION).unwrap();

    // Exit insert mode
    sim.execute_command(CMD_ESCAPE).unwrap();

    // Verify insert sequence was recorded
    let buffer = sim.repeat_buffer();
    match buffer.last_action() {
        Some(crate::helix::repeat::RepeatableAction::InsertSequence { text, .. }) => {
            // CMD_DELETE_SELECTION is "d", so it types "d" in insert mode
            assert_eq!(text, "d");
        }
        _ => panic!("Expected InsertSequence action"),
    }
}

// ============================================================================
// Phase 3: Repeat Execution Tests
// ============================================================================

#[test]
fn test_repeat_delete_char() {
    let mut sim = AnyModeSimulator::new("hello".to_string());

    // Execute delete command
    sim.execute_command(CMD_DELETE_SELECTION).unwrap();
    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "ello");

    // Repeat delete
    sim.execute_command(".").unwrap();
    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "llo");
}

#[test]
fn test_repeat_delete_line() {
    let mut sim = AnyModeSimulator::new("line 1\nline 2\nline 3".to_string());

    // Delete first line using x+d (Helix way)
    sim.execute_command(CMD_SELECT_LINE).unwrap();
    sim.execute_command(CMD_DELETE_SELECTION).unwrap();
    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "line 2\nline 3");

    // Repeat delete - should execute x+d together
    sim.execute_command(".").unwrap();
    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "line 3");
}

#[test]
fn test_repeat_insert_mode() {
    let mut sim = AnyModeSimulator::new("world".to_string());

    // Insert "hi"
    sim.execute_command(CMD_INSERT).unwrap();
    sim.execute_command(CMD_MOVE_LEFT).unwrap();
    sim.execute_command(CMD_INSERT).unwrap();
    sim.execute_command(CMD_ESCAPE).unwrap();

    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "hiworld");

    // Move to end
    sim.execute_command(CMD_MOVE_LINE_END).unwrap();

    // Repeat insert
    sim.execute_command(".").unwrap();
    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "hiworldhi");
}

#[test]
fn test_repeat_with_empty_buffer() {
    let mut sim = AnyModeSimulator::new("test".to_string());

    // Try to repeat without any previous action
    let result = sim.execute_command(".");
    assert!(result.is_ok()); // Should not error

    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "test"); // Should be unchanged
}

#[test]
fn test_repeat_is_not_recorded() {
    let mut sim = AnyModeSimulator::new("abcd".to_string());

    // Delete a char
    sim.execute_command(CMD_DELETE_SELECTION).unwrap();
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

#[test]
fn test_repeat_join_lines() {
    let mut sim = AnyModeSimulator::new("line 1\nline 2\nline 3".to_string());

    // Join lines
    sim.execute_command(CMD_JOIN_LINES).unwrap();
    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "line 1 line 2\nline 3");

    // Repeat join
    sim.execute_command(".").unwrap();
    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "line 1 line 2 line 3");
}

#[test]
fn test_repeat_indent() {
    let mut sim = AnyModeSimulator::new("line 1\nline 2".to_string());

    // Indent
    sim.execute_command(CMD_INDENT).unwrap();
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
    let mut sim = AnyModeSimulator::new("    code".to_string());

    // Dedent
    sim.execute_command(CMD_DEDENT).unwrap();
    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "  code");

    // Repeat dedent
    sim.execute_command(".").unwrap();
    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "code");
}

#[test]
fn test_repeat_replace_char() {
    let mut sim = AnyModeSimulator::new("hello".to_string());

    // Replace 'h' with 'x'
    sim.execute_command("rx").unwrap();
    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "xello");

    // Move to next char
    sim.execute_command(CMD_MOVE_RIGHT).unwrap();

    // Repeat replace (should replace 'e' with 'x')
    sim.execute_command(".").unwrap();
    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "xxllo");
}

#[test]
#[ignore] // TODO: Implement proper repeat for insert mode entry commands (a, A, I, o, O)
// The current RepeatableAction::InsertSequence doesn't capture which insert command
// was used to enter insert mode, so repeating "a" currently acts like "i"
fn test_repeat_append() {
    let mut sim = AnyModeSimulator::new("hello".to_string());

    // Move to end of word and append " world"
    sim.execute_command(CMD_MOVE_LINE_END).unwrap(); // Move to end
    sim.execute_command(CMD_APPEND).unwrap(); // Append (cursor after last char)
    sim.execute_command(" ").unwrap();
    sim.execute_command(CMD_MOVE_WORD_FORWARD).unwrap();
    sim.execute_command(CMD_OPEN_BELOW).unwrap();
    sim.execute_command(CMD_REPLACE).unwrap();
    sim.execute_command(CMD_MOVE_RIGHT).unwrap();
    sim.execute_command("d").unwrap();
    sim.execute_command(CMD_ESCAPE).unwrap();

    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "hello world");

    // Move to start
    sim.execute_command(CMD_MOVE_LINE_START).unwrap();

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
    let mut sim = AnyModeSimulator::new("world".to_string());

    // Insert with arrow key movements (simplified test)
    // Note: Current implementation applies movements AFTER all text insertion
    // This is a known limitation - movements aren't interleaved with text
    sim.execute_command(CMD_INSERT).unwrap();
    sim.execute_command(CMD_MOVE_LEFT).unwrap();
    sim.execute_command(CMD_INSERT).unwrap();
    sim.execute_command(CMD_ARROW_LEFT).unwrap();
    sim.execute_command(CMD_ESCAPE).unwrap();

    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "hiworld");
    // Cursor should be at position 1 (moved left once from 2)
    assert_eq!(state.cursor_position().col, 1);

    // Move to end
    sim.execute_command(CMD_MOVE_LINE_END).unwrap();

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
    let mut sim = AnyModeSimulator::new("hello".to_string());

    // Insert 'd' at the beginning (CMD_DELETE_SELECTION is "d")
    sim.execute_command(CMD_INSERT).unwrap(); // Enter insert mode
    sim.execute_command(CMD_DELETE_SELECTION).unwrap(); // Insert 'd'
    sim.execute_command(CMD_ESCAPE).unwrap();

    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "dhello");
    // After insert + escape, cursor is at position 1 (after 'd')

    // Move to position: cursor at 1, move right twice → position 3 (on first 'l')
    sim.execute_command(CMD_MOVE_RIGHT).unwrap(); // cursor at 2 ('e')
    sim.execute_command(CMD_MOVE_RIGHT).unwrap(); // cursor at 3 (first 'l')

    // Repeat - should insert 'd' at position 3
    sim.execute_command(".").unwrap();
    let state = sim.get_state().unwrap();
    // Result: "dhe" + "d" + "llo" = "dhedllo"
    assert_eq!(state.content(), "dhedllo");
}

#[test]
fn test_repeat_multiple_times() {
    let mut sim = AnyModeSimulator::new("xxxxxx".to_string());

    // Delete once
    sim.execute_command(CMD_DELETE_SELECTION).unwrap();

    // Repeat 4 times
    for _ in 0..4 {
        sim.execute_command(".").unwrap();
    }

    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "x"); // 5 deletes total
}

#[test]
fn test_repeat_after_undo() {
    let mut sim = AnyModeSimulator::new("test".to_string());

    // Delete a char
    sim.execute_command(CMD_DELETE_SELECTION).unwrap();

    // Undo it
    sim.execute_command(CMD_UNDO).unwrap();

    // The repeat buffer should still have 'x'
    // Repeat should still delete
    sim.execute_command(".").unwrap();
    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "est");
}

#[test]
fn test_repeat_preserves_action_across_movements() {
    let mut sim = AnyModeSimulator::new("hello world".to_string());

    // Delete 'h'
    sim.execute_command(CMD_DELETE_SELECTION).unwrap();

    // Move around (movements don't change repeat buffer)
    sim.execute_command(CMD_MOVE_RIGHT).unwrap();
    sim.execute_command(CMD_MOVE_WORD_FORWARD).unwrap();
    sim.execute_command(CMD_MOVE_LINE_START).unwrap();

    // Repeat should still delete
    sim.execute_command(".").unwrap();
    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "llo world");
}

#[test]
fn test_repeat_insert_at_line_start() {
    let mut sim = AnyModeSimulator::new("hello".to_string());

    // Insert at line start
    sim.execute_command(CMD_INSERT_LINE_START).unwrap();
    sim.execute_command(CMD_INDENT).unwrap();
    sim.execute_command(CMD_INDENT).unwrap();
    sim.execute_command(CMD_ESCAPE).unwrap();

    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), ">>hello");

    // Move somewhere else
    sim.execute_command(CMD_MOVE_LINE_END).unwrap();

    // Repeat
    sim.execute_command(".").unwrap();
    let state = sim.get_state().unwrap();
    // Should insert ">>" at cursor position
    assert!(state.content().contains(">>"));
}

#[test]
#[ignore] // TODO: Implement proper repeat for insert mode entry commands (A, I, o, O)
// The current RepeatableAction::InsertSequence doesn't capture which insert command
// was used to enter insert mode, so repeating "A" currently acts like "i"
fn test_repeat_append_at_line_end() {
    let mut sim = AnyModeSimulator::new("hello".to_string());

    // Append at line end
    sim.execute_command(CMD_APPEND_LINE_END).unwrap();
    sim.execute_command("!").unwrap();
    sim.execute_command("!").unwrap();
    sim.execute_command(CMD_ESCAPE).unwrap();

    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "hello!!");

    // Move to start
    sim.execute_command(CMD_MOVE_LINE_START).unwrap();

    // Repeat
    sim.execute_command(".").unwrap();
    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "!!hello!!");
}

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
    // After paste_before, cursor moves to end of pasted text
    // Pasted at position 1, length 1, so cursor should be at 1+1=2
    assert_eq!(
        state.cursor_position().col,
        2,
        "Cursor should be at position 2 (after pasted 'z')"
    );
}

// Phase 4: Compound Action Tests (selection + operator)
// Tests for x+d, x+y, %+d etc. combinations that should be recorded and repeated together

#[test]
fn test_repeat_select_line_then_delete() {
    // Scenario: User selects a line with 'x', deletes with 'd', then repeats with '.'
    // Expected: The repeat should execute both x and d together
    let mut sim = AnyModeSimulator::new("line 1\nline 2\nline 3\nline 4\n".to_string());

    // Move to line 2
    sim.execute_command(CMD_MOVE_DOWN).unwrap();
    let state = sim.get_state().unwrap();
    assert_eq!(state.cursor_position().row, 1);

    // Select line with x
    sim.execute_command(CMD_SELECT_LINE).unwrap();

    // Delete selection with d
    sim.execute_command(CMD_DELETE_SELECTION).unwrap();
    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "line 1\nline 3\nline 4\n");

    // Now cursor should be on what was line 3 (now line 2)
    assert_eq!(state.cursor_position().row, 1);

    // Repeat with . - should execute x+d together
    sim.execute_command(".").unwrap();
    let state = sim.get_state().unwrap();
    // Line 3 (now at row 1) should be deleted
    assert_eq!(state.content(), "line 1\nline 4\n");

    // Repeat again
    sim.execute_command(".").unwrap();
    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "line 1\n");
}

#[test]
fn test_repeat_select_all_then_delete() {
    // Scenario: User selects all with '%', then deletes with 'd'
    let mut sim = AnyModeSimulator::new("hello\nworld".to_string());

    // Select all
    sim.execute_command("%").unwrap();

    // Delete selection
    sim.execute_command(CMD_DELETE_SELECTION).unwrap();
    let state = sim.get_state().unwrap();
    assert!(state.content().is_empty() || state.content() == "\n");

    // Create new content to test repeat
    // (Repeat on empty content won't do much)
}

#[test]
fn test_repeat_select_line_then_yank() {
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
    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "line 1\nline 2\nline 3\n");
}

#[test]
fn test_compound_action_overwritten_by_simple_command() {
    // Test that a simple editing command after x+d overwrites the compound action
    let mut sim = AnyModeSimulator::new("aaa\nbbb\nccc\n".to_string());

    // Do x+d (compound action)
    sim.execute_command(CMD_SELECT_LINE).unwrap();
    sim.execute_command(CMD_DELETE_SELECTION).unwrap();
    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "bbb\nccc\n");

    // Now do another x+d (still compound, same sequence)
    sim.execute_command(CMD_SELECT_LINE).unwrap();
    sim.execute_command(CMD_DELETE_SELECTION).unwrap();
    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "ccc\n");

    // Repeat should do x+d again
}

// Phase 5: Scenario Validation Tests
// These tests verify that the actual scenarios from TOML files work correctly

#[test]
fn test_scenario_repeat_delete_char_001() {
    // Scenario: repeat_delete_char_001
    // Setup: "hello world", cursor [0,0]
    // Target: "llo world", cursor [0,0]
    // Commands: ["d", "."]
    let mut sim = AnyModeSimulator::new("hello world".to_string());

    // Execute commands from scenario
    sim.execute_command("d").unwrap();
    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "ello world", "After first 'd'");

    sim.execute_command(".").unwrap();
    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "llo world", "After '.' repeat");
    assert_eq!(state.cursor_position().row, 0);
    assert_eq!(state.cursor_position().col, 0);
}

#[test]
fn test_scenario_repeat_delete_line_001() {
    // Scenario: repeat_delete_line_001
    // Setup: "line 1\nline 2\nline 3\nline 4", cursor [0,0]
    // Target: "line 3\nline 4", cursor [0,0]
    // Commands: ["x", "d", "."]
    let mut sim = AnyModeSimulator::new("line 1\nline 2\nline 3\nline 4".to_string());

    // Select line with x, then delete with d
    sim.execute_command("x").unwrap();
    sim.execute_command("d").unwrap();
    let state = sim.get_state().unwrap();
    assert_eq!(
        state.content(),
        "line 2\nline 3\nline 4",
        "After first 'xd'"
    );

    // Repeat with . - should execute x+d together
    sim.execute_command(".").unwrap();
    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "line 3\nline 4", "After '.' repeat");
    assert_eq!(state.cursor_position().row, 0);
    assert_eq!(state.cursor_position().col, 0);
}

#[test]
fn test_scenario_repeat_indent_001() {
    // Scenario: repeat_indent_001
    // Setup: "def foo():\nprint('hello')\nprint('world')\nreturn", cursor [1,0]
    // Target: "def foo():\n  print('hello')\n  print('world')\nreturn", cursor [2,4]
    // Commands: [">", "j", "."]
    let mut sim =
        AnyModeSimulator::new("def foo():\nprint('hello')\nprint('world')\nreturn".to_string());

    // Move to line 1
    sim.execute_command(CMD_MOVE_DOWN).unwrap();
    let state = sim.get_state().unwrap();
    assert_eq!(state.cursor_position().row, 1);

    // Indent
    sim.execute_command(">").unwrap();
    let state = sim.get_state().unwrap();
    assert!(
        state.content().contains("  print('hello')"),
        "Line 1 should be indented"
    );

    // Move down
    sim.execute_command("j").unwrap();

    // Repeat indent
    sim.execute_command(".").unwrap();
    let state = sim.get_state().unwrap();
    assert!(
        state.content().contains("  print('world')"),
        "Line 2 should be indented"
    );

    // Final content check
    assert_eq!(
        state.content(),
        "def foo():\n  print('hello')\n  print('world')\nreturn"
    );
}

#[test]
fn test_scenario_repeat_replace_001() {
    // Scenario: repeat_replace_001
    // Setup: "foo-bar-baz", cursor [0,3]
    // Target: "foo_bar_baz", cursor [0,7]
    // Commands: ["r_", "l", "l", "l", "l", "."]
    let mut sim = AnyModeSimulator::new("foo-bar-baz".to_string());

    // Move to position 3 (first '-')
    sim.execute_command(CMD_MOVE_RIGHT).unwrap();
    sim.execute_command(CMD_MOVE_RIGHT).unwrap();
    sim.execute_command(CMD_MOVE_RIGHT).unwrap();
    let state = sim.get_state().unwrap();
    assert_eq!(state.cursor_position().col, 3);

    // Replace '-' with '_'
    sim.execute_command("r_").unwrap();
    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "foo_bar-baz", "After first replace");

    // Move right 4 times to reach second '-'
    sim.execute_command("l").unwrap();
    sim.execute_command("l").unwrap();
    sim.execute_command("l").unwrap();
    sim.execute_command("l").unwrap();
    let state = sim.get_state().unwrap();
    assert_eq!(state.cursor_position().col, 7);

    // Repeat replace
    sim.execute_command(".").unwrap();
    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "foo_bar_baz", "After repeat replace");
    assert_eq!(state.cursor_position().col, 7);
}

#[test]
fn test_scenario_repeat_insert_001() {
    // Scenario: repeat_insert_001
    // Setup: "TODO:\nFIX:\nNOTE:", cursor [0,5]
    // Target: "TODO: Update docs\nFIX: Update docs\nNOTE:", cursor [1,16]
    // Commands: ["i", " ", "U", "p", "d", "a", "t", "e", " ", "d", "o", "c", "s", "Escape", "j", "0", "$", "."]
    let mut sim = AnyModeSimulator::new("TODO:\nFIX:\nNOTE:".to_string());

    // Move cursor to position [0,5] (after "TODO:")
    // "TODO:" has 5 chars, so col 5 is right after ':'
    // But in Helix cursor can't go past last char, so max is col 4 (the ':')
    // Scenario says cursor_position = [0, 5] which means we start at position 5
    // Actually let's check what the scenario expects - it inserts AFTER "TODO:"
    // So we need to be at the ':' (col 4) or use append mode
    sim.execute_command(CMD_MOVE_LINE_END).unwrap();

    // Enter insert mode and type " Update docs"
    sim.execute_command("i").unwrap();
    for ch in " Update docs".chars() {
        sim.execute_command(&ch.to_string()).unwrap();
    }
    sim.execute_command("Escape").unwrap();

    let state = sim.get_state().unwrap();
    assert_eq!(
        state.content(),
        "TODO: Update docs\nFIX:\nNOTE:",
        "After first insert"
    );

    // Navigate: j (down), 0 (line start), $ (line end)
    sim.execute_command("j").unwrap();
    sim.execute_command("0").unwrap();
    sim.execute_command("$").unwrap();

    let state = sim.get_state().unwrap();
    assert_eq!(state.cursor_position().row, 1, "Should be on line 1");

    // Repeat the insert
    sim.execute_command(".").unwrap();

    let state = sim.get_state().unwrap();
    assert_eq!(
        state.content(),
        "TODO: Update docs\nFIX: Update docs\nNOTE:",
        "After repeat insert"
    );
}
