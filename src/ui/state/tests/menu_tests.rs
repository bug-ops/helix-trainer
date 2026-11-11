//! Tests for menu navigation and selection

use crate::testing::cid;
use std::assert_matches;

use super::common::{create_test_app_state, create_test_scenario};
use crate::testing::update_executing_effects as update;
use crate::ui::state::{Message, TypedScreen};

#[test]
fn test_menu_navigation_up() {
    let mut state = create_test_app_state(vec![]);
    // Navigate to Menu screen first (starts on ModeSelection)
    update(&mut state, Message::ModeSelectionSelect).unwrap();
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
    update(&mut state, Message::ModeSelectionSelect).unwrap();

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
    update(&mut state, Message::ModeSelectionSelect).unwrap();

    update(&mut state, Message::MenuSelect).unwrap();

    // After TypedScreen refactoring, session is inside TaskData
    if let TypedScreen::Task(task_data) = &state.screen {
        // Session exists inside TaskData
        assert!(!task_data.session.playable().current_content().is_empty());
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
    update(&mut state, Message::ModeSelectionSelect).unwrap();

    // Select Quit option (index = scenario_count + 3)
    if let TypedScreen::Menu(menu_data) = &mut state.screen {
        menu_data.selected_item = 5; // 2 scenarios + Review + Profile + Statistics + Quit = index 5
    }

    update(&mut state, Message::MenuSelect).unwrap();

    assert!(!state.ui.running);
}

fn pause_arcade_with_score(state: &mut crate::ui::state::AppState, score: u64) {
    update(state, Message::SelectArcadeMode).unwrap();
    let session = state.minigame_session_mut().unwrap();
    for _ in 0..3 {
        session.tick_countdown();
    }
    session.stats.score = score;
    update(state, Message::PauseMiniGame).unwrap();
}

/// Regression test for #324: quitting while an arcade run is parked behind a detour screen
/// (here Profile) must still award the run's XP and score. The detour screen owns the run
/// (#461), so `QuitApp` finds it through the screen.
#[test]
fn test_quit_awards_paused_arcade_session_parked_behind_detour_screen() {
    let scenario = create_test_scenario();
    let mut state = create_test_app_state(vec![scenario]);
    pause_arcade_with_score(&mut state, 7777);

    update(&mut state, Message::ShowProfile).unwrap();
    assert_matches!(state.screen, TypedScreen::Profile(_));
    let initial_xp = state.progress.profile.total_xp;

    update(&mut state, Message::QuitApp).unwrap();

    assert!(!state.ui.running);
    assert!(state.progress.profile.total_xp > initial_xp);
    assert_eq!(state.progress.profile.minigame_high_score, 7777);
}

/// Hopping between detour screens must keep handing the paused run along, so Back still
/// returns to it.
#[test]
fn test_detour_chain_returns_to_paused_arcade_session() {
    let scenario = create_test_scenario();
    let mut state = create_test_app_state(vec![scenario]);
    pause_arcade_with_score(&mut state, 100);

    update(&mut state, Message::ShowProfile).unwrap();
    update(&mut state, Message::ShowStatistics).unwrap();
    update(&mut state, Message::ShowAchievements).unwrap();
    update(&mut state, Message::BackToMenu).unwrap();

    assert_matches!(state.screen, TypedScreen::MiniGame(_));
    let session = state.minigame_session().unwrap();
    assert!(session.state().is_paused());
    assert_eq!(session.stats().score, 100);
}

#[test]
fn test_menu_select_profile() {
    let scenario = create_test_scenario();
    let mut state = create_test_app_state(vec![scenario]);
    // Navigate to menu first
    update(&mut state, Message::ModeSelectionSelect).unwrap();

    if let TypedScreen::Menu(menu_data) = &mut state.screen {
        menu_data.selected_item = 2; // Profile is at index 2 (after 1 scenario + Review)
    }

    update(&mut state, Message::MenuSelect).unwrap();
    assert_matches!(state.screen, TypedScreen::Profile(_));
}

#[test]
fn test_menu_select_statistics() {
    let scenario = create_test_scenario();
    let mut state = create_test_app_state(vec![scenario]);
    // Navigate to menu first
    update(&mut state, Message::ModeSelectionSelect).unwrap();

    if let TypedScreen::Menu(menu_data) = &mut state.screen {
        menu_data.selected_item = 3; // Statistics is at index 3 (after 1 scenario + Review + Profile)
    }

    update(&mut state, Message::MenuSelect).unwrap();
    assert_matches!(state.screen, TypedScreen::Statistics(_));
}

#[test]
fn test_menu_with_zero_scenarios() {
    // Edge case: no scenarios loaded
    let mut state = create_test_app_state(vec![]);
    // Navigate to menu first
    update(&mut state, Message::ModeSelectionSelect).unwrap();

    // Review should be at index 0 (no scenarios)
    update(&mut state, Message::MenuSelect).unwrap();
    // Should stay on MainMenu if no reviews are due
    assert_matches!(state.screen, TypedScreen::Menu(_));

    // Profile at index 1
    if let TypedScreen::Menu(menu_data) = &mut state.screen {
        menu_data.selected_item = 1;
    }
    update(&mut state, Message::MenuSelect).unwrap();
    assert_matches!(state.screen, TypedScreen::Profile(_));

    // Statistics at index 2
    state.screen = TypedScreen::Menu(crate::ui::state::MenuData::default());
    if let TypedScreen::Menu(menu_data) = &mut state.screen {
        menu_data.selected_item = 2;
    }
    update(&mut state, Message::MenuSelect).unwrap();
    assert_matches!(state.screen, TypedScreen::Statistics(_));

    // Quit at index 3
    state.screen = TypedScreen::Menu(crate::ui::state::MenuData::default());
    state.ui.running = true;
    if let TypedScreen::Menu(menu_data) = &mut state.screen {
        menu_data.selected_item = 3;
    }
    update(&mut state, Message::MenuSelect).unwrap();
    assert!(!state.ui.running);
}

#[test]
fn test_menu_up_by() {
    let scenarios = vec![
        create_test_scenario(),
        create_test_scenario(),
        create_test_scenario(),
    ];
    let mut state = create_test_app_state(scenarios);
    // Navigate to menu first
    update(&mut state, Message::ModeSelectionSelect).unwrap();

    // Set to item 5
    if let TypedScreen::Menu(menu_data) = &mut state.screen {
        menu_data.selected_item = 5;
    }

    // Move up by 3
    update(&mut state, Message::MenuUpBy(3)).unwrap();
    if let TypedScreen::Menu(menu_data) = &state.screen {
        assert_eq!(menu_data.selected_item, 2);
    }

    // Move up by more than available - should saturate to 0
    update(&mut state, Message::MenuUpBy(10)).unwrap();
    if let TypedScreen::Menu(menu_data) = &state.screen {
        assert_eq!(menu_data.selected_item, 0);
    }
}

#[test]
fn test_menu_down_by() {
    let scenarios = vec![create_test_scenario(), create_test_scenario()];
    let mut state = create_test_app_state(scenarios);
    // Navigate to menu first
    update(&mut state, Message::ModeSelectionSelect).unwrap();

    // Move down by 2
    update(&mut state, Message::MenuDownBy(2)).unwrap();
    if let TypedScreen::Menu(menu_data) = &state.screen {
        assert_eq!(menu_data.selected_item, 2);
    }

    // Move down by more than available - should clamp to max
    update(&mut state, Message::MenuDownBy(100)).unwrap();
    if let TypedScreen::Menu(menu_data) = &state.screen {
        // Max items = 2 scenarios + 4 menu items = 6, max index = 5
        assert_eq!(menu_data.selected_item, 5);
    }
}

#[test]
fn test_menu_jump_to_first() {
    let scenarios = vec![create_test_scenario(), create_test_scenario()];
    let mut state = create_test_app_state(scenarios);
    // Navigate to menu first
    update(&mut state, Message::ModeSelectionSelect).unwrap();

    // Set to some item
    if let TypedScreen::Menu(menu_data) = &mut state.screen {
        menu_data.selected_item = 4;
    }

    // Jump to first (gg command)
    update(&mut state, Message::MenuJumpToFirst).unwrap();
    if let TypedScreen::Menu(menu_data) = &state.screen {
        assert_eq!(menu_data.selected_item, 0);
    }
}

#[test]
fn test_menu_jump_to_last() {
    let scenarios = vec![create_test_scenario(), create_test_scenario()];
    let mut state = create_test_app_state(scenarios);
    // Navigate to menu first
    update(&mut state, Message::ModeSelectionSelect).unwrap();

    // Jump to last (G command)
    update(&mut state, Message::MenuJumpToLast).unwrap();
    if let TypedScreen::Menu(menu_data) = &state.screen {
        // Max items = 2 scenarios + 4 menu items = 6, max index = 5
        assert_eq!(menu_data.selected_item, 5);
    }
}

#[test]
fn test_menu_jump_to_specific() {
    let scenarios = vec![
        create_test_scenario(),
        create_test_scenario(),
        create_test_scenario(),
    ];
    let mut state = create_test_app_state(scenarios);
    // Navigate to menu first
    update(&mut state, Message::ModeSelectionSelect).unwrap();

    // Jump to line 3 (1-indexed, so 0-indexed = 2)
    update(&mut state, Message::MenuJumpTo(3)).unwrap();
    if let TypedScreen::Menu(menu_data) = &state.screen {
        assert_eq!(menu_data.selected_item, 2);
    }

    // Jump to line 1
    update(&mut state, Message::MenuJumpTo(1)).unwrap();
    if let TypedScreen::Menu(menu_data) = &state.screen {
        assert_eq!(menu_data.selected_item, 0);
    }

    // Jump to line beyond max - should clamp
    update(&mut state, Message::MenuJumpTo(100)).unwrap();
    if let TypedScreen::Menu(menu_data) = &state.screen {
        assert_eq!(menu_data.selected_item, 6); // 3 scenarios + 4 menu items - 1 = 6
    }

    // Jump to line 0 - should clamp to 0 (1-indexed minimum)
    update(&mut state, Message::MenuJumpTo(0)).unwrap();
    if let TypedScreen::Menu(menu_data) = &state.screen {
        // saturating_sub(1) of 0 = 0
        assert_eq!(menu_data.selected_item, 0);
    }
}

#[test]
fn test_menu_select_review_no_due() {
    let scenario = create_test_scenario();
    let mut state = create_test_app_state(vec![scenario]);
    // Navigate to menu first
    update(&mut state, Message::ModeSelectionSelect).unwrap();

    // Select Review option (index = scenario_count = 1)
    if let TypedScreen::Menu(menu_data) = &mut state.screen {
        menu_data.selected_item = 1;
    }

    update(&mut state, Message::MenuSelect).unwrap();

    // No reviews due, should stay on menu
    assert_matches!(state.screen, TypedScreen::Menu(_));
}

/// Starting a review while an arcade run is parked behind a detour screen ends that run for
/// good: its score and XP are awarded and the screen is the review.
#[test]
fn test_starting_review_ends_the_paused_arcade_run_and_awards_it() {
    use std::time::Duration;

    let scenario = create_test_scenario();
    let mut state = create_test_app_state(vec![scenario]);
    pause_arcade_with_score(&mut state, 4242);
    update(&mut state, Message::ShowProfile).unwrap();
    state.progress.performance_tracker.record_attempt(
        &cid("x"),
        Duration::from_secs(5),
        false,
        Duration::from_secs(1),
    );
    let initial_xp = state.progress.profile.total_xp;

    update(&mut state, Message::StartReviewSession).unwrap();

    assert_matches!(state.screen, TypedScreen::Review(_));
    assert!(state.minigame_session().is_none(), "the run must be gone");
    assert!(state.progress.profile.total_xp > initial_xp);
    assert_eq!(state.progress.profile.minigame_high_score, 4242);
}

/// Without due reviews the paused run is left alone.
#[test]
fn test_starting_review_without_due_reviews_keeps_the_paused_run() {
    let scenario = create_test_scenario();
    let mut state = create_test_app_state(vec![scenario]);
    pause_arcade_with_score(&mut state, 100);
    update(&mut state, Message::ShowProfile).unwrap();

    update(&mut state, Message::StartReviewSession).unwrap();

    assert_matches!(state.screen, TypedScreen::Profile(_));
    assert!(state.minigame_session().is_some());
}
