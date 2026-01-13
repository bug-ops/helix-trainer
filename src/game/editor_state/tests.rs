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
    let state = EditorState::from_setup("line 1\nline 2\n", [1, 0], None);
    assert!(state.is_ok());
    let state = state.unwrap();
    assert_eq!(state.cursor_position().0, 1);
    assert_eq!(state.cursor_position().1, 0);
}

#[test]
fn test_from_setup_with_selection() {
    let state = EditorState::from_setup("hello world", [0, 0], Some([0, 0, 0, 5]));
    assert!(state.is_ok());
    let state = state.unwrap();
    assert!(state.selection().is_some());
    let sel = state.selection().unwrap();
    assert_eq!(sel.start.col, 0);
    assert_eq!(sel.end.col, 5);
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
    let mut state = EditorState::new("line 1\nline 2\nline 3\n".to_string(), cursor, None).unwrap();

    // Set content with fewer lines
    state.set_content("only one line\n".to_string()).unwrap();

    // Cursor should be clamped to line 0
    assert_eq!(state.cursor_position().0, 0);
}

#[test]
fn test_move_cursor() {
    let cursor = CursorPosition::new(0, 0).unwrap();
    let mut state = EditorState::new("line 1\nline 2\n".to_string(), cursor, None).unwrap();

    let new_pos = CursorPosition::new(1, 3).unwrap();
    state.move_cursor(new_pos).unwrap();

    assert_eq!(state.cursor_position(), (1, 3));
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
    assert_eq!(state.cursor_position().0, 0);
    assert_eq!(state.cursor_position().1, 0);
    assert!(state.selection().is_none());
}

// Tests for matches() with selection comparison (Phase 1 text objects)

#[test]
fn test_matches_with_target_selection() {
    // Current state has selection matching target - should match
    let cursor = CursorPosition::new(0, 5).unwrap();
    let sel = Selection::new(
        CursorPosition::new(0, 0).unwrap(),
        CursorPosition::new(0, 5).unwrap(),
    );
    let state1 = EditorState::new("hello world".to_string(), cursor, Some(sel)).unwrap();
    let state2 = EditorState::new("hello world".to_string(), cursor, Some(sel)).unwrap();

    assert!(state1.matches(&state2));
}

#[test]
fn test_matches_selection_mismatch() {
    // Same content, same cursor, different selections - should NOT match
    let cursor = CursorPosition::new(0, 0).unwrap();
    let sel1 = Selection::new(
        CursorPosition::new(0, 0).unwrap(),
        CursorPosition::new(0, 5).unwrap(),
    );
    let sel2 = Selection::new(
        CursorPosition::new(0, 0).unwrap(),
        CursorPosition::new(0, 3).unwrap(),
    );
    let state1 = EditorState::new("hello world".to_string(), cursor, Some(sel1)).unwrap();
    let state2 = EditorState::new("hello world".to_string(), cursor, Some(sel2)).unwrap();

    assert!(!state1.matches(&state2));
}

#[test]
fn test_matches_target_has_selection_current_does_not() {
    // Target has selection, current doesn't - should NOT match
    let cursor = CursorPosition::new(0, 0).unwrap();
    let sel = Selection::new(
        CursorPosition::new(0, 0).unwrap(),
        CursorPosition::new(0, 5).unwrap(),
    );
    let state1 = EditorState::new("hello world".to_string(), cursor, None).unwrap();
    let state2 = EditorState::new("hello world".to_string(), cursor, Some(sel)).unwrap();

    assert!(!state1.matches(&state2));
}

#[test]
fn test_matches_current_has_selection_target_does_not() {
    // Current has selection, target doesn't - checks cursor only, should match
    let cursor = CursorPosition::new(0, 0).unwrap();
    let sel = Selection::new(
        CursorPosition::new(0, 0).unwrap(),
        CursorPosition::new(0, 5).unwrap(),
    );
    let state1 = EditorState::new("hello world".to_string(), cursor, Some(sel)).unwrap();
    let state2 = EditorState::new("hello world".to_string(), cursor, None).unwrap();

    // When target has no selection, only cursor is checked
    assert!(state1.matches(&state2));
}

