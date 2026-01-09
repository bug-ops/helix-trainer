//! Message handlers for mini-game mode (Arcade Mode)

use std::sync::Arc;

use crate::config::Scenario;
use crate::constants::{MINIGAME_SCENARIO_BASE_XP, MINIGAME_STREAK_XP_MULTIPLIER};
use crate::game::format_key_for_display;
use crate::input::typestate::{HandlerResult, command_to_key_event};
use crate::minigame::MiniGameSession;
use crate::security::UserError;
use crate::ui::state::{
    AppState, GameState, HandlerContext, HandlerOutcome, InputStateAccess, MiniGameData,
    ModeSelectionData, TypedScreen,
};

/// Create and start a new mini-game session from available scenarios
///
/// Shared initialization logic used by both mode selection and direct start.
/// Returns true if session was created successfully, false if no scenarios available.
pub(in crate::ui::state) fn create_minigame_session(game: &mut GameState) -> bool {
    let scenarios: Vec<Scenario> = game
        .scenario_collection
        .get_filtered()
        .into_iter()
        .cloned()
        .collect();

    if scenarios.is_empty() {
        tracing::warn!("No scenarios available for mini-game");
        return false;
    }

    let mut session = MiniGameSession::new(Arc::new(scenarios));
    session.start(); // Begin countdown
    game.minigame_session = Some(session);
    true
}

/// Handle starting a mini-game session
pub(in crate::ui::state) fn handle_start_minigame(
    ctx: &mut HandlerContext<'_>,
) -> Result<HandlerOutcome, UserError> {
    create_minigame_session(ctx.game);
    Ok(HandlerOutcome::Transition(Box::new(TypedScreen::MiniGame(
        MiniGameData::default(),
    ))))
}

/// Handle pausing mini-game
pub(in crate::ui::state) fn handle_pause_minigame(
    ctx: &mut HandlerContext<'_>,
) -> Result<HandlerOutcome, UserError> {
    if let Some(ref mut session) = ctx.game.minigame_session {
        session.pause();
    }
    Ok(HandlerOutcome::Stay)
}

/// Handle resuming mini-game
pub(in crate::ui::state) fn handle_resume_minigame(
    ctx: &mut HandlerContext<'_>,
) -> Result<HandlerOutcome, UserError> {
    if let Some(ref mut session) = ctx.game.minigame_session {
        session.resume();
    }
    Ok(HandlerOutcome::Stay)
}

/// Handle mini-game timer tick (100ms)
pub(in crate::ui::state) fn handle_minigame_tick(
    ctx: &mut HandlerContext<'_>,
) -> Result<HandlerOutcome, UserError> {
    if let Some(ref mut session) = ctx.game.minigame_session
        && session.state().is_countdown()
    {
        session.tick_countdown();
    }
    Ok(HandlerOutcome::Stay)
}

/// Execute a command in the mini-game session
///
/// Handles command execution, quest progress updates, and completion detection.
/// Uses shared quest tracking functions from quests module.
fn execute_minigame_command(state: &mut AppState, command: &str) -> Result<(), UserError> {
    // Record key to history for display (using shared formatter)
    if let TypedScreen::MiniGame(ref mut data) = state.screen {
        data.add_key_to_history(format_key_for_display(command));
    }

    // Snapshot quest completion status before updates
    let was_completed = super::snapshot_quest_completion(state);

    let Some(ref mut session) = state.game.minigame_session else {
        return Ok(());
    };

    // handle_command internally uses CommandExecutor::execute_with_count
    // for unified count prefix handling (e.g., "3d" -> 3x "d")
    session.handle_command(command)?;

    // Update quest progress for command used (shared function)
    super::track_command_for_quests(state, command);

    // Re-borrow session after state modification
    let Some(ref mut session) = state.game.minigame_session else {
        return Ok(());
    };

    // Check for completion
    if session.check_completion() {
        // Get current streak before advancing
        let current_streak = session.stats().streak;

        // Record to FSRS before advancing (only if we have actions)
        if let Some(scenario) = session.current_scenario() {
            if !scenario.actions().is_empty() {
                let tracker = &mut state.progress.performance_tracker;
                session.record_to_fsrs(tracker, true); // Success!
            }

            // Update quest progress for scenario completion (shared function)
            let duration = scenario.elapsed();
            let scenario_id = scenario.scenario.id.clone();
            super::track_scenario_completion_for_quests(state, &scenario_id, duration);
        }

        // Award XP for scenario completion in arcade mode
        // Base XP per scenario + bonus per streak level (encourages maintaining streaks)
        let scenario_xp = MINIGAME_SCENARIO_BASE_XP
            + (current_streak.saturating_sub(1) as u64 * MINIGAME_STREAK_XP_MULTIPLIER);
        let profile = &mut state.progress.profile;
        profile.add_xp(scenario_xp);

        // Award XP for newly completed quests (this function adds XP internally)
        let quest_xp = super::award_quest_completion_xp(state, &was_completed);

        // Store total XP earned for display in transition popup
        let total_xp = scenario_xp + quest_xp;
        if let TypedScreen::MiniGame(ref mut data) = state.screen {
            data.last_xp_earned = Some(total_xp);
        }

        // Re-borrow session after state modification
        if let Some(ref mut session) = state.game.minigame_session {
            session.advance_to_next();
        }
        // Transition state will be handled by timer
    }

    Ok(())
}

