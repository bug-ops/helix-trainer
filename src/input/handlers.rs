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

use crate::input::keymap::{CanonicalKeys, PhysicalKey};
use crate::ui::state::InputStateAccess;
use crate::ui::{AppState, Message, Screen, state::TypedScreen};

use super::typestate::{
    InputStateMachine, handle_insert_mode_input, map_key_to_helix_command, normalize_key_event,
};

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
        KeyCode::Char('s')
            if matches!(
                state.screen,
                TypedScreen::Profile(_) | TypedScreen::Achievements(_)
            ) =>
        {
            Some(Message::ShowStatistics)
        }
        KeyCode::Char('p')
            if matches!(
                state.screen,
                TypedScreen::Statistics(_) | TypedScreen::Achievements(_)
            ) =>
        {
            Some(Message::ShowProfile)
        }
        KeyCode::Char('a')
            if matches!(
                state.screen,
                TypedScreen::Profile(_) | TypedScreen::Statistics(_)
            ) =>
        {
            Some(Message::ShowAchievements)
        }
        KeyCode::Down | KeyCode::Char('j')
            if matches!(state.screen, TypedScreen::Achievements(_)) =>
        {
            Some(Message::ScrollAchievements(1))
        }
        KeyCode::Up | KeyCode::Char('k')
            if matches!(state.screen, TypedScreen::Achievements(_)) =>
        {
            Some(Message::ScrollAchievements(-1))
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
                'a' => return Some(Message::ShowAchievements),
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

/// Get the gameplay `InputStateMachine` for the current screen, if any.
///
/// Works for both training mode (Task screen) and arcade mode (MiniGame
/// screen). Used to check for a pending prefix/command-line state before
/// routing keys that would otherwise bypass the state machine (Esc, arrows).
fn gameplay_input_state(state: &AppState) -> Option<&InputStateMachine> {
    match &state.screen {
        TypedScreen::Task(task_data) => Some(task_data.input_state()),
        TypedScreen::MiniGame(minigame_data) => Some(minigame_data.input_state()),
        _ => None,
    }
}

/// Map arrow keys to movement commands
///
/// Returns the corresponding movement command if the key is an arrow key,
/// None otherwise.
fn map_arrow_to_movement(key_code: KeyCode) -> Option<&'static str> {
    use crate::helix::commands::{CMD_MOVE_DOWN, CMD_MOVE_LEFT, CMD_MOVE_RIGHT, CMD_MOVE_UP};
    match key_code {
        KeyCode::Left => Some(CMD_MOVE_LEFT),   // "h"
        KeyCode::Right => Some(CMD_MOVE_RIGHT), // "l"
        KeyCode::Up => Some(CMD_MOVE_UP),       // "k"
        KeyCode::Down => Some(CMD_MOVE_DOWN),   // "j"
        _ => None,
    }
}

