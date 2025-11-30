//! Profile and statistics message handlers
//!
//! Handles profile screen navigation and XP awards

use crate::security::UserError;
use crate::ui::state::{AppState, ProfileData, ReturnDestination, StatisticsData, TypedScreen};

/// Determine return destination based on current screen
///
/// If coming from a paused mini-game, return there; otherwise return to menu.
fn determine_return_destination(state: &AppState) -> ReturnDestination {
    if let TypedScreen::MiniGame(_) = &state.screen
        && let Some(session) = &state.game.minigame_session
        && session.state().is_paused()
    {
        return ReturnDestination::PausedMiniGame;
    }
    ReturnDestination::Menu
}

/// Handle ShowProfile message
///
/// Navigates to the profile screen, tracking where to return
pub fn handle_show_profile(state: &mut AppState) -> Result<(), UserError> {
    let return_to = determine_return_destination(state);
    state.screen = TypedScreen::Profile(ProfileData { return_to });
    Ok(())
}

/// Handle ShowStatistics message
///
/// Navigates to the statistics screen, tracking where to return
pub fn handle_show_statistics(state: &mut AppState) -> Result<(), UserError> {
    let return_to = determine_return_destination(state);
    state.screen = TypedScreen::Statistics(StatisticsData { return_to });
    Ok(())
}

/// Handle AwardXP message
///
/// Awards XP to the user profile and saves if level up occurs
pub fn handle_award_xp(state: &mut AppState, amount: u64) -> Result<(), UserError> {
    let mut profile = state.progress.profile.borrow_mut();
    let leveled_up = profile.add_xp(amount);

    if leveled_up {
        drop(profile); // Release borrow before save
        state
            .save_profile_immediate()
            .map_err(|_| UserError::OperationFailed)?;
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
        ConfigState, GameState, MenuData, ModeSelectionData, ProgressState, UIState,
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
                commands: vec!["dd".to_string()],
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

        handle_show_profile(&mut state).unwrap();

        if let TypedScreen::Profile(data) = &state.screen {
            assert_eq!(data.return_to, ReturnDestination::Menu);
        } else {
            panic!("Expected Profile screen");
        }
    }

    #[test]
    fn test_show_statistics_from_menu() {
        let mut state = create_test_state();
        state.screen = TypedScreen::Menu(MenuData::default());

        handle_show_statistics(&mut state).unwrap();

        if let TypedScreen::Statistics(data) = &state.screen {
            assert_eq!(data.return_to, ReturnDestination::Menu);
        } else {
            panic!("Expected Statistics screen");
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

        handle_show_profile(&mut state).unwrap();

        if let TypedScreen::Profile(data) = &state.screen {
            assert_eq!(data.return_to, ReturnDestination::PausedMiniGame);
        } else {
            panic!("Expected Profile screen");
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

        handle_show_statistics(&mut state).unwrap();

        if let TypedScreen::Statistics(data) = &state.screen {
            assert_eq!(data.return_to, ReturnDestination::PausedMiniGame);
        } else {
            panic!("Expected Statistics screen");
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

        handle_show_profile(&mut state).unwrap();

        // Since game is playing (not paused), should return to menu
        if let TypedScreen::Profile(data) = &state.screen {
            assert_eq!(data.return_to, ReturnDestination::Menu);
        } else {
            panic!("Expected Profile screen");
        }
    }
}
