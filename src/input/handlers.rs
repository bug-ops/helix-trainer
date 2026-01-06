//! Screen-specific keyboard event handlers
//!
//! Each handler function processes keyboard input for a specific screen.
//!
//! # Mode-Safe Key Mapping
//!
//! This module uses the typestate-based key mapping system from [`super::typestate`].
//! The convenience functions (`map_key_to_helix_command`, `handle_insert_mode_input`)
//! provide simple key-to-command mapping.
//!
//! For advanced input handling with multi-key sequences, use `InputStateMachine`:
//!
//! ```ignore
//! use super::typestate::{InputStateMachine, HandlerResult};
//!
//! let mut state_machine = InputStateMachine::new();
//! let result = state_machine.process_key(key);
//! match result {
//!     HandlerResult::Execute(cmd) => { /* execute command */ }
//!     HandlerResult::Transition(_) => { /* waiting for more input */ }
//!     HandlerResult::Cancel => { /* cancelled */ }
//!     HandlerResult::Stay => { /* no change */ }
//! }
//! ```

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::borrow::Cow;

use crate::helix::commands::CMD_CANCEL;
use crate::ui::state::InputStateAccess;
use crate::ui::{AppState, Message, state::TypedScreen};

use super::typestate::{handle_insert_mode_input, map_key_to_helix_command};

/// Check if the input state machine is waiting for a character argument
///
/// Uses the typestate-based `InputStateMachine` to determine if we're in a
/// state that expects character input (FindCharPending, ReplaceCharPending)
/// or waiting for the second key in a multi-key sequence (GotoPending, etc.).
fn is_waiting_for_char_arg(state: &AppState) -> bool {
    match &state.screen {
        TypedScreen::Task(task_data) => task_data.input_state().is_prefix_state(),
        TypedScreen::MiniGame(minigame_data) => minigame_data.input_state().is_prefix_state(),
        _ => false,
    }
}

/// Check if the input state machine is building a count prefix (digits like "3", "12")
///
/// Uses the typestate-based `InputStateMachine` to check if we're in
/// CountPending state (building a numeric prefix for a command).
fn is_building_count_prefix(state: &AppState) -> bool {
    match &state.screen {
        TypedScreen::Task(task_data) => task_data.input_state().state().is_count_pending(),
        TypedScreen::MiniGame(minigame_data) => {
            minigame_data.input_state().state().is_count_pending()
        }
        _ => false,
    }
}

