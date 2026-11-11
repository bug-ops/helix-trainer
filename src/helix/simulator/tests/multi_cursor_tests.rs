//! Tests for multi-cursor snapshot conversion (Issue #141)
//!
//! These tests verify that:
//! - `from_snapshot()` imports ALL selections into `helix_core::Selection`
//! - `display()` exports ALL non-point selections as row/col bounds
//! - Round-trip: `EditorSnapshot` -> Simulator -> `EditorSnapshot` preserves all selections

use crate::helix::simulator::{AnyModeSimulator, EditorSnapshot, HelixSimulator, NormalMode};

#[test]
fn test_from_snapshot_single_cursor() {
    let sim = HelixSimulator::<NormalMode>::from_snapshot(&EditorSnapshot::from_row_col(
        "hello world".to_string(),
        0,
        0,
    ));
    let exported = sim.display();

    assert_eq!(exported.content(), "hello world");
    assert_eq!(exported.cursor_position(), (0, 0));
    assert!(exported.all_selection_bounds().is_empty());
}

#[test]
fn test_from_snapshot_single_selection() {
    let sim = HelixSimulator::<NormalMode>::from_snapshot(&EditorSnapshot::from_row_col_selection(
        "hello world".to_string(),
        [0, 0, 0, 5],
    ));

    assert_eq!(sim.display().all_selection_bounds(), [((0, 0), (0, 5))]);
}

#[test]
fn test_from_snapshot_multiple_selections() {
    let sim = HelixSimulator::<NormalMode>::from_snapshot(&EditorSnapshot::from_multi_selection(
        "hello world".to_string(),
        &[[0, 0, 0, 5], [0, 6, 0, 11]],
    ));

    assert_eq!(
        sim.display().all_selection_bounds(),
        [((0, 0), (0, 5)), ((0, 6), (0, 11))]
    );
}

#[test]
fn test_from_snapshot_multiline_selections() {
    let sim = HelixSimulator::<NormalMode>::from_snapshot(&EditorSnapshot::from_multi_selection(
        "hello\nfoo bar".to_string(),
        &[[0, 0, 0, 5], [1, 0, 1, 3]],
    ));

    assert_eq!(
        sim.display().all_selection_bounds(),
        [((0, 0), (0, 5)), ((1, 0), (1, 3))]
    );
}

#[test]
fn test_round_trip_single_selection() {
    let original = EditorSnapshot::from_row_col_selection("hello world".to_string(), [0, 0, 0, 5]);

    let sim = HelixSimulator::<NormalMode>::from_snapshot(&original);

    assert_eq!(sim.to_snapshot(), original);
    assert_eq!(sim.display().all_selection_bounds(), [((0, 0), (0, 5))]);
}

#[test]
fn test_round_trip_multiple_selections() {
    let original = EditorSnapshot::from_multi_selection(
        "hello\nfoo bar".to_string(),
        &[[0, 0, 0, 5], [1, 0, 1, 3]],
    );

    let sim = HelixSimulator::<NormalMode>::from_snapshot(&original);

    assert_eq!(sim.to_snapshot().content, original.content);
    assert_eq!(sim.to_snapshot().selections.len(), 2);
    assert!(sim.matches_snapshot(&original));
}

#[test]
fn test_round_trip_preserves_primary_idx() {
    let mut original = EditorSnapshot::from_multi_selection(
        "hello world".to_string(),
        &[[0, 0, 0, 5], [0, 6, 0, 11]],
    );
    original.primary_idx = 1;

    let sim = HelixSimulator::<NormalMode>::from_snapshot(&original);

    assert_eq!(sim.to_snapshot().primary_idx, 1);
}

#[test]
fn test_any_mode_simulator_multi_selection() {
    let sim = AnyModeSimulator::from_snapshot(&EditorSnapshot::from_multi_selection(
        "hello world".to_string(),
        &[[0, 0, 0, 5], [0, 6, 0, 11]],
    ));

    assert_eq!(sim.display().all_selection_bounds().len(), 2);
}

#[test]
fn test_point_selections_are_not_exported_as_bounds() {
    // Point selections (anchor == head) are cursors, not selections
    let sim = AnyModeSimulator::from_snapshot(&EditorSnapshot::from_row_col(
        "hello world".to_string(),
        0,
        0,
    ));

    let exported = sim.display();
    assert!(exported.all_selection_bounds().is_empty());
    assert!(exported.selection().is_none());
}

#[test]
fn test_empty_content_multi_cursor() {
    let sim = HelixSimulator::<NormalMode>::from_snapshot(&EditorSnapshot::from_row_col(
        String::new(),
        0,
        0,
    ));
    let exported = sim.display();

    assert_eq!(exported.content(), "");
    assert!(exported.all_selection_bounds().is_empty());
}

#[test]
fn test_selection_at_end_of_line() {
    let sim = HelixSimulator::<NormalMode>::from_snapshot(&EditorSnapshot::from_row_col_selection(
        "hello\nworld".to_string(),
        [0, 0, 0, 5],
    ));

    assert_eq!(sim.display().all_selection_bounds(), [((0, 0), (0, 5))]);
}

#[test]
fn test_cross_line_selection() {
    let sim = HelixSimulator::<NormalMode>::from_snapshot(&EditorSnapshot::from_row_col_selection(
        "hello\nworld".to_string(),
        [0, 2, 1, 3],
    ));

    assert_eq!(sim.display().all_selection_bounds(), [((0, 2), (1, 3))]);
}
