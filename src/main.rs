//! Main entry point for the Helix Keybindings Trainer
//!
//! This is the application's entry point. It initializes the terminal UI,
//! loads scenarios, and runs the main event loop.

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use helix_trainer::{
    config::ScenarioLoader,
    ui::{self, AppState, Message},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::io;
use std::time::Duration;
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::prelude::*;
use tracing_subscriber::{EnvFilter, fmt};

/// Initialize secure logging
fn init_secure_logging() -> Result<()> {
    // Create filter that excludes sensitive modules at high log levels
    let filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .from_env_lossy()
        // Never log PTY communication at debug level in production
        .add_directive("helix_trainer::helix::pty_controller=warn".parse()?)
        .add_directive("helix_trainer::config::scenarios=info".parse()?);

    // Configure formatter to sanitize output
    let fmt_layer = fmt::layer()
        .with_target(true)
        .with_thread_ids(false) // Don't leak thread info
        .with_thread_names(false)
        .with_file(false) // Don't leak file paths in production
        .with_line_number(false);

    tracing_subscriber::registry()
        .with(filter)
        .with(fmt_layer)
        .init();

    Ok(())
}

/// Main entry point
#[tokio::main]
async fn main() -> Result<()> {
    // Warn if running debug build
    #[cfg(debug_assertions)]
    {
        eprintln!("WARNING: Running debug build. Not for production use!");
        eprintln!("Build with: cargo build --release");
    }

    // Initialize secure logging
    init_secure_logging()?;

    tracing::info!("Starting Helix Keybindings Trainer");

    // Load scenarios from default location
    let loader = ScenarioLoader::new();
    let scenario_file = loader.load(std::path::Path::new("./scenarios/basic.toml"))?;

    tracing::info!("Loaded {} scenarios", scenario_file.len());

    // Initialize app state
    let mut app_state = AppState::new(scenario_file);

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    tracing::debug!("Terminal initialized");

    // Run the main event loop
    let result = run_app(&mut terminal, &mut app_state);

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    tracing::info!("Exiting Helix Keybindings Trainer");

    result
}

/// Main application event loop
///
/// This function runs the core event loop that:
/// 1. Renders the current state
/// 2. Handles user input
/// 3. Updates state based on messages
/// 4. Repeats until the app exits
fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    state: &mut AppState,
) -> Result<()> {
    loop {
        // Render the current state
        terminal.draw(|f| ui::render(f, state))?;

        // Check if we should exit
        if !state.running {
            break;
        }

        // Check if scenario completed and delay elapsed
        if let Some(completion_time) = state.completion_time
            && completion_time.elapsed() >= Duration::from_millis(1500)
        {
            tracing::debug!("Success screen delay elapsed, transitioning to results");
            ui::update(state, Message::CompleteScenario)?;
            state.completion_time = None;
        }

        // Handle events with timeout
        if event::poll(Duration::from_millis(100))?
            && let Event::Key(key) = event::read()?
        {
            // Handle global quit shortcut first
            if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                tracing::debug!("User pressed Ctrl+C");
                ui::update(state, Message::QuitApp)?;
                continue;
            }

            // Dispatch to screen-specific handlers
            if let Some(msg) = handle_key_event(key, state) {
                tracing::debug!("Message: {:?}", msg);
                ui::update(state, msg)?;
            }
        }
    }

    Ok(())
}

/// Handle keyboard events and convert them to messages
///
/// This function is responsible for converting keyboard input into
/// application messages based on the current screen.
fn handle_key_event(key: KeyEvent, state: &AppState) -> Option<Message> {
    match state.screen {
        ui::Screen::MainMenu => handle_menu_keys(key),
        ui::Screen::Task => handle_task_keys(key, state),
        ui::Screen::Results => handle_results_keys(key),
    }
}

/// Handle keyboard events on the main menu screen
fn handle_menu_keys(key: KeyEvent) -> Option<Message> {
    match key.code {
        KeyCode::Char('q') => Some(Message::QuitApp),
        KeyCode::Up | KeyCode::Char('k') => Some(Message::MenuUp),
        KeyCode::Down | KeyCode::Char('j') => Some(Message::MenuDown),
        KeyCode::Enter => Some(Message::MenuSelect),
        _ => None,
    }
}

