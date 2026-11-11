//! Profile and statistics message handlers
//!
//! Handles profile screen navigation

use crate::ui::state::{
    AchievementsData, HandlerOutcome, ProfileData, StatisticsData, TypedScreen,
};

/// Handle ShowProfile message
///
/// Navigates to the profile screen, tracking where to return
pub fn handle_show_profile(screen: &mut TypedScreen) -> HandlerOutcome {
    let return_to = screen.detour_return_destination();
    HandlerOutcome::Transition(Box::new(TypedScreen::Profile(ProfileData { return_to })))
}

/// Handle ShowStatistics message
///
/// Navigates to the statistics screen, tracking where to return
pub fn handle_show_statistics(screen: &mut TypedScreen) -> HandlerOutcome {
    let return_to = screen.detour_return_destination();
    HandlerOutcome::Transition(Box::new(TypedScreen::Statistics(StatisticsData {
        return_to,
    })))
}

/// Handle ShowAchievements message
///
/// Navigates to the achievements screen, tracking where to return
pub fn handle_show_achievements(screen: &mut TypedScreen) -> HandlerOutcome {
    let return_to = screen.detour_return_destination();
    HandlerOutcome::Transition(Box::new(TypedScreen::Achievements(AchievementsData {
        return_to,
        scroll_offset: 0,
    })))
}