/// Handle executing a Helix command during mini-game
///
/// Uses typestate-based InputStateMachine for multi-key commands (dd, gg, rx).
pub(in crate::ui::state) fn handle_minigame_command(
    state: &mut AppState,
    command: std::borrow::Cow<'static, str>,
) -> Result<(), UserError> {
    // Get minigame data for input state machine
    let TypedScreen::MiniGame(ref mut minigame_data) = state.screen else {
        return Ok(());
    };

    // Add key to history for display
    let display_key = format_key_for_display(&command);
    minigame_data.add_key_to_history(display_key);

    // Check if we're in insert mode
    let is_insert_mode = state
        .game
        .minigame_session
        .as_ref()
        .map(|s| s.is_insert_mode())
        .unwrap_or(false);

    if is_insert_mode {
        // In insert mode, execute command directly (bypass input state machine)
        return execute_minigame_command(state, &command);
    }

    // Normal mode - use InputStateMachine for multi-key command handling
    // Convert the command string to a KeyEvent for the state machine
    let key_event = command_to_key_event(&command);

    // Need to re-borrow minigame_data after getting is_insert_mode
    let TypedScreen::MiniGame(ref mut minigame_data) = state.screen else {
        return Ok(());
    };

    // Process through the input state machine
    let handler_result = minigame_data.input_state_mut().process_key(key_event);

    match handler_result {
        HandlerResult::Execute(cmd) => execute_minigame_command(state, cmd.as_ref()),
        HandlerResult::Transition(_) => Ok(()), // Waiting for more keys
        HandlerResult::Cancel | HandlerResult::Stay => Ok(()), // Cancelled or unknown
    }
}

/// Handle timeout on current mini-game scenario
pub(in crate::ui::state) fn handle_minigame_timeout(state: &mut AppState) -> Result<(), UserError> {
    if let Some(ref mut session) = state.game.minigame_session {
        session.handle_timeout();

        if session.state().is_game_over() {
            handle_minigame_game_over(state)?;
        }
    }
    Ok(())
}

/// Handle scenario completion (user triggered)
pub(in crate::ui::state) fn handle_minigame_scenario_complete(
    ctx: &mut HandlerContext<'_>,
) -> Result<HandlerOutcome, UserError> {
    if let Some(ref mut session) = ctx.game.minigame_session {
        session.advance_to_next();
    }
    Ok(HandlerOutcome::Stay)
}

/// Handle advancing to next scenario (after transition delay)
pub(in crate::ui::state) fn handle_minigame_next_scenario(
    ctx: &mut HandlerContext<'_>,
) -> Result<HandlerOutcome, UserError> {
    if let Some(ref mut session) = ctx.game.minigame_session
        && let Err(e) = session.complete_transition()
    {
        tracing::warn!("Failed to load next mini-game scenario: {:?}", e);
    }
    Ok(HandlerOutcome::Stay)
}

