//! Message handlers for mini-game mode (Arcade Mode)

use std::sync::Arc;

use super::SessionInput;
use super::key_history;
use crate::config::Scenario;
use crate::game::services::ScenarioCompletionService;
use crate::gamification::XPCalculator;
use crate::helix::ModeKind;
use crate::input::resolve::{Resolution, resolve_keys};
use crate::learning::PerformanceTracker;
use crate::minigame::MiniGameSession;
use crate::security::UserError;
use crate::sound::SoundEffect;
use crate::ui::notification::{Notification, NotificationType};
use crate::ui::state::Effect;
use crate::ui::state::{
    AppState, GameState, HandlerContext, HandlerOutcome, InputStateAccess, MiniGameData,
    ModeSelectionData, TypedScreen,
};

/// Create and start a new mini-game session from available scenarios.
///
/// When a tracker is provided, scenarios containing commands that need practice
/// (overdue, weak, or novel) will be prioritized via FSRS-weighted selection.
///
/// # Arguments
///
/// * `game` - Game state holding the (filtered) scenario collection
/// * `tracker` - Optional reference to performance tracker for FSRS weighting.
///   If provided, a clone is taken for the session's internal use.
///
/// # Returns
///
/// The started session, or `None` if no scenarios are available.
pub(in crate::ui::state) fn create_minigame_session(
    game: &GameState,
    tracker: Option<&PerformanceTracker>,
) -> Option<MiniGameSession> {
    let scenarios: Vec<Scenario> = game
        .scenario_collection
        .filtered()
        .into_iter()
        .cloned()
        .collect();

    if scenarios.is_empty() {
        return None;
    }

    let mut session = MiniGameSession::new(Arc::new(scenarios), tracker.cloned());
    session.start(); // Begin countdown
    Some(session)
}

/// Handle pausing mini-game
pub(in crate::ui::state) fn handle_pause_minigame(data: &mut MiniGameData) -> HandlerOutcome {
    data.session.pause();
    HandlerOutcome::Stay
}

/// Handle resuming mini-game
pub(in crate::ui::state) fn handle_resume_minigame(data: &mut MiniGameData) -> HandlerOutcome {
    data.session.resume();
    HandlerOutcome::Stay
}

/// Handle mini-game timer tick (100ms)
pub(in crate::ui::state) fn handle_minigame_tick(
    data: &mut MiniGameData,
    ctx: &mut HandlerContext<'_>,
) -> HandlerOutcome {
    if data.session.state().is_countdown() {
        // Play countdown sound on each tick
        ctx.ui.effects.push(Effect::Play(SoundEffect::Countdown));
        data.session.tick_countdown();
    }
    HandlerOutcome::Stay
}

