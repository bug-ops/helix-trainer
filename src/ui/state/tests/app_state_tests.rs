//! Tests for AppState functionality

use super::common::{create_test_app_state, create_test_scenario};
use crate::ui::state::TypedScreen;
use tempfile::TempDir;

#[test]
fn test_app_state_debug_impl() {
    let state = create_test_app_state(vec![]);
    // Test that Debug trait works without panic
    let debug_str = format!("{:?}", state);
    assert!(debug_str.contains("AppState"));
    assert!(debug_str.contains("screen"));
}

#[test]
fn test_save_profile_debounced_skips_if_not_needed() {
    let scenario = create_test_scenario();
    let mut state = create_test_app_state(vec![scenario]);

    // Mark as just saved
    state.progress.mark_saved();

    // Should not save again immediately (debounce in effect)
    let result = state.save_profile_debounced();
    assert!(result.is_ok());
}

#[test]
fn test_save_profile_immediate_always_saves() {
    let temp_dir = TempDir::new().unwrap();
    let profile_path = temp_dir.path().join("profile.json");

    let scenario = create_test_scenario();
    let mut state = create_test_app_state(vec![scenario]);

    // Change storage path to temp directory
    state.progress.storage = crate::gamification::ProfileStorage::with_path(&profile_path);

    // Should save immediately
    let result = state.save_profile_immediate();
    assert!(result.is_ok());
    assert!(profile_path.exists());
}

#[test]
fn test_save_profile_debounced_saves_when_first_needed() {
    let temp_dir = TempDir::new().unwrap();
    let profile_path = temp_dir.path().join("profile.json");

    let scenario = create_test_scenario();
    let mut state = create_test_app_state(vec![scenario]);

    // Change storage path to temp directory
    state.progress.storage = crate::gamification::ProfileStorage::with_path(&profile_path);

    // Should save first time (no last_save_time set)
    assert!(state.progress.should_save());
    let result = state.save_profile_debounced();
    assert!(result.is_ok());
    assert!(profile_path.exists());
}

#[test]
fn test_scenario_count_empty() {
    let state = create_test_app_state(vec![]);
    assert_eq!(state.scenario_count(), 0);
}

#[test]
fn test_scenario_count_multiple() {
    let scenarios = vec![create_test_scenario(), create_test_scenario()];
    let state = create_test_app_state(scenarios);
    assert_eq!(state.scenario_count(), 2);
}

#[test]
fn test_get_scenario_valid_index() {
    let scenario = create_test_scenario();
    let scenarios = vec![scenario];
    let state = create_test_app_state(scenarios);

    let fetched = state.get_scenario(0);
    assert!(fetched.is_some());
    assert_eq!(fetched.unwrap().id, "test_001");
}

#[test]
fn test_get_scenario_invalid_index() {
    let scenario = create_test_scenario();
    let state = create_test_app_state(vec![scenario]);

    let fetched = state.get_scenario(100);
    assert!(fetched.is_none());
}

#[test]
fn test_initial_screen_is_mode_selection() {
    let state = create_test_app_state(vec![]);
    assert!(matches!(state.screen, TypedScreen::ModeSelection(_)));
}

#[test]
fn test_initial_ui_state() {
    let state = create_test_app_state(vec![]);
    assert!(state.ui.running);
}
