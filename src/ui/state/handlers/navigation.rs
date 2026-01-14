//! Navigation message handlers
//!
//! Handles screen transitions and application lifecycle messages
//!
//! Type-safe handlers that use HandlerOutcome for explicit transitions.

use crate::security::UserError;
use crate::ui::state::{
    CategoryFiltersData, HandlerContext, HandlerOutcome, MenuData, MiniGameData, ModeSelectionData,
    ProfileData, ReturnDestination, Screen, StatisticsData, TypedScreen,
};

/// Handle QuitApp message
///
/// Sets the running flag to false to exit the application.
/// Does not require screen data - operates on UIState.
pub fn handle_quit_app(ctx: &mut HandlerContext<'_>) -> Result<HandlerOutcome, UserError> {
    ctx.ui.running = false;
    Ok(HandlerOutcome::Stay)
}

/// Handle NavigateTo message
///
/// Changes the current screen to the specified screen.
/// Returns a Transition outcome with the new screen.
pub fn handle_navigate_to(screen: Screen) -> Result<HandlerOutcome, UserError> {
    // Convert Screen enum to TypedScreen variant
    let new_screen = match screen {
        Screen::ModeSelection => TypedScreen::ModeSelection(ModeSelectionData::default()),
        Screen::MainMenu => TypedScreen::Menu(MenuData::default()),
        Screen::Profile => TypedScreen::Profile(ProfileData::default()),
        Screen::Statistics => TypedScreen::Statistics(StatisticsData::default()),
        Screen::CategoryFilters => TypedScreen::CategoryFilters(CategoryFiltersData::default()),
        Screen::MiniGame => TypedScreen::MiniGame(MiniGameData::default()),
        // NOTE: Task, Results, and Review screens require data and should not be
        // navigated to via NavigateTo - they have their own handlers
        Screen::Task | Screen::Results | Screen::Review => {
            // Stay on current screen if trying to navigate to a data-dependent screen
            return Ok(HandlerOutcome::Stay);
        }
    };
    Ok(HandlerOutcome::Transition(Box::new(new_screen)))
}