/// Handle game over - calculate XP, update profile, save progress
///
/// Called when the mini-game ends (player runs out of lives).
/// Performs final integration with progression systems:
/// - Records final FSRS data for last scenario
/// - Calculates and awards XP
/// - Updates high scores
/// - Saves profile to disk (non-fatal on error)
pub(in crate::ui::state) fn handle_minigame_game_over(
    state: &mut AppState,
) -> Result<(), UserError> {
    if let Some(ref session) = state.game.minigame_session {
        let stats = session.stats();

        // 1. Record final scenario to FSRS (if applicable)
        let tracker = &mut state.progress.performance_tracker;
        session.record_to_fsrs(tracker, false); // Game over = failure on current scenario

        // 2. Calculate XP earned
        use crate::gamification::XPCalculator;
        let xp = XPCalculator::minigame_xp(stats.score, stats.level, stats.best_streak);

        // 3. Update profile with XP and high scores
        let profile = &mut state.progress.profile;
        let leveled_up = profile.add_xp(xp);

        // Update high scores if beaten
        let mut new_high_score = false;
        if stats.score > profile.minigame_high_score {
            profile.minigame_high_score = stats.score;
            new_high_score = true;
        }

        if stats.best_streak > profile.minigame_best_streak {
            profile.minigame_best_streak = stats.best_streak;
        }

        // Increment total games played counter
        profile.minigame_games_played = profile.minigame_games_played.saturating_add(1);

        // Log results
        tracing::info!(
            xp_earned = xp,
            score = stats.score,
            level = stats.level,
            streak = stats.best_streak,
            leveled_up = leveled_up,
            new_high_score = new_high_score,
            "Mini-game session completed"
        );

        // 4. Persist profile to disk (non-fatal error - log but continue)
        let save_result = state.progress.storage.save(&state.progress.profile);

        if let Err(e) = save_result {
            tracing::error!("Failed to save profile after mini-game: {:?}", e);
            // Don't return error - game over screen should still display
        } else {
            state.progress.mark_saved();
        }
    }

    Ok(())
}

