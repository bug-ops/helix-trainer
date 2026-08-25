//! Tests for navigation and screen transitions

use std::assert_matches;

use super::common::{create_test_app_state, create_test_scenario};
use crate::ui::state::{Message, Screen, TypedScreen, update};

#[test]
fn test_new_state() {
    let state = create_test_app_state(vec![]);
    if let TypedScreen::ModeSelection(mode_data) = &state.screen {
        assert_eq!(mode_data.selected_mode, 0);
    } else {
        panic!("Should be on ModeSelection screen");
    }
    assert!(state.ui.running);
    assert!(state.game.review_session.is_none());
    assert!(state.game.pending_completed_session.is_none());
}

#[test]
fn test_quit_app_message() {
    let mut state = create_test_app_state(vec![]);
    assert!(state.ui.running);

    update(&mut state, Message::QuitApp).unwrap();
    assert!(!state.ui.running);
}

/// Regression test for #324: a mid-session `QuitApp` (Ctrl-C) must award the
/// same arcade game-over bookkeeping (XP, high score, profile save) as
/// quitting to the menu does, instead of silently discarding it.
#[test]
fn test_quit_app_awards_minigame_game_over() {
    let scenario = create_test_scenario();
    let mut state = create_test_app_state(vec![scenario]);

    update(&mut state, Message::StartMiniGame).unwrap();
    assert!(state.game.minigame_session.is_some());

    if let Some(ref mut session) = state.game.minigame_session {
        session.stats.score = 5000;
    }
    let initial_xp = state.progress.profile.total_xp;

    update(&mut state, Message::QuitApp).unwrap();

    assert!(!state.ui.running);
    assert!(
        state.progress.profile.total_xp > initial_xp,
        "mid-session Ctrl-C should award XP just like quitting to the menu"
    );
    assert_eq!(state.progress.profile.minigame_high_score, 5000);
}

/// Regression test for #323/#324: a per-scenario timeout depleting the last
/// life drives the session to `GameOver` via `handle_minigame_timeout`,
/// which already runs `handle_minigame_game_over` at that real call site. A
/// later Ctrl-C (`QuitApp`) for the same, already-finished session must not
/// re-award XP or double-count the game.
#[test]
fn test_quit_app_after_timeout_game_over_does_not_double_award() {
    let scenario = create_test_scenario();
    let mut state = create_test_app_state(vec![scenario]);

    update(&mut state, Message::StartMiniGame).unwrap();
    for _ in 0..3 {
        update(&mut state, Message::MiniGameTick).unwrap();
    }
    if let Some(ref mut session) = state.game.minigame_session {
        session.stats.score = 5000;
    }

    // Deplete all 3 lives via timeout - the last one drives the session to
    // GameOver and runs bookkeeping through the real timeout call site.
    for _ in 0..3 {
        update(&mut state, Message::MiniGameTimeout).unwrap();
    }
    assert!(
        state
            .game
            .minigame_session
            .as_ref()
            .map(|s| s.state().is_game_over())
            .unwrap_or(false)
    );

    let xp_after_timeout_game_over = state.progress.profile.total_xp;
    let games_played_after_timeout = state.progress.profile.minigame_games_played;

    // Ctrl-C fires for the same already-finished session.
    update(&mut state, Message::QuitApp).unwrap();

    assert_eq!(
        state.progress.profile.total_xp, xp_after_timeout_game_over,
        "QuitApp must not re-award XP for a session already finished via timeout"
    );
    assert_eq!(
        state.progress.profile.minigame_games_played, games_played_after_timeout,
        "QuitApp must not double-increment games_played for an already-finished session"
    );
}

/// Regression test for #324: Ctrl-C on a fresh, zero-score session (no
/// scenario completed yet) must still run game-over bookkeeping cleanly -
/// no panic, and the game still counts toward `minigame_games_played`.
#[test]
fn test_quit_app_zero_score_session_awards_cleanly() {
    let scenario = create_test_scenario();
    let mut state = create_test_app_state(vec![scenario]);

    update(&mut state, Message::StartMiniGame).unwrap();
    assert!(state.game.minigame_session.is_some());

    let initial_games_played = state.progress.profile.minigame_games_played;

    update(&mut state, Message::QuitApp).unwrap();

    assert!(!state.ui.running);
    assert_eq!(
        state.progress.profile.minigame_games_played,
        initial_games_played + 1,
        "a zero-score session should still be counted as a played game"
    );
    assert_eq!(state.progress.profile.minigame_high_score, 0);
}

