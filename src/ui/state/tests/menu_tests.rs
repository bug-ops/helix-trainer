//! Tests for menu navigation and selection

use super::common::{create_test_app_state, create_test_scenario};
use crate::ui::state::{Message, TypedScreen, update};

#[test]
fn test_menu_navigation_up() {
    let mut state = create_test_app_state(vec![]);
    // Navigate to Menu screen first (starts on ModeSelection)
    update(&mut state, Message::SelectTrainingMode).unwrap();
    // Set initial menu item to 1
    if let TypedScreen::Menu(menu_data) = &mut state.screen {
        menu_data.selected_item = 1;
    }

    update(&mut state, Message::MenuUp).unwrap();
    if let TypedScreen::Menu(menu_data) = &state.screen {
        assert_eq!(menu_data.selected_item, 0);
    }

    // Can't go below 0
    update(&mut state, Message::MenuUp).unwrap();
    if let TypedScreen::Menu(menu_data) = &state.screen {
        assert_eq!(menu_data.selected_item, 0);
    }
}

#[test]
fn test_menu_navigation_down() {
    let scenario1 = create_test_scenario();
    let scenario2 = create_test_scenario();
    let mut state = create_test_app_state(vec![scenario1, scenario2]);

    // Navigate to Menu screen first (starts on ModeSelection)
    update(&mut state, Message::SelectTrainingMode).unwrap();

    if let TypedScreen::Menu(menu_data) = &state.screen {
        assert_eq!(menu_data.selected_item, 0);
    }

    // Move down once
    update(&mut state, Message::MenuDown).unwrap();
    if let TypedScreen::Menu(menu_data) = &state.screen {
        assert_eq!(menu_data.selected_item, 1);
    }

    // Move down to Review
    update(&mut state, Message::MenuDown).unwrap();
    if let TypedScreen::Menu(menu_data) = &state.screen {
        assert_eq!(menu_data.selected_item, 2); // Review
    }

    // Move down to Profile
    update(&mut state, Message::MenuDown).unwrap();
    if let TypedScreen::Menu(menu_data) = &state.screen {
        assert_eq!(menu_data.selected_item, 3); // Profile
    }

    // Move down to Statistics
    update(&mut state, Message::MenuDown).unwrap();
    if let TypedScreen::Menu(menu_data) = &state.screen {
        assert_eq!(menu_data.selected_item, 4); // Statistics
    }

    // Move down to Quit
    update(&mut state, Message::MenuDown).unwrap();
    if let TypedScreen::Menu(menu_data) = &state.screen {
        assert_eq!(menu_data.selected_item, 5); // Quit
    }

    // Can't go past max items
    update(&mut state, Message::MenuDown).unwrap();
    if let TypedScreen::Menu(menu_data) = &state.screen {
        assert_eq!(menu_data.selected_item, 5);
    }
}

#[test]
fn test_menu_select_start_training() {
    let scenario = create_test_scenario();
    let mut state = create_test_app_state(vec![scenario]);
    // Navigate to menu first (start on ModeSelection)
    update(&mut state, Message::SelectTrainingMode).unwrap();

    update(&mut state, Message::MenuSelect).unwrap();

    // After TypedScreen refactoring, session is inside TaskData
    if let TypedScreen::Task(task_data) = &state.screen {
        // Session exists inside TaskData
        assert!(!task_data.session.current_state().content().is_empty());
    } else {
        panic!("Should be on Task screen with active session");
    }
}

#[test]
fn test_menu_select_quit() {
    let scenario1 = create_test_scenario();
    let scenario2 = create_test_scenario();
    let mut state = create_test_app_state(vec![scenario1, scenario2]);
    // Navigate to menu first
    update(&mut state, Message::SelectTrainingMode).unwrap();

    // Select Quit option (index = scenario_count + 3)
    if let TypedScreen::Menu(menu_data) = &mut state.screen {
        menu_data.selected_item = 5; // 2 scenarios + Review + Profile + Statistics + Quit = index 5
    }

    update(&mut state, Message::MenuSelect).unwrap();

    assert!(!state.ui.running);
}

#[test]
fn test_menu_select_profile() {
    let scenario = create_test_scenario();
    let mut state = create_test_app_state(vec![scenario]);
    // Navigate to menu first
    update(&mut state, Message::SelectTrainingMode).unwrap();

    if let TypedScreen::Menu(menu_data) = &mut state.screen {
        menu_data.selected_item = 2; // Profile is at index 2 (after 1 scenario + Review)
    }

    update(&mut state, Message::MenuSelect).unwrap();
    assert!(matches!(state.screen, TypedScreen::Profile(_)));
}

#[test]
fn test_menu_select_statistics() {
    let scenario = create_test_scenario();
    let mut state = create_test_app_state(vec![scenario]);
    // Navigate to menu first
    update(&mut state, Message::SelectTrainingMode).unwrap();

    if let TypedScreen::Menu(menu_data) = &mut state.screen {
        menu_data.selected_item = 3; // Statistics is at index 3 (after 1 scenario + Review + Profile)
    }

    update(&mut state, Message::MenuSelect).unwrap();
    assert!(matches!(state.screen, TypedScreen::Statistics(_)));
}

#[test]
fn test_menu_with_zero_scenarios() {
    // Edge case: no scenarios loaded
    let mut state = create_test_app_state(vec![]);
    // Navigate to menu first
    update(&mut state, Message::SelectTrainingMode).unwrap();

    // Review should be at index 0 (no scenarios)
    update(&mut state, Message::MenuSelect).unwrap();
    // Should stay on MainMenu if no reviews are due
    assert!(matches!(state.screen, TypedScreen::Menu(_)));

    // Profile at index 1
    if let TypedScreen::Menu(menu_data) = &mut state.screen {
        menu_data.selected_item = 1;
    }
    update(&mut state, Message::MenuSelect).unwrap();
    assert!(matches!(state.screen, TypedScreen::Profile(_)));

    // Statistics at index 2
    state.screen = TypedScreen::Menu(Default::default());
    if let TypedScreen::Menu(menu_data) = &mut state.screen {
        menu_data.selected_item = 2;
    }
    update(&mut state, Message::MenuSelect).unwrap();
    assert!(matches!(state.screen, TypedScreen::Statistics(_)));

    // Quit at index 3
    state.screen = TypedScreen::Menu(Default::default());
    state.ui.running = true;
    if let TypedScreen::Menu(menu_data) = &mut state.screen {
        menu_data.selected_item = 3;
    }
    update(&mut state, Message::MenuSelect).unwrap();
    assert!(!state.ui.running);
}
