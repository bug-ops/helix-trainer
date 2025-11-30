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

/// Handle executing a Helix command during mini-game
pub(in crate::ui::state) fn handle_minigame_command(
    state: &mut AppState,
    command: std::borrow::Cow<'static, str>,
) -> Result<(), UserError> {
    if let Some(ref mut session) = state.game.minigame_session {
        session.handle_command(&command)?;

        // Update quest progress for command used
        let mut profile = state.progress.profile.borrow_mut();
        crate::gamification::QuestTracker::update_command_progress(&mut profile.daily_quests, &command);
        drop(profile);

        // Track commands used today
        state
            .progress
            .commands_used_today
            .insert(command.to_string());

        // Check for completion
        if session.check_completion() {
            // Record to FSRS before advancing (only if we have actions)
            if let Some(scenario) = session.current_scenario() {
                if !scenario.actions().is_empty() {
                    let mut tracker = state.progress.performance_tracker.borrow_mut();
                    session.record_to_fsrs(&mut tracker, true); // Success!
                    drop(tracker);
                }

                // Update quest progress for scenario completion
                let duration = scenario.elapsed();
                let scenario_id = scenario.scenario.id.clone();

                let mut profile = state.progress.profile.borrow_mut();
                crate::gamification::QuestTracker::update_scenario_progress(
                    &mut profile.daily_quests,
                    &scenario_id,
                    duration,
                );
                drop(profile);
            }

            session.advance_to_next();
            // Transition state will be handled by timer
        }
    }
    Ok(())
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
pub(in crate::ui::state) fn handle_minigame_next_scenario(
    state: &mut AppState,
) -> Result<(), UserError> {
    if let Some(ref mut session) = state.game.minigame_session {
        session.complete_transition()?;
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
/// - Saves profile to disk
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

        // 4. Persist profile to disk
        let profile_borrowed = state.progress.profile.borrow();
        state
            .progress
            .storage
            .save(&profile_borrowed)
            .map_err(|_| UserError::OperationFailed)?;
        drop(profile_borrowed);

        state.progress.mark_saved();
    }

    Ok(())
}

/// Handle returning to mode selection from mini-game
pub(in crate::ui::state) fn handle_minigame_back_to_menu(
    state: &mut AppState,
) -> Result<(), UserError> {
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
}
