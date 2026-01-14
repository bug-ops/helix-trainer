//! Screen-specific keyboard event handlers
//!
//! Each handler function processes keyboard input for a specific screen.
//!
//! # Key Mapping Architecture
//!
//! This module converts `KeyEvent` to `Message` for the Elm Architecture.
//! Multi-key command handling (gg, dd, fx, rx, 3j) is delegated to
//! `InputStateMachine` in the command handlers (`gameplay.rs`, `minigame.rs`).
//!
//! ## Data Flow
//!
//! ```text
//! KeyEvent -> handle_*_keys() -> Message::ExecuteCommand
//!          -> update() -> handle_execute_command()
//!          -> InputStateMachine::process_key() -> execute
//! ```
//!
//! For menu navigation, a separate command buffer pattern is used
//! (see `MenuData::command_buffer`).

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::borrow::Cow;

use crate::ui::{AppState, Message, Screen, state::TypedScreen};

use super::typestate::{handle_insert_mode_input, map_key_to_helix_command, normalize_key_event};

/// Handle keyboard events on profile and statistics screens
pub fn handle_profile_stats_keys(key: KeyEvent, state: &AppState) -> Option<Message> {
    // Ctrl-Q returns to menu (unified exit key)
    if key.code == KeyCode::Char('q') && key.modifiers.contains(KeyModifiers::CONTROL) {
        return Some(Message::BackToMenu);
    }

    match key.code {
        KeyCode::Esc | KeyCode::Char('m') => Some(Message::BackToMenu),
        KeyCode::Char('q') => Some(Message::QuitApp),
        KeyCode::Char('r') => Some(Message::StartReviewSession),
        KeyCode::Char('s') if matches!(state.screen, TypedScreen::Profile(_)) => {
            Some(Message::ShowStatistics)
        }
        KeyCode::Char('p') if matches!(state.screen, TypedScreen::Statistics(_)) => {
            Some(Message::ShowProfile)
        }
        KeyCode::Char('M') => Some(Message::ToggleSound),
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

    // Handle Escape - clear buffer or go back to mode selection
    if key.code == KeyCode::Esc {
        if let TypedScreen::Menu(ref mut data) = state.screen
            && !data.command_buffer.is_empty()
        {
            data.command_buffer.clear();
            return None; // Consumed the escape
        }
        // Buffer is empty - go back to mode selection
        return Some(Message::NavigateTo(Screen::ModeSelection));
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
                'm' => return Some(Message::NavigateTo(Screen::ModeSelection)),
                'r' => return Some(Message::StartReviewSession),
                'p' => return Some(Message::ShowProfile),
                's' => return Some(Message::ShowStatistics),
                'f' => return Some(Message::ShowCategoryFilters),
                'G' => return Some(Message::MenuJumpToLast),
                'M' => return Some(Message::ToggleSound),
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
/// In normal mode, passes keys directly to the message handler where
/// `InputStateMachine` processes multi-key sequences (gg, dd, fx, rx, 3j).
fn handle_gameplay_input<F>(key: KeyEvent, state: &AppState, make_message: F) -> Option<Message>
where
    F: FnOnce(Cow<'static, str>) -> Message,
{
    if is_gameplay_insert_mode(state) {
        // Insert mode: use dedicated handler
        handle_insert_mode_input(key).map(make_message)
    } else {
        // Normal mode: convert key to string for InputStateMachine
        // State machine handles multi-key commands in gameplay.rs/minigame.rs
        key_to_command_string(key).map(make_message)
    }
}

/// Convert a key event to a command string for the state machine
///
/// In normal mode, all keys are passed to `InputStateMachine` which handles:
/// - Multi-key commands (gg, dd)
/// - Character arguments (fx, rx)
/// - Count prefixes (3j, 10k)
/// - Modifier commands (Alt-C, Ctrl-r)
///
/// Uses `map_key_to_helix_command` for known commands (which returns constants
/// like CMD_COPY_SELECTION_PREV = "Alt-C"), and builds strings for prefix keys
/// that the state machine needs to handle.
fn key_to_command_string(key: KeyEvent) -> Option<Cow<'static, str>> {
    // First try to get the command constant from map_key_to_helix_command
    // This handles all known single-key commands including modifier commands
    if let Some(cmd) = map_key_to_helix_command(key) {
        return Some(Cow::Borrowed(cmd));
    }

    // For unknown commands (prefix keys like 'g', 'm', 'f', etc.),
    // normalize and return the character for the state machine to process
    let key = normalize_key_event(key);

    match key.code {
        KeyCode::Char(c) => Some(Cow::Owned(c.to_string())),
        KeyCode::Enter => Some(Cow::Borrowed("Enter")),
        KeyCode::Tab => Some(Cow::Borrowed("Tab")),
        _ => None,
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
///
/// Key bindings:
/// - `r` - Retry current scenario
/// - `n` - Navigate to next lesson in filtered list
/// - `l` - Navigate to scenario list (Menu screen)
/// - `m` - Return to mode selection (main menu)
/// - `p` - Show profile
/// - `q` - Quit application
/// - `Ctrl-Q` - Return to mode selection
pub fn handle_results_keys(key: KeyEvent) -> Option<Message> {
    // Ctrl-Q returns to menu (unified exit key)
    if key.code == KeyCode::Char('q') && key.modifiers.contains(KeyModifiers::CONTROL) {
        return Some(Message::BackToMenu);
    }

    match key.code {
        KeyCode::Char('q') => Some(Message::QuitApp),
        KeyCode::Char('r') => Some(Message::RetryScenario),
        KeyCode::Char('n') => Some(Message::NextLesson),
        KeyCode::Char('l') => Some(Message::GoToScenarioList),
        KeyCode::Char('m') => Some(Message::BackToMenu),
        KeyCode::Char('p') => Some(Message::ShowProfile),
        KeyCode::Char('M') => Some(Message::ToggleSound),
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

/// Handle keyboard events on category filters screen
///
/// Key bindings:
/// - `j` or `Down` - Move selection down (CategoryFilterDown)
/// - `k` or `Up` - Move selection up (CategoryFilterUp)
/// - `Space` or `Enter` - Toggle selected category filter (CategoryFilterToggle)
/// - `a` - Select all (clear filters) (CategoryFilterSelectAll)
/// - `Esc` or `q` - Return to previous screen (BackToMenu)
/// - `Ctrl-Q` - Return to previous screen (BackToMenu)
pub fn handle_category_filters_keys(key: KeyEvent) -> Option<Message> {
    // Ctrl-Q returns to previous screen (unified exit key)
    if key.code == KeyCode::Char('q') && key.modifiers.contains(KeyModifiers::CONTROL) {
        return Some(Message::BackToMenu);
    }

    match key.code {
        KeyCode::Down | KeyCode::Char('j') => Some(Message::CategoryFilterDown),
        KeyCode::Up | KeyCode::Char('k') => Some(Message::CategoryFilterUp),
        KeyCode::Char(' ') | KeyCode::Enter => Some(Message::CategoryFilterToggle),
        KeyCode::Char('a') => Some(Message::CategoryFilterSelectAll),
        KeyCode::Esc | KeyCode::Char('q') => Some(Message::BackToMenu),
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
        KeyCode::Esc => Some(Message::ModeSelectionBack),
        KeyCode::Up | KeyCode::Char('k') => Some(Message::ModeSelectionUp),
        KeyCode::Down | KeyCode::Char('j') => Some(Message::ModeSelectionDown),
        KeyCode::Enter => Some(Message::ModeSelectionSelect),
        KeyCode::Char('r') => Some(Message::StartReviewSession),
        KeyCode::Char('p') => Some(Message::ShowProfile),
        KeyCode::Char('s') => Some(Message::ShowStatistics),
        KeyCode::Char('M') => Some(Message::ToggleSound),
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
        // Paused - allow resume, quit, back to menu, view profile/stats, or toggle sound
        return match key.code {
            KeyCode::Esc => Some(Message::ResumeMiniGame),
            KeyCode::Char('q') => Some(Message::MiniGameBackToMenu),
            KeyCode::Char('p') => Some(Message::ShowProfile),
            KeyCode::Char('s') => Some(Message::ShowStatistics),
            KeyCode::Char('M') => Some(Message::ToggleSound),
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
        testing::empty_test_app_state,
        ui::state::{MenuData, TaskData},
    };

    fn create_test_app_state() -> AppState {
        empty_test_app_state()
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
    fn test_results_key_n_next_lesson() {
        let key = KeyEvent::new(KeyCode::Char('n'), KeyModifiers::NONE);
        let msg = handle_results_keys(key);
        assert_eq!(msg, Some(Message::NextLesson));
    }

    #[test]
    fn test_results_key_l_goes_to_list() {
        let key = KeyEvent::new(KeyCode::Char('l'), KeyModifiers::NONE);
        let msg = handle_results_keys(key);
        assert_eq!(msg, Some(Message::GoToScenarioList));
    }

    #[test]
    fn test_results_key_p_shows_profile() {
        let key = KeyEvent::new(KeyCode::Char('p'), KeyModifiers::NONE);
        let msg = handle_results_keys(key);
        assert_eq!(msg, Some(Message::ShowProfile));
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
                    cursor_position: Some((0, 0)),
                    selection: None,
                    cursors: None,
                    selections: None,
                },
                target: TargetState {
                    file_content: "test2".to_string(),
                    cursor_position: Some((0, 0)),
                    selection: None,
                    cursors: None,
                    selections: None,
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

    // Test for Esc key in handle_mode_selection_keys()
    #[test]
    fn test_mode_selection_key_esc_goes_back() {
        let key = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
        let msg = handle_mode_selection_keys(key);
        assert_eq!(msg, Some(Message::ModeSelectionBack));
    }

    // CR-002: Test for 'M' key in handle_mode_selection_keys()
    #[test]
    fn test_mode_selection_key_m_toggles_sound() {
        let key = KeyEvent::new(KeyCode::Char('M'), KeyModifiers::NONE);
        let msg = handle_mode_selection_keys(key);
        assert_eq!(msg, Some(Message::ToggleSound));
    }

    // CR-003: Test for 'M' key in handle_menu_keys()
    #[test]
    fn test_menu_key_m_toggles_sound() {
        let key = KeyEvent::new(KeyCode::Char('M'), KeyModifiers::NONE);
        let mut state = create_test_app_state();
        state.screen = TypedScreen::Menu(MenuData::default());
        let msg = handle_menu_keys(key, &mut state);
        assert_eq!(msg, Some(Message::ToggleSound));
    }

    // Issue #138 Phase 5: Test for 'f' key in handle_menu_keys()
    #[test]
    fn test_menu_key_f_shows_category_filters() {
        let key = KeyEvent::new(KeyCode::Char('f'), KeyModifiers::NONE);
        let mut state = create_test_app_state();
        state.screen = TypedScreen::Menu(MenuData::default());
        let msg = handle_menu_keys(key, &mut state);
        assert_eq!(msg, Some(Message::ShowCategoryFilters));
    }

    // CR-004: Test for 'M' key in handle_results_keys()
    #[test]
    fn test_results_key_m_toggles_sound() {
        let key = KeyEvent::new(KeyCode::Char('M'), KeyModifiers::NONE);
        let msg = handle_results_keys(key);
        assert_eq!(msg, Some(Message::ToggleSound));
    }

    // CR-005: Test for 'M' key in handle_profile_stats_keys()
    #[test]
    fn test_profile_stats_key_m_toggles_sound() {
        let key = KeyEvent::new(KeyCode::Char('M'), KeyModifiers::NONE);
        let state = create_test_app_state();
        let msg = handle_profile_stats_keys(key, &state);
        assert_eq!(msg, Some(Message::ToggleSound));
    }

    // CR-006: Test for 'M' key in handle_minigame_keys() when paused
    #[test]
    fn test_minigame_key_m_toggles_sound_when_paused() {
        use crate::config::Difficulty;
        use crate::minigame::MiniGameSession;
        use crate::testing::ScenarioBuilder;
        use std::sync::Arc;

        let mut state = create_test_app_state();

        // Create minimal scenario for minigame session
        let scenario = ScenarioBuilder::new()
            .id("test_minigame")
            .difficulty(Difficulty::Beginner)
            .build();

        let scenarios = Arc::new(vec![scenario]);

        // Create and pause the minigame session
        let mut session = MiniGameSession::new(scenarios, None);
        session.start();
        // Complete countdown to get to Playing state
        session.tick_countdown();
        session.tick_countdown();
        session.tick_countdown();
        // Pause the game
        session.pause();

        // Set the session in game state
        state.game.minigame_session = Some(session);

        // Test that 'M' key returns ToggleSound when paused
        let key = KeyEvent::new(KeyCode::Char('M'), KeyModifiers::NONE);
        let msg = handle_minigame_keys(key, &state);
        assert_eq!(msg, Some(Message::ToggleSound));
    }

    // Unit tests for key_to_command_string()
    mod key_to_command_string_tests {
        use super::*;

        #[test]
        fn test_char_key() {
            let key = KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE);
            assert_eq!(
                key_to_command_string(key),
                Some(Cow::Owned("h".to_string()))
            );
        }

        #[test]
        fn test_uppercase_char_key() {
            let key = KeyEvent::new(KeyCode::Char('G'), KeyModifiers::SHIFT);
            assert_eq!(
                key_to_command_string(key),
                Some(Cow::Owned("G".to_string()))
            );
        }

        #[test]
        fn test_escape_key() {
            let key = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
            assert_eq!(key_to_command_string(key), Some(Cow::Borrowed("Escape")));
        }

        #[test]
        fn test_backspace_key() {
            let key = KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE);
            assert_eq!(key_to_command_string(key), Some(Cow::Borrowed("Backspace")));
        }

        #[test]
        fn test_arrow_keys() {
            assert_eq!(
                key_to_command_string(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE)),
                Some(Cow::Borrowed("Left"))
            );
            assert_eq!(
                key_to_command_string(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE)),
                Some(Cow::Borrowed("Right"))
            );
            assert_eq!(
                key_to_command_string(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE)),
                Some(Cow::Borrowed("Up"))
            );
            assert_eq!(
                key_to_command_string(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE)),
                Some(Cow::Borrowed("Down"))
            );
        }

        #[test]
        fn test_enter_key() {
            let key = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
            assert_eq!(key_to_command_string(key), Some(Cow::Borrowed("Enter")));
        }

        #[test]
        fn test_tab_key() {
            let key = KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE);
            assert_eq!(key_to_command_string(key), Some(Cow::Borrowed("Tab")));
        }

        #[test]
        fn test_unknown_key_returns_none() {
            let key = KeyEvent::new(KeyCode::F(5), KeyModifiers::NONE);
            assert_eq!(key_to_command_string(key), None);
        }

        #[test]
        fn test_home_key_returns_none() {
            let key = KeyEvent::new(KeyCode::Home, KeyModifiers::NONE);
            assert_eq!(key_to_command_string(key), None);
        }
    }

    mod handle_category_filters_keys_tests {
        use super::*;

        #[test]
        fn test_j_moves_down() {
            let key = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE);
            assert_eq!(
                handle_category_filters_keys(key),
                Some(Message::CategoryFilterDown)
            );
        }

        #[test]
        fn test_down_arrow_moves_down() {
            let key = KeyEvent::new(KeyCode::Down, KeyModifiers::NONE);
            assert_eq!(
                handle_category_filters_keys(key),
                Some(Message::CategoryFilterDown)
            );
        }

        #[test]
        fn test_k_moves_up() {
            let key = KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE);
            assert_eq!(
                handle_category_filters_keys(key),
                Some(Message::CategoryFilterUp)
            );
        }

        #[test]
        fn test_up_arrow_moves_up() {
            let key = KeyEvent::new(KeyCode::Up, KeyModifiers::NONE);
            assert_eq!(
                handle_category_filters_keys(key),
                Some(Message::CategoryFilterUp)
            );
        }

        #[test]
        fn test_space_toggles() {
            let key = KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE);
            assert_eq!(
                handle_category_filters_keys(key),
                Some(Message::CategoryFilterToggle)
            );
        }

        #[test]
        fn test_enter_toggles() {
            let key = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
            assert_eq!(
                handle_category_filters_keys(key),
                Some(Message::CategoryFilterToggle)
            );
        }

        #[test]
        fn test_a_selects_all() {
            let key = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE);
            assert_eq!(
                handle_category_filters_keys(key),
                Some(Message::CategoryFilterSelectAll)
            );
        }

        #[test]
        fn test_esc_returns_to_menu() {
            let key = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
            assert_eq!(handle_category_filters_keys(key), Some(Message::BackToMenu));
        }

        #[test]
        fn test_q_returns_to_menu() {
            let key = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE);
            assert_eq!(handle_category_filters_keys(key), Some(Message::BackToMenu));
        }

        #[test]
        fn test_ctrl_q_returns_to_menu() {
            let key = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::CONTROL);
            assert_eq!(handle_category_filters_keys(key), Some(Message::BackToMenu));
        }

        #[test]
        fn test_unknown_key_returns_none() {
            let key = KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE);
            assert_eq!(handle_category_filters_keys(key), None);
        }
    }
}
