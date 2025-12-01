//! Screen-specific keyboard event handlers
//!
//! Each handler function processes keyboard input for a specific screen.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::borrow::Cow;

use helix_trainer::ui::state::CommandBufferAccess;
use helix_trainer::ui::{AppState, Message, state::TypedScreen};

use super::mapping::{handle_insert_mode_input, map_key_to_helix_command};

/// Check if the command buffer is waiting for a character argument
///
/// Commands like `r`, `f`, `F`, `t`, `T` expect a character to follow.
/// When the buffer contains one of these, we should accept any printable character.
fn is_waiting_for_char_arg(state: &AppState) -> bool {
    match &state.screen {
        TypedScreen::Task(task_data) => {
            let buffer = task_data.command_buffer();
            matches!(buffer, "r" | "f" | "F" | "t" | "T")
        }
        TypedScreen::MiniGame(minigame_data) => {
            let buffer = minigame_data.command_buffer();
            matches!(buffer, "r" | "f" | "F" | "t" | "T")
        }
        _ => false,
    }
}

/// Handle keyboard events on profile and statistics screens
pub fn handle_profile_stats_keys(key: KeyEvent, state: &AppState) -> Option<Message> {
    match key.code {
        KeyCode::Esc | KeyCode::Char('m') => Some(Message::BackToMenu),
        KeyCode::Char('q') => Some(Message::QuitApp),
        KeyCode::Char('s') if matches!(state.screen, TypedScreen::Profile(_)) => {
            Some(Message::ShowStatistics)
        }
        KeyCode::Char('p') if matches!(state.screen, TypedScreen::Statistics(_)) => {
            Some(Message::ShowProfile)
        }
        _ => None,
    }
}

