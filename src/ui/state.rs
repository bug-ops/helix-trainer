//! UI state management using Elm Architecture
//!
//! This module implements the Elm Architecture pattern for the TUI application.
//! It provides a centralized AppState with pure update functions, enabling
//! predictable state transitions and easy testing.
//!
//! # Architecture
//!
//! The Elm Architecture pattern consists of:
//! - `AppState`: The complete application state
//! - `Screen`: The current screen being displayed
//! - `Message`: User actions and events that trigger state changes
//! - `update()`: Pure function that transforms state based on messages
//!
//! This ensures:
//! - All state changes go through the update function
//! - No hidden side effects in state changes
//! - State transitions are testable and reproducible
//! - UI rendering is pure (no side effects)

use crate::config::Scenario;
use crate::game::GameSession;
use crate::security::UserError;
use std::fmt;

/// The current screen being displayed in the UI
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    /// Main menu screen
    MainMenu,
    /// Task/scenario gameplay screen
    Task,
    /// Results screen after scenario completion
    Results,
}

/// Messages that trigger state updates
///
/// Each message represents a user action or system event that should
/// change the application state. The `update()` function handles all
/// messages in a pure, side-effect-free manner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Message {
    /// Navigate to a specific screen
    NavigateTo(Screen),

    /// Quit the application
    QuitApp,

    /// Menu navigation: move selection up
    MenuUp,

    /// Menu navigation: move selection down
    MenuDown,

    /// Menu action: select current menu item
    MenuSelect,

    /// Start a scenario at the given index
    StartScenario(usize),

    /// Mark the current scenario as complete
    CompleteScenario,

    /// Abandon the current scenario
    AbandonScenario,

    /// Request to show the next hint
    ShowHint,

    /// Execute a Helix command during gameplay
    ExecuteCommand(std::borrow::Cow<'static, str>),

    /// Retry the current scenario
    RetryScenario,

    /// Move to next scenario
    NextScenario,

    /// Return to main menu
    BackToMenu,
}

/// Main application state
///
/// Contains all the data needed to render the UI and handle user interactions.
/// This is the single source of truth for the application.
///
/// Note: This doesn't derive Clone because GameSession doesn't implement Clone.
/// Instead, we implement Debug manually.
///
/// # Memory Layout
///
/// Fields are ordered for optimal memory layout and cache efficiency:
/// 1. Large allocations first (Vec, Option<GameSession>)
/// 2. Frequently accessed fields next (screen, session)
/// 3. Medium-sized types (Option<String>, String)
/// 4. Small types last (usize, bool, enum)
pub struct AppState {
    /// All available scenarios
    /// Size: 24 bytes (Vec) - placed first for alignment
    pub scenarios: Vec<Scenario>,

    /// Active game session (Some if on Task screen)
    /// Size: ~200+ bytes - large type, placed early
    pub session: Option<GameSession>,

    /// The current hint being displayed (if any)
    /// Size: 24-32 bytes (Option<String>)
    pub current_hint: Option<String>,

    /// Last command executed (for display purposes)
    /// Size: 24-32 bytes (Option<String>)
    pub last_command: Option<String>,

    /// History of last 5 key presses (most recent first)
    /// Size: 24 bytes (Vec)
    pub key_history: Vec<String>,

    /// Command buffer for accumulating multi-key commands (e.g., "d" waiting for "d")
    /// Size: 24 bytes (String)
    pub command_buffer: String,

    /// Time when scenario was completed (for showing success screen before results)
    /// Size: 16 bytes (Option<Instant>)
    pub completion_time: Option<std::time::Instant>,

    /// Index of the currently selected menu item
    /// Size: 8 bytes (usize)
    pub selected_menu_item: usize,

    /// The screen currently being displayed
    /// Size: 1 byte (enum)
    pub screen: Screen,

    /// Whether the application is running
    /// Size: 1 byte (bool)
    pub running: bool,

    /// Whether to show hint on task screen
    /// Size: 1 byte (bool)
    pub show_hint_panel: bool,

    /// Whether to show key history popup
    /// Size: 1 byte (bool)
    pub show_key_history: bool,
}

