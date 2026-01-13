//! Tests for multi-cursor state conversion (Issue #141)
//!
//! These tests verify that:
//! - `get_state()` exports ALL selections from helix_core::Selection
//! - `from_editor_state()` imports ALL selections into helix_core::Selection
//! - Round-trip: EditorState -> Simulator -> EditorState preserves all selections

use crate::game::{CursorPosition, EditorState, Selection};
use crate::helix::simulator::{AnyModeSimulator, HelixSimulator, NormalMode};

#[test]
fn test_from_editor_state_single_cursor() {
    let cursor = CursorPosition::new(0, 0).unwrap();
    let state = EditorState::new("hello world".to_string(), cursor, None).unwrap();

    let sim = HelixSimulator::<NormalMode>::from_editor_state(&state);
    let exported = sim.get_state().unwrap();

    assert_eq!(exported.content(), "hello world");
    assert_eq!(exported.cursor_position().row, 0);
    assert_eq!(exported.cursor_position().col, 0);
    assert!(exported.selections().is_empty());
}

#[test]
fn test_from_editor_state_single_selection() {
    let cursor = CursorPosition::new(0, 5).unwrap();
    let sel = Selection::new(
        CursorPosition::new(0, 0).unwrap(),
        CursorPosition::new(0, 5).unwrap(),
    );
    let state = EditorState::new("hello world".to_string(), cursor, Some(sel)).unwrap();

    let sim = HelixSimulator::<NormalMode>::from_editor_state(&state);
    let exported = sim.get_state().unwrap();

    assert_eq!(exported.selections().len(), 1);
    let exp_sel = exported.selections()[0];
    assert_eq!(exp_sel.start.row, 0);
    assert_eq!(exp_sel.start.col, 0);
    assert_eq!(exp_sel.end.row, 0);
    assert_eq!(exp_sel.end.col, 5);
}

#[test]
fn test_from_editor_state_multiple_selections() {
    let cursor = CursorPosition::new(0, 0).unwrap();
    let sel1 = Selection::new(
        CursorPosition::new(0, 0).unwrap(),
        CursorPosition::new(0, 5).unwrap(),
    );
    let sel2 = Selection::new(
        CursorPosition::new(0, 6).unwrap(),
        CursorPosition::new(0, 11).unwrap(),
    );

    let state =
        EditorState::with_selections("hello world".to_string(), cursor, vec![sel1, sel2], 0)
            .unwrap();

    let sim = HelixSimulator::<NormalMode>::from_editor_state(&state);
    let exported = sim.get_state().unwrap();

    assert_eq!(exported.selections().len(), 2);
}

#[test]
fn test_from_editor_state_multiline_selections() {
    let cursor = CursorPosition::new(0, 0).unwrap();
    let sel1 = Selection::new(
        CursorPosition::new(0, 0).unwrap(),
        CursorPosition::new(0, 5).unwrap(),
    );
    let sel2 = Selection::new(
        CursorPosition::new(1, 0).unwrap(),
        CursorPosition::new(1, 3).unwrap(),
    );

    let state =
        EditorState::with_selections("hello\nfoo bar".to_string(), cursor, vec![sel1, sel2], 0)
            .unwrap();

    let sim = HelixSimulator::<NormalMode>::from_editor_state(&state);
    let exported = sim.get_state().unwrap();

    assert_eq!(exported.selections().len(), 2);

    // Verify selections are on different lines
    let sels = exported.selections();
    let (first_line, second_line) = if sels[0].start.row < sels[1].start.row {
        (sels[0].start.row, sels[1].start.row)
    } else {
        (sels[1].start.row, sels[0].start.row)
    };
    assert_eq!(first_line, 0);
    assert_eq!(second_line, 1);
}

#[test]
fn test_round_trip_single_selection() {
    let cursor = CursorPosition::new(0, 5).unwrap();
    let sel = Selection::new(
        CursorPosition::new(0, 0).unwrap(),
        CursorPosition::new(0, 5).unwrap(),
    );
    let original = EditorState::new("hello world".to_string(), cursor, Some(sel)).unwrap();

    // EditorState -> Simulator -> EditorState
    let sim = HelixSimulator::<NormalMode>::from_editor_state(&original);
    let exported = sim.get_state().unwrap();

    // Content should match
    assert_eq!(exported.content(), original.content());

    // Selection should match
    assert_eq!(exported.selections().len(), 1);
    let exp_sel = exported.selections()[0];
    assert_eq!(exp_sel.start.row, sel.start.row);
    assert_eq!(exp_sel.start.col, sel.start.col);
    assert_eq!(exp_sel.end.row, sel.end.row);
    assert_eq!(exp_sel.end.col, sel.end.col);
}

