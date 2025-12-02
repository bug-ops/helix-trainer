//! User input handling
//!
//! Converts keyboard events into application messages based on current screen state.

pub mod handlers;
pub mod mapping;

use crossterm::event::KeyEvent;

use helix_trainer::ui::{AppState, Message};

use handlers::*;

/// Handle keyboard events and convert them to messages
///
/// This function is the main entry point for input handling.
/// It dispatches to screen-specific handlers based on current screen state.
///
/// Note: Some handlers (like menu) need mutable access to state
/// for command buffer management.
pub fn handle_key_event(key: KeyEvent, state: &mut AppState) -> Option<Message> {
    use helix_trainer::ui::state::TypedScreen;

    match &state.screen {
        TypedScreen::ModeSelection(_) => handle_mode_selection_keys(key),
        TypedScreen::Menu(_) => handle_menu_keys(key, state),
        TypedScreen::Task(_) => handle_task_keys(key, state),
        TypedScreen::Results(_) => handle_results_keys(key),
        TypedScreen::Profile(_) | TypedScreen::Statistics(_) => {
            handle_profile_stats_keys(key, state)
        }
        TypedScreen::Review(_) => handle_review_keys(key),
        TypedScreen::MiniGame(_) => handle_minigame_keys(key, state),
    }
}
