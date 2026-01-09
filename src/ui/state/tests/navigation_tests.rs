//! Tests for navigation and screen transitions

use super::common::{create_test_app_state, create_test_scenario};
use crate::ui::state::{Message, Screen, TypedScreen, update};

#[test]
fn test_new_state() {
    let state = create_test_app_state(vec![]);
    if let TypedScreen::ModeSelection(mode_data) = &state.screen {
        assert_eq!(mode_data.selected_mode, 0);
    } else {
        panic!("Should be on ModeSelection screen");
    }
    assert!(state.ui.running);
    assert!(state.game.review_session.is_none());
    assert!(state.game.pending_completed_session.is_none());
}

#[test]
fn test_quit_app_message() {
    let mut state = create_test_app_state(vec![]);
    assert!(state.ui.running);

    update(&mut state, Message::QuitApp).unwrap();
    assert!(!state.ui.running);
}

#[test]
fn test_navigate_to_screen() {
    let mut state = create_test_app_state(vec![]);
    assert!(matches!(state.screen, TypedScreen::ModeSelection(_)));

    // After TypedScreen refactoring, only screens with standalone data can be navigated to
    // Task and Results require active sessions, so only test Profile/Statistics/Menu/ModeSelection
    update(&mut state, Message::NavigateTo(Screen::Profile)).unwrap();
    assert!(matches!(state.screen, TypedScreen::Profile(_)));

    update(&mut state, Message::NavigateTo(Screen::Statistics)).unwrap();
    assert!(matches!(state.screen, TypedScreen::Statistics(_)));

    update(&mut state, Message::NavigateTo(Screen::MainMenu)).unwrap();
    assert!(matches!(state.screen, TypedScreen::Menu(_)));

    update(&mut state, Message::NavigateTo(Screen::ModeSelection)).unwrap();
    assert!(matches!(state.screen, TypedScreen::ModeSelection(_)));
}

#[test]
fn test_back_to_menu_clears_session() {
    let scenario = create_test_scenario();
    let mut state = create_test_app_state(vec![scenario]);

    update(&mut state, Message::StartScenario(0)).unwrap();
    // After TypedScreen refactoring, verify we're on Task screen
    assert!(matches!(state.screen, TypedScreen::Task(_)));

    update(&mut state, Message::BackToMenu).unwrap();
    // Should transition back to ModeSelection screen (the main menu)
    assert!(matches!(state.screen, TypedScreen::ModeSelection(_)));
}

#[test]
fn test_scenario_count() {
    let scenarios = vec![create_test_scenario(), create_test_scenario()];
    let state = create_test_app_state(scenarios);
    assert_eq!(state.scenario_count(), 2); // Filtered count
}

#[test]
fn test_get_scenario() {
    let scenario = create_test_scenario();
    let mut scenarios = vec![scenario.clone()];
    scenarios.push(scenario);
    let state = create_test_app_state(scenarios);

    assert!(state.get_scenario(0).is_some());
    assert!(state.get_scenario(1).is_some());
    assert!(state.get_scenario(999).is_none());
}