/// Handle BackToMenu message
///
/// Returns to the appropriate screen based on context:
/// - From Profile/Statistics with PausedMiniGame return destination: returns to paused mini-game
/// - Otherwise: returns to mode selection screen (the main menu)
pub fn handle_back_to_menu(
    current_screen: &TypedScreen,
    _ctx: &mut HandlerContext<'_>,
) -> Result<HandlerOutcome, UserError> {
    // Check if we should return to paused mini-game instead of mode selection
    let return_to_minigame = match current_screen {
        TypedScreen::Profile(data) => data.return_to == ReturnDestination::PausedMiniGame,
        TypedScreen::Statistics(data) => data.return_to == ReturnDestination::PausedMiniGame,
        _ => false,
    };

    if return_to_minigame {
        // Return to paused mini-game screen
        return Ok(HandlerOutcome::Transition(Box::new(TypedScreen::MiniGame(
            MiniGameData::default(),
        ))));
    }

    // Default: return to mode selection (the main menu)
    Ok(HandlerOutcome::Transition(Box::new(
        TypedScreen::ModeSelection(ModeSelectionData::default()),
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gamification::{ProfileStorage, UserProfile};
    use crate::learning::PerformanceTracker;
    use crate::ui::state::{ConfigState, GameState, ProgressState, UIState};

    fn create_test_context() -> (UIState, GameState, ProgressState, ConfigState) {
        (
            UIState::new(),
            GameState::new(vec![]),
            ProgressState::new(
                UserProfile::new(),
                PerformanceTracker::new(),
                ProfileStorage::new(),
            ),
            ConfigState::default(),
        )
    }

    #[test]
    fn test_quit_app() {
        let (mut ui, mut game, mut progress, config) = create_test_context();
        let mut ctx = HandlerContext::new(&mut ui, &mut game, &mut progress, &config);

        assert!(ctx.ui.running);
        let outcome = handle_quit_app(&mut ctx).unwrap();

        assert!(outcome.is_stay());
        assert!(!ctx.ui.running);
    }

    #[test]
    fn test_navigate_to_mode_selection() {
        let outcome = handle_navigate_to(Screen::ModeSelection).unwrap();

        assert!(outcome.is_transition());
        if let HandlerOutcome::Transition(screen) = outcome {
            assert!(matches!(*screen, TypedScreen::ModeSelection(_)));
        }
    }

    #[test]
    fn test_navigate_to_profile() {
        let outcome = handle_navigate_to(Screen::Profile).unwrap();

        assert!(outcome.is_transition());
        if let HandlerOutcome::Transition(screen) = outcome {
            assert!(matches!(*screen, TypedScreen::Profile(_)));
        }
    }

    #[test]
    fn test_navigate_to_task_stays_on_current() {
        let outcome = handle_navigate_to(Screen::Task).unwrap();

        // Task screen requires data, so should stay on current screen
        assert!(outcome.is_stay());
    }

    #[test]
    fn test_back_to_menu_from_profile_returns_to_mode_selection() {
        let screen = TypedScreen::Profile(ProfileData {
            return_to: ReturnDestination::Menu,
        });
        let (mut ui, mut game, mut progress, config) = create_test_context();
        let mut ctx = HandlerContext::new(&mut ui, &mut game, &mut progress, &config);

        let outcome = handle_back_to_menu(&screen, &mut ctx).unwrap();

        assert!(outcome.is_transition());
        if let HandlerOutcome::Transition(new_screen) = outcome {
            assert!(matches!(*new_screen, TypedScreen::ModeSelection(_)));
        }
    }

    #[test]
    fn test_back_to_menu_from_statistics_returns_to_mode_selection() {
        let screen = TypedScreen::Statistics(StatisticsData {
            return_to: ReturnDestination::Menu,
        });
        let (mut ui, mut game, mut progress, config) = create_test_context();
        let mut ctx = HandlerContext::new(&mut ui, &mut game, &mut progress, &config);

        let outcome = handle_back_to_menu(&screen, &mut ctx).unwrap();

        assert!(outcome.is_transition());
        if let HandlerOutcome::Transition(new_screen) = outcome {
            assert!(matches!(*new_screen, TypedScreen::ModeSelection(_)));
        }
    }

    #[test]
    fn test_back_to_menu_from_profile_returns_to_paused_minigame() {
        let screen = TypedScreen::Profile(ProfileData {
            return_to: ReturnDestination::PausedMiniGame,
        });
        let (mut ui, mut game, mut progress, config) = create_test_context();
        let mut ctx = HandlerContext::new(&mut ui, &mut game, &mut progress, &config);

        let outcome = handle_back_to_menu(&screen, &mut ctx).unwrap();

        assert!(outcome.is_transition());
        if let HandlerOutcome::Transition(new_screen) = outcome {
            assert!(matches!(*new_screen, TypedScreen::MiniGame(_)));
        }
    }

    #[test]
    fn test_back_to_menu_from_statistics_returns_to_paused_minigame() {
        let screen = TypedScreen::Statistics(StatisticsData {
            return_to: ReturnDestination::PausedMiniGame,
        });
        let (mut ui, mut game, mut progress, config) = create_test_context();
        let mut ctx = HandlerContext::new(&mut ui, &mut game, &mut progress, &config);

        let outcome = handle_back_to_menu(&screen, &mut ctx).unwrap();

        assert!(outcome.is_transition());
        if let HandlerOutcome::Transition(new_screen) = outcome {
            assert!(matches!(*new_screen, TypedScreen::MiniGame(_)));
        }
    }

    #[test]
    fn test_back_to_menu_from_other_screen_returns_to_mode_selection() {
        let screen = TypedScreen::MiniGame(MiniGameData::default());
        let (mut ui, mut game, mut progress, config) = create_test_context();
        let mut ctx = HandlerContext::new(&mut ui, &mut game, &mut progress, &config);

        let outcome = handle_back_to_menu(&screen, &mut ctx).unwrap();

        assert!(outcome.is_transition());
        if let HandlerOutcome::Transition(new_screen) = outcome {
            assert!(matches!(*new_screen, TypedScreen::ModeSelection(_)));
        }
    }

    #[test]
    fn test_navigate_to_main_menu() {
        let outcome = handle_navigate_to(Screen::MainMenu).unwrap();

        assert!(outcome.is_transition());
        if let HandlerOutcome::Transition(screen) = outcome {
            assert!(matches!(*screen, TypedScreen::Menu(_)));
        }
    }

    #[test]
    fn test_navigate_to_statistics() {
        let outcome = handle_navigate_to(Screen::Statistics).unwrap();

        assert!(outcome.is_transition());
        if let HandlerOutcome::Transition(screen) = outcome {
            assert!(matches!(*screen, TypedScreen::Statistics(_)));
        }
    }

    #[test]
    fn test_navigate_to_minigame() {
        let outcome = handle_navigate_to(Screen::MiniGame).unwrap();

        assert!(outcome.is_transition());
        if let HandlerOutcome::Transition(screen) = outcome {
            assert!(matches!(*screen, TypedScreen::MiniGame(_)));
        }
    }

    #[test]
    fn test_navigate_to_category_filters() {
        let outcome = handle_navigate_to(Screen::CategoryFilters).unwrap();

        assert!(outcome.is_transition());
        if let HandlerOutcome::Transition(screen) = outcome {
            assert!(matches!(*screen, TypedScreen::CategoryFilters(_)));
        }
    }

    #[test]
    fn test_navigate_to_results_stays_on_current() {
        let outcome = handle_navigate_to(Screen::Results).unwrap();
        assert!(outcome.is_stay());
    }

    #[test]
    fn test_navigate_to_review_stays_on_current() {
        let outcome = handle_navigate_to(Screen::Review).unwrap();
        assert!(outcome.is_stay());
    }

    #[test]
    fn test_back_to_menu_from_menu_returns_to_mode_selection() {
        let screen = TypedScreen::Menu(MenuData::default());
        let (mut ui, mut game, mut progress, config) = create_test_context();
        let mut ctx = HandlerContext::new(&mut ui, &mut game, &mut progress, &config);

        let outcome = handle_back_to_menu(&screen, &mut ctx).unwrap();

        assert!(outcome.is_transition());
        if let HandlerOutcome::Transition(new_screen) = outcome {
            assert!(matches!(*new_screen, TypedScreen::ModeSelection(_)));
        }
    }
}
