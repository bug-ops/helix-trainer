//! Tests for the side effects `update` requests (sounds, saves, start-fresh)

use super::common::create_test_app_state;
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
        .quarantine_profile(crate::testing::quarantine(crate::testing::failed_backup()));
    crate::ui::state::update(&mut state, Message::StartFreshProfile, &mut effects).unwrap();
    assert!(
        effects.is_empty(),
        "without a backup the original must stay"
    );

    state
        .progress
        .quarantine_profile(crate::testing::quarantine(QuarantineOutcome::BackedUp(
            "backup".into(),
        )));
    crate::ui::state::update(&mut state, Message::StartFreshProfile, &mut effects).unwrap();
    assert_eq!(effects, vec![Effect::StartFreshProfile]);
}

/// #518: a repeated request while the first start-fresh is in flight is ignored, so the
/// profile just written is not quarantined and replaced again
#[test]
fn test_second_start_fresh_request_is_ignored_while_the_first_is_in_flight() {
    use crate::gamification::QuarantineOutcome;
    use crate::ui::state::{Effect, ProfilePersistence};

    let mut state = create_test_app_state(vec![]);
    state
        .progress
        .quarantine_profile(crate::testing::quarantine(QuarantineOutcome::BackedUp(
            "backup".into(),
        )));
    let mut effects = Vec::new();

    crate::ui::state::update(&mut state, Message::StartFreshProfile, &mut effects).unwrap();
    crate::ui::state::update(&mut state, Message::StartFreshProfile, &mut effects).unwrap();

    assert_eq!(effects, vec![Effect::StartFreshProfile]);
    std::assert_matches!(state.progress.persistence(), ProfilePersistence::Loading);
}