/// Handle keyboard events on profile and statistics screens
pub fn handle_profile_stats_keys(key: KeyEvent, state: &AppState) -> Option<Message> {
    // Ctrl-Q returns to menu (unified exit key)
    if key.code == KeyCode::Char('q') && key.modifiers.contains(KeyModifiers::CONTROL) {
        return Some(Message::BackToMenu);
    }

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

/// Get menu command buffer from state
fn get_menu_buffer(state: &AppState) -> &str {
    match &state.screen {
        TypedScreen::Menu(data) => &data.command_buffer,
        _ => "",
    }
}

/// Parse menu command buffer and return appropriate message
///
/// Handles:
/// - Count prefixes (3j, 10k)
/// - Jump commands (gg, G, 15G, 15gg)
/// - Simple navigation (j, k)
fn parse_menu_command(buffer: &str) -> Option<Message> {
    if buffer.is_empty() {
        return None;
    }

    // Check for gg (goto first)
    if buffer == "gg" {
        return Some(Message::MenuJumpToFirst);
    }

    // Check for G (goto last) or {n}G (goto line n)
    if buffer == "G" {
        return Some(Message::MenuJumpToLast);
    }

    // Check for {n}G pattern
    if let Some(num_str) = buffer.strip_suffix('G')
        && let Ok(n) = num_str.parse::<usize>()
    {
        return Some(Message::MenuJumpTo(n));
    }

    // Check for {n}gg pattern
    if let Some(num_str) = buffer.strip_suffix("gg")
        && !num_str.is_empty()
        && let Ok(n) = num_str.parse::<usize>()
    {
        return Some(Message::MenuJumpTo(n));
    }

    // Check for count + j/k pattern
    if buffer.ends_with('j') || buffer.ends_with('k') {
        let direction = buffer.chars().last().unwrap();
        let count_str = &buffer[..buffer.len() - 1];

        if count_str.is_empty() {
            // Simple j or k
            return match direction {
                'j' => Some(Message::MenuDown),
                'k' => Some(Message::MenuUp),
                _ => None,
            };
        }

        if let Ok(count) = count_str.parse::<usize>() {
            return match direction {
                'j' => Some(Message::MenuDownBy(count)),
                'k' => Some(Message::MenuUpBy(count)),
                _ => None,
            };
        }
    }

    None
}

/// Check if menu buffer is in a partial state (waiting for more input)
fn is_menu_buffer_partial(buffer: &str) -> bool {
    if buffer.is_empty() {
        return false;
    }

    // "g" alone is partial (waiting for second g)
    if buffer == "g" {
        return true;
    }

    // Digits alone are partial (waiting for j/k/g/G)
    if buffer.chars().all(|c| c.is_ascii_digit()) {
        return true;
    }

    // Digits followed by "g" is partial (waiting for second g)
    if buffer.ends_with('g') && buffer.len() > 1 {
        let prefix = &buffer[..buffer.len() - 1];
        if prefix.chars().all(|c| c.is_ascii_digit()) {
            return true;
        }
    }

    false
}

/// Handle keyboard events on the main menu screen
///
/// Supports Helix-style navigation:
/// - j/k: move down/up
/// - 5j/10k: move down/up by count
/// - gg: jump to first item
/// - G: jump to last item
/// - 15G/15gg: jump to item 15
pub fn handle_menu_keys(key: KeyEvent, state: &mut AppState) -> Option<Message> {
    // Ctrl-Q exits application from anywhere
    if key.code == KeyCode::Char('q') && key.modifiers.contains(KeyModifiers::CONTROL) {
        return Some(Message::QuitApp);
    }

    // Get current buffer
    let buffer = get_menu_buffer(state);

    // Handle Escape - clear buffer
    if key.code == KeyCode::Esc {
        if let TypedScreen::Menu(ref mut data) = state.screen
            && !data.command_buffer.is_empty()
        {
            data.command_buffer.clear();
            return None; // Consumed the escape
        }
        return None;
    }

    // Handle Enter - always select
    if key.code == KeyCode::Enter {
        // Clear buffer first
        if let TypedScreen::Menu(ref mut data) = state.screen {
            data.command_buffer.clear();
        }
        return Some(Message::MenuSelect);
    }

    // Arrow keys bypass command buffer
    if key.code == KeyCode::Up {
        if let TypedScreen::Menu(ref mut data) = state.screen {
            data.command_buffer.clear();
        }
        return Some(Message::MenuUp);
    }
    if key.code == KeyCode::Down {
        if let TypedScreen::Menu(ref mut data) = state.screen {
            data.command_buffer.clear();
        }
        return Some(Message::MenuDown);
    }

    // Handle character input
    if let KeyCode::Char(c) = key.code {
        // Special single-key commands (when buffer is empty)
        if buffer.is_empty() {
            match c {
                'q' => return Some(Message::QuitApp),
                'r' => return Some(Message::StartReviewSession),
                'p' => return Some(Message::ShowProfile),
                's' => return Some(Message::ShowStatistics),
                'G' => return Some(Message::MenuJumpToLast),
                _ => {}
            }
        }

        // Build new buffer
        let new_buffer = format!("{}{}", buffer, c);

        // Check if it's a complete command
        if let Some(msg) = parse_menu_command(&new_buffer) {
            // Clear buffer and return message
            if let TypedScreen::Menu(ref mut data) = state.screen {
                data.command_buffer.clear();
            }
            return Some(msg);
        }

        // Check if it's a valid partial state
        if is_menu_buffer_partial(&new_buffer) {
            // Update buffer and wait for more input
            if let TypedScreen::Menu(ref mut data) = state.screen {
                data.command_buffer = new_buffer;
            }
            return None;
        }

        // Invalid sequence - clear buffer
        if let TypedScreen::Menu(ref mut data) = state.screen {
            data.command_buffer.clear();
        }
    }

    None
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
                    return Some(make_message(Cow::Borrowed(CMD_CANCEL)));
                }
                _ => {}
            }
        }

        // Check if we're building a count prefix (e.g., "3", "12")
        // Accept digits to continue building the count, or a command to complete
        if is_building_count_prefix(state) {
            if let KeyCode::Char(c) = key.code {
                // Accept more digits or a command character
                return Some(make_message(Cow::Owned(c.to_string())));
            }
            // Non-char keys cancel the pending count
            if matches!(
                key.code,
                KeyCode::Esc
                    | KeyCode::Left
                    | KeyCode::Right
                    | KeyCode::Up
                    | KeyCode::Down
                    | KeyCode::Backspace
                    | KeyCode::Enter
            ) {
                return Some(make_message(Cow::Borrowed(CMD_CANCEL)));
            }
        }

        // Handle digit keys (1-9) to start count prefix
        // Note: '0' is a command (goto line start), not a count prefix start
        if let KeyCode::Char(c) = key.code
            && c.is_ascii_digit()
            && c != '0'
        {
            return Some(make_message(Cow::Owned(c.to_string())));
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
    // Ctrl-Q returns to menu (unified exit key)
    if key.code == KeyCode::Char('q') && key.modifiers.contains(KeyModifiers::CONTROL) {
        return Some(Message::BackToMenu);
    }

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
    // Ctrl-Q abandons review session (unified exit key)
    if key.code == KeyCode::Char('q') && key.modifiers.contains(KeyModifiers::CONTROL) {
        return Some(Message::AbandonReviewSession);
    }

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
    // Ctrl-Q exits application (unified exit key - mode selection is root screen)
    if key.code == KeyCode::Char('q') && key.modifiers.contains(KeyModifiers::CONTROL) {
        return Some(Message::QuitApp);
    }

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

    // Ctrl-Q returns to menu from any minigame state (unified exit key)
    if key.code == KeyCode::Char('q') && key.modifiers.contains(KeyModifiers::CONTROL) {
        return Some(Message::MiniGameBackToMenu);
    }

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
    use crate::{
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
        let mut state = create_test_app_state();
        state.screen = TypedScreen::Menu(MenuData::default());
        let msg = handle_menu_keys(key, &mut state);
        assert_eq!(msg, Some(Message::QuitApp));
    }

    #[test]
    fn test_menu_key_j_moves_down() {
        let key = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE);
        let mut state = create_test_app_state();
        state.screen = TypedScreen::Menu(MenuData::default());
        let msg = handle_menu_keys(key, &mut state);
        assert_eq!(msg, Some(Message::MenuDown));
    }

    #[test]
    fn test_menu_key_k_moves_up() {
        let key = KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE);
        let mut state = create_test_app_state();
        state.screen = TypedScreen::Menu(MenuData::default());
        let msg = handle_menu_keys(key, &mut state);
        assert_eq!(msg, Some(Message::MenuUp));
    }

    #[test]
    fn test_menu_key_enter_selects() {
        let key = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        let mut state = create_test_app_state();
        state.screen = TypedScreen::Menu(MenuData::default());
        let msg = handle_menu_keys(key, &mut state);
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
        use crate::helix::commands::CMD_MOVE_LEFT;
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
    fn test_results_key_ctrl_q_returns_menu() {
        let key = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::CONTROL);
        let msg = handle_results_keys(key);
        assert_eq!(msg, Some(Message::BackToMenu));
    }

    #[test]
    fn test_menu_key_ctrl_q_quits() {
        let key = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::CONTROL);
        let mut state = create_test_app_state();
        state.screen = TypedScreen::Menu(MenuData::default());
        let msg = handle_menu_keys(key, &mut state);
        assert_eq!(msg, Some(Message::QuitApp));
    }

    #[test]
    fn test_mode_selection_key_ctrl_q_quits() {
        let key = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::CONTROL);
        let msg = handle_mode_selection_keys(key);
        assert_eq!(msg, Some(Message::QuitApp));
    }

    #[test]
    fn test_review_key_ctrl_q_abandons() {
        let key = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::CONTROL);
        let msg = handle_review_keys(key);
        assert_eq!(msg, Some(Message::AbandonReviewSession));
    }

    #[test]
    fn test_profile_stats_key_ctrl_q_returns_menu() {
        let key = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::CONTROL);
        let state = create_test_app_state();
        let msg = handle_profile_stats_keys(key, &state);
        assert_eq!(msg, Some(Message::BackToMenu));
    }

    #[test]
    fn test_unknown_key_returns_none() {
        let key = KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE);
        let mut state = create_test_app_state();
        state.screen = TypedScreen::Menu(MenuData::default());
        let msg = handle_menu_keys(key, &mut state);
        assert_eq!(msg, None);
    }

    // Tests for Helix-style menu navigation
    mod menu_navigation_tests {
        use super::*;

        #[test]
        fn test_menu_gg_jumps_to_first() {
            let mut state = create_test_app_state();
            state.screen = TypedScreen::Menu(MenuData::default());

            // Press 'g' - partial, no message yet
            let key_g = KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE);
            let msg = handle_menu_keys(key_g, &mut state);
            assert_eq!(msg, None);

            // Press second 'g' - complete command
            let msg = handle_menu_keys(key_g, &mut state);
            assert_eq!(msg, Some(Message::MenuJumpToFirst));
        }

        #[test]
        fn test_menu_shift_g_jumps_to_last() {
            let mut state = create_test_app_state();
            state.screen = TypedScreen::Menu(MenuData::default());

            let key = KeyEvent::new(KeyCode::Char('G'), KeyModifiers::NONE);
            let msg = handle_menu_keys(key, &mut state);
            assert_eq!(msg, Some(Message::MenuJumpToLast));
        }

        #[test]
        fn test_menu_5j_moves_down_5() {
            let mut state = create_test_app_state();
            state.screen = TypedScreen::Menu(MenuData::default());

            // Press '5' - partial
            let key_5 = KeyEvent::new(KeyCode::Char('5'), KeyModifiers::NONE);
            let msg = handle_menu_keys(key_5, &mut state);
            assert_eq!(msg, None);

            // Press 'j' - complete
            let key_j = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE);
            let msg = handle_menu_keys(key_j, &mut state);
            assert_eq!(msg, Some(Message::MenuDownBy(5)));
        }

        #[test]
        fn test_menu_10k_moves_up_10() {
            let mut state = create_test_app_state();
            state.screen = TypedScreen::Menu(MenuData::default());

            // Press '1' - partial
            let key_1 = KeyEvent::new(KeyCode::Char('1'), KeyModifiers::NONE);
            let msg = handle_menu_keys(key_1, &mut state);
            assert_eq!(msg, None);

            // Press '0' - partial
            let key_0 = KeyEvent::new(KeyCode::Char('0'), KeyModifiers::NONE);
            let msg = handle_menu_keys(key_0, &mut state);
            assert_eq!(msg, None);

            // Press 'k' - complete
            let key_k = KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE);
            let msg = handle_menu_keys(key_k, &mut state);
            assert_eq!(msg, Some(Message::MenuUpBy(10)));
        }

        #[test]
        fn test_menu_15_shift_g_jumps_to_15() {
            let mut state = create_test_app_state();
            state.screen = TypedScreen::Menu(MenuData::default());

            // Press '1' - partial
            let key_1 = KeyEvent::new(KeyCode::Char('1'), KeyModifiers::NONE);
            let msg = handle_menu_keys(key_1, &mut state);
            assert_eq!(msg, None);

            // Press '5' - partial
            let key_5 = KeyEvent::new(KeyCode::Char('5'), KeyModifiers::NONE);
            let msg = handle_menu_keys(key_5, &mut state);
            assert_eq!(msg, None);

            // Press 'G' - complete
            let key_shift_g = KeyEvent::new(KeyCode::Char('G'), KeyModifiers::NONE);
            let msg = handle_menu_keys(key_shift_g, &mut state);
            assert_eq!(msg, Some(Message::MenuJumpTo(15)));
        }

        #[test]
        fn test_menu_15gg_jumps_to_15() {
            let mut state = create_test_app_state();
            state.screen = TypedScreen::Menu(MenuData::default());

            // Press "15gg"
            let key_1 = KeyEvent::new(KeyCode::Char('1'), KeyModifiers::NONE);
            handle_menu_keys(key_1, &mut state);

            let key_5 = KeyEvent::new(KeyCode::Char('5'), KeyModifiers::NONE);
            handle_menu_keys(key_5, &mut state);

            let key_g = KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE);
            handle_menu_keys(key_g, &mut state);

            let msg = handle_menu_keys(key_g, &mut state);
            assert_eq!(msg, Some(Message::MenuJumpTo(15)));
        }

        #[test]
        fn test_menu_escape_clears_buffer() {
            let mut state = create_test_app_state();
            state.screen = TypedScreen::Menu(MenuData::default());

            // Start building "5j"
            let key_5 = KeyEvent::new(KeyCode::Char('5'), KeyModifiers::NONE);
            handle_menu_keys(key_5, &mut state);

            // Press Escape - should clear buffer
            let key_esc = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
            handle_menu_keys(key_esc, &mut state);

            // Now press 'j' - should be simple down, not 5j
            let key_j = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE);
            let msg = handle_menu_keys(key_j, &mut state);
            assert_eq!(msg, Some(Message::MenuDown));
        }

        #[test]
        fn test_menu_arrow_keys_still_work() {
            let mut state = create_test_app_state();
            state.screen = TypedScreen::Menu(MenuData::default());

            let key_up = KeyEvent::new(KeyCode::Up, KeyModifiers::NONE);
            let msg = handle_menu_keys(key_up, &mut state);
            assert_eq!(msg, Some(Message::MenuUp));

            let key_down = KeyEvent::new(KeyCode::Down, KeyModifiers::NONE);
            let msg = handle_menu_keys(key_down, &mut state);
            assert_eq!(msg, Some(Message::MenuDown));
        }
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
            use crate::config::{ScoringConfig, Setup, Solution, TargetState};

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
