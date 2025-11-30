//! Message handlers for mini-game mode (Arcade Mode)

use crate::config::Scenario;
use crate::minigame::MiniGameSession;
use crate::security::UserError;
use crate::ui::state::{AppState, MiniGameData, ModeSelectionData, TypedScreen};
use std::sync::Arc;

/// Handle starting a mini-game session
pub(in crate::ui::state) fn handle_start_minigame(state: &mut AppState) -> Result<(), UserError> {
    // Navigate to mini-game screen first (renderer will show error if no scenarios)
    state.screen = TypedScreen::MiniGame(MiniGameData::default());

    // Create mini-game session with available scenarios
    let scenarios: Vec<Scenario> = state
        .game
        .scenario_collection
        .get_filtered()
        .into_iter()
        .cloned()
        .collect();

    if scenarios.is_empty() {
        tracing::warn!("No scenarios available for mini-game");
        // Still navigated to screen, renderer will show error
        return Ok(());
    }

    let session = MiniGameSession::new(Arc::new(scenarios));
    state.game.minigame_session = Some(session);

    // Start the game (begins countdown)
    if let Some(ref mut session) = state.game.minigame_session {
        session.start();
    }

    Ok(())
}

/// Handle pausing mini-game
pub(in crate::ui::state) fn handle_pause_minigame(state: &mut AppState) -> Result<(), UserError> {
    if let Some(ref mut session) = state.game.minigame_session {
        session.pause();
    }
    Ok(())
}

/// Handle resuming mini-game
pub(in crate::ui::state) fn handle_resume_minigame(state: &mut AppState) -> Result<(), UserError> {
    if let Some(ref mut session) = state.game.minigame_session {
        session.resume();
    }
    Ok(())
}

/// Handle mini-game timer tick (100ms)
///
/// Checks for timeouts and updates countdown.
pub(in crate::ui::state) fn handle_minigame_tick(state: &mut AppState) -> Result<(), UserError> {
    if let Some(ref mut session) = state.game.minigame_session {
        // Tick countdown if in countdown state
        if session.state().is_countdown() {
            session.tick_countdown();
        }
    }
    Ok(())
}

/// Execute a command in the mini-game session
///
/// Handles command execution, quest progress updates, and completion detection.
/// Uses shared quest tracking functions from quests module.
fn execute_minigame_command(state: &mut AppState, command: &str) -> Result<(), UserError> {
    // Record key to history for display
    if let TypedScreen::MiniGame(ref mut data) = state.screen {
        data.add_key_to_history(command.to_string());
    }

    // Snapshot quest completion status before updates
    let was_completed = super::snapshot_quest_completion(state);

    let Some(ref mut session) = state.game.minigame_session else {
        return Ok(());
    };

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
                let mut tracker = state.progress.performance_tracker.borrow_mut();
                session.record_to_fsrs(&mut tracker, true); // Success!
                drop(tracker);
            }

            // Update quest progress for scenario completion (shared function)
            let duration = scenario.elapsed();
            let scenario_id = scenario.scenario.id.clone();
            super::track_scenario_completion_for_quests(state, &scenario_id, duration);
        }

        // Award XP for scenario completion in arcade mode
        // Base: 15 XP per scenario + 2 XP per streak level (encourages maintaining streaks)
        let scenario_xp = 15 + (current_streak.saturating_sub(1) * 2) as u64;
        {
            let mut profile = state.progress.profile.borrow_mut();
            profile.add_xp(scenario_xp);
        }

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
/// Uses command buffer to handle multi-key commands (dd, gg, rx).
pub(in crate::ui::state) fn handle_minigame_command(
    state: &mut AppState,
    command: std::borrow::Cow<'static, str>,
) -> Result<(), UserError> {
    use crate::ui::state::CommandBufferAccess;

    // Get minigame data for command buffer
    let TypedScreen::MiniGame(ref mut minigame_data) = state.screen else {
        return Ok(());
    };

    // Check if we're in insert mode (send directly without buffering)
    let is_insert_mode = state
        .game
        .minigame_session
        .as_ref()
        .map(|s| s.is_insert_mode())
        .unwrap_or(false);

    if is_insert_mode {
        return execute_minigame_command(state, &command);
    }

    // Normal mode: handle command buffer for multi-key commands
    minigame_data.push_command(&command);

    // Try to match a complete command
    let final_command = super::gameplay::parse_command_buffer(minigame_data.command_buffer());

    match final_command {
        Some("") => {
            // Invalid sequence - clear buffer
            minigame_data.clear_buffer();
            Ok(())
        }
        Some(cmd) => {
            // Complete command - execute it
            let cmd_string = cmd.to_string();
            minigame_data.clear_buffer();
            execute_minigame_command(state, &cmd_string)
        }
        None => {
            // Waiting for more keys - nothing to do
            Ok(())
        }
    }
}

/// Handle timeout on current mini-game scenario
pub(in crate::ui::state) fn handle_minigame_timeout(state: &mut AppState) -> Result<(), UserError> {
    if let Some(ref mut session) = state.game.minigame_session {
        session.handle_timeout();

        // Check if game is over
        if session.state().is_game_over() {
            // Process game over (calculate XP, save progress)
            handle_minigame_game_over(state)?;
            // Game over - stay on screen to show final score
            // User must press a key to return to menu
        }
    }
    Ok(())
}

/// Handle scenario completion (user triggered)
pub(in crate::ui::state) fn handle_minigame_scenario_complete(
    state: &mut AppState,
) -> Result<(), UserError> {
    if let Some(ref mut session) = state.game.minigame_session {
        session.advance_to_next();
    }
    Ok(())
}

