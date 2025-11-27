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
        Screen::Profile => render_profile_screen_placeholder(frame, state),
        Screen::Statistics => render_statistics_screen_placeholder(frame, state),
    }
}

/// Placeholder for profile screen
// TODO: Iteration 4 - Implement full profile screen with achievements, stats, level progress
fn render_profile_screen_placeholder(frame: &mut Frame, _state: &mut AppState) {
    use ratatui::{
        layout::Alignment,
        style::{Color, Modifier, Style},
        widgets::{Block, Borders, Paragraph},
    };

    let area = frame.area();
    let placeholder =
        Paragraph::new("Profile Screen - Coming Soon!\n\nPress Esc to return to menu")
            .style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .alignment(Alignment::Center)
            .block(Block::default().title(" Profile ").borders(Borders::ALL));

    frame.render_widget(placeholder, area);
}

/// Placeholder for statistics screen
// TODO: Iteration 4 - Implement statistics with mastery breakdown, activity graph, weak commands
fn render_statistics_screen_placeholder(frame: &mut Frame, _state: &mut AppState) {
    use ratatui::{
        layout::Alignment,
        style::{Color, Modifier, Style},
        widgets::{Block, Borders, Paragraph},
    };

    let area = frame.area();
    let placeholder =
        Paragraph::new("Statistics Screen - Coming Soon!\n\nPress Esc to return to menu")
            .style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .alignment(Alignment::Center)
            .block(Block::default().title(" Statistics ").borders(Borders::ALL));

    frame.render_widget(placeholder, area);
}
