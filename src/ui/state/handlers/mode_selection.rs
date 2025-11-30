//! Message handlers for mode selection screen

use crate::security::UserError;
use crate::ui::state::{AppState, MenuData, TypedScreen};

/// Handle mode selection up navigation
pub(in crate::ui::state) fn handle_mode_selection_up(
    state: &mut AppState,
) -> Result<(), UserError> {
    if let TypedScreen::ModeSelection(mode_data) = &mut state.screen
        && mode_data.selected_mode > 0
    {
        mode_data.selected_mode -= 1;
    }
    Ok(())
}

/// Handle mode selection down navigation
pub(in crate::ui::state) fn handle_mode_selection_down(
    state: &mut AppState,
) -> Result<(), UserError> {
    if let TypedScreen::ModeSelection(mode_data) = &mut state.screen {
        // Only 2 modes: Training (0) and Arcade (1)
        if mode_data.selected_mode < 1 {
            mode_data.selected_mode += 1;
        }
    }
    Ok(())
}

/// Handle mode selection confirmation
pub(in crate::ui::state) fn handle_mode_selection_select(
    state: &mut AppState,
) -> Result<(), UserError> {
    if let TypedScreen::ModeSelection(mode_data) = &state.screen {
        match mode_data.selected_mode {
            0 => handle_select_training_mode(state),
            1 => handle_select_arcade_mode(state),
            _ => Ok(()), // Invalid selection, do nothing
        }
    } else {
        Ok(())
    }
}

/// Handle selecting Training Mode
pub(in crate::ui::state) fn handle_select_training_mode(
    state: &mut AppState,
) -> Result<(), UserError> {
    state.screen = TypedScreen::Menu(MenuData::default());
    Ok(())
}

/// Handle selecting Arcade Mode
pub(in crate::ui::state) fn handle_select_arcade_mode(
    state: &mut AppState,
) -> Result<(), UserError> {
    // Delegate to start_minigame handler
    super::minigame::handle_start_minigame(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gamification::{ProfileStorage, UserProfile};
    use crate::learning::PerformanceTracker;
    use crate::ui::state::{ConfigState, GameState, ModeSelectionData, ProgressState, UIState};

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
    fn test_mode_selection_up() {
        let mut state = create_test_state();
        if let TypedScreen::ModeSelection(mode_data) = &mut state.screen {
            mode_data.selected_mode = 1; // Start at Arcade
        }

        handle_mode_selection_up(&mut state).unwrap();

        if let TypedScreen::ModeSelection(mode_data) = &state.screen {
            assert_eq!(mode_data.selected_mode, 0); // Should move to Training
        } else {
            panic!("Should be on ModeSelection screen");
        }
    }

    #[test]
    fn test_mode_selection_up_at_top() {
        let mut state = create_test_state();
        // Already at top (0)

        handle_mode_selection_up(&mut state).unwrap();

        if let TypedScreen::ModeSelection(mode_data) = &state.screen {
            assert_eq!(mode_data.selected_mode, 0); // Should stay at 0
        } else {
            panic!("Should be on ModeSelection screen");
        }
    }

    #[test]
    fn test_mode_selection_down() {
        let mut state = create_test_state();
        // Start at 0 (Training)

        handle_mode_selection_down(&mut state).unwrap();

        if let TypedScreen::ModeSelection(mode_data) = &state.screen {
            assert_eq!(mode_data.selected_mode, 1); // Should move to Arcade
        } else {
            panic!("Should be on ModeSelection screen");
        }
    }

    #[test]
    fn test_mode_selection_down_at_bottom() {
        let mut state = create_test_state();
        if let TypedScreen::ModeSelection(mode_data) = &mut state.screen {
            mode_data.selected_mode = 1; // Already at bottom (Arcade)
        }

        handle_mode_selection_down(&mut state).unwrap();

        if let TypedScreen::ModeSelection(mode_data) = &state.screen {
            assert_eq!(mode_data.selected_mode, 1); // Should stay at 1
        } else {
            panic!("Should be on ModeSelection screen");
        }
    }

    #[test]
    fn test_select_training_mode() {
        let mut state = create_test_state();

        handle_select_training_mode(&mut state).unwrap();

        assert!(
            matches!(state.screen, TypedScreen::Menu(_)),
            "Should navigate to Menu screen"
        );
    }

    #[test]
    fn test_select_arcade_mode() {
        let mut state = create_test_state();

        handle_select_arcade_mode(&mut state).unwrap();

        assert!(
            matches!(state.screen, TypedScreen::MiniGame(_)),
            "Should navigate to MiniGame screen"
        );
    }

    #[test]
    fn test_mode_selection_select_training() {
        let mut state = create_test_state();
        // Default is 0 (Training)

        handle_mode_selection_select(&mut state).unwrap();

        assert!(
            matches!(state.screen, TypedScreen::Menu(_)),
            "Should navigate to Menu screen when selecting Training"
        );
    }

    #[test]
    fn test_mode_selection_select_arcade() {
        let mut state = create_test_state();
        if let TypedScreen::ModeSelection(mode_data) = &mut state.screen {
            mode_data.selected_mode = 1; // Select Arcade
        }

        handle_mode_selection_select(&mut state).unwrap();

        assert!(
            matches!(state.screen, TypedScreen::MiniGame(_)),
            "Should navigate to MiniGame screen when selecting Arcade"
        );
    }
}