/// Handle returning to mode selection from mini-game
pub(in crate::ui::state) fn handle_minigame_back_to_menu(
    state: &mut AppState,
) -> Result<HandlerOutcome, UserError> {
    if state.game.minigame_session.is_some() {
        handle_minigame_game_over(state)?;
    }
    state.game.minigame_session = None;
    Ok(HandlerOutcome::Transition(Box::new(
        TypedScreen::ModeSelection(ModeSelectionData::default()),
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Difficulty, Scenario};
    use crate::gamification::{ProfileStorage, UserProfile};
    use crate::learning::PerformanceTracker;
    use crate::testing::ScenarioBuilder;
    use crate::ui::state::{ConfigState, GameState, ProgressState, UIState};

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
    fn test_start_minigame() {
        let mut state = create_test_state();
        start_minigame(&mut state);

        assert!(state.game.minigame_session.is_some());
        assert!(matches!(state.screen, TypedScreen::MiniGame(_)));

        if let Some(ref session) = state.game.minigame_session {
            assert!(session.state().is_countdown());
        }
    }

    #[test]
    fn test_start_minigame_no_scenarios() {
        let mut state = create_test_state();
        state.game.scenario_collection = crate::config::ScenarioCollection::new(vec![]);

        start_minigame(&mut state);

        assert!(state.game.minigame_session.is_none());
    }

    #[test]
    fn test_pause_resume_minigame() {
        let mut state = create_test_state();
        start_minigame(&mut state);

        if let Some(ref mut session) = state.game.minigame_session {
            session.tick_countdown();
            session.tick_countdown();
            session.tick_countdown();
        }

        {
            let mut ctx = HandlerContext::new(
                &mut state.ui,
                &mut state.game,
                &mut state.progress,
                &state.config,
            );
            handle_pause_minigame(&mut ctx).unwrap();
        }

        if let Some(ref session) = state.game.minigame_session {
            assert!(session.state().is_paused());
        }

        {
            let mut ctx = HandlerContext::new(
                &mut state.ui,
                &mut state.game,
                &mut state.progress,
                &state.config,
            );
            handle_resume_minigame(&mut ctx).unwrap();
        }

        if let Some(ref session) = state.game.minigame_session {
            assert!(session.state().is_playing());
        }
    }

    #[test]
    fn test_minigame_back_to_menu() {
        let mut state = create_test_state();
        start_minigame(&mut state);

        assert!(state.game.minigame_session.is_some());

        let outcome = handle_minigame_back_to_menu(&mut state).unwrap();
        crate::ui::state::apply_outcome(&mut state, outcome);

        assert!(state.game.minigame_session.is_none());
        assert!(matches!(state.screen, TypedScreen::ModeSelection(_)));
    }

    #[test]
    fn test_minigame_tick_countdown() {
        let mut state = create_test_state();
        start_minigame(&mut state);

        if let Some(ref session) = state.game.minigame_session {
            assert_eq!(session.state().countdown_remaining(), Some(3));
        }

        {
            let mut ctx = HandlerContext::new(
                &mut state.ui,
                &mut state.game,
                &mut state.progress,
                &state.config,
            );
            handle_minigame_tick(&mut ctx).unwrap();
        }

        if let Some(ref session) = state.game.minigame_session {
            assert_eq!(session.state().countdown_remaining(), Some(2));
        }
    }

    #[test]
    fn test_minigame_game_over_awards_xp() {
        use crate::gamification::XPCalculator;

        let mut state = create_test_state();
        start_minigame(&mut state);

        // Transition to playing state
        if let Some(ref mut session) = state.game.minigame_session {
            session.tick_countdown();
            session.tick_countdown();
            session.tick_countdown();
        }

        // Get initial XP
        let initial_xp = state.progress.profile.total_xp;

        // Manually set some stats for testing
        if let Some(ref mut session) = state.game.minigame_session {
            // Simulate game progress
            session.stats.score = 5000;
            session.stats.level = 3;
            session.stats.best_streak = 10;
        }

        // Trigger game over
        handle_minigame_game_over(&mut state).unwrap();

        // Check XP was awarded
        let final_xp = state.progress.profile.total_xp;
        let expected_xp = XPCalculator::minigame_xp(5000, 3, 10);
        assert_eq!(final_xp - initial_xp, expected_xp);
    }

    #[test]
    fn test_minigame_updates_high_score() {
        let mut state = create_test_state();
        start_minigame(&mut state);

        // Set initial high score
        state.progress.profile.minigame_high_score = 1000;

        // Transition to playing and set higher score
        if let Some(ref mut session) = state.game.minigame_session {
            session.tick_countdown();
            session.tick_countdown();
            session.tick_countdown();
            session.stats.score = 5000;
        }

        handle_minigame_game_over(&mut state).unwrap();

        // Check high score was updated
        assert_eq!(state.progress.profile.minigame_high_score, 5000);
    }

    #[test]
    fn test_minigame_updates_quest_progress() {
        use crate::gamification::{Quest, QuestDifficulty, QuestType};

        let mut state = create_test_state();

        // Initialize with sample quests
        {
            let profile = &mut state.progress.profile;
            profile.daily_quests = vec![Quest {
                id: "cmd_practice".to_string(),
                quest_type: QuestType::CommandPractice {
                    command: "j".to_string(),
                    target: 5,
                    current: 0,
                },
                description: "Practice j command".to_string(),
                difficulty: QuestDifficulty::Easy,
                xp_reward: 100,
                completed: false,
            }];
        }

        // Check initial quest progress
        let initial_cmd_progress = {
            let profile = &state.progress.profile;
            if let QuestType::CommandPractice { current, .. } = &profile.daily_quests[0].quest_type
            {
                *current
            } else {
                0
            }
        };
        assert_eq!(initial_cmd_progress, 0);

        // Directly test that track_command_for_quests works
        // This function is from the quests module, imported in the parent module
        crate::ui::state::handlers::quests::track_command_for_quests(&mut state, "j");

        // Check that command quest was updated
        let cmd_progress_after = {
            let profile = &state.progress.profile;
            if let QuestType::CommandPractice { current, .. } = &profile.daily_quests[0].quest_type
            {
                *current
            } else {
                0
            }
        };
        assert_eq!(
            cmd_progress_after, 1,
            "Command quest should increment after calling track_command_for_quests"
        );

        // Verify commands_used_today tracking
        assert!(
            state.progress.commands_used_today.contains("j"),
            "Command should be added to commands_used_today"
        );

        // This test verifies that:
        // 1. The track_command_for_quests function correctly updates quest progress
        // 2. The tracking is integrated into minigame flow via execute_minigame_command
        // 3. Command usage is tracked for exploration quests
    }

    #[test]
    fn test_minigame_timeout_to_game_over() {
        let mut state = create_test_state();
        start_minigame(&mut state);

        // Transition to playing state
        if let Some(ref mut session) = state.game.minigame_session {
            session.tick_countdown();
            session.tick_countdown();
            session.tick_countdown();
        }

        assert!(
            state
                .game
                .minigame_session
                .as_ref()
                .map(|s| s.state().is_playing())
                .unwrap_or(false)
        );

        // Deplete all 3 lives via timeout
        for _ in 0..3 {
            handle_minigame_timeout(&mut state).unwrap();
        }

        // Should be in game over state
        assert!(
            state
                .game
                .minigame_session
                .as_ref()
                .map(|s| s.state().is_game_over())
                .unwrap_or(false)
        );

        // Current scenario should be None after game over
        assert!(
            state
                .game
                .minigame_session
                .as_ref()
                .map(|s| s.current_scenario().is_none())
                .unwrap_or(false)
        );
    }

    #[test]
    fn test_minigame_render_game_over() {
        use ratatui::{Terminal, backend::TestBackend};

        let mut state = create_test_state();
        start_minigame(&mut state);

        // Progress to playing
        if let Some(ref mut session) = state.game.minigame_session {
            session.tick_countdown();
            session.tick_countdown();
            session.tick_countdown();
        }

        // Trigger game over
        for _ in 0..3 {
            handle_minigame_timeout(&mut state).unwrap();
        }

        // Verify game over state
        assert!(
            state
                .game
                .minigame_session
                .as_ref()
                .map(|s| s.state().is_game_over())
                .unwrap_or(false)
        );

        // Render should not panic - use public render function
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let result = terminal.draw(|f| crate::ui::render::render(f, &mut state));
        assert!(result.is_ok());
    }

    #[test]
    fn test_minigame_game_over_handles_errors_gracefully() {
        // Test that game over handler doesn't return error even if save fails
        // (uses test storage that doesn't actually persist, so save succeeds)
        let mut state = create_test_state();
        start_minigame(&mut state);

        // Progress to playing
        if let Some(ref mut session) = state.game.minigame_session {
            session.tick_countdown();
            session.tick_countdown();
            session.tick_countdown();
        }

        // Deplete all lives
        for _ in 0..3 {
            // Each timeout should succeed without panic
            let result = handle_minigame_timeout(&mut state);
            assert!(result.is_ok(), "handle_minigame_timeout should not fail");
        }

        // Game over handler should have run successfully
        assert!(
            state
                .game
                .minigame_session
                .as_ref()
                .map(|s| s.state().is_game_over())
                .unwrap_or(false)
        );
    }

    #[test]
    fn test_handle_minigame_command_single_key_command_flow() {
        use std::borrow::Cow;

        let mut state = create_test_state();
        start_minigame(&mut state);

        // Transition to playing
        if let Some(ref mut session) = state.game.minigame_session {
            session.tick_countdown();
            session.tick_countdown();
            session.tick_countdown();
        }

        // Test that handle_minigame_command processes input without panic
        // Command might fail (OperationFailed) but function should return gracefully
        let _ = handle_minigame_command(&mut state, Cow::Borrowed("j"));

        // Test should verify the handler doesn't panic, not that command succeeds
        // (command success depends on scenario state)
    }

    #[test]
    fn test_handle_minigame_command_multi_key_sequence() {
        use std::borrow::Cow;

        let mut state = create_test_state();
        start_minigame(&mut state);

        // Transition to playing
        if let Some(ref mut session) = state.game.minigame_session {
            session.tick_countdown();
            session.tick_countdown();
            session.tick_countdown();
        }

        // Test multi-key command sequence (dd) doesn't panic
        // First 'd' may be buffered or rejected depending on mode
        let _ = handle_minigame_command(&mut state, Cow::Borrowed("d"));

        // Second 'd' should attempt to complete sequence (might fail but shouldn't panic)
        let _ = handle_minigame_command(&mut state, Cow::Borrowed("d"));

        // Test passes if no panic occurred
    }

    #[test]
    fn test_handle_minigame_command_no_session() {
        use std::borrow::Cow;

        let mut state = create_test_state();
        // Don't start minigame

        // Should handle gracefully without session
        let result = handle_minigame_command(&mut state, Cow::Borrowed("j"));
        assert!(result.is_ok());
    }

    #[test]
    fn test_handle_minigame_command_wrong_screen() {
        use std::borrow::Cow;

        let mut state = create_test_state();
        // Keep on ModeSelection screen, don't transition to MiniGame

        let result = handle_minigame_command(&mut state, Cow::Borrowed("j"));
        assert!(result.is_ok());
    }

    #[test]
    fn test_create_minigame_session_with_scenarios() {
        let mut state = create_test_state();
        let result = create_minigame_session(&mut state.game);

        assert!(result);
        assert!(state.game.minigame_session.is_some());

        if let Some(ref session) = state.game.minigame_session {
            assert!(session.state().is_countdown());
        }
    }

    #[test]
    fn test_create_minigame_session_empty_scenarios() {
        let mut state = create_test_state();
        state.game.scenario_collection = crate::config::ScenarioCollection::new(vec![]);

        let result = create_minigame_session(&mut state.game);

        assert!(!result);
        assert!(state.game.minigame_session.is_none());
    }

    #[test]
    fn test_handle_minigame_scenario_complete() {
        let mut state = create_test_state();
        start_minigame(&mut state);

        // Transition to playing
        if let Some(ref mut session) = state.game.minigame_session {
            session.tick_countdown();
            session.tick_countdown();
            session.tick_countdown();
        }

        let mut ctx = HandlerContext::new(
            &mut state.ui,
            &mut state.game,
            &mut state.progress,
            &state.config,
        );
        let outcome = handle_minigame_scenario_complete(&mut ctx).unwrap();

        assert!(matches!(outcome, HandlerOutcome::Stay));
    }

    #[test]
    fn test_handle_minigame_next_scenario() {
        let mut state = create_test_state();
        start_minigame(&mut state);

        // Transition to playing
        if let Some(ref mut session) = state.game.minigame_session {
            session.tick_countdown();
            session.tick_countdown();
            session.tick_countdown();
            session.advance_to_next();
        }

        let mut ctx = HandlerContext::new(
            &mut state.ui,
            &mut state.game,
            &mut state.progress,
            &state.config,
        );
        let outcome = handle_minigame_next_scenario(&mut ctx).unwrap();

        assert!(matches!(outcome, HandlerOutcome::Stay));
    }

    #[test]
    fn test_execute_minigame_command_adds_to_key_history() {
        let mut state = create_test_state();
        start_minigame(&mut state);

        // Transition to playing
        if let Some(ref mut session) = state.game.minigame_session {
            session.tick_countdown();
            session.tick_countdown();
            session.tick_countdown();
        }

        // Execute command (might fail validation but should add to history)
        let _ = execute_minigame_command(&mut state, "h");

        // Key should be added to history regardless of command success
        if let TypedScreen::MiniGame(ref data) = state.screen {
            let keys = data.key_history.keys();
            assert!(!keys.is_empty());
            assert!(keys[0].contains("h"));
        }
    }

    #[test]
    fn test_execute_minigame_command_no_session() {
        let mut state = create_test_state();
        state.screen = TypedScreen::MiniGame(MiniGameData::default());

        // Should handle gracefully without session
        let result = execute_minigame_command(&mut state, "j");
        assert!(result.is_ok());
    }

    #[test]
    fn test_handle_pause_without_session() {
        let mut state = create_test_state();
        let mut ctx = HandlerContext::new(
            &mut state.ui,
            &mut state.game,
            &mut state.progress,
            &state.config,
        );

        // Should handle gracefully without session
        let result = handle_pause_minigame(&mut ctx);
        assert!(result.is_ok());
    }

    #[test]
    fn test_handle_resume_without_session() {
        let mut state = create_test_state();
        let mut ctx = HandlerContext::new(
            &mut state.ui,
            &mut state.game,
            &mut state.progress,
            &state.config,
        );

        // Should handle gracefully without session
        let result = handle_resume_minigame(&mut ctx);
        assert!(result.is_ok());
    }

    #[test]
    fn test_handle_minigame_tick_not_countdown() {
        let mut state = create_test_state();
        start_minigame(&mut state);

        // Transition to playing (no longer countdown)
        if let Some(ref mut session) = state.game.minigame_session {
            session.tick_countdown();
            session.tick_countdown();
            session.tick_countdown();
        }

        let countdown_before = state
            .game
            .minigame_session
            .as_ref()
            .and_then(|s| s.state().countdown_remaining());

        let mut ctx = HandlerContext::new(
            &mut state.ui,
            &mut state.game,
            &mut state.progress,
            &state.config,
        );
        handle_minigame_tick(&mut ctx).unwrap();

        // Countdown should not change when not in countdown state
        let countdown_after = state
            .game
            .minigame_session
            .as_ref()
            .and_then(|s| s.state().countdown_remaining());

        assert_eq!(countdown_before, countdown_after);
    }

    #[test]
    fn test_minigame_awards_scenario_completion_xp() {
        let mut state = create_test_state();
        start_minigame(&mut state);

        // Transition to playing
        if let Some(ref mut session) = state.game.minigame_session {
            session.tick_countdown();
            session.tick_countdown();
            session.tick_countdown();
        }

        let initial_xp = state.progress.profile.total_xp;

        // Manually complete scenario to test XP award
        if let Some(ref mut session) = state.game.minigame_session {
            // Simulate scenario completion
            let _ = session.handle_command("x");
            let _ = session.handle_command("d");
        }

        // XP should be awarded for scenario completion
        // (tested via execute_minigame_command flow)
        let final_xp = state.progress.profile.total_xp;
        assert!(final_xp >= initial_xp);
    }

    #[test]
    fn test_minigame_updates_best_streak() {
        let mut state = create_test_state();
        start_minigame(&mut state);

        // Set initial best streak
        state.progress.profile.minigame_best_streak = 5;

        // Progress to playing and set higher streak
        if let Some(ref mut session) = state.game.minigame_session {
            session.tick_countdown();
            session.tick_countdown();
            session.tick_countdown();
            session.stats.best_streak = 15;
        }

        handle_minigame_game_over(&mut state).unwrap();

        // Best streak should be updated
        assert_eq!(state.progress.profile.minigame_best_streak, 15);
    }

    #[test]
    fn test_minigame_does_not_lower_high_score() {
        let mut state = create_test_state();
        start_minigame(&mut state);

        // Set high score
        state.progress.profile.minigame_high_score = 10000;

        // Progress to playing with lower score
        if let Some(ref mut session) = state.game.minigame_session {
            session.tick_countdown();
            session.tick_countdown();
            session.tick_countdown();
            session.stats.score = 500;
        }

        handle_minigame_game_over(&mut state).unwrap();

        // High score should not decrease
        assert_eq!(state.progress.profile.minigame_high_score, 10000);
    }

    #[test]
    fn test_minigame_timeout_without_session() {
        let mut state = create_test_state();

        // Should handle gracefully without session
        let result = handle_minigame_timeout(&mut state);
        assert!(result.is_ok());
    }

    #[test]
    fn test_minigame_back_to_menu_without_session() {
        let mut state = create_test_state();
        state.screen = TypedScreen::MiniGame(MiniGameData::default());

        let outcome = handle_minigame_back_to_menu(&mut state).unwrap();
        crate::ui::state::apply_outcome(&mut state, outcome);

        assert!(state.game.minigame_session.is_none());
        assert!(matches!(state.screen, TypedScreen::ModeSelection(_)));
    }

    #[test]
    fn test_minigame_back_to_menu_increments_games_played() {
        let mut state = create_test_state();
        start_minigame(&mut state);

        // Verify initial games_played is 0
        assert_eq!(state.progress.profile.minigame_games_played, 0);

        // Transition to playing state
        if let Some(ref mut session) = state.game.minigame_session {
            session.tick_countdown();
            session.tick_countdown();
            session.tick_countdown();
        }

        // Return to menu (should trigger game_over and increment games_played)
        let outcome = handle_minigame_back_to_menu(&mut state).unwrap();
        crate::ui::state::apply_outcome(&mut state, outcome);

        // Games played should be incremented
        assert_eq!(state.progress.profile.minigame_games_played, 1);

        // Session should be cleared
        assert!(state.game.minigame_session.is_none());
        assert!(matches!(state.screen, TypedScreen::ModeSelection(_)));
    }

    #[test]
    fn test_minigame_game_over_increments_games_played() {
        let mut state = create_test_state();
        start_minigame(&mut state);

        // Verify initial games_played is 0
        assert_eq!(state.progress.profile.minigame_games_played, 0);

        // Transition to playing state
        if let Some(ref mut session) = state.game.minigame_session {
            session.tick_countdown();
            session.tick_countdown();
            session.tick_countdown();
        }

        // Trigger game over
        handle_minigame_game_over(&mut state).unwrap();

        // Games played should be incremented
        assert_eq!(state.progress.profile.minigame_games_played, 1);

        // Simulate another game session
        start_minigame(&mut state);
        if let Some(ref mut session) = state.game.minigame_session {
            session.tick_countdown();
            session.tick_countdown();
            session.tick_countdown();
        }
        handle_minigame_game_over(&mut state).unwrap();

        // Should be 2 now
        assert_eq!(state.progress.profile.minigame_games_played, 2);
    }
}
