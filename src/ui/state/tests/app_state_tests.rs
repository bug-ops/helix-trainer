//! Tests for AppState functionality

use std::assert_matches;

use super::common::{create_test_app_state, create_test_scenario};
use crate::ui::state::TypedScreen;

#[test]
fn test_app_state_debug_impl() {
    let state = create_test_app_state(vec![]);
    // Test that Debug trait works without panic
    let debug_str = format!("{:?}", state);
    assert!(debug_str.contains("AppState"));
    assert!(debug_str.contains("screen"));
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
fn test_scenario_valid_index() {
    let scenario = create_test_scenario();
    let scenarios = vec![scenario];
    let state = create_test_app_state(scenarios);

    let fetched = state.scenario(0);
    assert!(fetched.is_some());
    assert_eq!(fetched.unwrap().id, "test_001");
}

#[test]
fn test_scenario_invalid_index() {
    let scenario = create_test_scenario();
    let state = create_test_app_state(vec![scenario]);

    let fetched = state.scenario(100);
    assert!(fetched.is_none());
}

#[test]
fn test_initial_screen_is_mode_selection() {
    let state = create_test_app_state(vec![]);
    assert_matches!(state.screen, TypedScreen::ModeSelection(_));
}

#[test]
fn test_initial_ui_state() {
    let state = create_test_app_state(vec![]);
    assert!(state.ui.running);
}
