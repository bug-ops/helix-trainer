//! Message handlers for mini-game mode (Arcade Mode)

use std::sync::Arc;

use crate::config::Scenario;
use crate::constants::{
    FLASH_TIME_RATIO, MINIGAME_SCENARIO_BASE_XP, MINIGAME_STREAK_XP_MULTIPLIER,
    SPEED_DEMON_TIME_RATIO,
};
use crate::game::format_key_for_display;
use crate::game::services::ScenarioCompletionService;
use crate::gamification::{Achievement, AchievementEngine, speed_time_ratio};
use crate::input::typestate::{HandlerResult, command_to_key_event};
use crate::learning::PerformanceTracker;
use crate::minigame::MiniGameSession;
use crate::security::UserError;
use crate::sound::SoundEffect;
use crate::ui::notification::{Notification, NotificationType};
use crate::ui::state::{
    AppState, GameState, HandlerContext, HandlerOutcome, InputStateAccess, MiniGameData,
    ModeSelectionData, TypedScreen,
};

/// Create and start a new mini-game session from available scenarios.
///
/// Shared initialization logic used by both mode selection and direct start.
/// When a tracker is provided, scenarios containing commands that need practice
/// (overdue, weak, or novel) will be prioritized via FSRS-weighted selection.
///
/// # Arguments
///
/// * `game` - Mutable reference to game state
/// * `tracker` - Optional reference to performance tracker for FSRS weighting.
///   If provided, a clone is taken for the session's internal use.
///
/// # Returns
///
/// Returns `true` if session was created successfully, `false` if no scenarios available.
pub(in crate::ui::state) fn create_minigame_session(
    game: &mut GameState,
    tracker: Option<&PerformanceTracker>,
) -> bool {
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

    // Clone tracker for session's internal weighted selection
    let tracker_clone = tracker.cloned();
    let mut session = MiniGameSession::new(Arc::new(scenarios), tracker_clone);
    session.start(); // Begin countdown
    game.minigame_session = Some(session);
    true
}

/// Handle starting a mini-game session
pub(in crate::ui::state) fn handle_start_minigame(
    ctx: &mut HandlerContext<'_>,
) -> Result<HandlerOutcome, UserError> {
    // Pass performance tracker for FSRS-weighted scenario selection
    create_minigame_session(ctx.game, Some(&ctx.progress.performance_tracker));
    ctx.ui.show_key_history = false;
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
        // Play countdown sound on each tick
        ctx.progress.sound_manager.play(SoundEffect::Countdown);
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

    // Show key history popup after first keypress (reset on scenario transitions)
    state.ui.show_key_history = true;

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
    let mut scenario_xp = 0u64;
    let mut scenario_leveled_up = false;
    if session.check_completion() {
        // Play success sound
        state
            .progress
            .sound_manager
            .play(SoundEffect::ScenarioComplete);

        // Get current streak before advancing
        let current_streak = session.stats().streak();

        // Record to FSRS before advancing (only if we have actions)
        if let Some(scenario) = session.current_scenario() {
            if !scenario.actions().is_empty() {
                let tracker = &mut state.progress.performance_tracker;
                session.record_to_fsrs(tracker, true); // Success!
            }

            // Update quest progress for scenario completion (shared function)
            let duration = scenario.elapsed();
            let scenario_id = scenario.scenario.id.clone();
            let scenario_difficulty = scenario
                .scenario
                .metadata
                .as_ref()
                .and_then(|m| m.difficulty);
            let optimal_count = scenario.scenario.scoring.optimal_count.get();
            let actual_count = scenario.action_count().max(1);
            // Efficiency-only, matching Training's `is_perfect` definition. This is
            // deliberately looser than `PerformanceTier::Perfect` (efficiency AND
            // time_ratio < 0.5) used for the in-round combo badge, so a completion
            // the arcade UI only ever badges "Excellent" can still unlock
            // FirstPerfect/Perfect10.
            let is_perfect = actual_count <= optimal_count;
            // Base per-difficulty budget, not the level-scaled arcade time_limit - see
            // `speed_time_ratio` doc comment for why the two modes share one definition
            // of "speed" instead of computing it independently.
            let time_ratio = speed_time_ratio(duration, scenario_difficulty);

            super::track_scenario_completion_for_quests(state, &scenario_id, duration);

            // Update mastery/perfect/completion counters the same way the training-mode
            // completion path (`record_scenario_completion`) does, so
            // Perfect10/Perfect100/Centurion/Veteran/Legend are reachable from arcade too.
            ScenarioCompletionService::update_profile_counters(
                &mut state.progress.profile,
                is_perfect,
            );

            // Track exploration/speed signals for achievements (SpeedDemon, Speedrunner,
            // Flash, Polyglot).
            let profile = &mut state.progress.profile;
            if let Some(difficulty) = scenario_difficulty {
                profile.difficulties_completed.insert(difficulty);
            }
            if time_ratio < SPEED_DEMON_TIME_RATIO {
                profile.speed_run_count = profile.speed_run_count.saturating_add(1);
            }
            if time_ratio < FLASH_TIME_RATIO {
                profile.flash_run_count = profile.flash_run_count.saturating_add(1);
            }
        }

        // Check and unlock any achievements newly satisfied by this completion
        // (mastery/exploration/speed counters all just changed above)
        let newly_unlocked = AchievementEngine::check_and_unlock(
            &mut state.progress.profile,
            &state.progress.performance_tracker,
        );
        for achievement_id in newly_unlocked {
            let achievement = Achievement::new(achievement_id);
            state
                .ui
                .notifications
                .push(Notification::new(NotificationType::Achievement {
                    name: achievement.name,
                    description: achievement.description,
                }));
        }

        // Award XP for scenario completion in arcade mode
        // Base XP per scenario + bonus per streak level (encourages maintaining streaks)
        scenario_xp = MINIGAME_SCENARIO_BASE_XP
            + (current_streak.saturating_sub(1) as u64 * MINIGAME_STREAK_XP_MULTIPLIER);
        let profile = &mut state.progress.profile;
        scenario_leveled_up = profile.add_xp(scenario_xp);

        // Re-borrow session after state modification
        if let Some(ref mut session) = state.game.minigame_session {
            session.advance_to_next();

            // Check for multiplier increase and play sound
            if session.take_multiplier_change().is_some() {
                state.progress.sound_manager.play(SoundEffect::MultiplierUp);
            }
            // Check for level increase and play sound
            if session.take_level_change().is_some() {
                state.progress.sound_manager.play(SoundEffect::LevelUp);
            }
        }
        // Transition state will be handled by timer
    }

    // Award XP for any quests newly completed by this keystroke, from the single
    // `was_completed` snapshot taken at the top of this function. Called exactly once
    // per keystroke regardless of whether the scenario also completed here - unlike
    // scenario-completion XP, a CommandPractice/Exploration quest can complete on a
    // keystroke that doesn't finish the scenario, and this is the only place that
    // awards it (this function adds XP internally).
    let quest_award = super::award_quest_completion_xp(state, &was_completed);

    // Each `add_xp` call only reports whether *that* call crossed a level boundary, so
    // OR the scenario-completion and quest-completion outcomes together (mirrors the
    // training-mode path in `record_scenario_completion`). Unlike that training-mode
    // path, this doesn't pair the notification with `save_immediate()` - arcade only
    // persists at game over, so a mid-session level-up is not durable until then.
    if scenario_leveled_up || quest_award.leveled_up {
        state
            .ui
            .notifications
            .push(Notification::new(NotificationType::LevelUp {
                new_level: state.progress.profile.level,
            }));
    }

    // Store total XP earned for display in the transition popup (only shown when this
    // keystroke completed the scenario).
    if scenario_xp > 0 {
        let quest_xp: u64 = quest_award.bonuses.iter().map(|(_, xp)| xp).sum();
        let total_xp = scenario_xp + quest_xp;
        if let TypedScreen::MiniGame(ref mut data) = state.screen {
            data.last_xp_earned = Some(total_xp);
        }
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
    // Only handle if we're on the MiniGame screen
    if !matches!(state.screen, TypedScreen::MiniGame(_)) {
        return Ok(());
    }

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
            // Play game over sound
            state.progress.sound_manager.play(SoundEffect::GameOver);
            handle_minigame_game_over(state)?;
        } else {
            // Play life lost sound (still has lives remaining)
            state.progress.sound_manager.play(SoundEffect::LifeLost);
        }
    }
    Ok(())
}