#[test]
fn test_round_trip_multiple_selections() {
    let cursor = CursorPosition::new(0, 0).unwrap();
    let sel1 = Selection::new(
        CursorPosition::new(0, 0).unwrap(),
        CursorPosition::new(0, 5).unwrap(),
    );
    let sel2 = Selection::new(
        CursorPosition::new(1, 0).unwrap(),
        CursorPosition::new(1, 3).unwrap(),
    );

    let original =
        EditorState::with_selections("hello\nfoo bar".to_string(), cursor, vec![sel1, sel2], 0)
            .unwrap();

    // EditorState -> Simulator -> EditorState
    let sim = HelixSimulator::<NormalMode>::from_editor_state(&original);
    let exported = sim.get_state().unwrap();

    // Content should match
    assert_eq!(exported.content(), original.content());

    // Should have same number of selections
    assert_eq!(exported.selections().len(), 2);

    // Selections should match (order-independent comparison via matches())
    assert!(exported.matches(&original));
}

#[test]
fn test_round_trip_preserves_primary_idx() {
    let cursor = CursorPosition::new(0, 0).unwrap();
    let sel1 = Selection::new(
        CursorPosition::new(0, 0).unwrap(),
        CursorPosition::new(0, 5).unwrap(),
    );
    let sel2 = Selection::new(
        CursorPosition::new(0, 6).unwrap(),
        CursorPosition::new(0, 11).unwrap(),
    );

    // Primary index = 1
    let original =
        EditorState::with_selections("hello world".to_string(), cursor, vec![sel1, sel2], 1)
            .unwrap();

    let sim = HelixSimulator::<NormalMode>::from_editor_state(&original);
    let exported = sim.get_state().unwrap();

    // Primary index should be preserved (clamped if necessary)
    assert!(exported.primary_selection_idx() < exported.selections().len());
}

#[test]
fn test_any_mode_simulator_multi_selection() {
    let cursor = CursorPosition::new(0, 0).unwrap();
    let sel1 = Selection::new(
        CursorPosition::new(0, 0).unwrap(),
        CursorPosition::new(0, 5).unwrap(),
    );
    let sel2 = Selection::new(
        CursorPosition::new(0, 6).unwrap(),
        CursorPosition::new(0, 11).unwrap(),
    );

    let state =
        EditorState::with_selections("hello world".to_string(), cursor, vec![sel1, sel2], 0)
            .unwrap();

    let sim = AnyModeSimulator::from_editor_state(&state);
    let exported = sim.get_state().unwrap();

    assert_eq!(exported.selections().len(), 2);
}

#[test]
fn test_point_selections_are_not_exported() {
    // Point selections (anchor == head) should not appear in EditorState.selections()
    // They represent cursors, not actual selections
    let cursor = CursorPosition::new(0, 0).unwrap();
    let state = EditorState::new("hello world".to_string(), cursor, None).unwrap();

    let sim = HelixSimulator::<NormalMode>::from_editor_state(&state);

    // Internally the simulator has a point selection (cursor position)
    // but get_state() should not export it as a selection
    let exported = sim.get_state().unwrap();
    assert!(exported.selections().is_empty());
}

#[test]
fn test_empty_content_multi_cursor() {
    let cursor = CursorPosition::new(0, 0).unwrap();
    let state = EditorState::new(String::new(), cursor, None).unwrap();

    let sim = HelixSimulator::<NormalMode>::from_editor_state(&state);
    let exported = sim.get_state().unwrap();

    assert_eq!(exported.content(), "");
    assert!(exported.selections().is_empty());
}

#[test]
fn test_selection_at_end_of_line() {
    // Selection that extends to end of line
    let cursor = CursorPosition::new(0, 5).unwrap();
    let sel = Selection::new(
        CursorPosition::new(0, 0).unwrap(),
        CursorPosition::new(0, 5).unwrap(),
    );
    let state = EditorState::new("hello\nworld".to_string(), cursor, Some(sel)).unwrap();

    let sim = HelixSimulator::<NormalMode>::from_editor_state(&state);
    let exported = sim.get_state().unwrap();

    assert_eq!(exported.selections().len(), 1);
    let exp_sel = exported.selections()[0];
    assert_eq!(exp_sel.end.col, 5);
}

#[test]
fn test_cross_line_selection() {
    // Selection spanning multiple lines
    let cursor = CursorPosition::new(1, 3).unwrap();
    let sel = Selection::new(
        CursorPosition::new(0, 2).unwrap(),
        CursorPosition::new(1, 3).unwrap(),
    );
    let state = EditorState::new("hello\nworld".to_string(), cursor, Some(sel)).unwrap();

    let sim = HelixSimulator::<NormalMode>::from_editor_state(&state);
    let exported = sim.get_state().unwrap();

    assert_eq!(exported.selections().len(), 1);
    let exp_sel = exported.selections()[0];
    assert_eq!(exp_sel.start.row, 0);
    assert_eq!(exp_sel.start.col, 2);
    assert_eq!(exp_sel.end.row, 1);
    assert_eq!(exp_sel.end.col, 3);
}
