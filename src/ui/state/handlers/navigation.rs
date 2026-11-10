//! Navigation message handlers
//!
//! Handles screen transitions and application lifecycle messages
//!
//! Type-safe handlers that use HandlerOutcome for explicit transitions.

use crate::security::UserError;
use crate::ui::state::{
    AchievementsData, AppState, CategoryFiltersData, HandlerContext, HandlerOutcome, MenuData,
    MiniGameData, ModeSelectionData, NavTarget, ProfileData, ReturnDestination, StatisticsData,
    TypedScreen,
};

/// Handle QuitApp message
///
/// The single sink for exiting the application: sets the running flag to
/// false so the event loop stops. Every code path that quits the app (the
/// global Ctrl-C shortcut, the main-menu "Quit" item, etc.) must route
/// through this function rather than setting `ui.running` directly, so a
/// mid-session arcade quit always awards its XP/score/FSRS bookkeeping via
/// [`super::minigame::handle_minigame_game_over`] - idempotent per session,
/// so calling it here is safe even if bookkeeping already ran for this
/// session through another path (e.g. an in-game timeout).
pub fn handle_quit_app(state: &mut AppState) -> Result<HandlerOutcome, UserError> {
    if state.game.minigame_session.is_some() {
        super::minigame::handle_minigame_game_over(state)?;
    }
    state.ui.running = false;
    Ok(HandlerOutcome::Stay)
}

/// Handle NavigateTo message
///
/// Changes the current screen to the target screen.
/// Returns a Transition outcome with the new screen.
pub fn handle_navigate_to(target: NavTarget) -> Result<HandlerOutcome, UserError> {
    let new_screen = match target {
        NavTarget::ModeSelection => TypedScreen::ModeSelection(ModeSelectionData::default()),
        NavTarget::MainMenu => TypedScreen::Menu(MenuData::default()),
        NavTarget::Profile => TypedScreen::Profile(ProfileData::default()),
        NavTarget::Statistics => TypedScreen::Statistics(StatisticsData::default()),
        NavTarget::Achievements => TypedScreen::Achievements(AchievementsData::default()),
        NavTarget::CategoryFilters => TypedScreen::CategoryFilters(CategoryFiltersData::default()),
    };
    Ok(HandlerOutcome::Transition(Box::new(new_screen)))
}