#[test]
fn test_from_target_with_selection() {
    let state = EditorState::from_target("hello world", [0, 5], Some([0, 0, 0, 5])).unwrap();

    assert!(state.selection().is_some());
    let sel = state.selection().unwrap();
    assert_eq!(sel.start.row, 0);
    assert_eq!(sel.start.col, 0);
    assert_eq!(sel.end.row, 0);
    assert_eq!(sel.end.col, 5);
}

// Tests for multi-selection support (Issue #141)

#[test]
fn test_with_selections_creates_multiple() {
    let cursor = CursorPosition::new(0, 0).unwrap();
    let sel1 = Selection::new(
        CursorPosition::new(0, 0).unwrap(),
        CursorPosition::new(0, 5).unwrap(),
    );
    let sel2 = Selection::new(
        CursorPosition::new(1, 0).unwrap(),
        CursorPosition::new(1, 3).unwrap(),
    );

    let state = EditorState::with_selections(
        "hello world\nfoo bar".to_string(),
        cursor,
        vec![sel1, sel2],
        0,
    )
    .unwrap();

    assert_eq!(state.selections().len(), 2);
    assert_eq!(state.selection(), Some(sel1)); // Primary is index 0
    assert_eq!(state.primary_selection_idx(), 0);
}

#[test]
fn test_with_selections_primary_index() {
    let cursor = CursorPosition::new(0, 0).unwrap();
    let sel1 = Selection::new(
        CursorPosition::new(0, 0).unwrap(),
        CursorPosition::new(0, 5).unwrap(),
    );
    let sel2 = Selection::new(
        CursorPosition::new(1, 0).unwrap(),
        CursorPosition::new(1, 3).unwrap(),
    );

    let state = EditorState::with_selections(
        "hello world\nfoo bar".to_string(),
        cursor,
        vec![sel1, sel2],
        1, // Primary is index 1
    )
    .unwrap();

    assert_eq!(state.selection(), Some(sel2)); // Primary is sel2
    assert_eq!(state.primary_selection_idx(), 1);
}

#[test]
fn test_with_selections_invalid_primary_idx() {
    let cursor = CursorPosition::new(0, 0).unwrap();
    let sel1 = Selection::new(
        CursorPosition::new(0, 0).unwrap(),
        CursorPosition::new(0, 5).unwrap(),
    );

    let result = EditorState::with_selections(
        "hello world".to_string(),
        cursor,
        vec![sel1],
        5, // Invalid index
    );

    assert!(result.is_err());
}

#[test]
fn test_add_selection() {
    let cursor = CursorPosition::new(0, 0).unwrap();
    let sel1 = Selection::new(
        CursorPosition::new(0, 0).unwrap(),
        CursorPosition::new(0, 5).unwrap(),
    );

    let mut state =
        EditorState::new("hello world\nfoo bar".to_string(), cursor, Some(sel1)).unwrap();

    assert_eq!(state.selections().len(), 1);

    let sel2 = Selection::new(
        CursorPosition::new(1, 0).unwrap(),
        CursorPosition::new(1, 3).unwrap(),
    );
    state.add_selection(sel2).unwrap();

    assert_eq!(state.selections().len(), 2);
    assert_eq!(state.selections()[1], sel2);
}

#[test]
fn test_add_selection_out_of_bounds() {
    let cursor = CursorPosition::new(0, 0).unwrap();
    let mut state = EditorState::new("hello world".to_string(), cursor, None).unwrap();

    let invalid_sel = Selection::new(
        CursorPosition::new(5, 0).unwrap(), // Out of bounds
        CursorPosition::new(5, 3).unwrap(),
    );
    let result = state.add_selection(invalid_sel);

    assert!(result.is_err());
}

