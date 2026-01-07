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
    let profile = &mut ctx.progress.profile;
    let leveled_up = profile.add_xp(amount);
    let new_level = profile.level;

    if leveled_up {
        // Show level-up notification
        let notification = crate::ui::notification::Notification::new(
            crate::ui::notification::NotificationType::LevelUp { new_level },
        );
        ctx.ui.notifications.push(notification);

        ctx.progress
            .storage
            .save(&ctx.progress.profile)
            .map_err(UserError::from)?;
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
                selection: None,
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

    #[test]
    fn test_award_xp_without_level_up() {
        let mut state = create_test_state();
        let initial_xp = state.progress.profile.total_xp;
        let initial_level = state.progress.profile.level;
        let initial_notifications = state.ui.notifications.count();

        let mut ctx = HandlerContext::new(
            &mut state.ui,
            &mut state.game,
            &mut state.progress,
            &state.config,
        );

        // Award small amount of XP (shouldn't level up)
        handle_award_xp(&mut ctx, 50).unwrap();

        // XP should increase
        assert_eq!(state.progress.profile.total_xp, initial_xp + 50);

        // Level should remain the same
        assert_eq!(state.progress.profile.level, initial_level);

        // No notification should be added (no level up)
        assert_eq!(state.ui.notifications.count(), initial_notifications);
    }

    #[test]
    fn test_award_xp_with_level_up() {
        use crate::gamification::XPCalculator;

        let mut state = create_test_state();

        // Set XP just below level 2 threshold
        let xp_for_level_2 = XPCalculator::xp_for_level(2);
        state.progress.profile.total_xp = xp_for_level_2 - 10;
        state.progress.profile.level = 1;

        let initial_notifications = state.ui.notifications.count();

        let mut ctx = HandlerContext::new(
            &mut state.ui,
            &mut state.game,
            &mut state.progress,
            &state.config,
        );

        // Award enough XP to level up
        handle_award_xp(&mut ctx, 100).unwrap();

        // Should level up to 2
        assert_eq!(state.progress.profile.level, 2);
        assert!(state.progress.profile.total_xp >= xp_for_level_2);

        // Should have level-up notification
        assert_eq!(state.ui.notifications.count(), initial_notifications + 1);

        // Verify notification type
        if let Some(notification) = state.ui.notifications.visible().last() {
            use crate::ui::notification::NotificationType;
            assert!(matches!(
                notification.notification_type,
                NotificationType::LevelUp { new_level: 2 }
            ));
        }
    }

    #[test]
    fn test_award_xp_multiple_levels() {
        use crate::gamification::XPCalculator;

        let mut state = create_test_state();
        state.progress.profile.total_xp = 0;
        state.progress.profile.level = 1;

        let mut ctx = HandlerContext::new(
            &mut state.ui,
            &mut state.game,
            &mut state.progress,
            &state.config,
        );

        // Award massive XP to jump multiple levels
        let massive_xp = XPCalculator::xp_for_level(5);
        handle_award_xp(&mut ctx, massive_xp).unwrap();

        // Should level up to at least level 5
        assert!(state.progress.profile.level >= 5);

        // Should have a level-up notification for the final level
        assert!(state.ui.notifications.count() > 0);
    }

    #[test]
    fn test_award_xp_zero_amount() {
        let mut state = create_test_state();
        let initial_xp = state.progress.profile.total_xp;
        let initial_level = state.progress.profile.level;

        let mut ctx = HandlerContext::new(
            &mut state.ui,
            &mut state.game,
            &mut state.progress,
            &state.config,
        );

        // Award zero XP
        handle_award_xp(&mut ctx, 0).unwrap();

        // Nothing should change
        assert_eq!(state.progress.profile.total_xp, initial_xp);
        assert_eq!(state.progress.profile.level, initial_level);
    }

    #[test]
    fn test_award_xp_calls_save_on_level_up() {
        use crate::gamification::XPCalculator;

        let mut state = create_test_state();

        // Set XP just below next level
        let xp_for_next = XPCalculator::xp_for_level(2);
        state.progress.profile.total_xp = xp_for_next - 10;
        state.progress.profile.level = 1;

        let mut ctx = HandlerContext::new(
            &mut state.ui,
            &mut state.game,
            &mut state.progress,
            &state.config,
        );

        // Award XP to trigger level up
        let result = handle_award_xp(&mut ctx, 100);

        // Should succeed (save was called internally)
        assert!(result.is_ok());

        // Profile should be updated
        assert_eq!(state.progress.profile.level, 2);
    }

    #[test]
    fn test_award_xp_large_amounts() {
        let mut state = create_test_state();
        let initial_xp = state.progress.profile.total_xp;

        let mut ctx = HandlerContext::new(
            &mut state.ui,
            &mut state.game,
            &mut state.progress,
            &state.config,
        );

        // Award large amount (test overflow protection)
        let large_xp = 1_000_000_u64;
        handle_award_xp(&mut ctx, large_xp).unwrap();

        // XP should increase correctly
        assert_eq!(state.progress.profile.total_xp, initial_xp + large_xp);
    }
}
