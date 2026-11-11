//! Tests for sound toggle functionality

use std::assert_matches;

use super::common::create_test_app_state;
use crate::testing::update_executing_effects as update;
use crate::ui::state::Message;

/// #481: sounds are returned as effects instead of being played inside `update`
#[test]
fn test_update_returns_sound_effect_for_countdown_tick() {
    use crate::testing::default_test_scenario;
    use crate::ui::state::Effect;

    let mut state = create_test_app_state(vec![default_test_scenario()]);
    let mut effects = Vec::new();
    crate::ui::state::update(&mut state, Message::SelectArcadeMode, &mut effects).unwrap();
    effects.clear();

    crate::ui::state::update(&mut state, Message::MiniGameTick, &mut effects).unwrap();

    assert_eq!(
        effects,
        vec![Effect::Play(crate::sound::SoundEffect::Countdown)]
    );
}

/// #481: effects queued before a failing step are delivered on the error path too
#[test]
fn test_update_returns_effects_on_the_error_path() {
    use crate::ui::state::Effect;

    let mut state = create_test_app_state(vec![]);
    state.ui.effects.push(Effect::SaveProfile);
    let mut effects = Vec::new();

    // RetryScenario fails with WrongScreen on the mode-selection screen
    let result = crate::ui::state::update(&mut state, Message::RetryScenario, &mut effects);

    assert!(result.is_err());
    assert_eq!(effects, vec![Effect::SaveProfile]);
}

/// #518: start-fresh is offered only after a backup was made
#[test]
fn test_start_fresh_profile_requires_a_backed_up_quarantine() {
    use crate::gamification::QuarantineOutcome;
    use crate::ui::state::Effect;

    let mut state = create_test_app_state(vec![]);
    let mut effects = Vec::new();
    crate::ui::state::update(&mut state, Message::StartFreshProfile, &mut effects).unwrap();
    assert!(effects.is_empty(), "no quarantine, nothing to replace");

    state
        .progress
        .quarantine_profile(QuarantineOutcome::Failed("no space".to_string()));
    crate::ui::state::update(&mut state, Message::StartFreshProfile, &mut effects).unwrap();
    assert!(
        effects.is_empty(),
        "without a backup the original must stay"
    );

    state
        .progress
        .quarantine_profile(QuarantineOutcome::BackedUp("backup".into()));
    crate::ui::state::update(&mut state, Message::StartFreshProfile, &mut effects).unwrap();
    assert_eq!(effects, vec![Effect::StartFreshProfile]);
}

/// CR-001: Test that Message::ToggleSound toggles sound_manager.config().enabled
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

/// Test that ToggleSound works regardless of current screen
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
