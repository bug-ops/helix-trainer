//! Message handlers for mode selection screen
//!
//! Type-safe handlers that receive ModeSelectionData directly instead of
//! performing runtime checks on AppState.

use crate::minigame::MiniGameSession;
use crate::security::UserError;
use crate::ui::state::{
    HandlerContext, HandlerOutcome, MenuData, MiniGameData, MiniGameModeSelection,
    ModeSelectionData, TypedScreen,
};
use std::sync::Arc;

/// Handle mode selection up navigation
///
/// Type-safe handler that only accepts ModeSelectionData, ensuring compile-time
/// guarantee that it's called on the correct screen.
pub(in crate::ui::state) fn handle_mode_selection_up(
    data: &mut ModeSelectionData,
) -> Result<HandlerOutcome, UserError> {
    // Check if mini-game mode selection is active
    if let Some(ref mut selection) = data.minigame_mode_selection {
        selection.select_previous();
    } else if data.selected_mode > 0 {
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
    // Check if mini-game mode selection is active
    if let Some(ref mut selection) = data.minigame_mode_selection {
        selection.select_next();
    } else if data.selected_mode < 1 {
        // Only 2 modes: Training (0) and Arcade (1)
        data.selected_mode += 1;
    }
    Ok(HandlerOutcome::Stay)
}

/// Handle mode selection confirmation
///
/// Type-safe handler that transitions to the selected mode screen.
/// Uses HandlerContext to delegate to other handlers.
pub(in crate::ui::state) fn handle_mode_selection_select(
    data: &mut ModeSelectionData,
    ctx: &mut HandlerContext<'_>,
) -> Result<HandlerOutcome, UserError> {
    // Check if mini-game mode selection is active
    if let Some(ref selection) = data.minigame_mode_selection {
        // Launch the selected mini-game mode
        let mode = selection.selected_mode(ctx.progress.today());
        return handle_launch_minigame_mode(ctx, mode);
    }

    match data.selected_mode {
        0 => handle_select_training_mode(ctx),
        1 => {
            // Show mini-game mode selection instead of immediately starting arcade
            data.minigame_mode_selection = Some(MiniGameModeSelection::new());
            Ok(HandlerOutcome::Stay)
        }
        _ => Ok(HandlerOutcome::Stay), // Invalid selection, do nothing
    }
}

/// Handle escape/back navigation
///
/// If mini-game mode selection is open, close it. Otherwise, do nothing.
pub(in crate::ui::state) fn handle_mode_selection_back(
    data: &mut ModeSelectionData,
) -> Result<HandlerOutcome, UserError> {
    if data.minigame_mode_selection.is_some() {
        data.minigame_mode_selection = None;
    }
    Ok(HandlerOutcome::Stay)
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

/// Handle selecting Arcade Mode (legacy - for backward compatibility)
///
/// Transitions to the mini-game screen with FSRS-weighted scenario selection.
pub(in crate::ui::state) fn handle_select_arcade_mode(
    ctx: &mut HandlerContext<'_>,
) -> Result<HandlerOutcome, UserError> {
    // Use shared session creation with FSRS tracker for weighted selection
    super::minigame::create_minigame_session(ctx.game, Some(&ctx.progress.performance_tracker));
    ctx.ui.show_key_history = false;

    Ok(HandlerOutcome::Transition(Box::new(TypedScreen::MiniGame(
        MiniGameData::default(),
    ))))
}

/// Handle launching a specific mini-game mode
///
/// Creates a session with the selected mode and transitions to the mini-game screen.
pub(in crate::ui::state) fn handle_launch_minigame_mode(
    ctx: &mut HandlerContext<'_>,
    mode: crate::minigame::MiniGameMode,
) -> Result<HandlerOutcome, UserError> {
    use crate::config::Scenario;

    // Collect scenarios
    let scenarios: Vec<Scenario> = ctx
        .game
        .scenario_collection
        .filtered()
        .into_iter()
        .cloned()
        .collect();

    if scenarios.is_empty() {
        tracing::warn!("No scenarios available for mini-game");
        return Ok(HandlerOutcome::Stay);
    }

    // Clone mode for MiniGameData
    let mode_for_data = mode.clone();

    // Clone tracker for session's internal weighted selection
    let tracker_clone = Some(ctx.progress.performance_tracker.clone());

    // Create session with the selected mode
    let mut session = MiniGameSession::with_mode(Arc::new(scenarios), tracker_clone, mode);
    session.start(); // Begin countdown
    ctx.game.minigame_session = Some(session);
    ctx.ui.show_key_history = false;

    // Create MiniGameData with mode info
    let minigame_data = MiniGameData {
        mode: Some(mode_for_data),
        ..MiniGameData::default()
    };

    Ok(HandlerOutcome::Transition(Box::new(TypedScreen::MiniGame(
        minigame_data,
    ))))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Difficulty;
    use crate::gamification::{ProfileStorage, UserProfile};
    use crate::learning::PerformanceTracker;
    use crate::testing::ScenarioBuilder;
    use crate::ui::state::{ConfigState, GameState, ProgressState, UIState};

    fn create_test_scenario(id: &str) -> crate::config::Scenario {
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

    fn create_test_context_with_scenarios() -> (UIState, GameState, ProgressState, ConfigState) {
        let scenarios = vec![
            create_test_scenario("s1"),
            create_test_scenario("s2"),
            create_test_scenario("s3"),
        ];
        (
            UIState::new(),
            GameState::new(scenarios),
            ProgressState::new(
                UserProfile::new(),
                PerformanceTracker::new(),
                ProfileStorage::for_test(),
            ),
            ConfigState::default(),
        )
    }

    #[test]
    fn test_mode_selection_up() {
        let mut data = ModeSelectionData {
            selected_mode: 1,
            minigame_mode_selection: None,
        };

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
        let mut data = ModeSelectionData {
            selected_mode: 1,
            minigame_mode_selection: None,
        };

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
    fn test_select_arcade_mode_hides_key_history() {
        // Regression test for S1: this is a live dispatch path (unlike
        // handle_start_minigame, which nothing emits), so it must reset the
        // flag itself rather than relying on a caller to do it.
        let (mut ui, mut game, mut progress, config) = create_test_context();
        ui.show_key_history = true;
        let mut ctx = HandlerContext::new(&mut ui, &mut game, &mut progress, &config);

        handle_select_arcade_mode(&mut ctx).unwrap();

        assert!(!ctx.ui.show_key_history);
    }

    #[test]
    fn test_launch_minigame_mode_hides_key_history() {
        // Regression test for S1 on the second live dispatch path.
        let (mut ui, mut game, mut progress, config) = create_test_context_with_scenarios();
        ui.show_key_history = true;
        let mut ctx = HandlerContext::new(&mut ui, &mut game, &mut progress, &config);

        let mode = crate::minigame::MiniGameMode::default();
        handle_launch_minigame_mode(&mut ctx, mode).unwrap();

        assert!(!ctx.ui.show_key_history);
    }

    #[test]
    fn test_mode_selection_select_training() {
        let mut data = ModeSelectionData::default(); // Default is 0 (Training)
        let (mut ui, mut game, mut progress, config) = create_test_context();
        let mut ctx = HandlerContext::new(&mut ui, &mut game, &mut progress, &config);

        let outcome = handle_mode_selection_select(&mut data, &mut ctx).unwrap();

        assert!(outcome.is_transition());
        if let HandlerOutcome::Transition(screen) = outcome {
            assert!(matches!(*screen, TypedScreen::Menu(_)));
        }
    }

    #[test]
    fn test_mode_selection_select_arcade_opens_mode_menu() {
        let mut data = ModeSelectionData {
            selected_mode: 1,
            minigame_mode_selection: None,
        };
        let (mut ui, mut game, mut progress, config) = create_test_context_with_scenarios();
        let mut ctx = HandlerContext::new(&mut ui, &mut game, &mut progress, &config);

        let outcome = handle_mode_selection_select(&mut data, &mut ctx).unwrap();

        // Should stay on screen but open mode selection
        assert!(outcome.is_stay());
        assert!(data.minigame_mode_selection.is_some());
    }

    #[test]
    fn test_minigame_mode_selection_navigation() {
        let mut data = ModeSelectionData {
            selected_mode: 1,
            minigame_mode_selection: Some(MiniGameModeSelection::new()),
        };

        // Start at 0 (Arcade)
        assert_eq!(
            data.minigame_mode_selection
                .as_ref()
                .unwrap()
                .selected_index,
            0
        );

        // Move down to Survival
        handle_mode_selection_down(&mut data).unwrap();
        assert_eq!(
            data.minigame_mode_selection
                .as_ref()
                .unwrap()
                .selected_index,
            1
        );

        // Move down to Challenge
        handle_mode_selection_down(&mut data).unwrap();
        assert_eq!(
            data.minigame_mode_selection
                .as_ref()
                .unwrap()
                .selected_index,
            2
        );

        // Wrap to Arcade
        handle_mode_selection_down(&mut data).unwrap();
        assert_eq!(
            data.minigame_mode_selection
                .as_ref()
                .unwrap()
                .selected_index,
            0
        );

        // Move up to Challenge (wrap)
        handle_mode_selection_up(&mut data).unwrap();
        assert_eq!(
            data.minigame_mode_selection
                .as_ref()
                .unwrap()
                .selected_index,
            2
        );
    }

    #[test]
    fn test_minigame_mode_selection_back_closes_menu() {
        let mut data = ModeSelectionData {
            selected_mode: 1,
            minigame_mode_selection: Some(MiniGameModeSelection::new()),
        };

        handle_mode_selection_back(&mut data).unwrap();

        assert!(data.minigame_mode_selection.is_none());
    }

    #[test]
    fn test_launch_survival_mode() {
        let (mut ui, mut game, mut progress, config) = create_test_context_with_scenarios();
        let mut ctx = HandlerContext::new(&mut ui, &mut game, &mut progress, &config);

        let mode =
            crate::minigame::MiniGameMode::Survival(crate::minigame::SurvivalConfig::default());
        let outcome = handle_launch_minigame_mode(&mut ctx, mode).unwrap();

        assert!(outcome.is_transition());
        if let HandlerOutcome::Transition(screen) = outcome {
            assert!(matches!(*screen, TypedScreen::MiniGame(_)));
            if let TypedScreen::MiniGame(data) = *screen {
                assert!(data.mode.as_ref().map(|m| m.is_survival()).unwrap_or(false));
            }
        }

        // Verify session is created with 1 life
        assert!(ctx.game.minigame_session.is_some());
        assert_eq!(
            ctx.game.minigame_session.as_ref().unwrap().stats().lives(),
            1
        );
    }

    #[test]
    fn test_launch_challenge_mode() {
        let (mut ui, mut game, mut progress, config) = create_test_context_with_scenarios();
        let mut ctx = HandlerContext::new(&mut ui, &mut game, &mut progress, &config);

        let mode = crate::minigame::MiniGameMode::Challenge(
            crate::minigame::ChallengeConfig::for_date(chrono::Utc::now().date_naive()),
        );
        let outcome = handle_launch_minigame_mode(&mut ctx, mode).unwrap();

        assert!(outcome.is_transition());
        if let HandlerOutcome::Transition(screen) = outcome {
            assert!(matches!(*screen, TypedScreen::MiniGame(_)));
        }

        // Verify session is created with 3 lives
        assert!(ctx.game.minigame_session.is_some());
        assert_eq!(
            ctx.game.minigame_session.as_ref().unwrap().stats().lives(),
            3
        );
    }

    #[test]
    fn test_launch_no_scenarios() {
        let (mut ui, mut game, mut progress, config) = create_test_context(); // No scenarios
        let mut ctx = HandlerContext::new(&mut ui, &mut game, &mut progress, &config);

        let mode = crate::minigame::MiniGameMode::default();
        let outcome = handle_launch_minigame_mode(&mut ctx, mode).unwrap();

        // Should stay on screen when no scenarios available
        assert!(outcome.is_stay());
        assert!(ctx.game.minigame_session.is_none());
    }

    // CR-017: Test invalid selection index returns Stay
    #[test]
    fn test_mode_selection_select_invalid_index() {
        let mut data = ModeSelectionData {
            selected_mode: 99, // Invalid index
            minigame_mode_selection: None,
        };
        let (mut ui, mut game, mut progress, config) = create_test_context();
        let mut ctx = HandlerContext::new(&mut ui, &mut game, &mut progress, &config);

        let outcome = handle_mode_selection_select(&mut data, &mut ctx).unwrap();

        // Invalid index should stay on screen (do nothing)
        assert!(outcome.is_stay());
    }
}
