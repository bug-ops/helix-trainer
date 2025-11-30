//! Pure rendering functions for the TUI
//!
//! This module contains all rendering logic for the terminal user interface.
//! Rendering functions may update view-related state (like scroll offsets) but
//! do not modify business logic state.

mod editor;
mod helpers;
mod menu;
mod minigame;
mod mode_selection;
mod popups;
mod profile;
mod results;
mod review;
mod statistics;
mod task;

#[cfg(test)]
mod tests;

use crate::ui::state::AppState;
use ratatui::Frame;

/// Main render function dispatches to screen-specific renderers
///
/// This is the entry point for all rendering. It may update view state like
/// scroll offsets to keep UI elements visible.
///
/// # Arguments
///
/// * `frame` - The ratatui frame to render to
/// * `state` - The application state (mutable for view state updates)
pub fn render(frame: &mut Frame, state: &mut AppState) {
    // Use TypedScreen pattern for type-safe dispatch
    match &state.screen {
        crate::ui::state::TypedScreen::ModeSelection(_) => {
            mode_selection::render_mode_selection(frame, state)
        }
        crate::ui::state::TypedScreen::Menu(_) => menu::render_main_menu(frame, state),
        crate::ui::state::TypedScreen::Task(_) => task::render_task_screen(frame, state),
        crate::ui::state::TypedScreen::Results(_) => results::render_results_screen(frame, state),
        crate::ui::state::TypedScreen::Profile(_) => profile::render_profile_screen(frame, state),
        crate::ui::state::TypedScreen::Statistics(_) => {
            statistics::render_statistics_screen(frame, state)
        }
        crate::ui::state::TypedScreen::Review(_) => review::render_review_screen(frame, state),
        crate::ui::state::TypedScreen::MiniGame(_) => minigame::render_minigame(frame, state),
    }
}