#[test]
fn test_set_selections() {
    let cursor = CursorPosition::new(0, 0).unwrap();
    let mut state = EditorState::new("hello world\nfoo bar".to_string(), cursor, None).unwrap();

    let sel1 = Selection::new(
        CursorPosition::new(0, 0).unwrap(),
        CursorPosition::new(0, 5).unwrap(),
    );
    let sel2 = Selection::new(
        CursorPosition::new(1, 0).unwrap(),
        CursorPosition::new(1, 3).unwrap(),
    );

    state.set_selections(vec![sel1, sel2], 1).unwrap();

    assert_eq!(state.selections().len(), 2);
    assert_eq!(state.primary_selection_idx(), 1);
    assert_eq!(state.selection(), Some(sel2));
}

#[test]
fn test_set_primary_selection() {
    let cursor = CursorPosition::new(0, 0).unwrap();
    let sel1 = Selection::new(
        CursorPosition::new(0, 0).unwrap(),
        CursorPosition::new(0, 5).unwrap(),
    );
    let sel2 = Selection::new(
        CursorPosition::new(1, 0).unwrap(),
        CursorPosition::new(1, 3).unwrap(),
    );

    let mut state = EditorState::with_selections(
        "hello world\nfoo bar".to_string(),
        cursor,
        vec![sel1, sel2],
        0,
    )
    .unwrap();

    assert_eq!(state.primary_selection_idx(), 0);
    state.set_primary_selection(1).unwrap();
    assert_eq!(state.primary_selection_idx(), 1);
    assert_eq!(state.selection(), Some(sel2));
}

#[test]
fn test_set_primary_selection_no_selections() {
    let cursor = CursorPosition::new(0, 0).unwrap();
    let mut state = EditorState::new("hello world".to_string(), cursor, None).unwrap();

    let result = state.set_primary_selection(0);
    assert!(result.is_err());
}

#[test]
fn test_set_primary_selection_out_of_bounds() {
    let cursor = CursorPosition::new(0, 0).unwrap();
    let sel = Selection::new(
        CursorPosition::new(0, 0).unwrap(),
        CursorPosition::new(0, 5).unwrap(),
    );

    let mut state = EditorState::new("hello world".to_string(), cursor, Some(sel)).unwrap();

    let result = state.set_primary_selection(5);
    assert!(result.is_err());
}

#[test]
fn test_matches_multiple_selections() {
    let cursor = CursorPosition::new(0, 0).unwrap();
    let sel1 = Selection::new(
        CursorPosition::new(0, 0).unwrap(),
        CursorPosition::new(0, 5).unwrap(),
    );
    let sel2 = Selection::new(
        CursorPosition::new(1, 0).unwrap(),
        CursorPosition::new(1, 3).unwrap(),
    );

    let state1 = EditorState::with_selections(
        "hello world\nfoo bar".to_string(),
        cursor,
        vec![sel1, sel2],
        0,
    )
    .unwrap();

    let state2 = EditorState::with_selections(
        "hello world\nfoo bar".to_string(),
        cursor,
        vec![sel1, sel2],
        1, // Different primary, but same selections
    )
    .unwrap();

    assert!(state1.matches(&state2));
}

#[test]
fn test_matches_multiple_selections_order_independent() {
    let cursor = CursorPosition::new(0, 0).unwrap();
    let sel1 = Selection::new(
        CursorPosition::new(0, 0).unwrap(),
        CursorPosition::new(0, 5).unwrap(),
    );
    let sel2 = Selection::new(
        CursorPosition::new(1, 0).unwrap(),
        CursorPosition::new(1, 3).unwrap(),
    );

    // Order is [sel1, sel2]
    let state1 = EditorState::with_selections(
        "hello world\nfoo bar".to_string(),
        cursor,
        vec![sel1, sel2],
        0,
    )
    .unwrap();

    // Order is [sel2, sel1] - reversed
    let state2 = EditorState::with_selections(
        "hello world\nfoo bar".to_string(),
        cursor,
        vec![sel2, sel1],
        0,
    )
    .unwrap();

    // Should still match because comparison is order-independent
    assert!(state1.matches(&state2));
}

