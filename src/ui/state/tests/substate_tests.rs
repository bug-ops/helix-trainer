//! Tests for UI state substates

use super::common::create_test_app_state;

#[test]
fn test_game_state_debug_impl() {
    let state = create_test_app_state(vec![]);
    let debug_str = format!("{:?}", state.game);
    assert!(debug_str.contains("GameState"));
    assert!(debug_str.contains("scenario_count"));
}

#[test]
fn test_progress_state_debug_impl() {
    let state = create_test_app_state(vec![]);
    let debug_str = format!("{:?}", state.progress);
    assert!(debug_str.contains("ProgressState"));
    assert!(debug_str.contains("profile"));
}

#[test]
fn test_progress_state_should_save_initial() {
    let state = create_test_app_state(vec![]);
    // Initial state should allow save (no last_save_time)
    assert!(state.progress.should_save());
}

#[test]
fn test_progress_state_mark_saved_prevents_immediate_resave() {
    let mut state = create_test_app_state(vec![]);

    // Initially should allow save
    assert!(state.progress.should_save());

    // Mark as saved
    state.progress.mark_saved();

    // Immediately after saving, should not allow resave (debounce)
    assert!(!state.progress.should_save());
}

#[test]
fn test_ui_state_initial() {
    let state = create_test_app_state(vec![]);
    assert!(state.ui.running);
}

#[test]
fn test_game_state_default() {
    let state = crate::ui::state::GameState::default();
    assert_eq!(state.scenario_collection.count(), 0);
    assert!(state.pending_completed_session.is_none());
}