/// Regression test for #324: the Ctrl-C (`QuitApp`) path must persist FSRS
/// review data to disk via `ProgressState::save_immediate`, not just
/// XP/high score - exercised through the `QuitApp` dispatcher rather than a
/// direct `handle_minigame_game_over` call, since that's the real call site
/// #324 was filed against.
#[test]
fn test_quit_app_persists_fsrs_data() {
    use crate::gamification::ProfileStorage;
    use std::time::Duration;
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    let profile_path = temp_dir.path().join("profile.json");

    let scenario = create_test_scenario();
    let mut state = create_test_app_state(vec![scenario]);
    state.progress.storage = ProfileStorage::with_path(&profile_path);

    update(&mut state, Message::StartMiniGame).unwrap();

    // Seed tracker state as if reviews happened earlier in the session.
    state.progress.performance_tracker.record_attempt(
        "j",
        Duration::from_millis(500),
        true,
        Duration::from_millis(500),
    );

    update(&mut state, Message::QuitApp).unwrap();

    let persisted = ProfileStorage::with_path(&profile_path).load().unwrap();
    assert!(!persisted.performance_data.is_empty());
    assert!(
        persisted.performance_data.contains_key("j"),
        "FSRS review data recorded before Ctrl-C must be persisted, not just XP/high score"
    );
}

/// Regression test for #324 (impl-critic finding S2): `MiniGameSession::record_to_fsrs`
/// only records when the active scenario has at least one taken action
/// (`session.rs`'s `record_to_fsrs` early-returns on an empty/absent current
/// scenario). `test_quit_app_persists_fsrs_data` above seeds the tracker
/// manually and only proves the save path syncs whatever is already in the
/// tracker; it never exercises that gate. This drives the session into
/// `Playing`, executes a real command so an action is recorded on the
/// active scenario, then fires Ctrl-C and asserts the resulting FSRS record
/// - not a manually seeded one - is what gets persisted.
#[test]
fn test_quit_app_persists_fsrs_data_from_live_action() {
    use crate::gamification::ProfileStorage;
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    let profile_path = temp_dir.path().join("profile.json");

    let scenario = create_test_scenario();
    let mut state = create_test_app_state(vec![scenario]);
    state.progress.storage = ProfileStorage::with_path(&profile_path);

    update(&mut state, Message::StartMiniGame).unwrap();
    if let Some(ref mut session) = state.game.minigame_session {
        session.tick_countdown();
        session.tick_countdown();
        session.tick_countdown();
        assert!(session.state().is_playing());
        session.handle_command("x").unwrap();
    }
    assert_eq!(
        state
            .game
            .minigame_session
            .as_ref()
            .and_then(|s| s.current_scenario())
            .map(|s| s.action_count()),
        Some(1),
        "the live keystroke must actually be recorded on the active scenario"
    );

    update(&mut state, Message::QuitApp).unwrap();

    let persisted = ProfileStorage::with_path(&profile_path).load().unwrap();
    assert!(
        persisted.performance_data.contains_key("x"),
        "an action taken mid-session before Ctrl-C must reach record_to_fsrs, \
         not just whatever was already in the tracker"
    );
}

#[test]
fn test_navigate_to_screen() {
    let mut state = create_test_app_state(vec![]);
    assert_matches!(state.screen, TypedScreen::ModeSelection(_));

    // After TypedScreen refactoring, only screens with standalone data can be navigated to
    // Task and Results require active sessions, so only test Profile/Statistics/Menu/ModeSelection
    update(&mut state, Message::NavigateTo(Screen::Profile)).unwrap();
    assert_matches!(state.screen, TypedScreen::Profile(_));

    update(&mut state, Message::NavigateTo(Screen::Statistics)).unwrap();
    assert_matches!(state.screen, TypedScreen::Statistics(_));

    update(&mut state, Message::NavigateTo(Screen::MainMenu)).unwrap();
    assert_matches!(state.screen, TypedScreen::Menu(_));

    update(&mut state, Message::NavigateTo(Screen::ModeSelection)).unwrap();
    assert_matches!(state.screen, TypedScreen::ModeSelection(_));
}

#[test]
fn test_back_to_menu_clears_session() {
    let scenario = create_test_scenario();
    let mut state = create_test_app_state(vec![scenario]);

    update(&mut state, Message::StartScenario(0)).unwrap();
    // After TypedScreen refactoring, verify we're on Task screen
    assert_matches!(state.screen, TypedScreen::Task(_));

    update(&mut state, Message::BackToMenu).unwrap();
    // Should transition back to ModeSelection screen (the main menu)
    assert_matches!(state.screen, TypedScreen::ModeSelection(_));
}

#[test]
fn test_scenario_count() {
    let scenarios = vec![create_test_scenario(), create_test_scenario()];
    let state = create_test_app_state(scenarios);
    assert_eq!(state.scenario_count(), 2); // Filtered count
}

#[test]
fn test_scenario() {
    let scenario = create_test_scenario();
    let mut scenarios = vec![scenario.clone()];
    scenarios.push(scenario);
    let state = create_test_app_state(scenarios);

    assert!(state.scenario(0).is_some());
    assert!(state.scenario(1).is_some());
    assert!(state.scenario(999).is_none());
}