#[test]
fn test_matches_different_selection_count() {
    let cursor = CursorPosition::new(0, 0).unwrap();
    let sel1 = Selection::new(
        CursorPosition::new(0, 0).unwrap(),
        CursorPosition::new(0, 5).unwrap(),
    );
    let sel2 = Selection::new(
        CursorPosition::new(1, 0).unwrap(),
        CursorPosition::new(1, 3).unwrap(),
    );

    let state1 = EditorState::with_selections(
        "hello world\nfoo bar".to_string(),
        cursor,
        vec![sel1, sel2],
        0,
    )
    .unwrap();

    let state2 = EditorState::new("hello world\nfoo bar".to_string(), cursor, Some(sel1)).unwrap();

    // Different number of selections
    assert!(!state1.matches(&state2));
}

#[test]
fn test_selections_empty_returns_empty_slice() {
    let cursor = CursorPosition::new(0, 0).unwrap();
    let state = EditorState::new("hello world".to_string(), cursor, None).unwrap();

    assert!(state.selections().is_empty());
    assert!(state.selection().is_none());
}

#[test]
fn test_set_content_removes_invalid_selections() {
    let cursor = CursorPosition::new(0, 0).unwrap();
    let sel1 = Selection::new(
        CursorPosition::new(0, 0).unwrap(),
        CursorPosition::new(0, 5).unwrap(),
    );
    let sel2 = Selection::new(
        CursorPosition::new(1, 0).unwrap(),
        CursorPosition::new(1, 3).unwrap(),
    );

    let mut state = EditorState::with_selections(
        "hello world\nfoo bar".to_string(),
        cursor,
        vec![sel1, sel2],
        0,
    )
    .unwrap();

    assert_eq!(state.selections().len(), 2);

    // Set content with only one line - sel2 should be removed
    state.set_content("only one line".to_string()).unwrap();

    assert_eq!(state.selections().len(), 1);
    assert_eq!(state.selection(), Some(sel1));
}

#[test]
fn test_default_selections_empty() {
    let state = EditorState::default();
    assert!(state.selections().is_empty());
    assert_eq!(state.primary_selection_idx(), 0);
}

// Additional tests for edge cases (Issue #141 code review)

#[test]
fn test_with_selections_empty_vec() {
    let cursor = CursorPosition::new(0, 0).unwrap();
    let state = EditorState::with_selections("hello world".to_string(), cursor, vec![], 0);
    assert!(state.is_ok());
    let state = state.unwrap();
    assert!(state.selections().is_empty());
    assert!(state.selection().is_none());
}

#[test]
fn test_set_selections_empty_clears_all() {
    let cursor = CursorPosition::new(0, 0).unwrap();
    let sel = Selection::new(
        CursorPosition::new(0, 0).unwrap(),
        CursorPosition::new(0, 5).unwrap(),
    );
    let mut state = EditorState::new("hello world".to_string(), cursor, Some(sel)).unwrap();

    state.set_selections(vec![], 0).unwrap();

    assert!(state.selections().is_empty());
    assert!(state.selection().is_none());
}

#[test]
fn test_set_selections_invalid_primary_idx() {
    let cursor = CursorPosition::new(0, 0).unwrap();
    let mut state = EditorState::new("hello world".to_string(), cursor, None).unwrap();

    let sel = Selection::new(
        CursorPosition::new(0, 0).unwrap(),
        CursorPosition::new(0, 5).unwrap(),
    );

    let result = state.set_selections(vec![sel], 5);
    assert!(result.is_err());
}

#[test]
fn test_add_selection_to_empty() {
    let cursor = CursorPosition::new(0, 0).unwrap();
    let mut state = EditorState::new("hello world".to_string(), cursor, None).unwrap();

    assert!(state.selections().is_empty());

    let sel = Selection::new(
        CursorPosition::new(0, 0).unwrap(),
        CursorPosition::new(0, 5).unwrap(),
    );
    state.add_selection(sel).unwrap();

    assert_eq!(state.selections().len(), 1);
    assert_eq!(state.selection(), Some(sel));
}