/// Execute a command in the mini-game session
///
/// Handles command execution, quest progress updates, and completion detection.
/// Uses shared quest tracking functions from quests module.
///
fn execute_minigame_command(
    state: &mut AppState,
    input: SessionInput<'_>,
) -> Result<(), UserError> {
    // Snapshot quest completion status before updates
    let was_completed = super::snapshot_quest_completion(state);

    let Some(session) = state.minigame_session_mut() else {
        return Ok(());
    };

    match input {
        SessionInput::Command(command) => session.apply_command(command)?,
        SessionInput::Text(text) => session.handle_command(text)?,
    }

    // Update quest progress for command used (shared function)
    super::track_command_for_quests(state, &input.text());

    // Re-borrow session after state modification
    let Some(session) = state
        .screen
        .minigame_data_mut()
        .map(|data| &mut *data.session)
    else {
        return Ok(());
    };

    // Check for completion
    let mut scenario_xp = 0u64;
    let mut scenario_leveled_up = false;
    if session.check_completion() {
        // Play success sound
        state
            .ui
            .effects
            .push(Effect::Play(SoundEffect::ScenarioComplete));

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
            let scenario_difficulty = scenario.scenario.metadata.difficulty;
            let optimal_count = scenario.scenario.scoring.optimal_count.get();
            let actual_count = scenario.action_count().max(1);
            // Efficiency-only, matching Training's `is_perfect` definition. This is
            // deliberately looser than `PerformanceTier::Perfect` (efficiency AND
            // time_ratio < 0.5) used for the in-round combo badge, so a completion
            // the arcade UI only ever badges "Excellent" can still unlock
            // FirstPerfect/Perfect10.
            let is_perfect = actual_count <= optimal_count;

            super::track_scenario_completion_for_quests(state, &scenario_id, duration);

            // Update mastery/perfect/completion counters the same way the training-mode
            // completion path (`record_scenario_completion`) does, so
            // Perfect10/Perfect100/Centurion/Veteran/Legend are reachable from arcade too.
            ScenarioCompletionService::update_profile_counters(
                &mut state.progress.profile,
                is_perfect,
            );

            // Track exploration/speed signals for achievements (SpeedDemon, Speedrunner,
            // Flash, Polyglot). Base per-difficulty budget, not the level-scaled arcade
            // time_limit - see `speed_time_ratio`'s doc comment for why the two modes
            // share one definition of "speed" instead of computing it independently.
            ScenarioCompletionService::track_speed_and_difficulty(
                &mut state.progress.profile,
                duration,
                scenario_difficulty,
            );
        }

        // Check and unlock any achievements newly satisfied by this completion
        // (mastery/exploration/speed counters all just changed above)
        for event in ScenarioCompletionService::check_and_notify_achievements(
            &mut state.progress.profile,
            &state.progress.performance_tracker,
        ) {
            state.ui.notifications.push(event.into());
        }

        // Award XP for scenario completion in arcade mode
        // Base XP per scenario + bonus per streak level (encourages maintaining streaks)
        scenario_xp = XPCalculator::arcade_scenario_xp(current_streak);
        let profile = &mut state.progress.profile;
        scenario_leveled_up = profile.add_xp(scenario_xp);

        // Re-borrow session after state modification
        if let Some(session) = state
            .screen
            .minigame_data_mut()
            .map(|data| &mut *data.session)
        {
            session.advance_to_next();

            // Check for multiplier increase and play sound
            if session.take_multiplier_change().is_some() {
                state
                    .ui
                    .effects
                    .push(Effect::Play(SoundEffect::MultiplierUp));
            }
            // Check for level increase and play sound
            if session.take_level_change().is_some() {
                state.ui.effects.push(Effect::Play(SoundEffect::LevelUp));
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
    let leveled_up = scenario_leveled_up || quest_award.leveled_up;
    if let Some(notification) =
        ScenarioCompletionService::level_up_notification(leveled_up, state.progress.profile.level)
    {
        state.ui.notifications.push(notification.into());
    }

    // Store total XP earned for display in the transition popup (only shown when this
    // keystroke completed the scenario).
    if scenario_xp > 0 {
        let quest_xp: u64 = quest_award.bonuses.iter().map(|bonus| bonus.xp).sum();
        let total_xp = scenario_xp + quest_xp;
        if let TypedScreen::MiniGame(ref mut data) = state.screen {
            data.last_xp_earned = Some(total_xp);
        }
    }

    Ok(())
}

/// Handle executing a Helix command during mini-game
///
/// Keys are interpreted by the shared [`resolve_keys`] (multi-key commands
/// such as dd, gg, rx included).
///
/// `keys` is the canonical command (post keymap-overlay translation) to
/// dispatch; `typed` is the physically-pressed key, for `KeyHistory`
/// display.
pub(in crate::ui::state) fn handle_minigame_command(
    state: &mut AppState,
    keys: &crate::input::keymap::GameplayKey,
    typed: &str,
) -> Result<(), UserError> {
    // Only handle if we're on the MiniGame screen
    if !matches!(state.screen, TypedScreen::MiniGame(_)) {
        return Ok(());
    }

    let mode = state.minigame_session().map_or(
        ModeKind::Normal,
        crate::minigame::MiniGameSession::mode_kind,
    );
    let TypedScreen::MiniGame(ref mut minigame_data) = state.screen else {
        return Ok(());
    };

    key_history::record_key(&mut state.ui, &mut minigame_data.key_history, typed);

    match resolve_keys(minigame_data.input_state_mut(), mode, keys) {
        Resolution::Insert(command) => {
            execute_minigame_command(state, SessionInput::Text(&command))
        }
        Resolution::Execute { command, .. } => {
            execute_minigame_command(state, SessionInput::Command(&command))
        }
        Resolution::Pending | Resolution::Discarded => Ok(()),
    }
}

/// Handle timeout on current mini-game scenario
pub(in crate::ui::state) fn handle_minigame_timeout(state: &mut AppState) {
    if let Some(session) = state
        .screen
        .minigame_data_mut()
        .map(|data| &mut *data.session)
    {
        session.handle_timeout();

        if session.state().is_game_over() {
            // Play game over sound
            state.ui.effects.push(Effect::Play(SoundEffect::GameOver));
            handle_minigame_game_over(state);
        } else {
            // Play life lost sound (still has lives remaining)
            state.ui.effects.push(Effect::Play(SoundEffect::LifeLost));
        }
    }
}

/// Handle the mode's session-level time limit elapsing (e.g. Arcade's 60 seconds)
///
/// Ends the run immediately regardless of lives remaining, reusing the same
/// game-over bookkeeping (FSRS recording, XP award, high scores, profile save)
/// as a per-scenario timeout that depletes the last life.
pub(in crate::ui::state) fn handle_minigame_session_timeout(state: &mut AppState) {
    if let Some(session) = state
        .screen
        .minigame_data_mut()
        .map(|data| &mut *data.session)
    {
        session.end_session_on_timeout();
        state.ui.effects.push(Effect::Play(SoundEffect::GameOver));
        handle_minigame_game_over(state);
    }
}

/// Handle advancing to next scenario (after transition delay)
pub(in crate::ui::state) fn handle_minigame_next_scenario(state: &mut AppState) -> HandlerOutcome {
    if let Some(session) = state.minigame_session_mut()
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
    HandlerOutcome::Stay
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
pub(in crate::ui::state) fn handle_minigame_game_over(state: &mut AppState) {
    let should_process = state
        .minigame_session_mut()
        .is_some_and(crate::minigame::MiniGameSession::try_begin_game_over);

    if !should_process {
        return;
    }

    if let Some(session) = state.screen.minigame_data().map(|data| &*data.session) {
        let stats = session.stats();

        // 1. Record final scenario to FSRS (if applicable)
        let tracker = &mut state.progress.performance_tracker;
        session.record_to_fsrs(tracker, false); // Game over = failure on current scenario

        // 2. Calculate XP earned
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

        // Record Daily Challenge result (attempts/best-score tracking)
        if session.mode().is_challenge() {
            let scenarios_completed = u8::try_from(stats.scenarios_completed).unwrap_or(u8::MAX);
            profile
                .challenge_progress
                .record_result(stats.score, scenarios_completed);
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
        state.ui.effects.push(Effect::SaveProfile);
    }
}

/// Handle returning to mode selection from mini-game
pub(in crate::ui::state) fn handle_minigame_back_to_menu(state: &mut AppState) -> HandlerOutcome {
    // `handle_minigame_game_over` is idempotent per session (see its doc comment),
    // so it's safe to call unconditionally here even if a prior timeout already
    // ran it for this same session.
    if state.minigame_session().is_some() {
        handle_minigame_game_over(state);
    }
    HandlerOutcome::Transition(Box::new(TypedScreen::ModeSelection(
        ModeSelectionData::default(),
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Difficulty, Scenario};
    use crate::gamification::{ProfileStorage, UserProfile};
    use crate::learning::PerformanceTracker;
    use crate::testing::ScenarioBuilder;
    use crate::testing::{cid, ncmd, qid};
    use crate::ui::state::handlers::mode_selection::handle_select_arcade_mode;
    use crate::ui::state::{ConfigState, GameState, MiniGameData, ProgressState, UIState};
    use std::assert_matches;

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
            )
            .loaded(),
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
            )
            .loaded(),
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
        let outcome = handle_select_arcade_mode(&mut ctx);
        crate::ui::state::apply_outcome(state, outcome);
    }

    #[test]
    fn test_start_minigame() {
        let mut state = create_test_state();
        start_minigame(&mut state);

        assert!(state.minigame_session().is_some());
        assert_matches!(state.screen, TypedScreen::MiniGame(_));

        if let Some(session) = state.minigame_session() {
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

    fn press_h(state: &mut AppState) {
        let key = crate::input::keymap::GameplayKey::Keys(crate::testing::keys("h"));
        handle_minigame_command(state, &key, "h").unwrap();
    }

    #[test]
    fn test_execute_minigame_command_shows_key_history() {
        let mut state = create_test_state();
        start_minigame(&mut state);
        state.ui.show_key_history = false;

        // Transition to playing
        if let Some(session) = state.minigame_session_mut() {
            session.tick_countdown();
            session.tick_countdown();
            session.tick_countdown();
        }

        press_h(&mut state);

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
            id: qid("cmd_practice_h"),
            quest_type: QuestType::CommandPractice {
                command: ncmd("h"),
                target: 1,
                current: 0,
            },
            description: "Use 'h' 1 time".to_string(),
            difficulty: QuestDifficulty::Easy,
            xp_reward: 50,
            completed: false,
        }];

        start_minigame(&mut state);
        if let Some(session) = state.minigame_session_mut() {
            session.tick_countdown();
            session.tick_countdown();
            session.tick_countdown();
        }
        let initial_xp = state.progress.profile.total_xp;

        execute_minigame_command(&mut state, SessionInput::Text("h")).unwrap();

        // The scenario itself must not have completed - only the quest should have.
        if let Some(session) = state.minigame_session() {
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
            state.ui.notifications.visible().any(|n| matches!(
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
            id: qid("cmd_practice_h"),
            quest_type: QuestType::CommandPractice {
                command: ncmd("h"),
                target: 1,
                current: 0,
            },
            description: "Use 'h' 1 time".to_string(),
            difficulty: QuestDifficulty::Easy,
            xp_reward: 1,
            completed: false,
        }];

        start_minigame(&mut state);
        if let Some(session) = state.minigame_session_mut() {
            session.tick_countdown();
            session.tick_countdown();
            session.tick_countdown();
        }

        execute_minigame_command(&mut state, SessionInput::Text("h")).unwrap();

        assert!(
            state.progress.profile.level >= 2,
            "expected a level-up driven by quest XP"
        );
        assert!(
            state.ui.notifications.visible().any(|n| matches!(
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
        if let Some(session) = state.minigame_session_mut() {
            session.tick_countdown();
            session.tick_countdown();
            session.tick_countdown();
        }

        execute_minigame_command(&mut state, SessionInput::Text("h")).unwrap();

        assert!(
            !state.ui.notifications.visible().any(|n| matches!(
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

        if let Some(session) = state.minigame_session_mut() {
            session.tick_countdown();
            session.tick_countdown();
            session.tick_countdown();
        }

        let _ = execute_minigame_command(&mut state, SessionInput::Text("x"));
        let _ = execute_minigame_command(&mut state, SessionInput::Text("d"));

        assert!(
            state.progress.profile.level >= 2,
            "expected a level-up driven by scenario-completion XP"
        );
        assert!(
            state.ui.notifications.visible().any(|n| matches!(
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
        if let Some(session) = state.minigame_session_mut() {
            session.tick_countdown();
            session.tick_countdown();
            session.tick_countdown();
            session.advance_to_next();
        }
        state.ui.show_key_history = true;
        if let TypedScreen::MiniGame(ref mut data) = state.screen {
            data.add_key_to_history("h".to_string());
        }

        handle_minigame_next_scenario(&mut state);

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
        state.game.scenario_collection = crate::game::ScenarioCollection::new(vec![]);

        start_minigame(&mut state);

        assert!(state.minigame_session().is_none());
    }

    #[test]
    fn test_pause_resume_minigame() {
        let mut state = create_test_state();
        start_minigame(&mut state);

        if let Some(session) = state.minigame_session_mut() {
            session.tick_countdown();
            session.tick_countdown();
            session.tick_countdown();
        }

        {
            crate::testing::update_executing_effects(
                &mut state,
                crate::ui::state::Message::PauseMiniGame,
            )
            .unwrap();
        }

        if let Some(session) = state.minigame_session() {
            assert!(session.state().is_paused());
        }

        {
            crate::testing::update_executing_effects(
                &mut state,
                crate::ui::state::Message::ResumeMiniGame,
            )
            .unwrap();
        }

        if let Some(session) = state.minigame_session() {
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

        assert!(state.minigame_session().is_some());

        let outcome = handle_minigame_back_to_menu(&mut state);
        crate::ui::state::apply_outcome(&mut state, outcome);

        assert!(state.minigame_session().is_none());
        assert_matches!(state.screen, TypedScreen::ModeSelection(_));
    }

    #[test]
    fn test_minigame_tick_countdown() {
        let mut state = create_test_state();
        start_minigame(&mut state);

        if let Some(session) = state.minigame_session() {
            assert_eq!(session.state().countdown_remaining(), Some(3));
        }

        {
            crate::testing::update_executing_effects(
                &mut state,
                crate::ui::state::Message::MiniGameTick,
            )
            .unwrap();
        }

        if let Some(session) = state.minigame_session() {
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
        if let Some(session) = state.minigame_session_mut() {
            session.tick_countdown();
            session.tick_countdown();
            session.tick_countdown();
        }

        // Get initial XP
        let initial_xp = state.progress.profile.total_xp;

        // Manually set some stats for testing
        if let Some(session) = state.minigame_session_mut() {
            // Simulate game progress
            session.stats.score = 5000;
            session.stats.increase_level();
            session.stats.increase_level();
            for _ in 0..10 {
                session.stats.record_completion();
            }
        }

        // Trigger game over
        handle_minigame_game_over(&mut state);
        crate::testing::run_queued_effects(&mut state.ui, &mut state.progress);

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
        if let Some(session) = state.minigame_session_mut() {
            session.tick_countdown();
            session.tick_countdown();
            session.tick_countdown();
            session.stats.score = 5000;
        }

        handle_minigame_game_over(&mut state);

        crate::testing::run_queued_effects(&mut state.ui, &mut state.progress);

        // Check high score was updated
        assert_eq!(state.progress.profile.minigame_high_score, 5000);
    }

    #[test]
    fn test_minigame_updates_challenge_progress() {
        use crate::gamification::ProfileStorage;
        use crate::minigame::{ChallengeConfig, MiniGameMode};
        use crate::ui::state::HandlerContext;
        use tempfile::TempDir;

        let mut state = create_test_state();
        let temp_dir = TempDir::new().unwrap();
        state.progress.storage = ProfileStorage::with_path(temp_dir.path().join("profile.json"));

        let today = state.progress.today();
        let mode = MiniGameMode::Challenge(ChallengeConfig::for_date(today));

        // Launch through the real handler (not a hand-built session) so
        // `start_attempt` -> `record_result` is exercised end-to-end.
        {
            let mut ctx = HandlerContext::new(
                &mut state.ui,
                &mut state.game,
                &mut state.progress,
                &state.config,
            );
            let outcome = super::super::mode_selection::handle_launch_minigame_mode(&mut ctx, mode);
            crate::ui::state::apply_outcome(&mut state, outcome);
        }
        assert_eq!(
            state
                .progress
                .profile
                .challenge_progress
                .attempts_used_today,
            1
        );

        if let Some(session) = state.minigame_session_mut() {
            session.tick_countdown();
            session.tick_countdown();
            session.tick_countdown();
            session.stats.score = 5000;
            session.stats.scenarios_completed = 7;
        }

        handle_minigame_game_over(&mut state);

        crate::testing::run_queued_effects(&mut state.ui, &mut state.progress);

        let progress = &state.progress.profile.challenge_progress;
        assert_eq!(progress.best_score_today, 5000);
        assert_eq!(progress.best_scenarios_today, 7);
        assert_eq!(progress.all_time_best_score, 5000);
        assert_eq!(progress.all_time_best_scenarios, 7);
        assert_eq!(progress.total_challenges_completed, 0); // 7 < CHALLENGE_SCENARIO_COUNT (10)
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

        if let Some(session) = state.minigame_session_mut() {
            session.tick_countdown();
            session.tick_countdown();
            session.tick_countdown();
            session.stats.score = 5000;
        }

        // Seed tracker state as if reviews happened earlier in the session.
        state.progress.performance_tracker.record_attempt(
            &cid("j"),
            Duration::from_millis(500),
            true,
            Duration::from_millis(500),
        );

        handle_minigame_game_over(&mut state);

        crate::testing::run_queued_effects(&mut state.ui, &mut state.progress);

        let persisted = ProfileStorage::with_path(&profile_path).load().unwrap();
        assert!(!persisted.performance_data.is_empty());
        assert!(persisted.performance_data.contains_key(&cid("j")));
    }

    #[test]
    fn test_minigame_updates_quest_progress() {
        use crate::gamification::{Quest, QuestDifficulty, QuestType};

        let mut state = create_test_state();

        // Initialize with sample quests
        {
            let profile = &mut state.progress.profile;
            profile.daily_quests = vec![Quest {
                id: qid("cmd_practice"),
                quest_type: QuestType::CommandPractice {
                    command: ncmd("j"),
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
        if let Some(session) = state.minigame_session_mut() {
            session.tick_countdown();
            session.tick_countdown();
            session.tick_countdown();
        }

        assert!(
            state
                .minigame_session()
                .is_some_and(|s| s.state().is_playing())
        );

        // Deplete all 3 lives via timeout
        for _ in 0..3 {
            handle_minigame_timeout(&mut state);
        }

        // Should be in game over state
        assert!(
            state
                .minigame_session()
                .is_some_and(|s| s.state().is_game_over())
        );

        // Current scenario should be None after game over
        assert!(
            state
                .minigame_session()
                .is_some_and(|s| s.current_scenario().is_none())
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
        if let Some(session) = state.minigame_session_mut() {
            session.tick_countdown();
            session.tick_countdown();
            session.tick_countdown();
        }

        // Trigger game over
        for _ in 0..3 {
            handle_minigame_timeout(&mut state);
        }

        // Verify game over state
        assert!(
            state
                .minigame_session()
                .is_some_and(|s| s.state().is_game_over())
        );

        // Render should not panic - use public render function
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let result = terminal.draw(|f| crate::ui::render::render(f, &state));
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
        if let Some(session) = state.minigame_session_mut() {
            session.tick_countdown();
            session.tick_countdown();
            session.tick_countdown();
        }

        // Deplete all lives
        for _ in 0..3 {
            // Each timeout should succeed without panic
            handle_minigame_timeout(&mut state);
        }

        // Game over handler should have run successfully
        assert!(
            state
                .minigame_session()
                .is_some_and(|s| s.state().is_game_over())
        );
    }

    #[test]
    fn test_handle_minigame_command_single_key_command_flow() {
        let mut state = create_test_state();
        start_minigame(&mut state);

        // Transition to playing
        if let Some(session) = state.minigame_session_mut() {
            session.tick_countdown();
            session.tick_countdown();
            session.tick_countdown();
        }

        // Test that handle_minigame_command processes input without panic
        // Command might fail (OperationFailed) but function should return gracefully
        let _ = handle_minigame_command(
            &mut state,
            &crate::input::keymap::GameplayKey::Keys(crate::testing::keys("j")),
            "j",
        );

        // Test should verify the handler doesn't panic, not that command succeeds
        // (command success depends on scenario state)
    }

    #[test]
    fn test_handle_minigame_command_multi_key_sequence() {
        let mut state = create_test_state();
        start_minigame(&mut state);

        // Transition to playing
        if let Some(session) = state.minigame_session_mut() {
            session.tick_countdown();
            session.tick_countdown();
            session.tick_countdown();
        }

        // Test multi-key command sequence (dd) doesn't panic
        // First 'd' may be buffered or rejected depending on mode
        let _ = handle_minigame_command(
            &mut state,
            &crate::input::keymap::GameplayKey::Keys(crate::testing::keys("d")),
            "d",
        );

        // Second 'd' should attempt to complete sequence (might fail but shouldn't panic)
        let _ = handle_minigame_command(
            &mut state,
            &crate::input::keymap::GameplayKey::Keys(crate::testing::keys("d")),
            "d",
        );

        // Test passes if no panic occurred
    }

    #[test]
    fn test_handle_minigame_command_pushes_key_history_once_per_keypress() {
        let mut state = create_test_state();
        start_minigame(&mut state);

        // Transition to playing
        if let Some(session) = state.minigame_session_mut() {
            session.tick_countdown();
            session.tick_countdown();
            session.tick_countdown();
        }

        // Regression test for S3: handle_minigame_command used to push to key
        // history itself AND call execute_minigame_command (which also pushes),
        // double-counting every resolved keystroke.
        let _ = handle_minigame_command(
            &mut state,
            &crate::input::keymap::GameplayKey::Keys(crate::testing::keys("j")),
            "j",
        );

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

    /// M7 regression test: with `KeymapOverlay::identity()` (the default -
    /// no remap active), the `was_base` display-label switch in
    /// `handle_minigame_command` must produce byte-identical `KeyHistory`
    /// contents to the pre-keymap-overlay behavior of always showing the
    /// resolved command, for both single-keystroke and multi-keystroke
    /// (count-prefixed, find-char, and stock two-key) sequences.
    #[test]
    fn test_handle_minigame_command_key_history_matches_stock_dispatch_under_identity_overlay() {
        use crate::input::keymap::GameplayKey;

        let mut state = create_test_state();
        start_minigame(&mut state);
        if let Some(session) = state.minigame_session_mut() {
            session.tick_countdown();
            session.tick_countdown();
            session.tick_countdown();
        }
        assert!(
            state.config.keymap.is_empty(),
            "test assumes the default identity overlay"
        );

        let press = |state: &mut AppState, key: &'static str| {
            handle_minigame_command(state, &GameplayKey::Keys(crate::testing::keys(key)), key)
                .unwrap();
        };

        for key in ["x", "3", "j", "f", "x", "g", "g"] {
            press(&mut state, key);
        }

        if let TypedScreen::MiniGame(ref data) = state.screen {
            // `keys()` is newest-first.
            assert_eq!(*data.key_history.keys(), ["g", "g", "x", "f", "j"]);
        } else {
            panic!("expected MiniGame screen");
        }
    }

    #[test]
    fn test_handle_minigame_command_no_session() {
        let mut state = create_test_state();
        // Don't start minigame

        // Should handle gracefully without session
        let result = handle_minigame_command(
            &mut state,
            &crate::input::keymap::GameplayKey::Keys(crate::testing::keys("j")),
            "j",
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_handle_minigame_command_wrong_screen() {
        let mut state = create_test_state();
        // Keep on ModeSelection screen, don't transition to MiniGame

        let result = handle_minigame_command(
            &mut state,
            &crate::input::keymap::GameplayKey::Keys(crate::testing::keys("j")),
            "j",
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_create_minigame_session_with_scenarios() {
        let state = create_test_state();
        // Test without tracker (backward compatibility)
        let session = create_minigame_session(&state.game, None).unwrap();

        assert!(session.state().is_countdown());
    }

    #[test]
    fn test_create_minigame_session_with_tracker() {
        let state = create_test_state();
        // Test with tracker for FSRS-weighted selection
        let session =
            create_minigame_session(&state.game, Some(&state.progress.performance_tracker))
                .unwrap();

        assert!(session.state().is_countdown());
    }

    #[test]
    fn test_create_minigame_session_empty_scenarios() {
        let mut state = create_test_state();
        state.game.scenario_collection = crate::game::ScenarioCollection::new(vec![]);

        assert!(create_minigame_session(&state.game, None).is_none());
    }

    #[test]
    fn test_handle_minigame_next_scenario() {
        let mut state = create_test_state();
        start_minigame(&mut state);

        // Transition to playing
        if let Some(session) = state.minigame_session_mut() {
            session.tick_countdown();
            session.tick_countdown();
            session.tick_countdown();
            session.advance_to_next();
        }

        let outcome = handle_minigame_next_scenario(&mut state);

        assert_matches!(outcome, HandlerOutcome::Stay);
    }

    /// Regression test for S4b: a half-typed prefix/register/command-line
    /// state must not leak into the next scenario after
    /// `handle_minigame_next_scenario` advances.
    #[test]
    fn test_handle_minigame_next_scenario_resets_pending_input_state() {
        let mut state = create_test_state();
        start_minigame(&mut state);

        if let Some(session) = state.minigame_session_mut() {
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

        handle_minigame_next_scenario(&mut state);

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
        if let Some(session) = state.minigame_session_mut() {
            session.tick_countdown();
            session.tick_countdown();
            session.tick_countdown();
        }

        press_h(&mut state);

        // Key should be added to history regardless of command success
        if let TypedScreen::MiniGame(ref data) = state.screen {
            let keys = data.key_history.keys();
            assert!(!keys.is_empty());
            assert!(keys[0].contains('h'));
        }
    }

    #[test]
    fn test_handle_minigame_tick_not_countdown() {
        let mut state = create_test_state();
        start_minigame(&mut state);

        // Transition to playing (no longer countdown)
        if let Some(session) = state.minigame_session_mut() {
            session.tick_countdown();
            session.tick_countdown();
            session.tick_countdown();
        }

        let countdown_before = state
            .minigame_session()
            .and_then(|s| s.state().countdown_remaining());

        crate::testing::update_executing_effects(
            &mut state,
            crate::ui::state::Message::MiniGameTick,
        )
        .unwrap();

        // Countdown should not change when not in countdown state
        let countdown_after = state
            .minigame_session()
            .and_then(|s| s.state().countdown_remaining());

        assert_eq!(countdown_before, countdown_after);
    }

    /// Regression test for #466: a simulator error in arcade mode is returned
    /// as a recoverable error and keeps the session and screen.
    #[test]
    fn test_rejected_command_keeps_minigame_session_and_screen() {
        let mut state = create_test_state();
        start_minigame(&mut state);
        if let Some(session) = state.minigame_session_mut() {
            session.tick_countdown();
            session.tick_countdown();
            session.tick_countdown();
        }

        let result = execute_minigame_command(&mut state, SessionInput::Text("not_a_command"));

        assert!(result.is_err());
        assert!(state.minigame_session().is_some());
        assert!(matches!(state.screen, TypedScreen::MiniGame(_)));
    }

    #[test]
    fn test_minigame_awards_scenario_completion_xp() {
        let mut state = create_test_state();
        start_minigame(&mut state);

        // Transition to playing
        if let Some(session) = state.minigame_session_mut() {
            session.tick_countdown();
            session.tick_countdown();
            session.tick_countdown();
        }

        let initial_xp = state.progress.profile.total_xp;

        // Manually complete scenario to test XP award
        if let Some(session) = state.minigame_session_mut() {
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
        if let Some(session) = state.minigame_session_mut() {
            session.tick_countdown();
            session.tick_countdown();
            session.tick_countdown();
            for _ in 0..15 {
                session.stats.record_completion();
            }
        }

        handle_minigame_game_over(&mut state);

        crate::testing::run_queued_effects(&mut state.ui, &mut state.progress);

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
        if let Some(session) = state.minigame_session_mut() {
            session.tick_countdown();
            session.tick_countdown();
            session.tick_countdown();
            session.stats.score = 500;
        }

        handle_minigame_game_over(&mut state);

        crate::testing::run_queued_effects(&mut state.ui, &mut state.progress);

        // High score should not decrease
        assert_eq!(state.progress.profile.minigame_high_score, 10000);
    }

    #[test]
    fn test_minigame_timeout_without_session() {
        let mut state = create_test_state();

        // Should handle gracefully without session
        handle_minigame_timeout(&mut state);
        assert!(state.minigame_session().is_none());
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
        if let Some(session) = state.minigame_session_mut() {
            session.tick_countdown();
            session.tick_countdown();
            session.tick_countdown();
        }

        // Return to menu (should trigger game_over and increment games_played)
        let outcome = handle_minigame_back_to_menu(&mut state);
        crate::ui::state::apply_outcome(&mut state, outcome);

        // Games played should be incremented
        assert_eq!(state.progress.profile.minigame_games_played, 1);

        // Session should be cleared
        assert!(state.minigame_session().is_none());
        assert_matches!(state.screen, TypedScreen::ModeSelection(_));
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
        if let Some(session) = state.minigame_session_mut() {
            session.tick_countdown();
            session.tick_countdown();
            session.tick_countdown();
        }

        // Complete the scenario ("x" selects the line, "d" deletes it) essentially
        // immediately, so time_ratio stays near 0.0.
        let _ = execute_minigame_command(&mut state, SessionInput::Text("x"));
        let _ = execute_minigame_command(&mut state, SessionInput::Text("d"));

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

        assert!(state.ui.notifications.visible().any(|n| matches!(
            &n.notification_type,
            NotificationType::Achievement { name, .. } if name == "Speed Demon"
        )));
        assert!(state.ui.notifications.visible().any(|n| matches!(
            &n.notification_type,
            NotificationType::Achievement { name, .. } if name == "Flash"
        )));
    }

    /// Regression test for #358: arcade completion must NOT persist mid-session even
    /// when it unlocks achievements and bumps speed counters - unlike Training's
    /// `record_scenario_completion`, `execute_minigame_command`'s completion block
    /// (via the shared `ScenarioCompletionService` methods) never calls
    /// `save_immediate`/`save_debounced`; only `handle_minigame_game_over` persists.
    #[test]
    fn test_execute_minigame_command_does_not_persist_mid_session() {
        use crate::gamification::{AchievementId, ProfileStorage};
        use tempfile::TempDir;

        let scenario =
            create_completable_scenario_with_difficulty("speed_test", Difficulty::Beginner);
        let mut state = create_single_scenario_state(scenario);
        let temp_dir = TempDir::new().unwrap();
        let profile_path = temp_dir.path().join("profile.json");
        state.progress.storage = ProfileStorage::with_path(&profile_path);
        start_minigame(&mut state);

        if let Some(session) = state.minigame_session_mut() {
            session.tick_countdown();
            session.tick_countdown();
            session.tick_countdown();
        }

        let _ = execute_minigame_command(&mut state, SessionInput::Text("x"));
        let _ = execute_minigame_command(&mut state, SessionInput::Text("d"));

        assert!(
            state
                .progress
                .profile
                .has_achievement(&AchievementId::SpeedDemon),
            "sanity check: this completion should unlock an achievement"
        );
        assert!(
            !profile_path.exists(),
            "arcade must not persist mid-session, even on achievement unlock"
        );

        handle_minigame_game_over(&mut state);

        crate::testing::run_queued_effects(&mut state.ui, &mut state.progress);

        assert!(profile_path.exists(), "arcade must persist at game over");
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
        let mut state = create_single_scenario_state(scenario.clone());
        let clock = Arc::new(crate::time::FakeMonotonicClock::new());
        let mut session = MiniGameSession::with_sources(
            Arc::new(vec![scenario]),
            None,
            crate::minigame::MiniGameMode::default(),
            clock.clone(),
            rand::SeedableRng::seed_from_u64(1),
        );
        session.start();
        for _ in 0..3 {
            session.tick_countdown();
        }
        state.screen = TypedScreen::MiniGame(MiniGameData::new(session));

        clock.advance(std::time::Duration::from_millis(3_500));

        let _ = execute_minigame_command(&mut state, SessionInput::Text("x"));
        let _ = execute_minigame_command(&mut state, SessionInput::Text("d"));

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

        if let Some(session) = state.minigame_session_mut() {
            session.tick_countdown();
            session.tick_countdown();
            session.tick_countdown();
        }

        let _ = execute_minigame_command(&mut state, SessionInput::Text("x"));
        let _ = execute_minigame_command(&mut state, SessionInput::Text("d"));

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

        if let Some(session) = state.minigame_session_mut() {
            session.tick_countdown();
            session.tick_countdown();
            session.tick_countdown();
        }

        let _ = execute_minigame_command(&mut state, SessionInput::Text("x"));
        let _ = execute_minigame_command(&mut state, SessionInput::Text("d"));

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
        if let Some(session) = state.minigame_session_mut() {
            session.tick_countdown();
            session.tick_countdown();
            session.tick_countdown();
        }

        // Trigger game over
        handle_minigame_game_over(&mut state);
        crate::testing::run_queued_effects(&mut state.ui, &mut state.progress);

        // Games played should be incremented
        assert_eq!(state.progress.profile.minigame_games_played, 1);

        // Simulate another game session
        start_minigame(&mut state);
        if let Some(session) = state.minigame_session_mut() {
            session.tick_countdown();
            session.tick_countdown();
            session.tick_countdown();
        }
        handle_minigame_game_over(&mut state);
        crate::testing::run_queued_effects(&mut state.ui, &mut state.progress);

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

        if let Some(session) = state.minigame_session_mut() {
            session.tick_countdown();
            session.tick_countdown();
            session.tick_countdown();
        }

        handle_minigame_game_over(&mut state);

        crate::testing::run_queued_effects(&mut state.ui, &mut state.progress);

        assert!(
            state.progress.profile.level >= 2,
            "expected a level-up driven by the end-of-game XP award"
        );
        assert!(
            state.ui.notifications.visible().any(|n| matches!(
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

        if let Some(session) = state.minigame_session_mut() {
            session.tick_countdown();
            session.tick_countdown();
            session.tick_countdown();
        }

        // Deplete all lives via timeout - this already runs `handle_minigame_game_over`
        // once, via `handle_minigame_timeout`.
        for _ in 0..3 {
            handle_minigame_timeout(&mut state);
        }
        assert!(
            state
                .minigame_session()
                .is_some_and(|s| s.state().is_game_over())
        );

        let xp_after_first_game_over = state.progress.profile.total_xp;
        let games_played_after_first_game_over = state.progress.profile.minigame_games_played;

        // Simulate Esc/`m`/Ctrl-q on the game-over screen.
        let outcome = handle_minigame_back_to_menu(&mut state);
        crate::ui::state::apply_outcome(&mut state, outcome);

        assert_eq!(
            state.progress.profile.total_xp, xp_after_first_game_over,
            "returning to menu from an already-processed game-over must not award XP again"
        );
        assert_eq!(
            state.progress.profile.minigame_games_played, games_played_after_first_game_over,
            "returning to menu from an already-processed game-over must not increment games_played again"
        );
        assert!(state.minigame_session().is_none());
        assert_matches!(state.screen, TypedScreen::ModeSelection(_));
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

        if let Some(session) = state.minigame_session_mut() {
            session.tick_countdown();
            session.tick_countdown();
            session.tick_countdown();
        }

        handle_minigame_game_over(&mut state);

        crate::testing::run_queued_effects(&mut state.ui, &mut state.progress);

        assert!(
            !state.ui.notifications.visible().any(|n| matches!(
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
        // waited out with a short real-time sleep, using an injected fake clock.
        let scenarios = Arc::new(vec![create_test_scenario("s1")]);
        let mode = MiniGameMode::Arcade(ArcadeConfig {
            session_duration: Duration::from_millis(200),
            ..ArcadeConfig::default()
        });
        let clock = Arc::new(crate::time::FakeMonotonicClock::new());
        let mut session = MiniGameSession::with_sources(
            scenarios,
            None,
            mode,
            clock.clone(),
            rand::SeedableRng::seed_from_u64(1),
        );
        session.start();
        session.tick_countdown();
        session.tick_countdown();
        session.tick_countdown();
        session.stats.score = 5000;
        for _ in 0..10 {
            session.stats.record_completion();
        }
        let lives_before = session.stats().lives();
        state.screen = TypedScreen::MiniGame(MiniGameData::new(session));

        clock.advance(Duration::from_millis(500));
        assert!(state.minigame_session().unwrap().is_session_expired());

        let initial_xp = state.progress.profile.total_xp;

        handle_minigame_session_timeout(&mut state);

        crate::testing::run_queued_effects(&mut state.ui, &mut state.progress);

        // Same bookkeeping as an ordinary timeout game-over: XP awarded, high
        // score updated, games_played incremented.
        let expected_xp = XPCalculator::minigame_xp(5000, 1, 10);
        assert_eq!(state.progress.profile.total_xp - initial_xp, expected_xp);
        assert_eq!(state.progress.profile.minigame_high_score, 5000);
        assert_eq!(state.progress.profile.minigame_games_played, 1);

        // Jumps straight to GameOver without consuming a life.
        let session = state.minigame_session().unwrap();
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
        let clock = Arc::new(crate::time::FakeMonotonicClock::new());
        let mut session = MiniGameSession::with_sources(
            scenarios,
            None,
            mode,
            clock.clone(),
            rand::SeedableRng::seed_from_u64(1),
        );
        session.start();
        session.tick_countdown();
        session.tick_countdown();
        session.tick_countdown();
        state.screen = TypedScreen::MiniGame(MiniGameData::new(session));

        clock.advance(Duration::from_millis(500));

        // Ends the run via the session-timer path (not lives depleted) - this
        // already runs `handle_minigame_game_over` once.
        handle_minigame_session_timeout(&mut state);
        crate::testing::run_queued_effects(&mut state.ui, &mut state.progress);
        assert!(
            state
                .minigame_session()
                .is_some_and(|s| s.state().is_game_over())
        );

        let xp_after_first_game_over = state.progress.profile.total_xp;
        let games_played_after_first_game_over = state.progress.profile.minigame_games_played;

        // Simulate Esc/`m`/Ctrl-q on the game-over screen.
        let outcome = handle_minigame_back_to_menu(&mut state);
        crate::ui::state::apply_outcome(&mut state, outcome);

        assert_eq!(
            state.progress.profile.total_xp, xp_after_first_game_over,
            "returning to menu from an already-processed session timeout must not award XP again"
        );
        assert_eq!(
            state.progress.profile.minigame_games_played, games_played_after_first_game_over,
            "returning to menu from an already-processed session timeout must not increment games_played again"
        );
        assert!(state.minigame_session().is_none());
        assert_matches!(state.screen, TypedScreen::ModeSelection(_));
    }
}
