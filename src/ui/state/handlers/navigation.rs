//! Navigation message handlers
//!
//! Handles screen transitions and application lifecycle messages

use crate::security::UserError;
use crate::ui::state::{AppState, MenuData, ProfileData, Screen, StatisticsData, TypedScreen};

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
    // Convert Screen enum to TypedScreen variant
    state.screen = match screen {
        Screen::MainMenu => TypedScreen::Menu(MenuData::default()),
        Screen::Profile => TypedScreen::Profile(ProfileData::default()),
        Screen::Statistics => TypedScreen::Statistics(StatisticsData::default()),
        // NOTE: Task, Results, and Review screens require data and should not be
        // navigated to via NavigateTo - they have their own handlers
        Screen::Task | Screen::Results | Screen::Review => {
            // Keep current screen if trying to navigate to a data-dependent screen
            return Ok(());
        }
    };
    Ok(())
}

/// Handle BackToMenu message
///
/// Returns to the main menu, preserving menu state if already on menu
pub fn handle_back_to_menu(state: &mut AppState) -> Result<(), UserError> {
    // Preserve menu selection if already on menu
    let menu_data = if let TypedScreen::Menu(data) = &state.screen {
        data.clone()
    } else {
        MenuData::default()
    };

    state.screen = TypedScreen::Menu(menu_data);
    state.game.session = None;
    Ok(())
}
