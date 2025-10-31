//! Pure rendering functions for the TUI
//!
//! This module contains all rendering logic for the terminal user interface.
//! Rendering functions may update view-related state (like scroll offsets) but
//! do not modify business logic state.

mod editor;
mod helpers;
mod menu;
mod popups;
mod results;
mod task;

#[cfg(test)]
mod tests;

use crate::ui::state::{AppState, Screen};
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
    match state.screen {
        Screen::MainMenu => menu::render_main_menu(frame, state),
        Screen::Task => task::render_task_screen(frame, state),
        Screen::Results => results::render_results_screen(frame, state),
    }
}