/// Handle BackToMenu message
///
/// Returns to the appropriate screen based on context:
/// - From Profile/Statistics with PausedMiniGame return destination: returns to paused mini-game
/// - From CategoryFilters: returns to Menu (scenario list) or PausedMiniGame based on return_to
/// - Otherwise: returns to mode selection screen (the main menu)
pub fn handle_back_to_menu(
    current_screen: &TypedScreen,
    _ctx: &mut HandlerContext<'_>,
) -> Result<HandlerOutcome, UserError> {
    // CategoryFilters returns to its original screen (Menu or PausedMiniGame)
    if let TypedScreen::CategoryFilters(data) = current_screen {
        return match data.return_to {
            ReturnDestination::Menu => Ok(HandlerOutcome::Transition(Box::new(TypedScreen::Menu(
                MenuData::default(),
            )))),
            ReturnDestination::PausedMiniGame => Ok(HandlerOutcome::Transition(Box::new(
                TypedScreen::MiniGame(MiniGameData::default()),
            ))),
        };
    }

    // Check if we should return to paused mini-game instead of mode selection
    let return_to_minigame = match current_screen {
        TypedScreen::Profile(data) => data.return_to == ReturnDestination::PausedMiniGame,
        TypedScreen::Statistics(data) => data.return_to == ReturnDestination::PausedMiniGame,
        TypedScreen::Achievements(data) => data.return_to == ReturnDestination::PausedMiniGame,
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
    use std::assert_matches;

    fn create_test_context() -> (UIState, GameState, ProgressState, ConfigState) {
        (
            UIState::new(),
            GameState::new(vec![]),
            ProgressState::new(
                UserProfile::new(),
                PerformanceTracker::new(),
                ProfileStorage::for_test(),
            ),
            ConfigState::default(),
        )
    }

    fn create_test_app_state() -> AppState {
        let (ui, game, progress, config) = create_test_context();
        AppState {
            screen: TypedScreen::ModeSelection(ModeSelectionData::default()),
            ui,
            game,
            progress,
            config,
        }
    }

    #[test]
    fn test_quit_app() {
        let mut state = create_test_app_state();

        assert!(state.ui.running);
        let outcome = handle_quit_app(&mut state).unwrap();

        assert!(outcome.is_stay());
        assert!(!state.ui.running);
    }

    #[test]
    fn test_navigate_to_mode_selection() {
        let outcome = handle_navigate_to(NavTarget::ModeSelection).unwrap();

        assert!(outcome.is_transition());
        if let HandlerOutcome::Transition(screen) = outcome {
            assert_matches!(*screen, TypedScreen::ModeSelection(_));
        }
    }

    #[test]
    fn test_navigate_to_profile() {
        let outcome = handle_navigate_to(NavTarget::Profile).unwrap();

        assert!(outcome.is_transition());
        if let HandlerOutcome::Transition(screen) = outcome {
            assert_matches!(*screen, TypedScreen::Profile(_));
        }
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
            assert_matches!(*new_screen, TypedScreen::ModeSelection(_));
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
            assert_matches!(*new_screen, TypedScreen::ModeSelection(_));
        }
    }

    #[test]
    fn test_back_to_menu_from_achievements_returns_to_mode_selection() {
        let screen = TypedScreen::Achievements(AchievementsData {
            return_to: ReturnDestination::Menu,
            scroll_offset: 0,
        });
        let (mut ui, mut game, mut progress, config) = create_test_context();
        let mut ctx = HandlerContext::new(&mut ui, &mut game, &mut progress, &config);

        let outcome = handle_back_to_menu(&screen, &mut ctx).unwrap();

        assert!(outcome.is_transition());
        if let HandlerOutcome::Transition(new_screen) = outcome {
            assert_matches!(*new_screen, TypedScreen::ModeSelection(_));
        }
    }

    #[test]
    fn test_back_to_menu_from_achievements_returns_to_paused_minigame() {
        let screen = TypedScreen::Achievements(AchievementsData {
            return_to: ReturnDestination::PausedMiniGame,
            scroll_offset: 0,
        });
        let (mut ui, mut game, mut progress, config) = create_test_context();
        let mut ctx = HandlerContext::new(&mut ui, &mut game, &mut progress, &config);

        let outcome = handle_back_to_menu(&screen, &mut ctx).unwrap();

        assert!(outcome.is_transition());
        if let HandlerOutcome::Transition(new_screen) = outcome {
            assert_matches!(*new_screen, TypedScreen::MiniGame(_));
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
            assert_matches!(*new_screen, TypedScreen::MiniGame(_));
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
            assert_matches!(*new_screen, TypedScreen::MiniGame(_));
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
            assert_matches!(*new_screen, TypedScreen::ModeSelection(_));
        }
    }

    #[test]
    fn test_navigate_to_main_menu() {
        let outcome = handle_navigate_to(NavTarget::MainMenu).unwrap();

        assert!(outcome.is_transition());
        if let HandlerOutcome::Transition(screen) = outcome {
            assert_matches!(*screen, TypedScreen::Menu(_));
        }
    }

    #[test]
    fn test_navigate_to_statistics() {
        let outcome = handle_navigate_to(NavTarget::Statistics).unwrap();

        assert!(outcome.is_transition());
        if let HandlerOutcome::Transition(screen) = outcome {
            assert_matches!(*screen, TypedScreen::Statistics(_));
        }
    }

    #[test]
    fn test_navigate_to_achievements() {
        let outcome = handle_navigate_to(NavTarget::Achievements).unwrap();

        assert!(outcome.is_transition());
        if let HandlerOutcome::Transition(screen) = outcome {
            assert_matches!(*screen, TypedScreen::Achievements(_));
        }
    }

    #[test]
    fn test_navigate_to_category_filters() {
        let outcome = handle_navigate_to(NavTarget::CategoryFilters).unwrap();

        assert!(outcome.is_transition());
        if let HandlerOutcome::Transition(screen) = outcome {
            assert_matches!(*screen, TypedScreen::CategoryFilters(_));
        }
    }

    #[test]
    fn test_back_to_menu_from_menu_returns_to_mode_selection() {
        let screen = TypedScreen::Menu(MenuData::default());
        let (mut ui, mut game, mut progress, config) = create_test_context();
        let mut ctx = HandlerContext::new(&mut ui, &mut game, &mut progress, &config);

        let outcome = handle_back_to_menu(&screen, &mut ctx).unwrap();

        assert!(outcome.is_transition());
        if let HandlerOutcome::Transition(new_screen) = outcome {
            assert_matches!(*new_screen, TypedScreen::ModeSelection(_));
        }
    }

    #[test]
    fn test_back_to_menu_from_category_filters_returns_to_menu() {
        let screen = TypedScreen::CategoryFilters(CategoryFiltersData {
            selected_index: 0,
            return_to: ReturnDestination::Menu,
        });
        let (mut ui, mut game, mut progress, config) = create_test_context();
        let mut ctx = HandlerContext::new(&mut ui, &mut game, &mut progress, &config);

        let outcome = handle_back_to_menu(&screen, &mut ctx).unwrap();

        assert!(outcome.is_transition());
        if let HandlerOutcome::Transition(new_screen) = outcome {
            // Should return to Menu (scenario list), not ModeSelection
            assert_matches!(*new_screen, TypedScreen::Menu(_));
        }
    }

    #[test]
    fn test_back_to_menu_from_category_filters_returns_to_paused_minigame() {
        let screen = TypedScreen::CategoryFilters(CategoryFiltersData {
            selected_index: 0,
            return_to: ReturnDestination::PausedMiniGame,
        });
        let (mut ui, mut game, mut progress, config) = create_test_context();
        let mut ctx = HandlerContext::new(&mut ui, &mut game, &mut progress, &config);

        let outcome = handle_back_to_menu(&screen, &mut ctx).unwrap();

        assert!(outcome.is_transition());
        if let HandlerOutcome::Transition(new_screen) = outcome {
            assert_matches!(*new_screen, TypedScreen::MiniGame(_));
        }
    }
}