#[test]
fn test_set_content_resets_primary_idx() {
    let cursor = CursorPosition::new(0, 0).unwrap();
    let sel1 = Selection::new(
        CursorPosition::new(0, 0).unwrap(),
        CursorPosition::new(0, 5).unwrap(),
    );
    let sel2 = Selection::new(
        CursorPosition::new(1, 0).unwrap(),
        CursorPosition::new(1, 3).unwrap(),
    );

    let mut state = EditorState::with_selections(
        "hello world\nfoo bar".to_string(),
        cursor,
        vec![sel1, sel2],
        1, // Primary is sel2 on line 1
    )
    .unwrap();

    assert_eq!(state.primary_selection_idx(), 1);

    // Shrink content to remove sel2
    state.set_content("only one line".to_string()).unwrap();

    // Primary idx should reset to 0
    assert_eq!(state.primary_selection_idx(), 0);
    assert_eq!(state.selections().len(), 1);
}

#[test]
fn test_matches_overlapping_selections() {
    let cursor = CursorPosition::new(0, 0).unwrap();
    let sel1 = Selection::new(
        CursorPosition::new(0, 0).unwrap(),
        CursorPosition::new(0, 5).unwrap(),
    );
    let sel2 = Selection::new(
        CursorPosition::new(0, 3).unwrap(),
        CursorPosition::new(0, 8).unwrap(),
    );

    let state1 =
        EditorState::with_selections("hello world".to_string(), cursor, vec![sel1, sel2], 0)
            .unwrap();

    let state2 =
        EditorState::with_selections("hello world".to_string(), cursor, vec![sel1, sel2], 0)
            .unwrap();

    assert!(state1.matches(&state2));
}

#[test]
fn test_matches_duplicate_selections() {
    let cursor = CursorPosition::new(0, 0).unwrap();
    let sel = Selection::new(
        CursorPosition::new(0, 0).unwrap(),
        CursorPosition::new(0, 5).unwrap(),
    );

    let state1 = EditorState::with_selections(
        "hello world".to_string(),
        cursor,
        vec![sel, sel], // Same selection twice
        0,
    )
    .unwrap();

    let state2 =
        EditorState::with_selections("hello world".to_string(), cursor, vec![sel, sel], 0).unwrap();

    assert!(state1.matches(&state2));
}

// ============================================================================
// TESTING-001: Tests for from_scenario_setup() and from_scenario_target()
// ============================================================================

#[test]
fn test_from_scenario_setup_single_cursor() {
    // Backward compatible single cursor format
    let state =
        EditorState::from_scenario_setup("test content", Some((0, 4)), None, None, None).unwrap();

    assert_eq!(state.cursor_position().0, 0);
    assert_eq!(state.cursor_position().1, 4);
    assert!(state.selections().is_empty());
}

#[test]
fn test_from_scenario_setup_with_cursors() {
    // Multi-cursor format using cursors array
    let cursors = vec![[0, 0], [0, 5]];
    let state =
        EditorState::from_scenario_setup("test content", None, None, Some(&cursors), None).unwrap();

    assert_eq!(state.selections().len(), 2);
    // First cursor position should be used
    assert_eq!(state.cursor_position().0, 0);
    assert_eq!(state.cursor_position().1, 0);
}

#[test]
fn test_from_scenario_setup_with_selections() {
    // Multi-selection format
    let selections = vec![[0, 0, 0, 4], [0, 5, 0, 12]];
    let state =
        EditorState::from_scenario_setup("test content", None, None, None, Some(&selections))
            .unwrap();

    assert_eq!(state.selections().len(), 2);
    // Cursor should be at first selection's end
    assert_eq!(state.cursor_position().0, 0);
    assert_eq!(state.cursor_position().1, 4);
}

#[test]
fn test_from_scenario_target_single_cursor() {
    let state = EditorState::from_scenario_target("target content", Some((0, 6)), None, None, None)
        .unwrap();

    assert_eq!(state.cursor_position().0, 0);
    assert_eq!(state.cursor_position().1, 6);
}

#[test]
fn test_from_scenario_target_with_cursors() {
    let cursors = vec![[0, 0], [0, 7]];
    let state =
        EditorState::from_scenario_target("target content", None, None, Some(&cursors), None)
            .unwrap();

    assert_eq!(state.selections().len(), 2);
}

