//! Profile and statistics message handlers
//!
//! Handles profile screen navigation and XP awards

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

/// Handle AwardXP message
///
/// Awards XP to the user profile and saves if level up occurs
pub fn handle_award_xp(ctx: &mut HandlerContext<'_>, amount: u64) -> Result<(), UserError> {
    let mut profile = ctx.progress.profile.borrow_mut();
    let leveled_up = profile.add_xp(amount);
    let new_level = profile.level;

    if leveled_up {
        drop(profile); // Release borrow before save

        // Show level-up notification
        let notification = crate::ui::notification::Notification::new(
            crate::ui::notification::NotificationType::LevelUp { new_level },
        );
        ctx.ui.notifications.push(notification);

        let profile_ref = ctx.progress.profile.borrow();
        ctx.progress
            .storage
            .save(&profile_ref)
            .map_err(|_| UserError::OperationFailed)?;
        drop(profile_ref);
        ctx.progress.mark_saved();
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{
        Difficulty, Scenario, ScenarioMetadata, ScoringConfig, Setup, Solution, TargetState,
    };
    use crate::gamification::{ProfileStorage, UserProfile};
    use crate::learning::PerformanceTracker;
    use crate::ui::state::handlers::minigame::handle_start_minigame;
    use crate::ui::state::{
        AppState, ConfigState, GameState, MenuData, ModeSelectionData, ProgressState, UIState,
    };

    fn create_test_scenario(id: &str) -> Scenario {
        Scenario {
            id: id.to_string(),
            name: format!("Test {}", id),
            description: "Test scenario".to_string(),
            setup: Setup {
                file_content: "line 1\nline 2\n".to_string(),
                cursor_position: (1, 0),
            },
            target: TargetState {
                file_content: "line 1\n".to_string(),
                cursor_position: (1, 0),
                selection: None,
            },
            solution: Solution {
                commands: vec!["x".to_string(), "d".to_string()],
                description: "Delete line".to_string(),
            },
            alternatives: vec![],
            hints: vec![],
            scoring: ScoringConfig {
                optimal_count: 1,
                max_points: 100,
                tolerance: 0,
            },
            metadata: Some(ScenarioMetadata {
                difficulty: Some(Difficulty::Beginner),
                ..Default::default()
            }),
        }
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
                ProfileStorage::new(),
            ),
            config: ConfigState::default(),
        }
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
        handle_start_minigame(&mut state).unwrap();

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
        handle_start_minigame(&mut state).unwrap();

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
        handle_start_minigame(&mut state).unwrap();

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
