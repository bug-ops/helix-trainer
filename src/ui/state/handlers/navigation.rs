//! Navigation message handlers
//!
//! Handles screen transitions and application lifecycle messages

use crate::security::UserError;
use crate::ui::state::{AppState, Screen};

/// Handle QuitApp message
///
/// Sets the running flag to false to exit the application
pub fn handle_quit_app(state: &mut AppState) -> Result<(), UserError> {
    state.ui.running = false;
    Ok(())
}

/// Handle NavigateTo message
///
/// Changes the current screen to the specified screen
pub fn handle_navigate_to(state: &mut AppState, screen: Screen) -> Result<(), UserError> {
    state.ui.screen = screen;
    Ok(())
}

/// Handle BackToMenu message
///
/// Returns to the main menu and clears the current session
pub fn handle_back_to_menu(state: &mut AppState) -> Result<(), UserError> {
    state.ui.screen = Screen::MainMenu;
    state.game.session = None;
    state.ui.show_hint_panel = false;
    state.ui.current_hint = None;
    Ok(())
}
