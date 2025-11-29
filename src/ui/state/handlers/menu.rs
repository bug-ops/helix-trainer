//! Menu interaction message handlers
//!
//! Handles menu navigation and selection

use crate::security::UserError;
use crate::ui::state::{AppState, Message, TypedScreen, update};

/// Handle MenuUp message
///
/// Moves menu selection up (with bounds checking)
pub fn handle_menu_up(state: &mut AppState) -> Result<(), UserError> {
    if let TypedScreen::Menu(menu_data) = &mut state.screen {
        if menu_data.selected_item > 0 {
            menu_data.selected_item -= 1;
        }
    }
    Ok(())
}

/// Handle MenuDown message
///
/// Moves menu selection down (with bounds checking)
pub fn handle_menu_down(state: &mut AppState) -> Result<(), UserError> {
    if let TypedScreen::Menu(menu_data) = &mut state.screen {
        // Total menu items = filtered scenarios + Review + Profile + Statistics + Quit
        let max_items = state.game.scenario_collection.count() + 4;
        if menu_data.selected_item < max_items - 1 {
            menu_data.selected_item += 1;
        }
    }
    Ok(())
}

/// Handle MenuSelect message
///
/// Executes action based on currently selected menu item
pub fn handle_menu_select(state: &mut AppState) -> Result<(), UserError> {
    let selected = if let TypedScreen::Menu(menu_data) = &state.screen {
        menu_data.selected_item
    } else {
        // Not on menu screen, nothing to do
        return Ok(());
    };

    let scenario_count = state.game.scenario_collection.count();

    if selected < scenario_count {
        // Start selected scenario (0..scenario_count-1)
        update(state, Message::StartScenario(selected))?;
    } else if selected == scenario_count {
        // Review Commands (index = scenario_count)
        update(state, Message::StartReviewSession)?;
    } else if selected == scenario_count + 1 {
        // View Profile (index = scenario_count + 1)
        update(state, Message::ShowProfile)?;
    } else if selected == scenario_count + 2 {
        // Statistics (index = scenario_count + 2)
        update(state, Message::ShowStatistics)?;
    } else if selected == scenario_count + 3 {
        // Quit (index = scenario_count + 3)
        update(state, Message::QuitApp)?;
    }
    Ok(())
}
