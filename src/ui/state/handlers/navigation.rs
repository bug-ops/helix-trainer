//! Navigation message handlers
//!
//! Handles screen transitions and application lifecycle messages

use crate::security::UserError;
use crate::ui::state::{
    AppState, MenuData, MiniGameData, ModeSelectionData, ProfileData, ReturnDestination, Screen,
    StatisticsData, TypedScreen,
};

/// Handle QuitApp message
///
/// Sets the running flag to false to exit the application
pub fn handle_quit_app(state: &mut AppState) -> Result<(), UserError> {
    state.ui.running = false;
    Ok(())
}

/// Handle NavigateTo message
///
/// Changes the current screen to the specified screen
pub fn handle_navigate_to(state: &mut AppState, screen: Screen) -> Result<(), UserError> {
    // Convert Screen enum to TypedScreen variant
    state.screen = match screen {
        Screen::ModeSelection => TypedScreen::ModeSelection(ModeSelectionData::default()),
        Screen::MainMenu => TypedScreen::Menu(MenuData::default()),
        Screen::Profile => TypedScreen::Profile(ProfileData::default()),
        Screen::Statistics => TypedScreen::Statistics(StatisticsData::default()),
        Screen::MiniGame => TypedScreen::MiniGame(MiniGameData::default()),
        // NOTE: Task, Results, and Review screens require data and should not be
        // navigated to via NavigateTo - they have their own handlers
        Screen::Task | Screen::Results | Screen::Review => {
            // Keep current screen if trying to navigate to a data-dependent screen
            return Ok(());
        }
    };
    Ok(())
}

/// Handle BackToMenu message
///
/// Returns to the appropriate screen based on context:
/// - From Profile/Statistics with PausedMiniGame return destination: returns to paused mini-game
/// - Otherwise: returns to mode selection screen (the main menu)
pub fn handle_back_to_menu(state: &mut AppState) -> Result<(), UserError> {
    // Check if we should return to paused mini-game instead of mode selection
    let return_to_minigame = match &state.screen {
        TypedScreen::Profile(data) => data.return_to == ReturnDestination::PausedMiniGame,
        TypedScreen::Statistics(data) => data.return_to == ReturnDestination::PausedMiniGame,
        _ => false,
    };

    if return_to_minigame {
        // Return to paused mini-game screen
        state.screen = TypedScreen::MiniGame(MiniGameData::default());
        return Ok(());
    }

    // Default: return to mode selection (the main menu)
    state.screen = TypedScreen::ModeSelection(ModeSelectionData::default());
    state.game.session = None;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gamification::{ProfileStorage, UserProfile};
    use crate::learning::PerformanceTracker;
    use crate::ui::state::{ConfigState, GameState, ProgressState, UIState};

    fn create_test_state() -> AppState {
        AppState {
            screen: TypedScreen::ModeSelection(ModeSelectionData::default()),
            ui: UIState::new(),
            game: GameState::new(vec![]),
            progress: ProgressState::new(
                UserProfile::new(),
                PerformanceTracker::new(),
                ProfileStorage::new(),
            ),
            config: ConfigState::default(),
        }
    }

    #[test]
    fn test_back_to_menu_from_profile_returns_to_mode_selection() {
        let mut state = create_test_state();
        state.screen = TypedScreen::Profile(ProfileData {
            return_to: ReturnDestination::Menu,
        });

        handle_back_to_menu(&mut state).unwrap();

        assert!(matches!(state.screen, TypedScreen::ModeSelection(_)));
    }

    #[test]
    fn test_back_to_menu_from_statistics_returns_to_mode_selection() {
        let mut state = create_test_state();
        state.screen = TypedScreen::Statistics(StatisticsData {
            return_to: ReturnDestination::Menu,
        });

        handle_back_to_menu(&mut state).unwrap();

        assert!(matches!(state.screen, TypedScreen::ModeSelection(_)));
    }

    #[test]
    fn test_back_to_menu_from_profile_returns_to_paused_minigame() {
        let mut state = create_test_state();
        state.screen = TypedScreen::Profile(ProfileData {
            return_to: ReturnDestination::PausedMiniGame,
        });

        handle_back_to_menu(&mut state).unwrap();

        assert!(matches!(state.screen, TypedScreen::MiniGame(_)));
    }

    #[test]
    fn test_back_to_menu_from_statistics_returns_to_paused_minigame() {
        let mut state = create_test_state();
        state.screen = TypedScreen::Statistics(StatisticsData {
            return_to: ReturnDestination::PausedMiniGame,
        });

        handle_back_to_menu(&mut state).unwrap();

        assert!(matches!(state.screen, TypedScreen::MiniGame(_)));
    }

    #[test]
    fn test_back_to_menu_from_other_screen_returns_to_mode_selection() {
        let mut state = create_test_state();
        state.screen = TypedScreen::MiniGame(MiniGameData::default());

        handle_back_to_menu(&mut state).unwrap();

        assert!(matches!(state.screen, TypedScreen::ModeSelection(_)));
    }
}