/// Handle advancing to next scenario (after transition delay)
///
/// If scenario loading fails (e.g., empty queue), logs error but continues.
/// This prevents crash when scenario pool is exhausted.
pub(in crate::ui::state) fn handle_minigame_next_scenario(
    state: &mut AppState,
) -> Result<(), UserError> {
    if let Some(ref mut session) = state.game.minigame_session
        && let Err(e) = session.complete_transition()
    {
        // Log error but don't crash - user can still see current state
        tracing::warn!("Failed to load next mini-game scenario: {:?}", e);
        // The game will continue in its current state
    }
    Ok(())
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
        let mut tracker = state.progress.performance_tracker.borrow_mut();
        session.record_to_fsrs(&mut tracker, false); // Game over = failure on current scenario
        drop(tracker);

        // 2. Calculate XP earned
        use crate::gamification::XPCalculator;
        let xp = XPCalculator::minigame_xp(stats.score, stats.level, stats.best_streak);

        // 3. Update profile with XP and high scores
        let mut profile = state.progress.profile.borrow_mut();
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

        drop(profile);

        // 4. Persist profile to disk (non-fatal error - log but continue)
        let save_result = {
            let profile_borrowed = state.progress.profile.borrow();
            state.progress.storage.save(&profile_borrowed)
        };

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
///
/// Awards XP for current progress before exiting.
pub(in crate::ui::state) fn handle_minigame_back_to_menu(
    state: &mut AppState,
) -> Result<(), UserError> {
    // Award XP for progress before exiting (if session exists)
    if state.game.minigame_session.is_some() {
        handle_minigame_game_over(state)?;
    }

    // Clear mini-game session
    state.game.minigame_session = None;

    // Navigate to mode selection
    state.screen = TypedScreen::ModeSelection(ModeSelectionData::default());

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
    use crate::ui::state::{ConfigState, GameState, ProgressState, UIState};

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
    fn test_start_minigame() {
        let mut state = create_test_state();

        handle_start_minigame(&mut state).unwrap();

        assert!(state.game.minigame_session.is_some());
        assert!(matches!(state.screen, TypedScreen::MiniGame(_)));

        // Session should be in countdown state
        if let Some(ref session) = state.game.minigame_session {
            assert!(session.state().is_countdown());
        }
    }

    #[test]
    fn test_start_minigame_no_scenarios() {
        let mut state = create_test_state();
        state.game.scenario_collection = crate::config::ScenarioCollection::new(vec![]);

        handle_start_minigame(&mut state).unwrap();

        // Should not create session without scenarios
        assert!(state.game.minigame_session.is_none());
    }

    #[test]
    fn test_pause_resume_minigame() {
        let mut state = create_test_state();
        handle_start_minigame(&mut state).unwrap();

        // Transition to playing state
        if let Some(ref mut session) = state.game.minigame_session {
            session.tick_countdown();
            session.tick_countdown();
            session.tick_countdown();
        }

        handle_pause_minigame(&mut state).unwrap();

        if let Some(ref session) = state.game.minigame_session {
            assert!(session.state().is_paused());
        }

        handle_resume_minigame(&mut state).unwrap();

        if let Some(ref session) = state.game.minigame_session {
            assert!(session.state().is_playing());
        }
    }

    #[test]
    fn test_minigame_back_to_menu() {
        let mut state = create_test_state();
        handle_start_minigame(&mut state).unwrap();

        assert!(state.game.minigame_session.is_some());

        handle_minigame_back_to_menu(&mut state).unwrap();

        assert!(state.game.minigame_session.is_none());
        assert!(matches!(state.screen, TypedScreen::ModeSelection(_)));
    }

    #[test]
    fn test_minigame_tick_countdown() {
        let mut state = create_test_state();
        handle_start_minigame(&mut state).unwrap();

        // Should be in countdown with 3 remaining
        if let Some(ref session) = state.game.minigame_session {
            assert_eq!(session.state().countdown_remaining(), Some(3));
        }

        handle_minigame_tick(&mut state).unwrap();

        if let Some(ref session) = state.game.minigame_session {
            assert_eq!(session.state().countdown_remaining(), Some(2));
        }
    }

    #[test]
    fn test_minigame_game_over_awards_xp() {
        use crate::gamification::XPCalculator;

        let mut state = create_test_state();
        handle_start_minigame(&mut state).unwrap();

        // Transition to playing state
        if let Some(ref mut session) = state.game.minigame_session {
            session.tick_countdown();
            session.tick_countdown();
            session.tick_countdown();
        }

        // Get initial XP
        let initial_xp = state.progress.profile.borrow().total_xp;

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
        let final_xp = state.progress.profile.borrow().total_xp;
        let expected_xp = XPCalculator::minigame_xp(5000, 3, 10);
        assert_eq!(final_xp - initial_xp, expected_xp);
    }

    #[test]
    fn test_minigame_updates_high_score() {
        let mut state = create_test_state();
        handle_start_minigame(&mut state).unwrap();

        // Set initial high score
        state.progress.profile.borrow_mut().minigame_high_score = 1000;

        // Transition to playing and set higher score
        if let Some(ref mut session) = state.game.minigame_session {
            session.tick_countdown();
            session.tick_countdown();
            session.tick_countdown();
            session.stats.score = 5000;
        }

        handle_minigame_game_over(&mut state).unwrap();

        // Check high score was updated
        assert_eq!(state.progress.profile.borrow().minigame_high_score, 5000);
    }

    // TODO: Add integration test for quest updates
    // This requires a properly loaded scenario with valid state transitions

    #[test]
    fn test_minigame_timeout_to_game_over() {
        let mut state = create_test_state();
        handle_start_minigame(&mut state).unwrap();

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
        handle_start_minigame(&mut state).unwrap();

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
        handle_start_minigame(&mut state).unwrap();

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
}