/// Handle keyboard events on the main menu screen
pub fn handle_menu_keys(key: KeyEvent, state: &AppState) -> Option<Message> {
    match key.code {
        KeyCode::Char('q') => Some(Message::QuitApp),
        KeyCode::Char('r') => Some(Message::StartReviewSession),
        KeyCode::Char('p') => Some(Message::ShowProfile),
        KeyCode::Char('s') => Some(Message::ShowStatistics),
        KeyCode::Up | KeyCode::Char('k') => Some(Message::MenuUp),
        KeyCode::Down | KeyCode::Char('j') => Some(Message::MenuDown),
        KeyCode::Enter => Some(Message::MenuSelect),
        // Quick jump with number keys (1-9)
        KeyCode::Char(c) if c.is_ascii_digit() => {
            let digit = c
                .to_digit(10)
                .expect("char is validated as ascii_digit by guard condition")
                as usize;
            if digit >= 1 && digit <= state.game.scenario_collection.count() {
                // Jump to scenario (digit - 1 because scenarios are 0-indexed)
                Some(Message::StartScenario(digit - 1))
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Handle special UI keys on task screen (F1, ?, Ctrl+Q)
fn handle_task_special_keys(key: KeyEvent) -> Option<Message> {
    match (key.code, key.modifiers) {
        // F1 always shows hint
        (KeyCode::F(1), _) => Some(Message::ShowHint),
        // '?' key (might come as Char('?') or Char('/') with SHIFT depending on platform)
        (KeyCode::Char('?'), _)
            if !key.modifiers.contains(KeyModifiers::CONTROL)
                && !key.modifiers.contains(KeyModifiers::ALT) =>
        {
            Some(Message::ShowHint)
        }
        (KeyCode::Char('/'), KeyModifiers::SHIFT) => Some(Message::ShowHint),
        // Ctrl+Q abandons scenario (Esc is needed for Helix insert mode exit)
        (KeyCode::Char('q'), KeyModifiers::CONTROL) => Some(Message::AbandonScenario),
        _ => None,
    }
}

/// Check if current gameplay session is in Insert mode
///
/// Works for both training mode (Task screen) and arcade mode (MiniGame screen).
fn is_gameplay_insert_mode(state: &AppState) -> bool {
    match &state.screen {
        TypedScreen::Task(task_data) => task_data.session.is_insert_mode(),
        TypedScreen::MiniGame(_) => state
            .game
            .minigame_session
            .as_ref()
            .map(|s| s.is_insert_mode())
            .unwrap_or(false),
        _ => false,
    }
}

/// Handle gameplay input (insert mode or normal mode command)
///
/// Shared logic for both training and arcade modes.
/// Returns the command wrapped in the provided message constructor.
fn handle_gameplay_input<F>(key: KeyEvent, state: &AppState, make_message: F) -> Option<Message>
where
    F: FnOnce(Cow<'static, str>) -> Message,
{
    if is_gameplay_insert_mode(state) {
        handle_insert_mode_input(key).map(make_message)
    } else {
        // Check if we're waiting for a character argument (e.g., after 'r', 'f', 't')
        if is_waiting_for_char_arg(state) {
            match key.code {
                // Accept any printable character as the argument
                KeyCode::Char(c) => {
                    return Some(make_message(Cow::Owned(c.to_string())));
                }
                // Esc or non-char keys cancel the pending command
                // Send a marker that will make the buffer invalid, triggering clear
                KeyCode::Esc
                | KeyCode::Left
                | KeyCode::Right
                | KeyCode::Up
                | KeyCode::Down
                | KeyCode::Backspace
                | KeyCode::Enter => {
                    return Some(make_message(Cow::Borrowed("<cancel>")));
                }
                _ => {}
            }
        }
        map_key_to_helix_command(key).map(|cmd| make_message(Cow::Borrowed(cmd)))
    }
}

/// Handle keyboard events on the task screen
pub fn handle_task_keys(key: KeyEvent, state: &AppState) -> Option<Message> {
    // Check special UI keys first
    if let Some(msg) = handle_task_special_keys(key) {
        return Some(msg);
    }

    // Handle gameplay input (insert mode or normal mode)
    handle_gameplay_input(key, state, Message::ExecuteCommand)
}

/// Handle keyboard events on the results screen
pub fn handle_results_keys(key: KeyEvent) -> Option<Message> {
    match key.code {
        KeyCode::Char('q') => Some(Message::QuitApp),
        KeyCode::Char('r') => Some(Message::RetryScenario),
        KeyCode::Char('m') => Some(Message::BackToMenu),
        KeyCode::Char('p') => Some(Message::ShowProfile),
        _ => None,
    }
}

/// Handle keyboard events on the review session screen
pub fn handle_review_keys(key: KeyEvent) -> Option<Message> {
    match key.code {
        KeyCode::Char('s') => Some(Message::CompleteReviewCommand { success: true }),
        KeyCode::Char('f') => Some(Message::CompleteReviewCommand { success: false }),
        KeyCode::Esc => Some(Message::AbandonReviewSession),
        KeyCode::Char('q') => Some(Message::QuitApp),
        _ => None,
    }
}

/// Handle keyboard events on mode selection screen
pub fn handle_mode_selection_keys(key: KeyEvent) -> Option<Message> {
    match key.code {
        KeyCode::Char('q') => Some(Message::QuitApp),
        KeyCode::Up | KeyCode::Char('k') => Some(Message::ModeSelectionUp),
        KeyCode::Down | KeyCode::Char('j') => Some(Message::ModeSelectionDown),
        KeyCode::Enter => Some(Message::ModeSelectionSelect),
        _ => None,
    }
}

/// Handle keyboard events on mini-game screen
pub fn handle_minigame_keys(key: KeyEvent, state: &AppState) -> Option<Message> {
    // Check if game over or paused
    let session = state.game.minigame_session.as_ref()?;

    if session.state().is_game_over() {
        // Game over - only allow quit or back to menu
        return match key.code {
            KeyCode::Char('q') => Some(Message::QuitApp),
            KeyCode::Esc | KeyCode::Char('m') => Some(Message::MiniGameBackToMenu),
            _ => None,
        };
    }

    if session.state().is_paused() {
        // Paused - allow resume, quit, back to menu, or view profile/stats
        return match key.code {
            KeyCode::Esc => Some(Message::ResumeMiniGame),
            KeyCode::Char('q') => Some(Message::MiniGameBackToMenu),
            KeyCode::Char('p') => Some(Message::ShowProfile),
            KeyCode::Char('s') => Some(Message::ShowStatistics),
            _ => None,
        };
    }

    // Handle Ctrl+Q for quit
    if key.code == KeyCode::Char('q') && key.modifiers.contains(KeyModifiers::CONTROL) {
        return Some(Message::QuitApp);
    }

    // In playing state - handle input using shared gameplay logic
    if session.state().is_playing() {
        // In insert mode - use shared handler (includes Esc to exit insert)
        if is_gameplay_insert_mode(state) {
            return handle_gameplay_input(key, state, Message::MiniGameCommand);
        }

        // Normal mode - Esc pauses, other keys are commands
        if key.code == KeyCode::Esc {
            return Some(Message::PauseMiniGame);
        }

        return handle_gameplay_input(key, state, Message::MiniGameCommand);
    }

    // Countdown state - Esc pauses
    if key.code == KeyCode::Esc {
        return Some(Message::PauseMiniGame);
    }

    None
}

#[cfg(test)]
#[allow(unused_variables)]
mod tests {
    use super::*;
    use helix_trainer::{
        config::Scenario,
        game::GameSession,
        gamification::{ProfileStorage, UserProfile},
        learning::PerformanceTracker,
        ui::state::{MenuData, TaskData},
    };

    fn create_test_app_state() -> AppState {
        let profile = UserProfile::new();
        let storage = ProfileStorage::new();
        let tracker = PerformanceTracker::new();
        AppState::new(vec![], profile, storage, tracker)
    }

    #[test]
    fn test_menu_key_q_quits() {
        let key = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE);
        let state = create_test_app_state();
        let msg = handle_menu_keys(key, &state);
        assert_eq!(msg, Some(Message::QuitApp));
    }

    #[test]
    fn test_menu_key_j_moves_down() {
        let key = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE);
        let state = create_test_app_state();
        let msg = handle_menu_keys(key, &state);
        assert_eq!(msg, Some(Message::MenuDown));
    }

    #[test]
    fn test_menu_key_k_moves_up() {
        let key = KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE);
        let state = create_test_app_state();
        let msg = handle_menu_keys(key, &state);
        assert_eq!(msg, Some(Message::MenuUp));
    }

    #[test]
    fn test_menu_key_enter_selects() {
        let key = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        let state = create_test_app_state();
        let msg = handle_menu_keys(key, &state);
        assert_eq!(msg, Some(Message::MenuSelect));
    }

    #[test]
    fn test_task_key_f1_shows_hint() {
        let key = KeyEvent::new(KeyCode::F(1), KeyModifiers::NONE);
        let state = create_test_app_state();
        let msg = handle_task_keys(key, &state);
        assert_eq!(msg, Some(Message::ShowHint));
    }

    #[test]
    fn test_task_key_question_shows_hint() {
        let key = KeyEvent::new(KeyCode::Char('?'), KeyModifiers::NONE);
        let state = create_test_app_state();
        let msg = handle_task_keys(key, &state);
        assert_eq!(msg, Some(Message::ShowHint));
    }

    #[test]
    fn test_task_key_h_moves_left() {
        use helix_trainer::helix::commands::CMD_MOVE_LEFT;
        let key = KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE);
        let state = create_test_app_state();
        let msg = handle_task_keys(key, &state);
        assert_eq!(
            msg,
            Some(Message::ExecuteCommand(Cow::Borrowed(CMD_MOVE_LEFT)))
        );
    }

    #[test]
    fn test_task_key_ctrl_q_abandons() {
        let key = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::CONTROL);
        let state = create_test_app_state();
        let msg = handle_task_keys(key, &state);
        assert_eq!(msg, Some(Message::AbandonScenario));
    }

    #[test]
    fn test_results_key_r_retries() {
        let key = KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE);
        let msg = handle_results_keys(key);
        assert_eq!(msg, Some(Message::RetryScenario));
    }

    #[test]
    fn test_results_key_m_returns_menu() {
        let key = KeyEvent::new(KeyCode::Char('m'), KeyModifiers::NONE);
        let msg = handle_results_keys(key);
        assert_eq!(msg, Some(Message::BackToMenu));
    }

    #[test]
    fn test_results_key_q_quits() {
        let key = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE);
        let msg = handle_results_keys(key);
        assert_eq!(msg, Some(Message::QuitApp));
    }

    #[test]
    fn test_unknown_key_returns_none() {
        let key = KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE);
        let state = create_test_app_state();
        let msg = handle_menu_keys(key, &state);
        assert_eq!(msg, None);
    }

    // Unit tests for is_gameplay_insert_mode()
    mod is_gameplay_insert_mode_tests {
        use super::*;

        #[test]
        fn test_is_gameplay_insert_mode_on_menu_screen() {
            let mut state = create_test_app_state();
            state.screen = TypedScreen::Menu(MenuData::default());
            assert!(!is_gameplay_insert_mode(&state));
        }

        #[test]
        fn test_is_gameplay_insert_mode_on_task_screen_normal_mode() {
            use helix_trainer::config::{ScoringConfig, Setup, Solution, TargetState};

            let mut state = create_test_app_state();

            // Create a simple scenario
            let scenario = Scenario {
                id: "test".to_string(),
                name: "Test".to_string(),
                description: "Test scenario".to_string(),
                setup: Setup {
                    file_content: "test".to_string(),
                    cursor_position: (0, 0),
                },
                target: TargetState {
                    file_content: "test2".to_string(),
                    cursor_position: (0, 0),
                    selection: None,
                },
                solution: Solution {
                    commands: vec!["x".to_string()],
                    description: "Delete char".to_string(),
                },
                scoring: ScoringConfig {
                    optimal_count: 1,
                    max_points: 100,
                    tolerance: 0,
                },
                hints: vec![],
                alternatives: vec![],
                metadata: None,
            };

            let session = GameSession::new(scenario).unwrap();
            state.screen = TypedScreen::Task(TaskData::new(session));

            // GameSession starts in Normal mode
            assert!(!is_gameplay_insert_mode(&state));
        }
    }

    // Unit tests for handle_task_special_keys()
    mod handle_task_special_keys_tests {
        use super::*;

        #[test]
        fn test_handle_task_special_keys_f1() {
            let key = KeyEvent::new(KeyCode::F(1), KeyModifiers::NONE);
            assert_eq!(handle_task_special_keys(key), Some(Message::ShowHint));
        }

        #[test]
        fn test_handle_task_special_keys_question_mark() {
            let key = KeyEvent::new(KeyCode::Char('?'), KeyModifiers::NONE);
            assert_eq!(handle_task_special_keys(key), Some(Message::ShowHint));
        }

        #[test]
        fn test_handle_task_special_keys_ctrl_q() {
            let key = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::CONTROL);
            assert_eq!(
                handle_task_special_keys(key),
                Some(Message::AbandonScenario)
            );
        }

        #[test]
        fn test_handle_task_special_keys_regular_key() {
            let key = KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE);
            assert_eq!(handle_task_special_keys(key), None);
        }

        #[test]
        fn test_handle_task_special_keys_escape() {
            let key = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
            assert_eq!(handle_task_special_keys(key), None);
        }
    }
}