/// Handle keyboard events on the task screen
fn handle_task_keys(key: KeyEvent, state: &AppState) -> Option<Message> {
    // Handle special UI keys first
    match key.code {
        KeyCode::F(1) => return Some(Message::ShowHint),
        KeyCode::Esc => return Some(Message::AbandonScenario),
        _ => {}
    }

    // Check if we're in Insert mode
    let in_insert_mode = state
        .session
        .as_ref()
        .map(|session| session.is_insert_mode())
        .unwrap_or(false);

    // In Insert mode, capture text input
    if in_insert_mode {
        match key.code {
            KeyCode::Char(c) => {
                return Some(Message::ExecuteCommand(c.to_string()));
            }
            KeyCode::Enter => {
                return Some(Message::ExecuteCommand("\n".to_string()));
            }
            KeyCode::Backspace => {
                return Some(Message::ExecuteCommand("Backspace".to_string()));
            }
            KeyCode::Left => {
                return Some(Message::ExecuteCommand("ArrowLeft".to_string()));
            }
            KeyCode::Right => {
                return Some(Message::ExecuteCommand("ArrowRight".to_string()));
            }
            KeyCode::Up => {
                return Some(Message::ExecuteCommand("ArrowUp".to_string()));
            }
            KeyCode::Down => {
                return Some(Message::ExecuteCommand("ArrowDown".to_string()));
            }
            _ => {}
        }
    }

    // Convert key to Helix command string (Normal mode)
    let command = match (key.code, key.modifiers) {
        // Movement commands
        (KeyCode::Char('h'), KeyModifiers::NONE) => "h",
        (KeyCode::Char('j'), KeyModifiers::NONE) => "j",
        (KeyCode::Char('k'), KeyModifiers::NONE) => "k",
        (KeyCode::Char('l'), KeyModifiers::NONE) => "l",

        // Word movement
        (KeyCode::Char('w'), KeyModifiers::NONE) => "w",
        (KeyCode::Char('b'), KeyModifiers::NONE) => "b",
        (KeyCode::Char('e'), KeyModifiers::NONE) => "e",

        // Line movement
        (KeyCode::Char('0'), KeyModifiers::NONE) => "0",
        (KeyCode::Char('$'), KeyModifiers::NONE) => "$",

        // Deletion commands
        (KeyCode::Char('x'), KeyModifiers::NONE) => "x",
        (KeyCode::Char('d'), KeyModifiers::NONE) => "d",
        (KeyCode::Char('c'), KeyModifiers::NONE) => "c",

        // Yank and paste
        (KeyCode::Char('y'), KeyModifiers::NONE) => "y",
        (KeyCode::Char('p'), KeyModifiers::NONE) => "p",
        (KeyCode::Char('P'), KeyModifiers::SHIFT) => "P",

        // Mode changes and editing
        (KeyCode::Char('i'), KeyModifiers::NONE) => "i",
        (KeyCode::Char('a'), KeyModifiers::NONE) => "a",
        (KeyCode::Char('I'), KeyModifiers::SHIFT) => "I",
        (KeyCode::Char('A'), KeyModifiers::SHIFT) => "A",
        (KeyCode::Char('o'), KeyModifiers::NONE) => "o",
        (KeyCode::Char('O'), KeyModifiers::SHIFT) => "O",

        // Undo/Redo
        (KeyCode::Char('u'), KeyModifiers::NONE) => "u",
        (KeyCode::Char('r'), KeyModifiers::CONTROL) => "ctrl-r",

        // Document movement
        (KeyCode::Char('g'), KeyModifiers::NONE) => "g",
        (KeyCode::Char('G'), KeyModifiers::NONE) => "G",

        _ => return None,
    };

    Some(Message::ExecuteCommand(command.to_string()))
}

/// Handle keyboard events on the results screen
fn handle_results_keys(key: KeyEvent) -> Option<Message> {
    match key.code {
        KeyCode::Char('q') => Some(Message::QuitApp),
        KeyCode::Char('r') => Some(Message::RetryScenario),
        KeyCode::Char('m') => Some(Message::BackToMenu),
        _ => None,
    }
}

#[cfg(test)]
#[allow(unused_variables)] // Test state setup
mod tests {
    use super::*;

    #[test]
    fn test_menu_key_q_quits() {
        let key = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE);
        let state = AppState::new(vec![]);
        let msg = handle_menu_keys(key);
        assert_eq!(msg, Some(Message::QuitApp));
    }

    #[test]
    fn test_menu_key_j_moves_down() {
        let key = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE);
        let state = AppState::new(vec![]);
        let msg = handle_menu_keys(key);
        assert_eq!(msg, Some(Message::MenuDown));
    }

    #[test]
    fn test_menu_key_k_moves_up() {
        let key = KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE);
        let state = AppState::new(vec![]);
        let msg = handle_menu_keys(key);
        assert_eq!(msg, Some(Message::MenuUp));
    }

    #[test]
    fn test_menu_key_enter_selects() {
        let key = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        let state = AppState::new(vec![]);
        let msg = handle_menu_keys(key);
        assert_eq!(msg, Some(Message::MenuSelect));
    }

    #[test]
    fn test_task_key_f1_shows_hint() {
        let key = KeyEvent::new(KeyCode::F(1), KeyModifiers::NONE);
        let state = AppState::new(vec![]);
        let msg = handle_task_keys(key, &state);
        assert_eq!(msg, Some(Message::ShowHint));
    }

    #[test]
    fn test_task_key_h_moves_left() {
        let key = KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE);
        let state = AppState::new(vec![]);
        let msg = handle_task_keys(key, &state);
        assert_eq!(msg, Some(Message::ExecuteCommand("h".to_string())));
    }

    #[test]
    fn test_task_key_esc_abandons() {
        let key = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
        let state = AppState::new(vec![]);
        let msg = handle_task_keys(key, &state);
        assert_eq!(msg, Some(Message::AbandonScenario));
    }

    #[test]
    fn test_results_key_r_retries() {
        let key = KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE);
        let state = AppState::new(vec![]);
        let msg = handle_results_keys(key);
        assert_eq!(msg, Some(Message::RetryScenario));
    }

    #[test]
    fn test_results_key_m_returns_menu() {
        let key = KeyEvent::new(KeyCode::Char('m'), KeyModifiers::NONE);
        let state = AppState::new(vec![]);
        let msg = handle_results_keys(key);
        assert_eq!(msg, Some(Message::BackToMenu));
    }

    #[test]
    fn test_results_key_q_quits() {
        let key = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE);
        let state = AppState::new(vec![]);
        let msg = handle_results_keys(key);
        assert_eq!(msg, Some(Message::QuitApp));
    }

    #[test]
    fn test_unknown_key_returns_none() {
        let key = KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE);
        let state = AppState::new(vec![]);
        let msg = handle_menu_keys(key);
        assert_eq!(msg, None);
    }
}