impl fmt::Debug for AppState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AppState")
            .field("screen", &self.screen)
            .field("session", &"<GameSession>")
            .field("scenarios", &self.scenarios.len())
            .field("selected_menu_item", &self.selected_menu_item)
            .field("running", &self.running)
            .field("current_hint", &self.current_hint.is_some())
            .field("show_hint_panel", &self.show_hint_panel)
            .field("show_key_history", &self.show_key_history)
            .field("last_command", &self.last_command)
            .field("completion_time", &self.completion_time.is_some())
            .field("key_history", &self.key_history.len())
            .field("command_buffer", &self.command_buffer)
            .finish()
    }
}

impl AppState {
    /// Create a new application state with the given scenarios
    ///
    /// # Arguments
    ///
    /// * `scenarios` - The list of available scenarios to play
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use helix_trainer::ui::AppState;
    /// use helix_trainer::config::Scenario;
    ///
    /// let scenarios = vec![/* ... */];
    /// let state = AppState::new(scenarios);
    /// assert_eq!(state.screen, Screen::MainMenu);
    /// ```
    pub fn new(scenarios: Vec<Scenario>) -> Self {
        Self {
            scenarios,
            session: None,
            current_hint: None,
            last_command: None,
            key_history: Vec::new(),
            command_buffer: String::new(),
            completion_time: None,
            selected_menu_item: 0,
            screen: Screen::MainMenu,
            running: true,
            show_hint_panel: false,
            show_key_history: false,
        }
    }

    /// Get reference to the current session
    pub fn session(&self) -> Option<&GameSession> {
        self.session.as_ref()
    }

    /// Get mutable reference to the current session
    pub fn session_mut(&mut self) -> Option<&mut GameSession> {
        self.session.as_mut()
    }

    /// Get the menu items for the main menu
    pub fn menu_items() -> Vec<&'static str> {
        vec!["Start Training", "Quit"]
    }

    /// Get the number of available scenarios
    pub fn scenario_count(&self) -> usize {
        self.scenarios.len()
    }

    /// Get a scenario by index
    pub fn get_scenario(&self, index: usize) -> Option<&Scenario> {
        self.scenarios.get(index)
    }

    /// Add a key to the history (keeps last 5)
    pub fn add_key_to_history(&mut self, key: String) {
        // Insert at the beginning (most recent first)
        self.key_history.insert(0, key);

        // Keep only last 5 keys
        if self.key_history.len() > 5 {
            self.key_history.truncate(5);
        }
    }

    /// Clear key history
    pub fn clear_key_history(&mut self) {
        self.key_history.clear();
    }
}

/// Format a key command for display in key history
///
/// Converts internal command names to user-friendly display strings
fn format_key_for_display(command: &str) -> String {
    match command {
        "ArrowLeft" => "←".to_string(),
        "ArrowRight" => "→".to_string(),
        "ArrowUp" => "↑".to_string(),
        "ArrowDown" => "↓".to_string(),
        "Backspace" => "⌫".to_string(),
        "Escape" => "Esc".to_string(),
        "\n" => "↵".to_string(),
        " " => "Space".to_string(),
        cmd if cmd.len() == 1 => cmd.to_string(),
        cmd => cmd.to_string(),
    }
}