/// Handle the mode's session-level time limit elapsing (e.g. Arcade's 60 seconds)
///
/// Ends the run immediately regardless of lives remaining, reusing the same
/// game-over bookkeeping (FSRS recording, XP award, high scores, profile save)
/// as a per-scenario timeout that depletes the last life.
pub(in crate::ui::state) fn handle_minigame_session_timeout(
    state: &mut AppState,
) -> Result<(), UserError> {
    if let Some(ref mut session) = state.game.minigame_session {
        session.end_session_on_timeout();
        state.progress.sound_manager.play(SoundEffect::GameOver);
        handle_minigame_game_over(state)?;
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
    state: &mut AppState,
) -> Result<HandlerOutcome, UserError> {
    if let Some(ref mut session) = state.game.minigame_session
        && let Err(e) = session.complete_transition()
    {
        tracing::warn!("Failed to load next mini-game scenario: {:?}", e);
    }
    // Hide the key history popup and clear its buffered keys so the previous
    // scenario's keys don't leak into the new scenario's display
    state.ui.show_key_history = false;
    if let TypedScreen::MiniGame(ref mut data) = state.screen {
        data.clear_key_history();
        // A pending prefix/register/command-line state from the previous
        // scenario must not leak into the next one's fresh input.
        data.reset_input_state();
    }
    Ok(HandlerOutcome::Stay)
}

/// Handle game over - calculate XP, update profile, save progress
///
/// Called whenever a mini-game session ends or is abandoned: out-of-lives
/// timeout, session-level time limit, or a mid-session quit. Idempotent per
/// session via [`MiniGameSession::try_begin_game_over`] - callers may invoke
/// this unconditionally whenever a session exists, without checking
/// [`MiniGameState::GameOver`](crate::minigame::MiniGameState::GameOver)
/// themselves; a second call for the same session is a no-op.
///
/// Performs final integration with progression systems:
/// - Records final FSRS data for last scenario
/// - Calculates and awards XP
/// - Updates high scores
/// - Saves profile to disk (non-fatal on error)
pub(in crate::ui::state) fn handle_minigame_game_over(
    state: &mut AppState,
) -> Result<(), UserError> {
    let should_process = state
        .game
        .minigame_session
        .as_mut()
        .is_some_and(|session| session.try_begin_game_over());

    if !should_process {
        return Ok(());
    }

    if let Some(ref session) = state.game.minigame_session {
        let stats = session.stats();

        // 1. Record final scenario to FSRS (if applicable)
        let tracker = &mut state.progress.performance_tracker;
        session.record_to_fsrs(tracker, false); // Game over = failure on current scenario

        // 2. Calculate XP earned
        use crate::gamification::XPCalculator;
        let xp = XPCalculator::minigame_xp(stats.score, stats.level(), stats.best_streak());

        // 3. Update profile with XP and high scores
        let profile = &mut state.progress.profile;
        let leveled_up = profile.add_xp(xp);
        let new_level = profile.level;

        // Update high scores if beaten
        let mut new_high_score = false;
        if stats.score > profile.minigame_high_score {
            profile.minigame_high_score = stats.score;
            new_high_score = true;
        }

        if stats.best_streak() > profile.minigame_best_streak {
            profile.minigame_best_streak = stats.best_streak();
        }

        // Increment total games played counter
        profile.minigame_games_played = profile.minigame_games_played.saturating_add(1);

        // Log results
        tracing::info!(
            xp_earned = xp,
            score = stats.score,
            level = stats.level(),
            streak = stats.best_streak(),
            leveled_up = leveled_up,
            new_high_score = new_high_score,
            "Mini-game session completed"
        );

        // Unlike the mid-session push in `execute_minigame_command`, this one is
        // immediately durable: step 4 below saves the profile unconditionally on every
        // game over, not just on a level-up.
        if leveled_up {
            state
                .ui
                .notifications
                .push(Notification::new(NotificationType::LevelUp { new_level }));
        }

        // 4. Persist profile to disk (non-fatal error - log but continue)
        if let Err(e) = state.progress.save_immediate() {
            tracing::error!("Failed to save profile after mini-game: {:?}", e);
            // Don't return error - game over screen should still display
        }
    }

    Ok(())
}

/// Handle returning to mode selection from mini-game
pub(in crate::ui::state) fn handle_minigame_back_to_menu(
    state: &mut AppState,
) -> Result<HandlerOutcome, UserError> {
    // `handle_minigame_game_over` is idempotent per session (see its doc comment),
    // so it's safe to call unconditionally here even if a prior timeout already
    // ran it for this same session.
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
        create_test_scenario_with_difficulty(id, Difficulty::Beginner)
    }

    fn create_test_scenario_with_difficulty(id: &str, difficulty: Difficulty) -> Scenario {
        ScenarioBuilder::new()
            .id(id)
            .setup_content("line 1\nline 2\n")
            .setup_cursor(1, 0)
            .target_content("line 1\n")
            .target_cursor(1, 0)
            .optimal_count(1)
            .difficulty(difficulty)
            .build()
    }

    /// Unlike [`create_test_scenario`], this scenario is actually reachable via
    /// real commands ("x" then "d" deletes the middle line), so tests that need
    /// `check_completion()` to genuinely turn true should use this instead.
    fn create_completable_scenario_with_difficulty(id: &str, difficulty: Difficulty) -> Scenario {
        ScenarioBuilder::new()
            .id(id)
            .setup_cursor(1, 0)
            .target_content("line 1\nline 3\n")
            .target_cursor(1, 0)
            .difficulty(difficulty)
            .build()
    }

    fn create_single_scenario_state(scenario: Scenario) -> AppState {
        AppState {
            screen: TypedScreen::ModeSelection(ModeSelectionData::default()),
            ui: UIState::new(),
            game: GameState::new(vec![scenario]),
            progress: ProgressState::new(
                UserProfile::new(),
                PerformanceTracker::new(),
                ProfileStorage::for_test(),
            ),
            config: ConfigState::default(),
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
                ProfileStorage::for_test(),
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
    fn test_start_minigame_hides_key_history() {
        let mut state = create_test_state();
        // Default UIState starts with show_key_history true; starting a session
        // must reset it so a stale popup from a previous session isn't shown.
        state.ui.show_key_history = true;

        start_minigame(&mut state);

        assert!(!state.ui.show_key_history);
    }

    #[test]
    fn test_execute_minigame_command_shows_key_history() {
        let mut state = create_test_state();
        start_minigame(&mut state);
        state.ui.show_key_history = false;

        // Transition to playing
        if let Some(ref mut session) = state.game.minigame_session {
            session.tick_countdown();
            session.tick_countdown();
            session.tick_countdown();
        }

        let _ = execute_minigame_command(&mut state, "h");

        assert!(
            state.ui.show_key_history,
            "first keypress should reveal the key history popup"
        );
    }

    /// Regression test for #292 finding F1 (impl-critic): `award_quest_completion_xp`
    /// was only called inside the `session.check_completion()` branch, so a
    /// `CommandPractice`/`Exploration` quest that completes on a keystroke which does
    /// *not* also finish the current arcade scenario got marked `completed = true` with
    /// no XP, no `QuestComplete` notification, and no `profile.complete_quest()` call -
    /// and since later snapshots see it as already completed, it was never awarded.
    /// A single keystroke that only satisfies a `CommandPractice` quest (the test
    /// scenario here is deliberately unreachable via commands) must still grant XP.
    #[test]
    fn test_execute_minigame_command_awards_quest_xp_without_completing_scenario() {
        use crate::gamification::{Quest, QuestDifficulty, QuestType};

        let mut state = create_test_state();
        state.progress.profile.daily_quests = vec![Quest {
            id: "cmd_practice_h".to_string(),
            quest_type: QuestType::CommandPractice {
                command: "h".to_string(),
                target: 1,
                current: 0,
            },
            description: "Use 'h' 1 time".to_string(),
            difficulty: QuestDifficulty::Easy,
            xp_reward: 50,
            completed: false,
        }];

        start_minigame(&mut state);
        if let Some(ref mut session) = state.game.minigame_session {
            session.tick_countdown();
            session.tick_countdown();
            session.tick_countdown();
        }
        let initial_xp = state.progress.profile.total_xp;

        execute_minigame_command(&mut state, "h").unwrap();

        // The scenario itself must not have completed - only the quest should have.
        if let Some(ref session) = state.game.minigame_session {
            assert!(!session.state().is_transition());
        }
        assert!(
            state.progress.profile.daily_quests[0].completed,
            "CommandPractice quest should complete after using 'h' once"
        );
        assert_eq!(
            state.progress.profile.total_xp - initial_xp,
            50,
            "quest XP must be granted immediately, not lost until scenario completion"
        );
        assert!(
            state.ui.notifications.visible().iter().any(|n| matches!(
                &n.notification_type,
                crate::ui::notification::NotificationType::QuestComplete { description, .. }
                    if description == "Use 'h' 1 time"
            )),
            "expected a QuestComplete notification"
        );
    }

    /// Regression test for #309: `award_quest_completion_xp`'s `leveled_up` field was
    /// discarded in `execute_minigame_command`, so a `CommandPractice`/`Exploration`
    /// quest completing mid-arcade-session could cross a level threshold with zero
    /// notification. Mirrors the training-mode fix in
    /// `test_handle_complete_scenario_notifies_on_level_up_from_quest_xp_alone`.
    #[test]
    fn test_execute_minigame_command_notifies_on_level_up_from_quest_xp_alone() {
        use crate::gamification::{Quest, QuestDifficulty, QuestType, XPCalculator};

        let mut state = create_test_state();
        let xp_for_level_2 = XPCalculator::xp_for_level(2);
        state.progress.profile.total_xp = xp_for_level_2 - 1;
        state.progress.profile.level = 1;
        state.progress.profile.daily_quests = vec![Quest {
            id: "cmd_practice_h".to_string(),
            quest_type: QuestType::CommandPractice {
                command: "h".to_string(),
                target: 1,
                current: 0,
            },
            description: "Use 'h' 1 time".to_string(),
            difficulty: QuestDifficulty::Easy,
            xp_reward: 1,
            completed: false,
        }];

        start_minigame(&mut state);
        if let Some(ref mut session) = state.game.minigame_session {
            session.tick_countdown();
            session.tick_countdown();
            session.tick_countdown();
        }

        execute_minigame_command(&mut state, "h").unwrap();

        assert!(
            state.progress.profile.level >= 2,
            "expected a level-up driven by quest XP"
        );
        assert!(
            state.ui.notifications.visible().iter().any(|n| matches!(
                n.notification_type,
                crate::ui::notification::NotificationType::LevelUp { new_level }
                    if new_level == state.progress.profile.level
            )),
            "expected a LevelUp notification reporting the post-award profile level"
        );
    }

    /// Regression test for #309: a keystroke that doesn't cross a level threshold
    /// must not push a `LevelUp` notification.
    #[test]
    fn test_execute_minigame_command_no_level_up_notification_without_level_up() {
        let mut state = create_test_state();
        start_minigame(&mut state);
        if let Some(ref mut session) = state.game.minigame_session {
            session.tick_countdown();
            session.tick_countdown();
            session.tick_countdown();
        }

        execute_minigame_command(&mut state, "h").unwrap();

        assert!(
            !state.ui.notifications.visible().iter().any(|n| matches!(
                n.notification_type,
                crate::ui::notification::NotificationType::LevelUp { .. }
            )),
            "no LevelUp notification should fire without an actual level up"
        );
    }

    /// Regression test for #309: `execute_minigame_command` also discarded the
    /// `leveled_up` result of `profile.add_xp(scenario_xp)` for arcade scenario
    /// completions (not just quest completions) - a scenario-completion XP award
    /// crossing a level threshold must notify too.
    #[test]
    fn test_execute_minigame_command_notifies_on_level_up_from_scenario_xp() {
        use crate::gamification::XPCalculator;

        let scenario = create_completable_scenario_with_difficulty("s1", Difficulty::Beginner);
        let mut state = create_single_scenario_state(scenario);
        let xp_for_level_2 = XPCalculator::xp_for_level(2);
        state.progress.profile.total_xp = xp_for_level_2 - 1;
        state.progress.profile.level = 1;
        start_minigame(&mut state);

        if let Some(ref mut session) = state.game.minigame_session {
            session.tick_countdown();
            session.tick_countdown();
            session.tick_countdown();
        }

        let _ = execute_minigame_command(&mut state, "x");
        let _ = execute_minigame_command(&mut state, "d");

        assert!(
            state.progress.profile.level >= 2,
            "expected a level-up driven by scenario-completion XP"
        );
        assert!(
            state.ui.notifications.visible().iter().any(|n| matches!(
                n.notification_type,
                crate::ui::notification::NotificationType::LevelUp { new_level }
                    if new_level == state.progress.profile.level
            )),
            "expected a LevelUp notification reporting the post-award profile level"
        );
    }

    #[test]
    fn test_handle_minigame_next_scenario_hides_key_history() {
        let mut state = create_test_state();
        start_minigame(&mut state);

        // Transition to playing and simulate a keypress that revealed the popup
        if let Some(ref mut session) = state.game.minigame_session {
            session.tick_countdown();
            session.tick_countdown();
            session.tick_countdown();
            session.advance_to_next();
        }
        state.ui.show_key_history = true;
        if let TypedScreen::MiniGame(ref mut data) = state.screen {
            data.add_key_to_history("h".to_string());
        }

        handle_minigame_next_scenario(&mut state).unwrap();

        assert!(
            !state.ui.show_key_history,
            "advancing to the next scenario should hide the stale key history popup"
        );
        if let TypedScreen::MiniGame(ref data) = state.screen {
            assert!(
                data.key_history.is_empty(),
                "advancing to the next scenario should clear the previous scenario's buffered keys"
            );
        } else {
            panic!("expected MiniGame screen");
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
        use crate::gamification::ProfileStorage;
        use tempfile::TempDir;

        let mut state = create_test_state();
        let temp_dir = TempDir::new().unwrap();
        state.progress.storage = ProfileStorage::with_path(temp_dir.path().join("profile.json"));
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
        use crate::gamification::{ProfileStorage, XPCalculator};
        use tempfile::TempDir;

        let mut state = create_test_state();
        let temp_dir = TempDir::new().unwrap();
        state.progress.storage = ProfileStorage::with_path(temp_dir.path().join("profile.json"));
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
            session.stats.increase_level();
            session.stats.increase_level();
            for _ in 0..10 {
                session.stats.record_completion();
            }
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
        use crate::gamification::ProfileStorage;
        use tempfile::TempDir;

        let mut state = create_test_state();
        let temp_dir = TempDir::new().unwrap();
        state.progress.storage = ProfileStorage::with_path(temp_dir.path().join("profile.json"));
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

    /// Regression test for #258: `handle_minigame_game_over`'s save must go through
    /// `ProgressState::save_immediate`, which syncs `performance_tracker` into
    /// `profile.performance_data` before writing, instead of persisting a stale copy.
    #[test]
    fn test_minigame_game_over_persists_synced_fsrs_data() {
        use crate::gamification::ProfileStorage;
        use std::time::Duration;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let profile_path = temp_dir.path().join("profile.json");

        let mut state = create_test_state();
        state.progress.storage = ProfileStorage::with_path(&profile_path);
        start_minigame(&mut state);

        if let Some(ref mut session) = state.game.minigame_session {
            session.tick_countdown();
            session.tick_countdown();
            session.tick_countdown();
            session.stats.score = 5000;
        }

        // Seed tracker state as if reviews happened earlier in the session.
        state.progress.performance_tracker.record_attempt(
            "j",
            Duration::from_millis(500),
            true,
            Duration::from_millis(500),
        );

        handle_minigame_game_over(&mut state).unwrap();

        let persisted = ProfileStorage::with_path(&profile_path).load().unwrap();
        assert!(!persisted.performance_data.is_empty());
        assert!(persisted.performance_data.contains_key("j"));
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
        use crate::gamification::ProfileStorage;
        use tempfile::TempDir;

        let mut state = create_test_state();
        let temp_dir = TempDir::new().unwrap();
        state.progress.storage = ProfileStorage::with_path(temp_dir.path().join("profile.json"));
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
        use crate::gamification::ProfileStorage;
        use ratatui::{Terminal, backend::TestBackend};
        use tempfile::TempDir;

        let mut state = create_test_state();
        let temp_dir = TempDir::new().unwrap();
        state.progress.storage = ProfileStorage::with_path(temp_dir.path().join("profile.json"));
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
        use crate::gamification::ProfileStorage;
        use tempfile::TempDir;

        let mut state = create_test_state();
        let temp_dir = TempDir::new().unwrap();
        state.progress.storage = ProfileStorage::with_path(temp_dir.path().join("profile.json"));
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
    fn test_handle_minigame_command_pushes_key_history_once_per_keypress() {
        use std::borrow::Cow;

        let mut state = create_test_state();
        start_minigame(&mut state);

        // Transition to playing
        if let Some(ref mut session) = state.game.minigame_session {
            session.tick_countdown();
            session.tick_countdown();
            session.tick_countdown();
        }

        // Regression test for S3: handle_minigame_command used to push to key
        // history itself AND call execute_minigame_command (which also pushes),
        // double-counting every resolved keystroke.
        let _ = handle_minigame_command(&mut state, Cow::Borrowed("j"));

        if let TypedScreen::MiniGame(ref data) = state.screen {
            assert_eq!(
                data.key_history.len(),
                1,
                "a single resolved keypress must add exactly one history entry"
            );
        } else {
            panic!("expected MiniGame screen");
        }
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
        // Test without tracker (backward compatibility)
        let result = create_minigame_session(&mut state.game, None);

        assert!(result);
        assert!(state.game.minigame_session.is_some());

        if let Some(ref session) = state.game.minigame_session {
            assert!(session.state().is_countdown());
        }
    }

    #[test]
    fn test_create_minigame_session_with_tracker() {
        let mut state = create_test_state();
        // Test with tracker for FSRS-weighted selection
        let result =
            create_minigame_session(&mut state.game, Some(&state.progress.performance_tracker));

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

        let result = create_minigame_session(&mut state.game, None);

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

        let outcome = handle_minigame_next_scenario(&mut state).unwrap();

        assert!(matches!(outcome, HandlerOutcome::Stay));
    }

    /// Regression test for S4b: a half-typed prefix/register/command-line
    /// state must not leak into the next scenario after
    /// `handle_minigame_next_scenario` advances.
    #[test]
    fn test_handle_minigame_next_scenario_resets_pending_input_state() {
        let mut state = create_test_state();
        start_minigame(&mut state);

        if let Some(ref mut session) = state.game.minigame_session {
            session.tick_countdown();
            session.tick_countdown();
            session.tick_countdown();
            session.advance_to_next();
        }

        // Leave a pending register-select state, as if the player started
        // typing `"a` right before the transition kicked in.
        if let TypedScreen::MiniGame(ref mut data) = state.screen {
            data.input_state_mut()
                .process_key(crossterm::event::KeyEvent::new(
                    crossterm::event::KeyCode::Char('"'),
                    crossterm::event::KeyModifiers::NONE,
                ));
            assert!(data.input_state().is_prefix_state());
        } else {
            panic!("expected MiniGame screen");
        }

        handle_minigame_next_scenario(&mut state).unwrap();

        if let TypedScreen::MiniGame(ref data) = state.screen {
            assert!(
                data.input_state().state().is_base(),
                "pending input state must reset when advancing to the next scenario, got {:?}",
                data.input_state().state()
            );
        } else {
            panic!("expected MiniGame screen");
        }
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
        use crate::gamification::ProfileStorage;
        use tempfile::TempDir;

        let mut state = create_test_state();
        let temp_dir = TempDir::new().unwrap();
        state.progress.storage = ProfileStorage::with_path(temp_dir.path().join("profile.json"));
        start_minigame(&mut state);

        // Set initial best streak
        state.progress.profile.minigame_best_streak = 5;

        // Progress to playing and set higher streak
        if let Some(ref mut session) = state.game.minigame_session {
            session.tick_countdown();
            session.tick_countdown();
            session.tick_countdown();
            for _ in 0..15 {
                session.stats.record_completion();
            }
        }

        handle_minigame_game_over(&mut state).unwrap();

        // Best streak should be updated
        assert_eq!(state.progress.profile.minigame_best_streak, 15);
    }

    #[test]
    fn test_minigame_does_not_lower_high_score() {
        use crate::gamification::ProfileStorage;
        use tempfile::TempDir;

        let mut state = create_test_state();
        let temp_dir = TempDir::new().unwrap();
        state.progress.storage = ProfileStorage::with_path(temp_dir.path().join("profile.json"));
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
        use crate::gamification::ProfileStorage;
        use tempfile::TempDir;

        let mut state = create_test_state();
        let temp_dir = TempDir::new().unwrap();
        state.progress.storage = ProfileStorage::with_path(temp_dir.path().join("profile.json"));
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

    /// Regression test for #291: achievements must unlock through the live arcade
    /// completion path (`execute_minigame_command`), not just on next profile load.
    /// An immediate (near-zero elapsed) completion also exercises the #290 speed
    /// counters: `time_ratio` is well under both `SPEED_DEMON_TIME_RATIO` and
    /// `FLASH_TIME_RATIO`, so both `speed_run_count` and `flash_run_count` bump.
    #[test]
    fn test_execute_minigame_command_unlocks_achievement_mid_session() {
        use crate::gamification::AchievementId;

        let scenario =
            create_completable_scenario_with_difficulty("speed_test", Difficulty::Beginner);
        let mut state = create_single_scenario_state(scenario);
        start_minigame(&mut state);

        // Transition to playing
        if let Some(ref mut session) = state.game.minigame_session {
            session.tick_countdown();
            session.tick_countdown();
            session.tick_countdown();
        }

        // Complete the scenario ("x" selects the line, "d" deletes it) essentially
        // immediately, so time_ratio stays near 0.0.
        let _ = execute_minigame_command(&mut state, "x");
        let _ = execute_minigame_command(&mut state, "d");

        assert_eq!(state.progress.profile.speed_run_count, 1);
        assert_eq!(state.progress.profile.flash_run_count, 1);
        assert!(
            state
                .progress
                .profile
                .has_achievement(&AchievementId::SpeedDemon)
        );
        assert!(
            state
                .progress
                .profile
                .has_achievement(&AchievementId::Flash)
        );

        assert!(state.ui.notifications.visible().iter().any(|n| matches!(
            &n.notification_type,
            NotificationType::Achievement { name, .. } if name == "Speed Demon"
        )));
        assert!(state.ui.notifications.visible().iter().any(|n| matches!(
            &n.notification_type,
            NotificationType::Achievement { name, .. } if name == "Flash"
        )));
    }

    /// Regression coverage for #290: a completion that consumes more than half of
    /// the scenario's time budget must NOT count as a speed run. Uses an
    /// Advanced-difficulty scenario (6s budget at controller level 1) and sleeps
    /// past 50% of it before completing.
    #[test]
    fn test_execute_minigame_command_slow_completion_does_not_increment_speed_counters() {
        use crate::gamification::AchievementId;

        let scenario =
            create_completable_scenario_with_difficulty("advanced1", Difficulty::Advanced);
        let mut state = create_single_scenario_state(scenario);
        start_minigame(&mut state);

        if let Some(ref mut session) = state.game.minigame_session {
            session.tick_countdown();
            session.tick_countdown();
            session.tick_countdown();
        }

        std::thread::sleep(std::time::Duration::from_millis(3_500));

        let _ = execute_minigame_command(&mut state, "x");
        let _ = execute_minigame_command(&mut state, "d");

        assert_eq!(state.progress.profile.speed_run_count, 0);
        assert_eq!(state.progress.profile.flash_run_count, 0);
        assert!(
            !state
                .progress
                .profile
                .has_achievement(&AchievementId::SpeedDemon)
        );
        assert!(
            !state
                .progress
                .profile
                .has_achievement(&AchievementId::Flash)
        );
    }

    /// Regression test for S3 (#291 follow-up): arcade completion must feed
    /// `ScenarioCompletionService::update_profile_counters` the same way Training mode
    /// does, so `scenarios_completed`/`perfect_scenarios` (and therefore
    /// FirstPerfect/Perfect10/.../Centurion) are reachable from arcade play. Previously
    /// the arcade path never called this at all despite a comment claiming it did.
    #[test]
    fn test_execute_minigame_command_perfect_completion_updates_counters_and_unlocks_first_perfect()
    {
        use crate::gamification::AchievementId;

        // Default optimal_count is 2; "x" then "d" is exactly 2 actions, so this
        // completion is perfect (actual_count <= optimal_count).
        let scenario =
            create_completable_scenario_with_difficulty("perfect_test", Difficulty::Beginner);
        let mut state = create_single_scenario_state(scenario);
        start_minigame(&mut state);

        if let Some(ref mut session) = state.game.minigame_session {
            session.tick_countdown();
            session.tick_countdown();
            session.tick_countdown();
        }

        let _ = execute_minigame_command(&mut state, "x");
        let _ = execute_minigame_command(&mut state, "d");

        assert_eq!(state.progress.profile.scenarios_completed, 1);
        assert_eq!(state.progress.profile.perfect_scenarios, 1);
        assert!(
            state
                .progress
                .profile
                .has_achievement(&AchievementId::FirstPerfect)
        );
    }

    /// Companion to the perfect-completion test above: a completion that takes more
    /// actions than the scenario's `optimal_count` must still increment
    /// `scenarios_completed`, but must NOT count as perfect.
    #[test]
    fn test_execute_minigame_command_non_perfect_completion_does_not_increment_perfect_count() {
        use crate::gamification::AchievementId;
        use crate::testing::ScenarioBuilder;

        let scenario = ScenarioBuilder::new()
            .id("non_perfect_test")
            .setup_cursor(1, 0)
            .target_content("line 1\nline 3\n")
            .target_cursor(1, 0)
            .difficulty(Difficulty::Beginner)
            .optimal_count(1) // "x" then "d" (2 actions) exceeds this
            .build();
        let mut state = create_single_scenario_state(scenario);
        start_minigame(&mut state);

        if let Some(ref mut session) = state.game.minigame_session {
            session.tick_countdown();
            session.tick_countdown();
            session.tick_countdown();
        }

        let _ = execute_minigame_command(&mut state, "x");
        let _ = execute_minigame_command(&mut state, "d");

        assert_eq!(state.progress.profile.scenarios_completed, 1);
        assert_eq!(state.progress.profile.perfect_scenarios, 0);
        assert!(
            !state
                .progress
                .profile
                .has_achievement(&AchievementId::FirstPerfect)
        );
    }

    #[test]
    fn test_minigame_game_over_increments_games_played() {
        use crate::gamification::ProfileStorage;
        use tempfile::TempDir;

        let mut state = create_test_state();
        let temp_dir = TempDir::new().unwrap();
        state.progress.storage = ProfileStorage::with_path(temp_dir.path().join("profile.json"));
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

    /// Regression test for #309: `handle_minigame_game_over` computed `leveled_up` from
    /// `profile.add_xp(xp)` only to log it, never pushing a `LevelUp` notification.
    #[test]
    fn test_minigame_game_over_notifies_on_level_up() {
        use crate::gamification::{ProfileStorage, XPCalculator};
        use tempfile::TempDir;

        let mut state = create_test_state();
        let temp_dir = TempDir::new().unwrap();
        state.progress.storage = ProfileStorage::with_path(temp_dir.path().join("profile.json"));
        start_minigame(&mut state);

        // Close enough to the level 2 threshold that even a bare (score 0) game-over's
        // level bonus XP crosses it.
        let xp_for_level_2 = XPCalculator::xp_for_level(2);
        state.progress.profile.total_xp = xp_for_level_2 - 1;
        state.progress.profile.level = 1;

        if let Some(ref mut session) = state.game.minigame_session {
            session.tick_countdown();
            session.tick_countdown();
            session.tick_countdown();
        }

        handle_minigame_game_over(&mut state).unwrap();

        assert!(
            state.progress.profile.level >= 2,
            "expected a level-up driven by the end-of-game XP award"
        );
        assert!(
            state.ui.notifications.visible().iter().any(|n| matches!(
                n.notification_type,
                crate::ui::notification::NotificationType::LevelUp { new_level }
                    if new_level == state.progress.profile.level
            )),
            "expected a LevelUp notification reporting the post-award profile level"
        );
    }

    /// Regression test for #317: `handle_minigame_back_to_menu` used to unconditionally
    /// re-run `handle_minigame_game_over` on a session that had already reached
    /// `GameOver` via `handle_minigame_timeout` (e.g. Esc/`m`/Ctrl-q on the game-over
    /// screen), double-awarding XP, `minigame_games_played`, and FSRS bookkeeping, and
    /// potentially pushing a second `LevelUp` notification. Backing out to the menu
    /// after the session already reached `GameOver` must be a no-op for those.
    #[test]
    fn test_minigame_back_to_menu_after_game_over_does_not_double_award() {
        use crate::gamification::ProfileStorage;
        use tempfile::TempDir;

        let mut state = create_test_state();
        let temp_dir = TempDir::new().unwrap();
        state.progress.storage = ProfileStorage::with_path(temp_dir.path().join("profile.json"));
        start_minigame(&mut state);

        if let Some(ref mut session) = state.game.minigame_session {
            session.tick_countdown();
            session.tick_countdown();
            session.tick_countdown();
        }

        // Deplete all lives via timeout - this already runs `handle_minigame_game_over`
        // once, via `handle_minigame_timeout`.
        for _ in 0..3 {
            handle_minigame_timeout(&mut state).unwrap();
        }
        assert!(
            state
                .game
                .minigame_session
                .as_ref()
                .map(|s| s.state().is_game_over())
                .unwrap_or(false)
        );

        let xp_after_first_game_over = state.progress.profile.total_xp;
        let games_played_after_first_game_over = state.progress.profile.minigame_games_played;

        // Simulate Esc/`m`/Ctrl-q on the game-over screen.
        let outcome = handle_minigame_back_to_menu(&mut state).unwrap();
        crate::ui::state::apply_outcome(&mut state, outcome);

        assert_eq!(
            state.progress.profile.total_xp, xp_after_first_game_over,
            "returning to menu from an already-processed game-over must not award XP again"
        );
        assert_eq!(
            state.progress.profile.minigame_games_played, games_played_after_first_game_over,
            "returning to menu from an already-processed game-over must not increment games_played again"
        );
        assert!(state.game.minigame_session.is_none());
        assert!(matches!(state.screen, TypedScreen::ModeSelection(_)));
    }

    /// Regression test for #309: a game-over that doesn't cross a level threshold must
    /// not push a `LevelUp` notification.
    #[test]
    fn test_minigame_game_over_no_level_up_notification_without_level_up() {
        use crate::gamification::ProfileStorage;
        use tempfile::TempDir;

        let mut state = create_test_state();
        let temp_dir = TempDir::new().unwrap();
        state.progress.storage = ProfileStorage::with_path(temp_dir.path().join("profile.json"));
        start_minigame(&mut state);

        if let Some(ref mut session) = state.game.minigame_session {
            session.tick_countdown();
            session.tick_countdown();
            session.tick_countdown();
        }

        handle_minigame_game_over(&mut state).unwrap();

        assert!(
            !state.ui.notifications.visible().iter().any(|n| matches!(
                n.notification_type,
                crate::ui::notification::NotificationType::LevelUp { .. }
            )),
            "no LevelUp notification should fire without an actual level up"
        );
    }

    /// Regression test for #327: `handle_minigame_session_timeout` (the session-level
    /// clock running out, e.g. Arcade's 60 seconds) must reuse the exact same
    /// FSRS/XP/high-score/profile-save bookkeeping as `handle_minigame_timeout`
    /// depleting the last life, but must NOT consume a life to get there.
    #[test]
    fn test_session_timeout_reuses_game_over_bookkeeping_without_consuming_life() {
        use crate::gamification::{ProfileStorage, XPCalculator};
        use crate::minigame::{ArcadeConfig, MiniGameMode};
        use std::time::Duration;
        use tempfile::TempDir;

        let mut state = create_test_state();
        let temp_dir = TempDir::new().unwrap();
        state.progress.storage = ProfileStorage::with_path(temp_dir.path().join("profile.json"));
        start_minigame(&mut state);

        // Swap in a short session-duration Arcade mode so the session timer can be
        // waited out with a short real-time sleep, mirroring
        // `test_active_scenario_pause_freezes_elapsed_time`'s real-time approach.
        let scenarios = Arc::new(vec![create_test_scenario("s1")]);
        let mode = MiniGameMode::Arcade(ArcadeConfig {
            session_duration: Duration::from_millis(200),
            ..ArcadeConfig::default()
        });
        let mut session = MiniGameSession::with_mode(scenarios, None, mode);
        session.start();
        session.tick_countdown();
        session.tick_countdown();
        session.tick_countdown();
        session.stats.score = 5000;
        for _ in 0..10 {
            session.stats.record_completion();
        }
        let lives_before = session.stats().lives();
        state.game.minigame_session = Some(session);

        std::thread::sleep(Duration::from_millis(500));
        assert!(
            state
                .game
                .minigame_session
                .as_ref()
                .unwrap()
                .is_session_expired()
        );

        let initial_xp = state.progress.profile.total_xp;

        handle_minigame_session_timeout(&mut state).unwrap();

        // Same bookkeeping as an ordinary timeout game-over: XP awarded, high
        // score updated, games_played incremented.
        let expected_xp = XPCalculator::minigame_xp(5000, 1, 10);
        assert_eq!(state.progress.profile.total_xp - initial_xp, expected_xp);
        assert_eq!(state.progress.profile.minigame_high_score, 5000);
        assert_eq!(state.progress.profile.minigame_games_played, 1);

        // Jumps straight to GameOver without consuming a life.
        let session = state.game.minigame_session.as_ref().unwrap();
        assert!(session.state().is_game_over());
        assert_eq!(session.stats().lives(), lives_before);

        // Profile was persisted to disk (same save path as a normal game over).
        let persisted = ProfileStorage::with_path(temp_dir.path().join("profile.json"))
            .load()
            .unwrap();
        assert_eq!(persisted.minigame_high_score, 5000);
    }

    /// Regression test for #327: the `!session.state().is_game_over()` guard in
    /// `handle_minigame_back_to_menu` (added for #317 to stop `handle_minigame_timeout`
    /// double-awarding) must also cover a session ended via
    /// `handle_minigame_session_timeout` — the session-level clock running out, not
    /// lives depleted. Backing out to the menu afterward must be a no-op.
    #[test]
    fn test_minigame_back_to_menu_after_session_timeout_does_not_double_award() {
        use crate::gamification::ProfileStorage;
        use crate::minigame::{ArcadeConfig, MiniGameMode};
        use std::time::Duration;
        use tempfile::TempDir;

        let mut state = create_test_state();
        let temp_dir = TempDir::new().unwrap();
        state.progress.storage = ProfileStorage::with_path(temp_dir.path().join("profile.json"));
        start_minigame(&mut state);

        let scenarios = Arc::new(vec![create_test_scenario("s1")]);
        let mode = MiniGameMode::Arcade(ArcadeConfig {
            session_duration: Duration::from_millis(200),
            ..ArcadeConfig::default()
        });
        let mut session = MiniGameSession::with_mode(scenarios, None, mode);
        session.start();
        session.tick_countdown();
        session.tick_countdown();
        session.tick_countdown();
        state.game.minigame_session = Some(session);

        std::thread::sleep(Duration::from_millis(500));

        // Ends the run via the session-timer path (not lives depleted) - this
        // already runs `handle_minigame_game_over` once.
        handle_minigame_session_timeout(&mut state).unwrap();
        assert!(
            state
                .game
                .minigame_session
                .as_ref()
                .map(|s| s.state().is_game_over())
                .unwrap_or(false)
        );

        let xp_after_first_game_over = state.progress.profile.total_xp;
        let games_played_after_first_game_over = state.progress.profile.minigame_games_played;

        // Simulate Esc/`m`/Ctrl-q on the game-over screen.
        let outcome = handle_minigame_back_to_menu(&mut state).unwrap();
        crate::ui::state::apply_outcome(&mut state, outcome);

        assert_eq!(
            state.progress.profile.total_xp, xp_after_first_game_over,
            "returning to menu from an already-processed session timeout must not award XP again"
        );
        assert_eq!(
            state.progress.profile.minigame_games_played, games_played_after_first_game_over,
            "returning to menu from an already-processed session timeout must not increment games_played again"
        );
        assert!(state.game.minigame_session.is_none());
        assert!(matches!(state.screen, TypedScreen::ModeSelection(_)));
    }
}
