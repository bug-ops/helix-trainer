//! Tests for EditorState

use super::*;

#[test]
fn test_create_valid_state() {
    let cursor = CursorPosition::new(0, 0).unwrap();
    let state = EditorState::new("line 1\nline 2\n".to_string(), cursor, None);
    assert!(state.is_ok());
}

#[test]
fn test_cursor_out_of_bounds_rejected() {
    let cursor = CursorPosition::new(10, 0).unwrap();
    let state = EditorState::new("line 1\n".to_string(), cursor, None);
    assert!(state.is_err());
}

#[test]
fn test_cursor_column_out_of_bounds() {
    let cursor = CursorPosition::new(0, 100).unwrap();
    let state = EditorState::new("short\n".to_string(), cursor, None);
    assert!(state.is_err());
}

#[test]
fn test_from_setup() {
    let state = EditorState::from_setup("line 1\nline 2\n", [1, 0]);
    assert!(state.is_ok());
    let state = state.unwrap();
    assert_eq!(state.cursor_position().row, 1);
    assert_eq!(state.cursor_position().col, 0);
}

#[test]
fn test_line_count() {
    let cursor = CursorPosition::new(0, 0).unwrap();
    let state = EditorState::new("line 1\nline 2\nline 3\n".to_string(), cursor, None).unwrap();
    assert_eq!(state.line_count(), 3);
}

#[test]
fn test_current_line() {
    let cursor = CursorPosition::new(1, 0).unwrap();
    let state = EditorState::new("line 1\nline 2\nline 3\n".to_string(), cursor, None).unwrap();
    assert_eq!(state.current_line(), Some("line 2"));
}

#[test]
fn test_set_content_adjusts_cursor() {
    let cursor = CursorPosition::new(2, 5).unwrap();
    let mut state =
        EditorState::new("line 1\nline 2\nline 3\n".to_string(), cursor, None).unwrap();

    // Set content with fewer lines
    state.set_content("only one line\n".to_string()).unwrap();

    // Cursor should be clamped to line 0
    assert_eq!(state.cursor_position().row, 0);
}

#[test]
fn test_move_cursor() {
    let cursor = CursorPosition::new(0, 0).unwrap();
    let mut state = EditorState::new("line 1\nline 2\n".to_string(), cursor, None).unwrap();

    let new_pos = CursorPosition::new(1, 3).unwrap();
    state.move_cursor(new_pos).unwrap();

    assert_eq!(state.cursor_position(), new_pos);
}

#[test]
fn test_move_cursor_out_of_bounds_rejected() {
    let cursor = CursorPosition::new(0, 0).unwrap();
    let mut state = EditorState::new("line 1\n".to_string(), cursor, None).unwrap();

    let invalid_pos = CursorPosition::new(10, 0).unwrap();
    let result = state.move_cursor(invalid_pos);

    assert!(result.is_err());
}

#[test]
fn test_state_matches() {
    let cursor = CursorPosition::new(0, 0).unwrap();
    let state1 = EditorState::new("test\n".to_string(), cursor, None).unwrap();
    let state2 = EditorState::new("test\n".to_string(), cursor, None).unwrap();

    assert!(state1.matches(&state2));
}

#[test]
fn test_content_matches() {
    let cursor1 = CursorPosition::new(0, 0).unwrap();
    let cursor2 = CursorPosition::new(0, 1).unwrap();

    let state1 = EditorState::new("test\n".to_string(), cursor1, None).unwrap();
    let state2 = EditorState::new("test\n".to_string(), cursor2, None).unwrap();

    assert!(state1.content_matches(&state2));
    assert!(!state1.matches(&state2)); // Different cursor positions
}

#[test]
fn test_selection_creation() {
    let start = CursorPosition::new(0, 0).unwrap();
    let end = CursorPosition::new(0, 5).unwrap();
    let selection = Selection::new(start, end);

    assert!(!selection.is_empty());
}

#[test]
fn test_selection_normalized() {
    let start = CursorPosition::new(2, 5).unwrap();
    let end = CursorPosition::new(1, 3).unwrap();
    let selection = Selection::new(start, end);

    let (norm_start, norm_end) = selection.normalized();
    assert_eq!(norm_start, end); // End comes first
    assert_eq!(norm_end, start);
}

#[test]
fn test_empty_content_handling() {
    let cursor = CursorPosition::new(0, 0).unwrap();
    let state = EditorState::new(String::new(), cursor, None).unwrap();
    assert_eq!(state.line_count(), 1); // Empty file still has 1 line
}

#[test]
fn test_selection_empty() {
    let pos = CursorPosition::new(0, 0).unwrap();
    let selection = Selection::new(pos, pos);
    assert!(selection.is_empty());
}

#[test]
fn test_set_selection_with_valid_bounds() {
    let cursor = CursorPosition::new(0, 0).unwrap();
    let mut state = EditorState::new("test content\n".to_string(), cursor, None).unwrap();

    let start = CursorPosition::new(0, 0).unwrap();
    let end = CursorPosition::new(0, 4).unwrap();
    let sel = Selection::new(start, end);

    assert!(state.set_selection(Some(sel)).is_ok());
    assert_eq!(state.selection(), Some(sel));
}

#[test]
fn test_set_selection_clear() {
    let cursor = CursorPosition::new(0, 0).unwrap();
    let start = CursorPosition::new(0, 0).unwrap();
    let end = CursorPosition::new(0, 4).unwrap();
    let sel = Selection::new(start, end);

    let mut state = EditorState::new("test\n".to_string(), cursor, Some(sel)).unwrap();
    assert!(state.selection().is_some());

    assert!(state.set_selection(None).is_ok());
    assert!(state.selection().is_none());
}

#[test]
fn test_default_editor_state() {
    let state = EditorState::default();
    assert_eq!(state.content(), "");
    assert_eq!(state.cursor_position().row, 0);
    assert_eq!(state.cursor_position().col, 0);
    assert!(state.selection().is_none());
}
