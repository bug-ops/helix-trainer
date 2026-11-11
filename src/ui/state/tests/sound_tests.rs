//! Tests for sound toggle functionality

use std::assert_matches;

use super::common::create_test_app_state;
use crate::testing::update_executing_effects as update;
use crate::ui::state::Message;

/// CR-001: Test that `Message::ToggleSound` toggles `sound_manager.config().enabled`
#[test]
fn test_toggle_sound_message() {
    let mut state = create_test_app_state(vec![]);

    // Sound should be enabled by default
    assert!(
        state.progress.sound_manager.config().enabled,
        "Sound should be enabled by default"
    );

    // Toggle sound off
    update(&mut state, Message::ToggleSound).unwrap();
    assert!(
        !state.progress.sound_manager.config().enabled,
        "Sound should be disabled after first toggle"
    );

    // Toggle sound back on
    update(&mut state, Message::ToggleSound).unwrap();
    assert!(
        state.progress.sound_manager.config().enabled,
        "Sound should be enabled after second toggle"
    );
}

/// Test that `ToggleSound` works regardless of current screen
#[test]
fn test_toggle_sound_works_on_any_screen() {
    use crate::ui::state::{NavTarget, TypedScreen};

    let mut state = create_test_app_state(vec![]);

    // Test on ModeSelection screen (default)
    assert_matches!(state.screen, TypedScreen::ModeSelection(_));
    update(&mut state, Message::ToggleSound).unwrap();
    assert!(!state.progress.sound_manager.config().enabled);

    // Navigate to Profile and test
    update(&mut state, Message::NavigateTo(NavTarget::Profile)).unwrap();
    assert_matches!(state.screen, TypedScreen::Profile(_));
    update(&mut state, Message::ToggleSound).unwrap();
    assert!(state.progress.sound_manager.config().enabled);

    // Navigate to Statistics and test
    update(&mut state, Message::NavigateTo(NavTarget::Statistics)).unwrap();
    assert_matches!(state.screen, TypedScreen::Statistics(_));
    update(&mut state, Message::ToggleSound).unwrap();
    assert!(!state.progress.sound_manager.config().enabled);
}
