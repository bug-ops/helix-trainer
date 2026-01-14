//! Helper functions for common rendering patterns

use ratatui::{
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders},
};

/// Calculate centered popup area with given dimensions
///
/// # Arguments
///
/// * `parent` - The parent area to center within
/// * `width` - Desired popup width
/// * `height` - Desired popup height
///
/// # Returns
///
/// A Rect centered within the parent area
pub(super) fn centered_popup(parent: Rect, width: u16, height: u16) -> Rect {
    let popup_x = (parent.width.saturating_sub(width)) / 2;
    let popup_y = (parent.height.saturating_sub(height)) / 2;

    Rect {
        x: popup_x,
        y: popup_y,
        width,
        height,
    }
}

/// Create a standard popup block with borders
///
/// # Arguments
///
/// * `title` - Optional title for the popup
/// * `border_color` - Color for the border
///
/// # Returns
///
/// A Block with standard styling
pub(super) fn popup_block(title: Option<&str>, border_color: Color) -> Block<'_> {
    let mut block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .style(Style::default().bg(Color::Black));

    if let Some(t) = title {
        block = block.title(t);
    }

    block
}

/// Calculate the inner area of a rect (excluding borders)
///
/// # Arguments
///
/// * `outer` - The outer rect with borders
///
/// # Returns
///
/// The inner rect without borders
pub(super) fn inner_rect(outer: Rect) -> Rect {
    Rect {
        x: outer.x + 1,
        y: outer.y + 1,
        width: outer.width.saturating_sub(2),
        height: outer.height.saturating_sub(2),
    }
}

/// Dark overlay color for dimming background behind popups
pub(super) const DIMMED_BG_COLOR: Color = Color::Rgb(20, 20, 25);

/// Render a dimmed overlay to darken the background behind a popup
///
/// This creates a modal effect by dimming the content behind the popup.
///
/// # Arguments
///
/// * `frame` - The frame to render to
/// * `area` - The area to dim (typically frame.area())
pub(super) fn render_dimmed_overlay(frame: &mut ratatui::Frame, area: Rect) {
    let overlay = Block::default().style(Style::default().bg(DIMMED_BG_COLOR));
    frame.render_widget(overlay, area);
}

// Re-export find_surrounding_brackets from helix module
pub(super) use crate::helix::find_surrounding_brackets;
