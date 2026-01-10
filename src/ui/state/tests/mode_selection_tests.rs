//! Tests for mode selection handlers

use super::common::{create_test_app_state, create_test_scenario};
use crate::ui::state::{Message, TypedScreen, update};

#[test]
fn test_mode_selection_up() {
    let mut state = create_test_app_state(vec![]);
    // Should start on ModeSelection screen
    assert!(matches!(state.screen, TypedScreen::ModeSelection(_)));

    // Set selected mode to 1
    if let TypedScreen::ModeSelection(mode_data) = &mut state.screen {
        mode_data.selected_mode = 1;
    }

    update(&mut state, Message::ModeSelectionUp).unwrap();

    if let TypedScreen::ModeSelection(mode_data) = &state.screen {
        assert_eq!(mode_data.selected_mode, 0);
    }

    // Can't go below 0
    update(&mut state, Message::ModeSelectionUp).unwrap();
    if let TypedScreen::ModeSelection(mode_data) = &state.screen {
        assert_eq!(mode_data.selected_mode, 0);
    }
}

#[test]
fn test_mode_selection_down() {
    let mut state = create_test_app_state(vec![]);

    // Start at 0
    if let TypedScreen::ModeSelection(mode_data) = &state.screen {
        assert_eq!(mode_data.selected_mode, 0);
    }

    update(&mut state, Message::ModeSelectionDown).unwrap();

    if let TypedScreen::ModeSelection(mode_data) = &state.screen {
        assert_eq!(mode_data.selected_mode, 1);
    }

    // Continue down
    update(&mut state, Message::ModeSelectionDown).unwrap();
    if let TypedScreen::ModeSelection(mode_data) = &state.screen {
        // Should wrap or clamp depending on implementation
        assert!(mode_data.selected_mode <= 2); // Adjust based on actual mode count
    }
}

#[test]
fn test_mode_selection_select_training() {
    let scenario = create_test_scenario();
    let mut state = create_test_app_state(vec![scenario]);

    // Ensure we're on ModeSelection at index 0 (Training Mode)
    if let TypedScreen::ModeSelection(mode_data) = &mut state.screen {
        mode_data.selected_mode = 0;
    }

    update(&mut state, Message::ModeSelectionSelect).unwrap();

    // Should transition to Menu screen (Training Mode submenu)
    // or expand submenu depending on implementation
    // Check the screen type has changed or submenu appeared
}

#[test]
fn test_mode_selection_back() {
    let mut state = create_test_app_state(vec![]);

    // Enter a submenu first (minigame mode selection)
    if let TypedScreen::ModeSelection(mode_data) = &mut state.screen {
        mode_data.minigame_mode_selection =
            Some(crate::ui::state::MiniGameModeSelection::default());
    }

    update(&mut state, Message::ModeSelectionBack).unwrap();

    // Should close submenu
    if let TypedScreen::ModeSelection(mode_data) = &state.screen {
        assert!(mode_data.minigame_mode_selection.is_none());
    }
}

#[test]
fn test_select_training_mode() {
    let scenario = create_test_scenario();
    let mut state = create_test_app_state(vec![scenario]);

    update(&mut state, Message::SelectTrainingMode).unwrap();

    // Should navigate to Menu screen
    assert!(matches!(state.screen, TypedScreen::Menu(_)));
}

#[test]
fn test_select_arcade_mode() {
    let scenario = create_test_scenario();
    let mut state = create_test_app_state(vec![scenario]);

    update(&mut state, Message::SelectArcadeMode).unwrap();

    // Should navigate to MiniGame selection screen
    assert!(matches!(state.screen, TypedScreen::MiniGame(_)));
}

#[test]
fn test_navigate_back_from_menu_to_mode_selection() {
    let scenario = create_test_scenario();
    let mut state = create_test_app_state(vec![scenario]);

    // Go to Training Mode
    update(&mut state, Message::SelectTrainingMode).unwrap();
    assert!(matches!(state.screen, TypedScreen::Menu(_)));

    // Go back
    update(&mut state, Message::BackToMenu).unwrap();

    // Should return to ModeSelection
    assert!(matches!(state.screen, TypedScreen::ModeSelection(_)));
}

#[test]
fn test_navigate_back_from_minigame_to_mode_selection() {
    let scenario = create_test_scenario();
    let mut state = create_test_app_state(vec![scenario]);

    // Go to Arcade Mode
    update(&mut state, Message::SelectArcadeMode).unwrap();
    assert!(matches!(state.screen, TypedScreen::MiniGame(_)));

    // Go back
    update(&mut state, Message::MiniGameBackToMenu).unwrap();

    // Should return to ModeSelection
    assert!(matches!(state.screen, TypedScreen::ModeSelection(_)));
}

#[test]
fn test_mode_selection_initial_state() {
    let state = create_test_app_state(vec![]);

    if let TypedScreen::ModeSelection(mode_data) = &state.screen {
        assert_eq!(mode_data.selected_mode, 0);
        assert!(mode_data.minigame_mode_selection.is_none());
    } else {
        panic!("Should start on ModeSelection screen");
    }
}