/// Handle gameplay input (insert mode or normal mode command)
///
/// Shared logic for both training and arcade modes.
/// In normal mode, passes keys directly to the message handler where
/// `InputStateMachine` processes multi-key sequences (gg, dd, fx, rx, 3j).
///
/// `make_message` receives both the canonical key(s) to dispatch and the
/// physically-typed key (for `KeyHistory` display, which must show what
/// the user actually pressed, not the translated command).
fn handle_gameplay_input<F>(key: KeyEvent, state: &AppState, make_message: F) -> Option<Message>
where
    F: FnOnce(CanonicalKeys, Cow<'static, str>) -> Message,
{
    if is_gameplay_insert_mode(state) {
        // Insert mode is raw character insertion, never translated by the
        // keymap overlay: only the normal-mode branch below consults it.
        // Translating here would corrupt typing for any remapped letter.
        return handle_insert_mode_input(key)
            .map(|typed| make_message(typed.clone().into(), typed));
    }

    // Normal mode: the keymap overlay is consulted first, directly on the
    // raw physical key, before the stock arrow-key mapping and before
    // `key_to_command_string` - only a miss falls through to the stock
    // path. States that consume the next key literally (command-line
    // buffer, find/replace targets, register names, ...) report
    // `key_context() == None` and are therefore never translated here.
    //
    // `key_context()` maps `CountPending`/`RegisterOpPending` to the same
    // `KeyContext::Base` lookup table as literal `Base` (a count prefix or
    // register selector doesn't change which command a key invokes), but
    // a *multi-token* expansion can only ever be dispatched by resetting
    // to `Base` (see `InputStateMachine::apply_canonical_expansion`) - so
    // it must not be used while the machine is actually mid-count or
    // mid-register-select, or the count/register context would be
    // silently discarded from underneath it. A single-token translation
    // has no such restriction: it's dispatched by continuing from
    // wherever the machine currently is (e.g. `3` + a remapped `G` ->
    // `k` still yields "3k", exactly like the un-remapped key would).
    if let Some(machine) = gameplay_input_state(state)
        && let Some(context) = machine.state().key_context()
        && let Some(canonical) = state
            .config
            .keymap
            .lookup(context, PhysicalKey::from_event(key))
        && (canonical.is_single_token() || machine.state().is_base())
    {
        let typed = Cow::Owned(PhysicalKey::from_event(key).label());
        return Some(make_message(canonical.clone(), typed));
    }

    // Normal mode: check if arrow keys should be mapped to movement commands
    // Only map if no modifiers are pressed (to avoid conflicts with Ctrl+Arrow, Alt+Arrow, etc.)
    // and no command-line is pending (its buffer needs raw arrow keys ignored,
    // not letters appended - see the CommandLinePending handler's Stay-on-arrow rule).
    if state.config.persistent.enable_arrow_keys_in_normal_mode
        && key.modifiers.is_empty()
        && gameplay_input_state(state)
            .map(|s| s.pending_command_line().is_none())
            .unwrap_or(true)
        && let Some(mapped_cmd) = map_arrow_to_movement(key.code)
    {
        let stock = Cow::Borrowed(mapped_cmd);
        return Some(make_message(stock.clone().into(), stock));
    }

    // Normal mode: convert key to string for InputStateMachine
    // State machine handles multi-key commands in gameplay.rs/minigame.rs
    let stock = key_to_command_string(key)?;
    Some(make_message(stock.clone().into(), stock))
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
    // normalize and return the character for the state machine to process.
    // Guard is applied AFTER normalization so macOS Option-composed chars
    // are judged on their post-normalization modifiers.
    let key = normalize_key_event(key);

    match key.code {
        // Only bare ASCII chars fall through here: known ALT/CTRL commands
        // are already returned above by `map_key_to_helix_command`, so an
        // ALT/CTRL-carrying char reaching this arm is unmapped and must be
        // dropped rather than silently serialized as the bare character
        // (previously: unmapped Alt-x reached the state machine as plain
        // "x", executing `select line` instead of being ignored).
        KeyCode::Char(c)
            if c.is_ascii()
                && !key.modifiers.contains(KeyModifiers::ALT)
                && !key.modifiers.contains(KeyModifiers::CONTROL) =>
        {
            Some(Cow::Owned(c.to_string()))
        }
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

    // If hint panel is visible, Escape dismisses it without counting as a game action.
    // Skip this while in Insert mode so Escape exits Insert mode first (matching the
    // arcade path); a second Escape then closes the panel.
    if key.code == KeyCode::Esc
        && let TypedScreen::Task(task_data) = &state.screen
        && task_data.show_hint_panel
        && !is_gameplay_insert_mode(state)
    {
        return Some(Message::ShowHint);
    }

    // Handle gameplay input (insert mode or normal mode)
    handle_gameplay_input(key, state, |keys, typed| Message::ExecuteCommand {
        keys,
        typed,
    })
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

/// Handle keyboard events on the end-game summary screen
///
/// Key bindings:
/// - `r` - Start a review session
/// - `a` - Select Arcade mode
/// - `l` - Navigate to scenario list (Menu screen)
/// - `m` / `Esc` - Return to mode selection (main menu)
/// - `q` - Quit application
/// - `Ctrl-Q` - Return to mode selection
///
/// No `p` (profile) key: this screen is reached only via the Results `(n)`
/// key and has no `ReturnDestination` variant to return to, so a Profile
/// detour would strand the user - see `ReturnDestination` in `ui::state::screen`.
pub fn handle_end_game_summary_keys(key: KeyEvent) -> Option<Message> {
    // Ctrl-Q returns to menu (unified exit key)
    if key.code == KeyCode::Char('q') && key.modifiers.contains(KeyModifiers::CONTROL) {
        return Some(Message::BackToMenu);
    }

    match key.code {
        KeyCode::Char('q') => Some(Message::QuitApp),
        KeyCode::Char('r') => Some(Message::StartReviewSession),
        KeyCode::Char('a') => Some(Message::SelectArcadeMode),
        KeyCode::Char('l') => Some(Message::NavigateTo(Screen::MainMenu)),
        KeyCode::Esc | KeyCode::Char('m') => Some(Message::BackToMenu),
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
        KeyCode::Char('a') => Some(Message::ShowAchievements),
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
            KeyCode::Char('a') => Some(Message::ShowAchievements),
            KeyCode::Char('M') => Some(Message::ToggleSound),
            _ => None,
        };
    }

    // In playing state - handle input using shared gameplay logic
    if session.state().is_playing() {
        // In insert mode - use shared handler (includes Esc to exit insert)
        if is_gameplay_insert_mode(state) {
            return handle_gameplay_input(key, state, |keys, typed| Message::MiniGameCommand {
                keys,
                typed,
            });
        }

        // Normal mode - Esc cancels a pending prefix/command-line state
        // (count, g, m, register-select, command-line, ...) if one is
        // active; otherwise it pauses. Without this, a pending state traps
        // every keystroke until it resolves or times out, costing lives.
        if key.code == KeyCode::Esc {
            let has_pending_state = gameplay_input_state(state)
                .map(|s| s.is_prefix_state())
                .unwrap_or(false);
            if has_pending_state {
                return handle_gameplay_input(key, state, |keys, typed| Message::MiniGameCommand {
                    keys,
                    typed,
                });
            }
            return Some(Message::PauseMiniGame);
        }

        return handle_gameplay_input(key, state, |keys, typed| Message::MiniGameCommand {
            keys,
            typed,
        });
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

    /// Build an `Active` `GameSession` for a minimal scenario, used to exercise
    /// insert/normal mode transitions in task-screen key handling tests.
    fn make_active_task_session() -> GameSession<crate::game::Active> {
        use crate::config::{CursorSpec, ScoringConfig, Setup, Solution, TargetState};

        let scenario = Scenario {
            id: "test".to_string(),
            name: "Test".to_string(),
            description: "Test scenario".to_string(),
            setup: Setup {
                file_content: "test".to_string(),
                language: None,
                cursor: CursorSpec {
                    cursor_position: Some((0, 0)),
                    selection: None,
                    cursors: None,
                    selections: None,
                },
            },
            target: TargetState {
                file_content: "test2".to_string(),
                cursor: CursorSpec {
                    cursor_position: Some((0, 0)),
                    selection: None,
                    cursors: None,
                    selections: None,
                },
            },
            solution: Solution {
                commands: vec!["x".to_string()],
                description: "Delete char".to_string(),
            },
            scoring: ScoringConfig {
                optimal_count: std::num::NonZeroUsize::new(1).unwrap(),
                max_points: 100,
                tolerance: 0,
            },
            hints: vec![],
            alternatives: vec![],
            metadata: None,
        };

        GameSession::new(scenario).unwrap()
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
            Some(Message::ExecuteCommand {
                keys: CanonicalKeys::from_static(CMD_MOVE_LEFT),
                typed: Cow::Borrowed(CMD_MOVE_LEFT),
            })
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
    fn test_profile_key_a_shows_achievements() {
        let key = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE);
        let mut state = create_test_app_state();
        state.screen = TypedScreen::Profile(crate::ui::state::ProfileData::default());
        let msg = handle_profile_stats_keys(key, &state);
        assert_eq!(msg, Some(Message::ShowAchievements));
    }

    #[test]
    fn test_statistics_key_a_shows_achievements() {
        let key = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE);
        let mut state = create_test_app_state();
        state.screen = TypedScreen::Statistics(crate::ui::state::StatisticsData::default());
        let msg = handle_profile_stats_keys(key, &state);
        assert_eq!(msg, Some(Message::ShowAchievements));
    }

    #[test]
    fn test_achievements_key_p_shows_profile() {
        let key = KeyEvent::new(KeyCode::Char('p'), KeyModifiers::NONE);
        let mut state = create_test_app_state();
        state.screen = TypedScreen::Achievements(crate::ui::state::AchievementsData::default());
        let msg = handle_profile_stats_keys(key, &state);
        assert_eq!(msg, Some(Message::ShowProfile));
    }

    #[test]
    fn test_achievements_key_s_shows_statistics() {
        let key = KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE);
        let mut state = create_test_app_state();
        state.screen = TypedScreen::Achievements(crate::ui::state::AchievementsData::default());
        let msg = handle_profile_stats_keys(key, &state);
        assert_eq!(msg, Some(Message::ShowStatistics));
    }

    #[test]
    fn test_achievements_key_m_returns_menu() {
        let key = KeyEvent::new(KeyCode::Char('m'), KeyModifiers::NONE);
        let mut state = create_test_app_state();
        state.screen = TypedScreen::Achievements(crate::ui::state::AchievementsData::default());
        let msg = handle_profile_stats_keys(key, &state);
        assert_eq!(msg, Some(Message::BackToMenu));
    }

    #[test]
    fn test_mode_selection_key_a_shows_achievements() {
        let key = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE);
        let msg = handle_mode_selection_keys(key);
        assert_eq!(msg, Some(Message::ShowAchievements));
    }

    #[test]
    fn test_menu_key_a_shows_achievements() {
        let key = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE);
        let mut state = create_test_app_state();
        state.screen = TypedScreen::Menu(MenuData::default());
        let msg = handle_menu_keys(key, &mut state);
        assert_eq!(msg, Some(Message::ShowAchievements));
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
            let mut state = create_test_app_state();
            state.screen = TypedScreen::Task(TaskData::new(make_active_task_session()));

            // GameSession starts in Normal mode
            assert!(!is_gameplay_insert_mode(&state));
        }
    }

    // Regression tests for the hint-panel Escape interaction fixed alongside #283:
    // Escape must exit Insert mode before it dismisses the hint panel.
    mod task_keys_hint_panel_escape_tests {
        use super::*;
        use crate::helix::commands::CMD_INSERT;

        #[test]
        fn test_escape_exits_insert_mode_even_with_hint_panel_open() {
            let mut state = create_test_app_state();
            let session = make_active_task_session();
            let session = match session.record_action(CMD_INSERT.to_string()).unwrap() {
                crate::game::SessionAfterAction::StillActive(session) => session,
                crate::game::SessionAfterAction::Completed(_) => {
                    panic!("entering insert mode should not complete the scenario")
                }
            };
            assert!(session.is_insert_mode());

            let mut task_data = TaskData::new(session);
            task_data.show_hint_panel = true;
            state.screen = TypedScreen::Task(task_data);

            let key = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
            let msg = handle_task_keys(key, &state);

            // Escape should exit Insert mode (gameplay Escape), not dismiss the hint panel.
            assert_eq!(
                msg,
                Some(Message::ExecuteCommand {
                    keys: CanonicalKeys::from_static(crate::helix::commands::CMD_ESCAPE),
                    typed: Cow::Borrowed(crate::helix::commands::CMD_ESCAPE),
                })
            );
        }

        #[test]
        fn test_escape_dismisses_hint_panel_in_normal_mode() {
            let mut state = create_test_app_state();
            let mut task_data = TaskData::new(make_active_task_session());
            task_data.show_hint_panel = true;
            state.screen = TypedScreen::Task(task_data);

            let key = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
            let msg = handle_task_keys(key, &state);

            assert_eq!(msg, Some(Message::ShowHint));
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
        fn test_map_arrow_to_movement() {
            use crate::helix::commands::{
                CMD_MOVE_DOWN, CMD_MOVE_LEFT, CMD_MOVE_RIGHT, CMD_MOVE_UP,
            };

            assert_eq!(map_arrow_to_movement(KeyCode::Left), Some(CMD_MOVE_LEFT));
            assert_eq!(map_arrow_to_movement(KeyCode::Right), Some(CMD_MOVE_RIGHT));
            assert_eq!(map_arrow_to_movement(KeyCode::Up), Some(CMD_MOVE_UP));
            assert_eq!(map_arrow_to_movement(KeyCode::Down), Some(CMD_MOVE_DOWN));
            assert_eq!(map_arrow_to_movement(KeyCode::Char('h')), None);
        }

        #[test]
        fn test_arrow_keys_ignored_by_default() {
            use crate::config::{CursorSpec, ScoringConfig, Setup, Solution, TargetState};
            use crate::ui::state::{ConfigState, screen::TaskData};

            let mut state = create_test_app_state();
            state.config = ConfigState::default(); // enable_arrow_keys_in_normal_mode = false

            // Create a simple scenario
            let scenario = Scenario {
                id: "test".to_string(),
                name: "Test".to_string(),
                description: "Test scenario".to_string(),
                setup: Setup {
                    file_content: "test".to_string(),
                    language: None,
                    cursor: CursorSpec {
                        cursor_position: Some((0, 0)),
                        selection: None,
                        cursors: None,
                        selections: None,
                    },
                },
                target: TargetState {
                    file_content: "test2".to_string(),
                    cursor: CursorSpec {
                        cursor_position: Some((0, 0)),
                        selection: None,
                        cursors: None,
                        selections: None,
                    },
                },
                solution: Solution {
                    commands: vec!["x".to_string()],
                    description: "Delete char".to_string(),
                },
                scoring: ScoringConfig {
                    optimal_count: std::num::NonZeroUsize::new(1).unwrap(),
                    max_points: 100,
                    tolerance: 0,
                },
                hints: vec![],
                alternatives: vec![],
                metadata: None,
            };

            let session = GameSession::new(scenario).unwrap();
            state.screen = TypedScreen::Task(TaskData::new(session));

            // Arrow keys should be converted to "Left"/"Right" strings, not movement commands
            let left_key = KeyEvent::new(KeyCode::Left, KeyModifiers::NONE);
            let result = handle_gameplay_input(left_key, &state, |keys, typed| {
                Message::ExecuteCommand { keys, typed }
            });

            // Should return "Left" string, not "h" command
            match result {
                Some(Message::ExecuteCommand { keys, .. }) => {
                    assert_eq!(keys.as_str(), "Left");
                }
                _ => panic!("Expected ExecuteCommand with 'Left'"),
            }
        }

        #[test]
        fn test_arrow_keys_mapped_when_enabled() {
            use crate::config::{CursorSpec, ScoringConfig, Setup, Solution, TargetState};
            use crate::helix::commands::{
                CMD_MOVE_DOWN, CMD_MOVE_LEFT, CMD_MOVE_RIGHT, CMD_MOVE_UP,
            };
            use crate::ui::state::screen::TaskData;

            let mut state = create_test_app_state();
            state.config.persistent.enable_arrow_keys_in_normal_mode = true;

            // Create a simple scenario
            let scenario = Scenario {
                id: "test".to_string(),
                name: "Test".to_string(),
                description: "Test scenario".to_string(),
                setup: Setup {
                    file_content: "test".to_string(),
                    language: None,
                    cursor: CursorSpec {
                        cursor_position: Some((0, 0)),
                        selection: None,
                        cursors: None,
                        selections: None,
                    },
                },
                target: TargetState {
                    file_content: "test2".to_string(),
                    cursor: CursorSpec {
                        cursor_position: Some((0, 0)),
                        selection: None,
                        cursors: None,
                        selections: None,
                    },
                },
                solution: Solution {
                    commands: vec!["x".to_string()],
                    description: "Delete char".to_string(),
                },
                scoring: ScoringConfig {
                    optimal_count: std::num::NonZeroUsize::new(1).unwrap(),
                    max_points: 100,
                    tolerance: 0,
                },
                hints: vec![],
                alternatives: vec![],
                metadata: None,
            };

            let session = GameSession::new(scenario).unwrap();
            state.screen = TypedScreen::Task(TaskData::new(session));

            // Arrow keys should be mapped to movement commands
            let left_key = KeyEvent::new(KeyCode::Left, KeyModifiers::NONE);
            let result = handle_gameplay_input(left_key, &state, |keys, typed| {
                Message::ExecuteCommand { keys, typed }
            });
            match result {
                Some(Message::ExecuteCommand { keys, .. }) => {
                    assert_eq!(keys.as_str(), CMD_MOVE_LEFT);
                }
                _ => panic!("Expected ExecuteCommand with CMD_MOVE_LEFT"),
            }

            let right_key = KeyEvent::new(KeyCode::Right, KeyModifiers::NONE);
            let result = handle_gameplay_input(right_key, &state, |keys, typed| {
                Message::ExecuteCommand { keys, typed }
            });
            match result {
                Some(Message::ExecuteCommand { keys, .. }) => {
                    assert_eq!(keys.as_str(), CMD_MOVE_RIGHT);
                }
                _ => panic!("Expected ExecuteCommand with CMD_MOVE_RIGHT"),
            }

            let up_key = KeyEvent::new(KeyCode::Up, KeyModifiers::NONE);
            let result = handle_gameplay_input(up_key, &state, |keys, typed| {
                Message::ExecuteCommand { keys, typed }
            });
            match result {
                Some(Message::ExecuteCommand { keys, .. }) => {
                    assert_eq!(keys.as_str(), CMD_MOVE_UP);
                }
                _ => panic!("Expected ExecuteCommand with CMD_MOVE_UP"),
            }

            let down_key = KeyEvent::new(KeyCode::Down, KeyModifiers::NONE);
            let result = handle_gameplay_input(down_key, &state, |keys, typed| {
                Message::ExecuteCommand { keys, typed }
            });
            match result {
                Some(Message::ExecuteCommand { keys, .. }) => {
                    assert_eq!(keys.as_str(), CMD_MOVE_DOWN);
                }
                _ => panic!("Expected ExecuteCommand with CMD_MOVE_DOWN"),
            }
        }

        #[test]
        fn test_enter_key() {
            let key = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
            assert_eq!(key_to_command_string(key), Some(Cow::Borrowed("Enter")));
        }

        #[test]
        fn test_arrow_keys_with_modifiers_not_mapped() {
            use crate::config::{CursorSpec, ScoringConfig, Setup, Solution, TargetState};
            use crate::ui::state::screen::TaskData;

            let mut state = create_test_app_state();
            state.config.persistent.enable_arrow_keys_in_normal_mode = true;

            // Create a simple scenario
            let scenario = Scenario {
                id: "test".to_string(),
                name: "Test".to_string(),
                description: "Test scenario".to_string(),
                setup: Setup {
                    file_content: "test".to_string(),
                    language: None,
                    cursor: CursorSpec {
                        cursor_position: Some((0, 0)),
                        selection: None,
                        cursors: None,
                        selections: None,
                    },
                },
                target: TargetState {
                    file_content: "test2".to_string(),
                    cursor: CursorSpec {
                        cursor_position: Some((0, 0)),
                        selection: None,
                        cursors: None,
                        selections: None,
                    },
                },
                solution: Solution {
                    commands: vec!["x".to_string()],
                    description: "Delete char".to_string(),
                },
                scoring: ScoringConfig {
                    optimal_count: std::num::NonZeroUsize::new(1).unwrap(),
                    max_points: 100,
                    tolerance: 0,
                },
                hints: vec![],
                alternatives: vec![],
                metadata: None,
            };

            let session = GameSession::new(scenario).unwrap();
            state.screen = TypedScreen::Task(TaskData::new(session));

            // Arrow keys with modifiers should NOT be mapped (should use normal handler)
            let ctrl_left = KeyEvent::new(KeyCode::Left, KeyModifiers::CONTROL);
            let result = handle_gameplay_input(ctrl_left, &state, |keys, typed| {
                Message::ExecuteCommand { keys, typed }
            });
            // Should return "Left" string, not "h" command
            match result {
                Some(Message::ExecuteCommand { keys, .. }) => {
                    assert_eq!(keys.as_str(), "Left");
                }
                _ => panic!("Expected ExecuteCommand with 'Left' for Ctrl+Left"),
            }

            let alt_up = KeyEvent::new(KeyCode::Up, KeyModifiers::ALT);
            let result = handle_gameplay_input(alt_up, &state, |keys, typed| {
                Message::ExecuteCommand { keys, typed }
            });
            match result {
                Some(Message::ExecuteCommand { keys, .. }) => {
                    assert_eq!(keys.as_str(), "Up");
                }
                _ => panic!("Expected ExecuteCommand with 'Up' for Alt+Up"),
            }

            // Test with Shift modifier
            let shift_right = KeyEvent::new(KeyCode::Right, KeyModifiers::SHIFT);
            let result = handle_gameplay_input(shift_right, &state, |keys, typed| {
                Message::ExecuteCommand { keys, typed }
            });
            match result {
                Some(Message::ExecuteCommand { keys, .. }) => {
                    assert_eq!(keys.as_str(), "Right");
                }
                _ => panic!("Expected ExecuteCommand with 'Right' for Shift+Right"),
            }
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

        /// Regression test: an unmapped Alt-modified key must be dropped by
        /// the fallback, not silently serialized as the bare character. Prior
        /// to this fix, an unmapped Alt-z would reach the state machine as
        /// plain "z", executing whatever "z" resolves to (view mode prefix)
        /// instead of being ignored as an unrecognized command.
        #[test]
        fn test_unmapped_alt_key_is_dropped_not_serialized_as_bare_char() {
            let key = KeyEvent::new(KeyCode::Char('z'), KeyModifiers::ALT);
            assert_eq!(key_to_command_string(key), None);
        }

        #[test]
        fn test_unmapped_ctrl_key_is_dropped_not_serialized_as_bare_char() {
            let key = KeyEvent::new(KeyCode::Char('z'), KeyModifiers::CONTROL);
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

    mod handle_end_game_summary_keys_tests {
        use super::*;

        #[test]
        fn test_r_starts_review_session() {
            let key = KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE);
            assert_eq!(
                handle_end_game_summary_keys(key),
                Some(Message::StartReviewSession)
            );
        }

        #[test]
        fn test_a_selects_arcade_mode() {
            let key = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE);
            assert_eq!(
                handle_end_game_summary_keys(key),
                Some(Message::SelectArcadeMode)
            );
        }

        #[test]
        fn test_l_navigates_to_main_menu() {
            let key = KeyEvent::new(KeyCode::Char('l'), KeyModifiers::NONE);
            assert_eq!(
                handle_end_game_summary_keys(key),
                Some(Message::NavigateTo(Screen::MainMenu))
            );
        }

        #[test]
        fn test_m_returns_to_menu() {
            let key = KeyEvent::new(KeyCode::Char('m'), KeyModifiers::NONE);
            assert_eq!(handle_end_game_summary_keys(key), Some(Message::BackToMenu));
        }

        #[test]
        fn test_esc_returns_to_menu() {
            let key = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
            assert_eq!(handle_end_game_summary_keys(key), Some(Message::BackToMenu));
        }

        #[test]
        fn test_q_quits() {
            let key = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE);
            assert_eq!(handle_end_game_summary_keys(key), Some(Message::QuitApp));
        }

        #[test]
        fn test_ctrl_q_returns_to_menu() {
            let key = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::CONTROL);
            assert_eq!(handle_end_game_summary_keys(key), Some(Message::BackToMenu));
        }

        #[test]
        fn test_shift_m_toggles_sound() {
            let key = KeyEvent::new(KeyCode::Char('M'), KeyModifiers::NONE);
            assert_eq!(
                handle_end_game_summary_keys(key),
                Some(Message::ToggleSound)
            );
        }

        #[test]
        fn test_p_is_not_bound() {
            // No re-entry from this screen via Profile - see the function's doc comment.
            let key = KeyEvent::new(KeyCode::Char('p'), KeyModifiers::NONE);
            assert_eq!(handle_end_game_summary_keys(key), None);
        }

        #[test]
        fn test_unknown_key_returns_none() {
            let key = KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE);
            assert_eq!(handle_end_game_summary_keys(key), None);
        }
    }

    // Arcade Esc-routing tests (S4a): Esc must cancel a pending prefix
    // state instead of pausing, and must still pause when nothing is
    // pending (the pre-existing behavior for every other state).
    mod minigame_esc_routing_tests {
        use super::*;
        use crate::config::Difficulty;
        use crate::minigame::MiniGameSession;
        use crate::testing::ScenarioBuilder;
        use crate::ui::state::{InputStateAccess, MiniGameData};
        use std::sync::Arc;

        fn playing_minigame_state() -> AppState {
            let mut state = create_test_app_state();

            let scenario = ScenarioBuilder::new()
                .id("test_minigame")
                .difficulty(Difficulty::Beginner)
                .build();
            let mut session = MiniGameSession::new(Arc::new(vec![scenario]), None);
            session.start();
            session.tick_countdown();
            session.tick_countdown();
            session.tick_countdown();
            assert!(session.state().is_playing());

            state.game.minigame_session = Some(session);
            state.screen = TypedScreen::MiniGame(MiniGameData::default());
            state
        }

        #[test]
        fn esc_with_no_pending_state_pauses() {
            let state = playing_minigame_state();
            let key = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
            assert_eq!(
                handle_minigame_keys(key, &state),
                Some(Message::PauseMiniGame)
            );
        }

        #[test]
        fn esc_with_pending_count_cancels_instead_of_pausing() {
            let mut state = playing_minigame_state();
            if let TypedScreen::MiniGame(data) = &mut state.screen {
                let result = data
                    .input_state_mut()
                    .process_key(KeyEvent::new(KeyCode::Char('3'), KeyModifiers::NONE));
                assert!(matches!(
                    result,
                    crate::input::typestate::HandlerResult::Transition(_)
                ));
                assert!(data.input_state().is_prefix_state());
            }

            let esc_msg =
                handle_minigame_keys(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE), &state);
            // Esc while a count is pending must route through the state
            // machine (Cancel), not pause the game.
            assert!(
                matches!(esc_msg, Some(Message::MiniGameCommand { .. })),
                "expected Esc to route as a MiniGameCommand (cancel) while a prefix state is pending, got {:?}",
                esc_msg
            );
        }

        #[test]
        fn esc_with_pending_register_cancels_instead_of_pausing() {
            let mut state = playing_minigame_state();
            if let TypedScreen::MiniGame(data) = &mut state.screen {
                data.input_state_mut()
                    .process_key(KeyEvent::new(KeyCode::Char('"'), KeyModifiers::NONE));
                assert!(data.input_state().is_prefix_state());
            }

            let esc_msg =
                handle_minigame_keys(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE), &state);
            assert!(
                matches!(esc_msg, Some(Message::MiniGameCommand { .. })),
                "expected Esc to route as a MiniGameCommand (cancel) while RegisterPending, got {:?}",
                esc_msg
            );
        }
    }

    /// Regression tests for the S1 review finding: a multi-token keymap
    /// remap (e.g. `G = "goto_last_line"` -> canonical `"ge"`) looked up
    /// while the real `InputState` is `CountPending`/`RegisterOpPending`
    /// (not literally `Base`) must fall through to the stock key instead
    /// of reaching `InputStateMachine::apply_canonical_expansion`, whose
    /// `debug_assert!(is_base())` would otherwise panic in a debug build.
    /// `key_context()` maps both those states to the same `KeyContext::Base`
    /// *lookup table* as literal `Base`, which is correct for single-token
    /// translations but not for multi-token ones (see `handlers.rs`'s
    /// `handle_gameplay_input` doc comment on the guard this exercises).
    mod multi_token_remap_outside_base_tests {
        use super::*;
        use crate::config::keymap::resolve_str;
        use crate::input::keymap::CanonicalKeys;
        use crate::ui::state::{ConfigState, TaskData, update};

        /// A task-screen `AppState` with a keymap overlay remapping `G` to
        /// `goto_last_line` (canonical `"ge"`, 2 tokens) at the top level.
        fn state_with_multi_token_remap() -> AppState {
            let (keymap, report) = resolve_str(
                r#"
                [keys.normal]
                G = "goto_last_line"
                "#,
            )
            .unwrap();
            assert_eq!(report.applied, 1);
            assert!(!keymap.is_empty());

            let mut state = AppState::with_config(
                vec![],
                crate::gamification::UserProfile::new(),
                crate::gamification::ProfileStorage::for_test(),
                crate::learning::PerformanceTracker::new(),
                ConfigState {
                    keymap,
                    ..ConfigState::default()
                },
            );
            state.screen = TypedScreen::Task(TaskData::new(make_active_task_session()));
            state
        }

        fn press(state: &mut AppState, key: KeyEvent) {
            if let Some(msg) = handle_task_keys(key, state) {
                update(state, msg).unwrap();
            }
        }

        fn action_count(state: &AppState) -> usize {
            let TypedScreen::Task(task_data) = &state.screen else {
                panic!("expected Task screen");
            };
            task_data.session.action_count()
        }

        fn input_state(state: &AppState) -> crate::input::typestate::InputState {
            let TypedScreen::Task(task_data) = &state.screen else {
                panic!("expected Task screen");
            };
            task_data.input_state().state().clone()
        }

        /// Sanity check: outside of `CountPending`, the remap does resolve
        /// to its multi-token canonical, proving the fixture overlay works.
        #[test]
        fn multi_token_remap_resolves_from_base() {
            let state = state_with_multi_token_remap();
            let msg = handle_task_keys(
                KeyEvent::new(KeyCode::Char('G'), KeyModifiers::SHIFT),
                &state,
            )
            .unwrap();
            match msg {
                Message::ExecuteCommand { keys, .. } => {
                    assert_eq!(keys, CanonicalKeys::from_static("ge"));
                }
                other => panic!("expected ExecuteCommand, got {:?}", other),
            }
        }

        #[test]
        fn count_then_multi_token_remap_does_not_panic_and_cancels() {
            let mut state = state_with_multi_token_remap();

            press(
                &mut state,
                KeyEvent::new(KeyCode::Char('3'), KeyModifiers::NONE),
            );
            assert!(input_state(&state).is_count_pending());

            // Must not panic (this is the regression under test) and must
            // fall through to the stock key, which "3" + "G" cancels (G is
            // not a count-compatible command in this trainer).
            press(
                &mut state,
                KeyEvent::new(KeyCode::Char('G'), KeyModifiers::SHIFT),
            );

            assert!(input_state(&state).is_base());
            assert_eq!(action_count(&state), 0);
        }

        #[test]
        fn register_op_then_multi_token_remap_does_not_panic_and_cancels() {
            let mut state = state_with_multi_token_remap();

            press(
                &mut state,
                KeyEvent::new(KeyCode::Char('"'), KeyModifiers::NONE),
            );
            press(
                &mut state,
                KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE),
            );
            assert!(matches!(
                input_state(&state),
                crate::input::typestate::InputState::RegisterOpPending { register: 'a' }
            ));

            // Must not panic and must fall through to the stock key, which
            // RegisterOpPending cancels on (only y/p/P/R/d/c are recognized ops).
            press(
                &mut state,
                KeyEvent::new(KeyCode::Char('G'), KeyModifiers::SHIFT),
            );

            assert!(input_state(&state).is_base());
            assert_eq!(action_count(&state), 0);
        }
    }
}
