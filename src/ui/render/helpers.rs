//! Helper functions for common rendering patterns

use ratatui::{
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders},
};

/// Split a string at a character index without allocating
///
/// Returns byte indices (start, char_byte_pos, end) for efficient slicing.
/// This avoids collecting chars into new Strings.
///
/// # Arguments
///
/// * `s` - The string to split
/// * `char_idx` - Character index (not byte index)
///
/// # Returns
///
/// Tuple of (before_end_byte, char_start_byte, char_end_byte, after_start_byte)
pub(super) fn split_at_char_index(s: &str, char_idx: usize) -> (usize, usize, usize, usize) {
    let mut char_indices = s.char_indices();

    // Find the byte position of the character at char_idx
    let char_byte_start = char_indices
        .nth(char_idx)
        .map(|(idx, _)| idx)
        .unwrap_or(s.len());

    // Find the end byte position of the character (start of next char or end of string)
    let char_byte_end = char_indices.next().map(|(idx, _)| idx).unwrap_or(s.len());

    (
        char_byte_start,
        char_byte_start,
        char_byte_end,
        char_byte_end,
    )
}

/// Get byte indices for a character range without allocating
///
/// Returns (start_byte, end_byte) for a range of characters.
///
/// # Arguments
///
/// * `s` - The string to analyze
/// * `start_char` - Starting character index
/// * `end_char` - Ending character index (exclusive)
///
/// # Returns
///
/// Tuple of (start_byte_index, end_byte_index)
pub(super) fn char_range_to_bytes(s: &str, start_char: usize, end_char: usize) -> (usize, usize) {
    let mut char_indices = s.char_indices().enumerate();

    let start_byte = char_indices
        .find_map(|(idx, (byte_pos, _))| (idx == start_char).then_some(byte_pos))
        .unwrap_or(s.len());

    let end_byte = char_indices
        .find_map(|(idx, (byte_pos, _))| (idx == end_char).then_some(byte_pos))
        .unwrap_or(s.len());

    (start_byte, end_byte)
}

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