/// Handle ScrollAchievements message
///
/// Adjusts the achievement list scroll offset by `delta` rows (negative scrolls up).
/// Only clamped to zero here; the upper bound depends on the rendered viewport height
/// and content length, so it's clamped in `render_achievements_screen` instead.
pub fn handle_scroll_achievements(data: &mut AchievementsData, delta: i32) -> HandlerOutcome {
    data.scroll_offset = data.scroll_offset.saturating_add_signed(delta as isize);
    HandlerOutcome::Stay
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Difficulty, Scenario};
    use crate::gamification::{ProfileStorage, UserProfile};
    use crate::learning::PerformanceTracker;
    use crate::testing::ScenarioBuilder;
    use crate::ui::state::handlers::mode_selection::handle_select_arcade_mode;
    use crate::ui::state::{
        AppState, ConfigState, GameState, MenuData, ModeSelectionData, ProgressState, UIState,
    };
    use crate::ui::state::{HandlerContext, ReturnDestination};
    use std::assert_matches;

    fn create_test_scenario(id: &str) -> Scenario {
        ScenarioBuilder::new()
            .id(id)
            .setup_content("line 1\nline 2\n")
            .setup_cursor(1, 0)
            .target_content("line 1\n")
            .target_cursor(1, 0)
            .optimal_count(1)
            .difficulty(Difficulty::Beginner)
            .build()
    }

    fn create_test_state() -> AppState {
        let scenarios = vec![
            create_test_scenario("s1"),
            create_test_scenario("s2"),
            create_test_scenario("s3"),
        ];

        AppState {
            screen: TypedScreen::ModeSelection(ModeSelectionData::default()),
            ui: UIState::new(),
            game: GameState::new(scenarios),
            progress: ProgressState::new(
                UserProfile::new(),
                PerformanceTracker::new(),
                ProfileStorage::for_test(),
            ),
            config: ConfigState::default(),
        }
    }

    fn start_minigame(state: &mut AppState) {
        let mut ctx = HandlerContext::new(
            &mut state.ui,
            &mut state.game,
            &mut state.progress,
            &state.config,
        );
        let outcome = handle_select_arcade_mode(&mut ctx);
        crate::ui::state::apply_outcome(state, outcome);
    }

    #[test]
    fn test_show_profile_from_menu() {
        let mut state = create_test_state();
        state.screen = TypedScreen::Menu(MenuData::default());

        let outcome = handle_show_profile(&mut state.screen);

        assert!(outcome.is_transition());
        if let HandlerOutcome::Transition(boxed) = outcome {
            if let TypedScreen::Profile(data) = *boxed {
                assert_matches!(data.return_to, ReturnDestination::Menu);
            } else {
                panic!("Expected Profile screen");
            }
        }
    }

    #[test]
    fn test_show_statistics_from_menu() {
        let mut state = create_test_state();
        state.screen = TypedScreen::Menu(MenuData::default());

        let outcome = handle_show_statistics(&mut state.screen);

        assert!(outcome.is_transition());
        if let HandlerOutcome::Transition(boxed) = outcome {
            if let TypedScreen::Statistics(data) = *boxed {
                assert_matches!(data.return_to, ReturnDestination::Menu);
            } else {
                panic!("Expected Statistics screen");
            }
        }
    }

    #[test]
    fn test_show_achievements_from_menu() {
        let mut state = create_test_state();
        state.screen = TypedScreen::Menu(MenuData::default());

        let outcome = handle_show_achievements(&mut state.screen);

        assert!(outcome.is_transition());
        if let HandlerOutcome::Transition(boxed) = outcome {
            if let TypedScreen::Achievements(data) = *boxed {
                assert_matches!(data.return_to, ReturnDestination::Menu);
            } else {
                panic!("Expected Achievements screen");
            }
        }
    }

    #[test]
    fn test_show_achievements_from_paused_minigame() {
        let mut state = create_test_state();

        start_minigame(&mut state);
        if let Some(session) = state.minigame_session_mut() {
            session.tick_countdown();
            session.tick_countdown();
            session.tick_countdown();
            session.pause();
        }

        let outcome = handle_show_achievements(&mut state.screen);

        assert!(outcome.is_transition());
        if let HandlerOutcome::Transition(boxed) = outcome {
            if let TypedScreen::Achievements(data) = *boxed {
                assert_matches!(data.return_to, ReturnDestination::PausedMiniGame(_));
            } else {
                panic!("Expected Achievements screen");
            }
        }
    }

    #[test]
    fn test_show_profile_from_paused_minigame() {
        let mut state = create_test_state();

        // Start minigame properly
        start_minigame(&mut state);

        // Transition to playing then pause
        if let Some(session) = state.minigame_session_mut() {
            session.tick_countdown();
            session.tick_countdown();
            session.tick_countdown();
            assert!(session.state().is_playing());

            session.pause();
            assert!(session.state().is_paused());
        }

        let outcome = handle_show_profile(&mut state.screen);

        assert!(outcome.is_transition());
        if let HandlerOutcome::Transition(boxed) = outcome {
            if let TypedScreen::Profile(data) = *boxed {
                assert_matches!(data.return_to, ReturnDestination::PausedMiniGame(_));
            } else {
                panic!("Expected Profile screen");
            }
        }
    }

    #[test]
    fn test_show_statistics_from_paused_minigame() {
        let mut state = create_test_state();

        // Start minigame properly
        start_minigame(&mut state);

        // Transition to playing then pause
        if let Some(session) = state.minigame_session_mut() {
            session.tick_countdown();
            session.tick_countdown();
            session.tick_countdown();
            session.pause();
        }

        let outcome = handle_show_statistics(&mut state.screen);

        assert!(outcome.is_transition());
        if let HandlerOutcome::Transition(boxed) = outcome {
            if let TypedScreen::Statistics(data) = *boxed {
                assert_matches!(data.return_to, ReturnDestination::PausedMiniGame(_));
            } else {
                panic!("Expected Statistics screen");
            }
        }
    }

    #[test]
    fn test_show_profile_from_playing_minigame_returns_to_menu() {
        let mut state = create_test_state();

        // Start minigame properly
        start_minigame(&mut state);

        // Transition to playing but don't pause
        if let Some(session) = state.minigame_session_mut() {
            session.tick_countdown();
            session.tick_countdown();
            session.tick_countdown();
            assert!(session.state().is_playing());
        }

        let outcome = handle_show_profile(&mut state.screen);

        assert!(outcome.is_transition());
        if let HandlerOutcome::Transition(boxed) = outcome {
            // Since game is playing (not paused), should return to menu
            if let TypedScreen::Profile(data) = *boxed {
                assert_matches!(data.return_to, ReturnDestination::Menu);
            } else {
                panic!("Expected Profile screen");
            }
        }
    }
}
