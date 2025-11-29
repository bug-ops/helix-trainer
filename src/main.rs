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
    gamification::{ProfileStorage, QuestGenerator, StreakManager},
    helix::commands::*,
    learning::PerformanceTracker,
    ui::{self, AppState, Message, state::TypedScreen},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::borrow::Cow;
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
fn main() -> Result<()> {
    // Warn if running debug build
    #[cfg(debug_assertions)]
    {
        eprintln!("WARNING: Running debug build. Not for production use!");
        eprintln!("Build with: cargo build --release");
    }

    // Initialize secure logging
    init_secure_logging()?;

    tracing::info!("Starting Helix Keybindings Trainer");

    // Load scenarios from language-specific directory (recursively)
    // Use current locale from rust-i18n
    let current_locale = rust_i18n::locale();
    let locale_str: &str = current_locale.as_ref();
    let scenarios_path = format!("./scenarios/{}", locale_str);

    tracing::info!("Loading scenarios for locale: {}", locale_str);

    let loader = ScenarioLoader::new();
    let scenarios = loader.load_directory(std::path::Path::new(&scenarios_path))?;

    tracing::info!(
        "Loaded {} scenarios from {} locale",
        scenarios.len(),
        locale_str
    );

    // Initialize profile system
    let profile_storage = ProfileStorage::new();
    let mut profile = profile_storage.load().unwrap_or_else(|e| {
        tracing::warn!("Failed to load profile: {}, creating new", e);
        helix_trainer::gamification::UserProfile::new()
    });

    // Check if we need to refresh daily quests
    let now = chrono::Utc::now();
    if should_refresh_quests(&profile, now) {
        tracing::info!("Refreshing daily quests for new day");
        let tracker = PerformanceTracker::new();
        profile.reset_daily_quests();
        profile.daily_quests = QuestGenerator::generate_quests(&profile, &tracker);
    }

    // Update streak (checks last activity)
    let streak_change = StreakManager::update_streak(&mut profile);
    tracing::debug!("Streak status: {:?}", streak_change);

    // Initialize performance tracker
    let tracker = PerformanceTracker::new();

    // Initialize app state
    let mut app_state = AppState::new(scenarios, profile, profile_storage, tracker);

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    tracing::debug!("Terminal initialized");

    // Run the main event loop
    let result = run_app(&mut terminal, &mut app_state);

    // Save profile before exit
    if let Err(e) = app_state.save_profile_immediate() {
        tracing::error!("Failed to save profile on exit: {}", e);
    } else {
        tracing::info!("Profile saved successfully");
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    tracing::info!("Exiting Helix Keybindings Trainer");

    result
}

/// Check if daily quests should be refreshed
fn should_refresh_quests(
    profile: &helix_trainer::gamification::UserProfile,
    now: chrono::DateTime<chrono::Utc>,
) -> bool {
    now.date_naive() != profile.last_quest_refresh.date_naive()
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
        if !state.ui.running {
            break;
        }

        // Check if scenario completed and delay elapsed
        if let Some(completion_time) = state.ui.completion_time
            && completion_time.elapsed() >= Duration::from_millis(1500)
        {
            tracing::debug!("Success screen delay elapsed, transitioning to results");
            ui::update(state, Message::CompleteScenario)?;
            state.ui.completion_time = None;
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
    match &state.screen {
        TypedScreen::Menu(_) => handle_menu_keys(key, state),
        TypedScreen::Task(_) => handle_task_keys(key, state),
        TypedScreen::Results(_) => handle_results_keys(key),
        TypedScreen::Profile(_) | TypedScreen::Statistics(_) => {
            handle_profile_stats_keys(key, state)
        }
        TypedScreen::Review(_) => handle_review_keys(key),
    }
}

/// Handle keyboard events on profile and statistics screens
fn handle_profile_stats_keys(key: KeyEvent, state: &AppState) -> Option<Message> {
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
fn handle_menu_keys(key: KeyEvent, state: &AppState) -> Option<Message> {
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

/// Handle keyboard events on the task screen
/// Handle special UI keys (F1, ?, Ctrl+Q)
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

/// Convert key to text input for Insert mode
fn handle_insert_mode_input(key: KeyEvent) -> Option<Cow<'static, str>> {
    match key.code {
        KeyCode::Char(c) => Some(Cow::Owned(c.to_string())),
        KeyCode::Enter => Some(Cow::Borrowed("\n")),
        KeyCode::Backspace => Some(Cow::Borrowed(CMD_BACKSPACE)),
        KeyCode::Left => Some(Cow::Borrowed(CMD_ARROW_LEFT)),
        KeyCode::Right => Some(Cow::Borrowed(CMD_ARROW_RIGHT)),
        KeyCode::Up => Some(Cow::Borrowed(CMD_ARROW_UP)),
        KeyCode::Down => Some(Cow::Borrowed(CMD_ARROW_DOWN)),
        _ => None,
    }
}

/// Map key to Helix command (Normal mode)
fn map_key_to_helix_command(key: KeyEvent) -> Option<&'static str> {
    match (key.code, key.modifiers) {
        // Movement commands
        (KeyCode::Char('h'), KeyModifiers::NONE) => Some(CMD_MOVE_LEFT),
        (KeyCode::Char('j'), KeyModifiers::NONE) => Some(CMD_MOVE_DOWN),
        (KeyCode::Char('k'), KeyModifiers::NONE) => Some(CMD_MOVE_UP),
        (KeyCode::Char('l'), KeyModifiers::NONE) => Some(CMD_MOVE_RIGHT),

        // Word movement
        (KeyCode::Char('w'), KeyModifiers::NONE) => Some(CMD_MOVE_WORD_FORWARD),
        (KeyCode::Char('b'), KeyModifiers::NONE) => Some(CMD_MOVE_WORD_BACKWARD),
        (KeyCode::Char('e'), KeyModifiers::NONE) => Some(CMD_MOVE_WORD_END),

        // Line movement
        (KeyCode::Char('0'), KeyModifiers::NONE) => Some(CMD_MOVE_LINE_START),
        (KeyCode::Char('$'), KeyModifiers::NONE) => Some(CMD_MOVE_LINE_END),

        // Deletion commands
        (KeyCode::Char('x'), KeyModifiers::NONE) => Some(CMD_DELETE_CHAR),
        (KeyCode::Char('d'), KeyModifiers::NONE) => Some("d"), // Single 'd' for multi-key handling
        (KeyCode::Char('c'), KeyModifiers::NONE) => Some(CMD_CHANGE),
        (KeyCode::Char('J'), KeyModifiers::SHIFT) => Some(CMD_JOIN_LINES),

        // Indentation
        (KeyCode::Char('>'), KeyModifiers::NONE) => Some(CMD_INDENT),
        (KeyCode::Char('<'), KeyModifiers::NONE) => Some(CMD_DEDENT),

        // Yank and paste
        (KeyCode::Char('y'), KeyModifiers::NONE) => Some(CMD_YANK),
        (KeyCode::Char('p'), KeyModifiers::NONE) => Some(CMD_PASTE_AFTER),
        (KeyCode::Char('P'), KeyModifiers::SHIFT) => Some(CMD_PASTE_BEFORE),

        // Mode changes and editing
        (KeyCode::Char('i'), KeyModifiers::NONE) => Some(CMD_INSERT),
        (KeyCode::Char('a'), KeyModifiers::NONE) => Some(CMD_APPEND),
        (KeyCode::Char('I'), KeyModifiers::SHIFT) => Some(CMD_INSERT_LINE_START),
        (KeyCode::Char('A'), KeyModifiers::SHIFT) => Some(CMD_APPEND_LINE_END),
        (KeyCode::Char('o'), KeyModifiers::NONE) => Some(CMD_OPEN_BELOW),
        (KeyCode::Char('O'), KeyModifiers::SHIFT) => Some(CMD_OPEN_ABOVE),

        // Replace character
        (KeyCode::Char('r'), KeyModifiers::NONE) => Some(CMD_REPLACE),

        // Undo/Redo
        (KeyCode::Char('u'), KeyModifiers::NONE) => Some(CMD_UNDO),
        (KeyCode::Char('U'), KeyModifiers::SHIFT) => Some(CMD_REDO),
        (KeyCode::Char('r'), KeyModifiers::CONTROL) => Some("ctrl-r"), // TODO: add constant

        // Repeat last action
        (KeyCode::Char('.'), KeyModifiers::NONE) => Some(CMD_REPEAT),

        // Document movement
        (KeyCode::Char('g'), KeyModifiers::NONE) => Some("g"), // Note: multi-key 'gg' handled elsewhere
        (KeyCode::Char('G'), KeyModifiers::NONE) => Some(CMD_GOTO_FILE_END),

        _ => None,
    }
}

/// Check if current game session is in Insert mode
fn is_in_insert_mode(state: &AppState) -> bool {
    if let TypedScreen::Task(task_data) = &state.screen {
        task_data.session.is_insert_mode()
    } else {
        false
    }
}

fn handle_task_keys(key: KeyEvent, state: &AppState) -> Option<Message> {
    // Check special UI keys first
    if let Some(msg) = handle_task_special_keys(key) {
        return Some(msg);
    }

    // Handle Insert mode input
    if is_in_insert_mode(state) {
        return handle_insert_mode_input(key).map(Message::ExecuteCommand);
    }

    // Handle Normal mode commands
    map_key_to_helix_command(key).map(|cmd| Message::ExecuteCommand(Cow::Borrowed(cmd)))
}

/// Handle keyboard events on the results screen
fn handle_results_keys(key: KeyEvent) -> Option<Message> {
    match key.code {
        KeyCode::Char('q') => Some(Message::QuitApp),
        KeyCode::Char('r') => Some(Message::RetryScenario),
        KeyCode::Char('m') => Some(Message::BackToMenu),
        KeyCode::Char('p') => Some(Message::ShowProfile),
        _ => None,
    }
}

/// Handle keyboard events on the review session screen
fn handle_review_keys(key: KeyEvent) -> Option<Message> {
    match key.code {
        KeyCode::Char('s') => Some(Message::CompleteReviewCommand { success: true }),
        KeyCode::Char('f') => Some(Message::CompleteReviewCommand { success: false }),
        KeyCode::Esc => Some(Message::AbandonReviewSession),
        KeyCode::Char('q') => Some(Message::QuitApp),
        _ => None,
    }
}

#[cfg(test)]
#[allow(unused_variables)] // Test state setup
mod tests {
    use super::*;

    fn create_test_app_state() -> AppState {
        let profile = helix_trainer::gamification::UserProfile::new();
        let storage = helix_trainer::gamification::ProfileStorage::new();
        let tracker = helix_trainer::learning::PerformanceTracker::new();
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
        let state = create_test_app_state();
        let msg = handle_results_keys(key);
        assert_eq!(msg, Some(Message::RetryScenario));
    }

    #[test]
    fn test_results_key_m_returns_menu() {
        let key = KeyEvent::new(KeyCode::Char('m'), KeyModifiers::NONE);
        let state = create_test_app_state();
        let msg = handle_results_keys(key);
        assert_eq!(msg, Some(Message::BackToMenu));
    }

    #[test]
    fn test_results_key_q_quits() {
        let key = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE);
        let state = create_test_app_state();
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
}
