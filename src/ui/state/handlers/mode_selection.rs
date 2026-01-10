//! Message handlers for mode selection screen
//!
//! Type-safe handlers that receive ModeSelectionData directly instead of
//! performing runtime checks on AppState.

use crate::security::UserError;
use crate::ui::state::{
    HandlerContext, HandlerOutcome, MenuData, MiniGameData, ModeSelectionData, TypedScreen,
};

/// Handle mode selection up navigation
///
/// Type-safe handler that only accepts ModeSelectionData, ensuring compile-time
/// guarantee that it's called on the correct screen.
pub(in crate::ui::state) fn handle_mode_selection_up(
    data: &mut ModeSelectionData,
) -> Result<HandlerOutcome, UserError> {
    if data.selected_mode > 0 {
        data.selected_mode -= 1;
    }
    Ok(HandlerOutcome::Stay)
}

/// Handle mode selection down navigation
///
/// Type-safe handler that only accepts ModeSelectionData.
pub(in crate::ui::state) fn handle_mode_selection_down(
    data: &mut ModeSelectionData,
) -> Result<HandlerOutcome, UserError> {
    // Only 2 modes: Training (0) and Arcade (1)
    if data.selected_mode < 1 {
        data.selected_mode += 1;
    }
    Ok(HandlerOutcome::Stay)
}

/// Handle mode selection confirmation
///
/// Type-safe handler that transitions to the selected mode screen.
/// Uses HandlerContext to delegate to other handlers.
pub(in crate::ui::state) fn handle_mode_selection_select(
    data: &ModeSelectionData,
    ctx: &mut HandlerContext<'_>,
) -> Result<HandlerOutcome, UserError> {
    match data.selected_mode {
        0 => handle_select_training_mode(ctx),
        1 => handle_select_arcade_mode(ctx),
        _ => Ok(HandlerOutcome::Stay), // Invalid selection, do nothing
    }
}

/// Handle selecting Training Mode
///
/// Transitions to the main menu screen.
pub(in crate::ui::state) fn handle_select_training_mode(
    _ctx: &mut HandlerContext<'_>,
) -> Result<HandlerOutcome, UserError> {
    Ok(HandlerOutcome::Transition(Box::new(TypedScreen::Menu(
        MenuData::default(),
    ))))
}

/// Handle selecting Arcade Mode
///
/// Transitions to the mini-game screen with FSRS-weighted scenario selection.
pub(in crate::ui::state) fn handle_select_arcade_mode(
    ctx: &mut HandlerContext<'_>,
) -> Result<HandlerOutcome, UserError> {
    // Use shared session creation with FSRS tracker for weighted selection
    super::minigame::create_minigame_session(ctx.game, Some(&ctx.progress.performance_tracker));

    Ok(HandlerOutcome::Transition(Box::new(TypedScreen::MiniGame(
        MiniGameData::default(),
    ))))
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
    fn test_mode_selection_up() {
        let mut data = ModeSelectionData { selected_mode: 1 }; // Start at Arcade

        let outcome = handle_mode_selection_up(&mut data).unwrap();

        assert!(outcome.is_stay());
        assert_eq!(data.selected_mode, 0); // Should move to Training
    }

    #[test]
    fn test_mode_selection_up_at_top() {
        let mut data = ModeSelectionData::default(); // Already at top (0)

        let outcome = handle_mode_selection_up(&mut data).unwrap();

        assert!(outcome.is_stay());
        assert_eq!(data.selected_mode, 0); // Should stay at 0
    }

    #[test]
    fn test_mode_selection_down() {
        let mut data = ModeSelectionData::default(); // Start at 0 (Training)

        let outcome = handle_mode_selection_down(&mut data).unwrap();

        assert!(outcome.is_stay());
        assert_eq!(data.selected_mode, 1); // Should move to Arcade
    }

    #[test]
    fn test_mode_selection_down_at_bottom() {
        let mut data = ModeSelectionData { selected_mode: 1 }; // Already at bottom (Arcade)

        let outcome = handle_mode_selection_down(&mut data).unwrap();

        assert!(outcome.is_stay());
        assert_eq!(data.selected_mode, 1); // Should stay at 1
    }

    #[test]
    fn test_select_training_mode() {
        let (mut ui, mut game, mut progress, config) = create_test_context();
        let mut ctx = HandlerContext::new(&mut ui, &mut game, &mut progress, &config);

        let outcome = handle_select_training_mode(&mut ctx).unwrap();

        assert!(outcome.is_transition());
        if let HandlerOutcome::Transition(screen) = outcome {
            assert!(matches!(*screen, TypedScreen::Menu(_)));
        }
    }

    #[test]
    fn test_select_arcade_mode() {
        let (mut ui, mut game, mut progress, config) = create_test_context();
        let mut ctx = HandlerContext::new(&mut ui, &mut game, &mut progress, &config);

        let outcome = handle_select_arcade_mode(&mut ctx).unwrap();

        assert!(outcome.is_transition());
        if let HandlerOutcome::Transition(screen) = outcome {
            assert!(matches!(*screen, TypedScreen::MiniGame(_)));
        }
    }

    #[test]
    fn test_mode_selection_select_training() {
        let data = ModeSelectionData::default(); // Default is 0 (Training)
        let (mut ui, mut game, mut progress, config) = create_test_context();
        let mut ctx = HandlerContext::new(&mut ui, &mut game, &mut progress, &config);

        let outcome = handle_mode_selection_select(&data, &mut ctx).unwrap();

        assert!(outcome.is_transition());
        if let HandlerOutcome::Transition(screen) = outcome {
            assert!(matches!(*screen, TypedScreen::Menu(_)));
        }
    }

    #[test]
    fn test_mode_selection_select_arcade() {
        let data = ModeSelectionData { selected_mode: 1 }; // Select Arcade
        let (mut ui, mut game, mut progress, config) = create_test_context();
        let mut ctx = HandlerContext::new(&mut ui, &mut game, &mut progress, &config);

        let outcome = handle_mode_selection_select(&data, &mut ctx).unwrap();

        assert!(outcome.is_transition());
        if let HandlerOutcome::Transition(screen) = outcome {
            assert!(matches!(*screen, TypedScreen::MiniGame(_)));
        }
    }
}