/// Pure update function for state transitions
///
/// This function is the heart of the Elm Architecture pattern.
/// It takes the current state and a message, and returns the new state.
/// It has no side effects - all effects are handled elsewhere.
///
/// # Arguments
///
/// * `state` - The current application state (will be modified)
/// * `msg` - The message/action that triggered the update
///
/// # Errors
///
/// Returns `UserError` if state validation fails (e.g., invalid scenario)
///
/// # Examples
///
/// ```ignore
/// use helix_trainer::ui::{AppState, Screen, Message, update};
///
/// let mut state = AppState::new(vec![]);
/// update(&mut state, Message::QuitApp)?;
/// assert!(!state.running);
/// # Ok::<(), helix_trainer::security::UserError>(())
/// ```
pub fn update(state: &mut AppState, msg: Message) -> Result<(), UserError> {
    match msg {
        Message::QuitApp => {
            state.running = false;
            Ok(())
        }

        Message::NavigateTo(screen) => {
            state.screen = screen;
            Ok(())
        }

        Message::MenuUp => {
            if state.selected_menu_item > 0 {
                state.selected_menu_item -= 1;
            }
            Ok(())
        }

        Message::MenuDown => {
            // Total menu items = scenarios + Quit option
            let max_items = state.scenarios.len() + 1;
            if state.selected_menu_item < max_items - 1 {
                state.selected_menu_item += 1;
            }
            Ok(())
        }

        Message::MenuSelect => {
            let scenario_count = state.scenarios.len();
            let selected = state.selected_menu_item;

            if selected < scenario_count {
                // Start selected scenario
                update(state, Message::StartScenario(selected))?;
            } else if selected == scenario_count {
                // Quit option (last item)
                update(state, Message::QuitApp)?;
            }
            Ok(())
        }

        Message::StartScenario(index) => {
            if let Some(scenario) = state.scenarios.get(index).cloned() {
                let session = GameSession::new(scenario)?;
                state.session = Some(session);
                state.screen = Screen::Task;
                state.show_hint_panel = false;
                state.show_key_history = false;
                state.current_hint = None;
                state.last_command = None;
                state.completion_time = None;
                state.clear_key_history();
                state.command_buffer.clear();
            }
            Ok(())
        }

        Message::CompleteScenario => {
            state.screen = Screen::Results;
            Ok(())
        }

        Message::AbandonScenario => {
            if let Some(session) = &mut state.session {
                session.abandon();
            }
            state.screen = Screen::Results;
            Ok(())
        }

        Message::ShowHint => {
            if let Some(session) = &mut state.session
                && let Some(hint) = session.get_hint()
            {
                state.current_hint = Some(hint);
                state.show_hint_panel = true;
            }
            Ok(())
        }

        Message::ExecuteCommand(command) => {
            // Add key to history for display (format for readability)
            let display_key = format_key_for_display(command.as_ref());
            state.add_key_to_history(display_key);

            // Show key history popup after first keypress
            state.show_key_history = true;

            if let Some(session) = &mut state.session {
                // In Insert mode, execute commands directly
                if session.is_insert_mode() {
                    // Store last command for display (skip special commands and single chars)
                    if command.as_ref() == "Escape" {
                        state.last_command = Some(command.to_string());
                    }

                    // Execute command through session
                    session.record_action(command.to_string())?;
                } else {
                    // Normal mode: handle command buffer for multi-key commands
                    state.command_buffer.push_str(&command);

                    // Try to match a complete command
                    let final_command = match state.command_buffer.as_str() {
                        // Multi-key commands
                        "dd" => Some("dd"),
                        "gg" => Some("gg"),

                        // Replace character command: r + any char
                        cmd if cmd.starts_with('r') && cmd.len() == 2 => {
                            Some(state.command_buffer.as_str())
                        }

                        // Partial commands - wait for more input
                        "d" | "g" | "r" => None,

                        // Single-key commands (clear buffer and execute)
                        _ if state.command_buffer.len() == 1 => Some(state.command_buffer.as_str()),

                        // Invalid sequence - clear buffer
                        _ => {
                            state.command_buffer.clear();
                            return Ok(());
                        }
                    };

                    if let Some(cmd) = final_command {
                        // We have a complete command
                        let cmd_string = cmd.to_string();
                        state.command_buffer.clear();

                        // Store for display
                        state.last_command = Some(cmd_string.clone());

                        // Execute command through session
                        session.record_action(cmd_string)?;
                    }
                    // If None, we're waiting for more keys (buffer not cleared)
                }

                // Check if scenario is complete
                if session.is_completed() {
                    // Mark completion time instead of immediately going to results
                    // This allows showing the success state before transition
                    state.completion_time = Some(std::time::Instant::now());
                }
            }
            Ok(())
        }

        Message::RetryScenario => {
            if let Some(session) = &mut state.session {
                session.reset()?;
                state.screen = Screen::Task;
                state.show_hint_panel = false;
                state.show_key_history = false;
                state.current_hint = None;
                state.last_command = None;
                state.completion_time = None;
                state.clear_key_history();
                state.command_buffer.clear();
            }
            Ok(())
        }

        Message::NextScenario => {
            state.screen = Screen::MainMenu;
            state.session = None;
            state.show_hint_panel = false;
            state.current_hint = None;
            Ok(())
        }

        Message::BackToMenu => {
            state.screen = Screen::MainMenu;
            state.session = None;
            state.show_hint_panel = false;
            state.current_hint = None;
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{ScoringConfig, Setup, Solution, TargetState};

    fn create_test_scenario() -> Scenario {
        Scenario {
            id: "test_001".to_string(),
            name: "Test Scenario".to_string(),
            description: "A test scenario for UI testing".to_string(),
            setup: Setup {
                file_content: "line 1\n".to_string(),
                cursor_position: (0, 0),
            },
            target: TargetState {
                file_content: "line 2\n".to_string(),
                cursor_position: (0, 0),
                selection: None,
            },
            solution: Solution {
                commands: vec!["dd".to_string()],
                description: "Delete line".to_string(),
            },
            alternatives: vec![],
            hints: vec!["Use dd to delete a line".to_string()],
            scoring: ScoringConfig {
                optimal_count: 1,
                max_points: 100,
                tolerance: 0,
            },
        }
    }

    #[test]
    fn test_new_state() {
        let state = AppState::new(vec![]);
        assert_eq!(state.screen, Screen::MainMenu);
        assert_eq!(state.selected_menu_item, 0);
        assert!(state.running);
        assert!(state.session.is_none());
        assert!(!state.show_hint_panel);
    }

    #[test]
    fn test_quit_app_message() {
        let mut state = AppState::new(vec![]);
        assert!(state.running);

        update(&mut state, Message::QuitApp).unwrap();
        assert!(!state.running);
    }

    #[test]
    fn test_navigate_to_screen() {
        let mut state = AppState::new(vec![]);
        assert_eq!(state.screen, Screen::MainMenu);

        update(&mut state, Message::NavigateTo(Screen::Task)).unwrap();
        assert_eq!(state.screen, Screen::Task);

        update(&mut state, Message::NavigateTo(Screen::Results)).unwrap();
        assert_eq!(state.screen, Screen::Results);
    }

    #[test]
    fn test_menu_navigation_up() {
        let mut state = AppState::new(vec![]);
        state.selected_menu_item = 1;

        update(&mut state, Message::MenuUp).unwrap();
        assert_eq!(state.selected_menu_item, 0);

        // Can't go below 0
        update(&mut state, Message::MenuUp).unwrap();
        assert_eq!(state.selected_menu_item, 0);
    }

    #[test]
    fn test_menu_navigation_down() {
        let scenario1 = create_test_scenario();
        let scenario2 = create_test_scenario();
        let mut state = AppState::new(vec![scenario1, scenario2]);
        assert_eq!(state.selected_menu_item, 0);

        // Move down once
        update(&mut state, Message::MenuDown).unwrap();
        assert_eq!(state.selected_menu_item, 1);

        // Move down again
        update(&mut state, Message::MenuDown).unwrap();
        assert_eq!(state.selected_menu_item, 2); // Now on Quit

        // Can't go past max items
        update(&mut state, Message::MenuDown).unwrap();
        assert_eq!(state.selected_menu_item, 2);
    }

    #[test]
    fn test_menu_select_start_training() {
        let scenario = create_test_scenario();
        let mut state = AppState::new(vec![scenario]);
        state.selected_menu_item = 0;

        update(&mut state, Message::MenuSelect).unwrap();

        assert_eq!(state.screen, Screen::Task);
        assert!(state.session.is_some());
    }

    #[test]
    fn test_menu_select_quit() {
        let scenario1 = create_test_scenario();
        let scenario2 = create_test_scenario();
        let mut state = AppState::new(vec![scenario1, scenario2]);
        // Select Quit option (index = scenario count)
        state.selected_menu_item = 2;

        update(&mut state, Message::MenuSelect).unwrap();

        assert!(!state.running);
    }

    #[test]
    fn test_start_scenario() {
        let scenario = create_test_scenario();
        let mut state = AppState::new(vec![scenario]);

        update(&mut state, Message::StartScenario(0)).unwrap();

        assert!(state.session.is_some());
        assert_eq!(state.screen, Screen::Task);
    }

    #[test]
    fn test_start_invalid_scenario_index() {
        let scenario = create_test_scenario();
        let mut state = AppState::new(vec![scenario]);

        update(&mut state, Message::StartScenario(999)).unwrap();

        // Should still have None session
        assert!(state.session.is_none());
    }

    #[test]
    fn test_complete_scenario_navigates_to_results() {
        let scenario = create_test_scenario();
        let mut state = AppState::new(vec![scenario]);

        update(&mut state, Message::StartScenario(0)).unwrap();
        assert_eq!(state.screen, Screen::Task);

        update(&mut state, Message::CompleteScenario).unwrap();
        assert_eq!(state.screen, Screen::Results);
    }

    #[test]
    fn test_abandon_scenario_navigates_to_results() {
        let scenario = create_test_scenario();
        let mut state = AppState::new(vec![scenario]);

        update(&mut state, Message::StartScenario(0)).unwrap();
        let session = state.session.as_ref().unwrap();
        assert!(session.is_active());

        update(&mut state, Message::AbandonScenario).unwrap();
        assert_eq!(state.screen, Screen::Results);
        let session = state.session.as_ref().unwrap();
        assert!(!session.is_active());
    }

    #[test]
    fn test_show_hint() {
        let scenario = create_test_scenario();
        let mut state = AppState::new(vec![scenario]);

        update(&mut state, Message::StartScenario(0)).unwrap();
        assert!(!state.show_hint_panel);

        update(&mut state, Message::ShowHint).unwrap();
        assert!(state.show_hint_panel);
        assert!(state.current_hint.is_some());
    }

    #[test]
    fn test_retry_scenario_resets_state() {
        let scenario = create_test_scenario();
        let mut state = AppState::new(vec![scenario]);

        update(&mut state, Message::StartScenario(0)).unwrap();
        if let Some(session) = &mut state.session {
            session.record_action("l".to_string()).unwrap();
        }
        assert_eq!(state.session.as_ref().unwrap().action_count(), 1);

        update(&mut state, Message::RetryScenario).unwrap();
        assert_eq!(state.screen, Screen::Task);
        assert_eq!(state.session.as_ref().unwrap().action_count(), 0);
    }

    #[test]
    fn test_next_scenario_clears_session() {
        let scenario = create_test_scenario();
        let mut state = AppState::new(vec![scenario]);

        update(&mut state, Message::StartScenario(0)).unwrap();
        assert!(state.session.is_some());

        update(&mut state, Message::NextScenario).unwrap();
        assert_eq!(state.screen, Screen::MainMenu);
        assert!(state.session.is_none());
    }

    #[test]
    fn test_back_to_menu_clears_session() {
        let scenario = create_test_scenario();
        let mut state = AppState::new(vec![scenario]);

        update(&mut state, Message::StartScenario(0)).unwrap();
        assert!(state.session.is_some());

        update(&mut state, Message::BackToMenu).unwrap();
        assert_eq!(state.screen, Screen::MainMenu);
        assert!(state.session.is_none());
    }

    #[test]
    fn test_menu_items() {
        let items = AppState::menu_items();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0], "Start Training");
        assert_eq!(items[1], "Quit");
    }

    #[test]
    fn test_scenario_count() {
        let scenarios = vec![create_test_scenario(), create_test_scenario()];
        let state = AppState::new(scenarios);
        assert_eq!(state.scenario_count(), 2);
    }

    #[test]
    fn test_get_scenario() {
        let scenario = create_test_scenario();
        let mut scenarios = vec![scenario.clone()];
        scenarios.push(scenario);
        let state = AppState::new(scenarios);

        assert!(state.get_scenario(0).is_some());
        assert!(state.get_scenario(1).is_some());
        assert!(state.get_scenario(999).is_none());
    }
}