#[test]
fn test_from_scenario_target_with_selections() {
    let selections = vec![[0, 0, 0, 6]];
    let state =
        EditorState::from_scenario_target("target content", None, None, None, Some(&selections))
            .unwrap();

    assert_eq!(state.selections().len(), 1);
    let sel = state.selections()[0];
    assert_eq!(sel.start.col, 0);
    assert_eq!(sel.end.col, 6);
}

// ============================================================================
// TESTING-002: Tests for from_multi_cursor_config() format priority
// ============================================================================

#[test]
fn test_format_priority_selections_over_cursors() {
    // When both selections and cursors are provided, selections takes precedence
    let cursors = vec![[0, 0], [0, 1]];
    let selections = vec![[0, 0, 0, 5]];

    let state = EditorState::from_scenario_setup(
        "test content",
        None,
        None,
        Some(&cursors),
        Some(&selections),
    )
    .unwrap();

    // Should have 1 selection (from selections), not 2 (from cursors)
    assert_eq!(state.selections().len(), 1);
    assert_eq!(state.selections()[0].end.col, 5);
}

#[test]
fn test_format_priority_cursors_over_cursor_position() {
    // When both cursors and cursor_position are provided, cursors takes precedence
    let cursors = vec![[0, 5], [0, 10]];

    let state = EditorState::from_scenario_setup(
        "test content",
        Some((0, 0)), // This should be ignored
        None,
        Some(&cursors),
        None,
    )
    .unwrap();

    // Should have 2 selections (from cursors), cursor at first cursor position
    assert_eq!(state.selections().len(), 2);
    assert_eq!(state.cursor_position().1, 5);
}

#[test]
fn test_format_priority_empty_selections_array() {
    // Empty selections array should still take precedence (empty result)
    let empty_selections: Vec<[usize; 4]> = vec![];

    let state = EditorState::from_scenario_setup(
        "test content",
        Some((0, 5)),
        None,
        None,
        Some(&empty_selections),
    )
    .unwrap();

    // Empty selections means no selections, cursor defaults to (0,0)
    assert!(state.selections().is_empty());
    assert_eq!(state.cursor_position().0, 0);
    assert_eq!(state.cursor_position().1, 0);
}

#[test]
fn test_format_priority_empty_cursors_array() {
    // Empty cursors array should still take precedence
    let empty_cursors: Vec<[usize; 2]> = vec![];

    let state = EditorState::from_scenario_setup(
        "test content",
        Some((0, 5)),
        None,
        Some(&empty_cursors),
        None,
    )
    .unwrap();

    // Empty cursors means no selections, cursor defaults to (0,0)
    assert!(state.selections().is_empty());
    assert_eq!(state.cursor_position().0, 0);
    assert_eq!(state.cursor_position().1, 0);
}

#[test]
fn test_format_default_cursor_fallback() {
    // When no cursor info is provided, default to (0, 0)
    let state = EditorState::from_scenario_setup("test content", None, None, None, None).unwrap();

    assert_eq!(state.cursor_position().0, 0);
    assert_eq!(state.cursor_position().1, 0);
}

#[test]
fn test_single_cursor_to_selection_conversion() {
    // cursors array converts to point selections
    let cursors = vec![[0, 3]];

    let state =
        EditorState::from_scenario_setup("test content", None, None, Some(&cursors), None).unwrap();

    assert_eq!(state.selections().len(), 1);
    let sel = state.selections()[0];
    // Point selection: start == end
    assert_eq!(sel.start.col, sel.end.col);
    assert_eq!(sel.start.row, sel.end.row);
}

// ============================================================================
// SEC-002: Early bounds validation tests
// ============================================================================

#[test]
fn test_from_scenario_setup_invalid_cursor_row() {
    // Cursor row exceeds content bounds
    let cursors = vec![[5, 0]]; // Row 5 doesn't exist in single-line content

    let result = EditorState::from_scenario_setup("test", None, None, Some(&cursors), None);

    assert!(result.is_err());
}

#[test]
fn test_from_scenario_setup_invalid_selection_row() {
    // Selection row exceeds content bounds
    let selections = vec![[0, 0, 5, 0]]; // End row 5 doesn't exist

    let result = EditorState::from_scenario_setup("test", None, None, None, Some(&selections));

    assert!(result.is_err());
}
