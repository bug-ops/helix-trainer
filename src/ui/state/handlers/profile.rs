//! Profile and statistics message handlers
//!
//! Handles profile screen navigation

use crate::security::UserError;
use crate::ui::state::{
    HandlerContext, HandlerOutcome, ProfileData, ReturnDestination, StatisticsData, TypedScreen,
};

/// Determine return destination based on current context
///
/// If coming from a paused mini-game, return there; otherwise return to menu.
fn determine_return_destination(ctx: &HandlerContext<'_>) -> ReturnDestination {
    if let Some(session) = &ctx.game.minigame_session
        && session.state().is_paused()
    {
        return ReturnDestination::PausedMiniGame;
    }
    ReturnDestination::Menu
}

/// Handle ShowProfile message
///
/// Navigates to the profile screen, tracking where to return
pub fn handle_show_profile(ctx: &mut HandlerContext<'_>) -> Result<HandlerOutcome, UserError> {
    let return_to = determine_return_destination(ctx);
    Ok(HandlerOutcome::Transition(Box::new(TypedScreen::Profile(
        ProfileData { return_to },
    ))))
}

/// Handle ShowStatistics message
///
/// Navigates to the statistics screen, tracking where to return
pub fn handle_show_statistics(ctx: &mut HandlerContext<'_>) -> Result<HandlerOutcome, UserError> {
    let return_to = determine_return_destination(ctx);
    Ok(HandlerOutcome::Transition(Box::new(
        TypedScreen::Statistics(StatisticsData { return_to }),
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Difficulty, Scenario};
    use crate::gamification::{ProfileStorage, UserProfile};
    use crate::learning::PerformanceTracker;
    use crate::testing::ScenarioBuilder;
    use crate::ui::state::handlers::minigame::handle_start_minigame;
    use crate::ui::state::{
        AppState, ConfigState, GameState, MenuData, ModeSelectionData, ProgressState, UIState,
    };

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
        let outcome = handle_start_minigame(&mut ctx).unwrap();
        crate::ui::state::apply_outcome(state, outcome);
    }

    #[test]
    fn test_show_profile_from_menu() {
        let mut state = create_test_state();
        state.screen = TypedScreen::Menu(MenuData::default());

        let mut ctx = HandlerContext::new(
            &mut state.ui,
            &mut state.game,
            &mut state.progress,
            &state.config,
        );
        let outcome = handle_show_profile(&mut ctx).unwrap();

        assert!(outcome.is_transition());
        if let HandlerOutcome::Transition(boxed) = outcome {
            if let TypedScreen::Profile(data) = *boxed {
                assert_eq!(data.return_to, ReturnDestination::Menu);
            } else {
                panic!("Expected Profile screen");
            }
        }
    }

    #[test]
    fn test_show_statistics_from_menu() {
        let mut state = create_test_state();
        state.screen = TypedScreen::Menu(MenuData::default());

        let mut ctx = HandlerContext::new(
            &mut state.ui,
            &mut state.game,
            &mut state.progress,
            &state.config,
        );
        let outcome = handle_show_statistics(&mut ctx).unwrap();

        assert!(outcome.is_transition());
        if let HandlerOutcome::Transition(boxed) = outcome {
            if let TypedScreen::Statistics(data) = *boxed {
                assert_eq!(data.return_to, ReturnDestination::Menu);
            } else {
                panic!("Expected Statistics screen");
            }
        }
    }

    #[test]
    fn test_show_profile_from_paused_minigame() {
        let mut state = create_test_state();

        // Start minigame properly
        start_minigame(&mut state);

        // Transition to playing then pause
        if let Some(ref mut session) = state.game.minigame_session {
            session.tick_countdown();
            session.tick_countdown();
            session.tick_countdown();
            assert!(session.state().is_playing());

            session.pause();
            assert!(session.state().is_paused());
        }

        let mut ctx = HandlerContext::new(
            &mut state.ui,
            &mut state.game,
            &mut state.progress,
            &state.config,
        );
        let outcome = handle_show_profile(&mut ctx).unwrap();

        assert!(outcome.is_transition());
        if let HandlerOutcome::Transition(boxed) = outcome {
            if let TypedScreen::Profile(data) = *boxed {
                assert_eq!(data.return_to, ReturnDestination::PausedMiniGame);
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
        if let Some(ref mut session) = state.game.minigame_session {
            session.tick_countdown();
            session.tick_countdown();
            session.tick_countdown();
            session.pause();
        }

        let mut ctx = HandlerContext::new(
            &mut state.ui,
            &mut state.game,
            &mut state.progress,
            &state.config,
        );
        let outcome = handle_show_statistics(&mut ctx).unwrap();

        assert!(outcome.is_transition());
        if let HandlerOutcome::Transition(boxed) = outcome {
            if let TypedScreen::Statistics(data) = *boxed {
                assert_eq!(data.return_to, ReturnDestination::PausedMiniGame);
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
        if let Some(ref mut session) = state.game.minigame_session {
            session.tick_countdown();
            session.tick_countdown();
            session.tick_countdown();
            assert!(session.state().is_playing());
        }

        let mut ctx = HandlerContext::new(
            &mut state.ui,
            &mut state.game,
            &mut state.progress,
            &state.config,
        );
        let outcome = handle_show_profile(&mut ctx).unwrap();

        assert!(outcome.is_transition());
        if let HandlerOutcome::Transition(boxed) = outcome {
            // Since game is playing (not paused), should return to menu
            if let TypedScreen::Profile(data) = *boxed {
                assert_eq!(data.return_to, ReturnDestination::Menu);
            } else {
                panic!("Expected Profile screen");
            }
        }
    }
}
