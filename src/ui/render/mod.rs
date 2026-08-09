//! Pure rendering functions for the TUI
//!
//! This module contains all rendering logic for the terminal user interface.
//! Rendering functions may update view-related state (like scroll offsets) but
//! do not modify business logic state.

use ratatui::style::Color;

/// Muted dark blue color for selection/hover highlighting
///
/// Used for menu item hover, editor selection, and mode selection.
/// Provides better text readability than bright blue.
pub(super) const SELECTION_BG_COLOR: Color = Color::Rgb(60, 80, 120);

mod achievements;
mod category_filters;
mod editor;
mod helpers;
mod highlight;
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
        crate::ui::state::TypedScreen::Achievements(_) => {
            achievements::render_achievements_screen(frame, state)
        }
        crate::ui::state::TypedScreen::CategoryFilters(filters_data) => {
            // Render background screen first based on return destination
            match filters_data.return_to {
                crate::ui::state::ReturnDestination::Menu => {
                    menu::render_main_menu_background(frame, state);
                }
                crate::ui::state::ReturnDestination::PausedMiniGame => {
                    minigame::render_minigame(frame, state);
                }
            }
            // Render category filters popup on top
            category_filters::render_category_filters(frame, state)
        }
        crate::ui::state::TypedScreen::Review(_) => review::render_review_screen(frame, state),
        crate::ui::state::TypedScreen::MiniGame(_) => minigame::render_minigame(frame, state),
    }

    // Render global notifications on top of all screens
    popups::render_notifications(frame, state);
}
