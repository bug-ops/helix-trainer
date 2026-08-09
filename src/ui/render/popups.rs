//! Popup rendering (hints, success, key history, notifications)

use super::helpers::{centered_popup, inner_rect, popup_block};
use crate::constants::{
    BIG_TEXT_CHAR_WIDTH_CELLS, BIG_TEXT_HEIGHT_LINES, HINT_POPUP_MAX_HEIGHT, HINT_POPUP_MAX_WIDTH,
    KEY_HISTORY_DISPLAY_SIZE, KEY_HISTORY_MIN_WIDTH,
};
use crate::ui::state::{AppState, TypedScreen};
use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};
use rust_i18n::t;
use tui_big_text::{BigText, PixelSize};

/// Render a centered hint popup
pub(super) fn render_hint_popup(frame: &mut Frame, state: &AppState) {
    // Only render if we're on Task screen
    let TypedScreen::Task(task_data) = &state.screen else {
        return;
    };

    let area = frame.area();

    // Calculate popup dimensions with constraints
    let popup_width = HINT_POPUP_MAX_WIDTH.min(area.width.saturating_sub(4));
    let popup_height = HINT_POPUP_MAX_HEIGHT.min(area.height.saturating_sub(4));

    // Create centered popup area
    let popup_area = centered_popup(area, popup_width, popup_height);

    // Render popup background with border
    let hint_title = t!("hint.title").to_string();
    let background = popup_block(Some(&hint_title), Color::White);
    frame.render_widget(&background, popup_area);

    // Render hint text inside popup
    if let Some(hint) = &task_data.current_hint {
        let inner = inner_rect(popup_area);

        let hint_text = Paragraph::new(hint.as_str())
            .wrap(Wrap { trim: true })
            .style(Style::default().fg(Color::Yellow))
            .alignment(Alignment::Center);

        frame.render_widget(hint_text, inner);
    }
}

/// Render key history popup showing last 5 keys pressed with large text
///
/// Used by both training mode and arcade mode. `reserved_bottom` is the height
/// (in rows) of the fixed-height status/HUD bars anchored to the bottom of the
/// screen (e.g. stats, timer, instructions bars); the popup is positioned
/// directly above that reserved strip so it never overlaps those panes, and is
/// skipped entirely if there isn't enough room to render without corrupting
/// the editor panes above it.
pub(super) fn render_key_history_popup(
    frame: &mut Frame,
    key_history: &[String],
    reserved_bottom: u16,
) {
    if key_history.is_empty() {
        return;
    }

    let area = frame.area();

    // Build text from recent keys
    let mut key_text = String::new();
    for (idx, key) in key_history
        .iter()
        .take(KEY_HISTORY_DISPLAY_SIZE)
        .enumerate()
    {
        if idx > 0 {
            key_text.push(' ');
        }
        key_text.push_str(key);
    }

    // Calculate required dimensions before consuming key_text
    // PixelSize::HalfHeight only halves the glyph height, not the width - each
    // character (including the space separators) is still BIG_TEXT_CHAR_WIDTH_CELLS wide
    let chars_count = key_text.chars().count();
    let popup_width =
        ((chars_count * BIG_TEXT_CHAR_WIDTH_CELLS).max(KEY_HISTORY_MIN_WIDTH as usize) as u16)
            .min(area.width.saturating_sub(4));
    // HalfHeight halves the glyph height (BIG_TEXT_HEIGHT_LINES) so the popup fits
    // within the space reserved above the bottom HUD bars instead of spilling
    // into the editor panes.
    let popup_height = BIG_TEXT_HEIGHT_LINES / 2 + 2; // +2 for borders

    // Not enough vertical room above the reserved HUD strip - skip rendering
    // rather than overlapping the editor panes or the HUD bars themselves.
    if area.height < reserved_bottom + popup_height {
        return;
    }

    // Create BigText widget with large font and cyan color
    let big_text = BigText::builder()
        .pixel_size(PixelSize::HalfHeight)
        .style(Style::default().fg(Color::Cyan))
        .lines(vec![key_text.into()])
        .centered()
        .build();

    // Position in bottom right corner, directly above the reserved HUD strip
    let popup_x = area.width.saturating_sub(popup_width + 2);
    let popup_y = area.height.saturating_sub(reserved_bottom + popup_height);

    let popup_area = Rect {
        x: popup_x,
        y: popup_y,
        width: popup_width,
        height: popup_height,
    };

    // Render with border
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .style(Style::default().bg(Color::Black));

    let inner_area = block.inner(popup_area);
    frame.render_widget(block, popup_area);
    frame.render_widget(big_text, inner_area);
}

/// Render success popup when scenario is completed
pub(super) fn render_success_popup(frame: &mut Frame) {
    render_result_popup(
        frame,
        t!("success.title").as_ref(),
        t!("success.message").as_ref(),
        Color::Green,
    );
}

/// Render a result popup with customizable title, message, and color
///
/// Used for both training mode (SUCCESS!) and arcade mode (SUCCESS!/TIME'S UP!)
pub(super) fn render_result_popup(frame: &mut Frame, title: &str, message: &str, color: Color) {
    let area = frame.area();

    // Create centered popup area
    let popup_area = centered_popup(area, 40, 7);

    // Result message
    let result_text = vec![
        Line::from(""),
        Line::from(Span::styled(
            title,
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(message, Style::default().fg(Color::White))),
        Line::from(""),
    ];

    let result_paragraph = Paragraph::new(result_text)
        .alignment(Alignment::Center)
        .block(popup_block(None, color));

    frame.render_widget(result_paragraph, popup_area);
}

/// Render notification popups in the top-right corner
///
/// Displays up to 3 notifications stacked vertically. Notifications auto-expire
/// after their configured duration (default 3 seconds).
pub(super) fn render_notifications(frame: &mut Frame, state: &AppState) {
    let visible = state.ui.notifications.visible();
    if visible.is_empty() {
        return;
    }

    let area = frame.area();

    // Notification dimensions
    let notification_width = 40.min(area.width.saturating_sub(4));
    let notification_height = 5; // Title + message + borders
    let spacing = 1; // Space between notifications

    // Position in top-right corner
    let base_x = area.width.saturating_sub(notification_width + 2);
    let base_y = 2;

    // Render each visible notification
    for (idx, notification) in visible.iter().enumerate() {
        let y_offset = base_y + (idx as u16 * (notification_height + spacing));

        // Don't render if it would go off-screen
        if y_offset + notification_height > area.height {
            break;
        }

        let popup_area = Rect {
            x: base_x,
            y: y_offset,
            width: notification_width,
            height: notification_height,
        };

        // Get color for this notification type
        let color = notification.color();

        // Build notification content
        let title = notification.title();
        let message = notification.message();

        let text = vec![
            Line::from(""),
            Line::from(Span::styled(
                title,
                Style::default().fg(color).add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(message, Style::default().fg(Color::White))),
        ];

        let paragraph = Paragraph::new(text)
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true })
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(color))
                    .style(Style::default().bg(Color::Black)),
            );

        frame.render_widget(paragraph, popup_area);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{Terminal, backend::TestBackend, buffer::Buffer};

    /// `BIG_TEXT_HEIGHT_LINES / 2 + 2` from `render_key_history_popup`.
    const POPUP_HEIGHT: u16 = BIG_TEXT_HEIGHT_LINES / 2 + 2;

    fn render_popup(width: u16, height: u16, reserved_bottom: u16, keys: &[String]) -> Buffer {
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| render_key_history_popup(frame, keys, reserved_bottom))
            .unwrap();
        terminal.backend().buffer().clone()
    }

    fn is_blank(buffer: &Buffer) -> bool {
        buffer
            .content
            .iter()
            .all(|cell| cell.symbol() == " " || cell.symbol().is_empty())
    }

    fn row_is_blank(buffer: &Buffer, y: u16) -> bool {
        (0..buffer.area.width).all(|x| {
            let symbol = buffer[(x, y)].symbol();
            symbol == " " || symbol.is_empty()
        })
    }

    #[test]
    fn test_render_key_history_popup_empty_history_skips_render() {
        let buffer = render_popup(80, 20, 6, &[]);
        assert!(is_blank(&buffer), "empty key history must render nothing");
    }

    #[test]
    fn test_render_key_history_popup_skips_when_not_enough_room() {
        let reserved_bottom = 6;
        // One row short of the minimum height the popup needs above the reserved strip.
        let height = reserved_bottom + POPUP_HEIGHT - 1;
        let buffer = render_popup(80, height, reserved_bottom, &["a".to_string()]);

        assert!(
            is_blank(&buffer),
            "popup must not render when there isn't enough vertical room"
        );
    }

    #[test]
    fn test_render_key_history_popup_reserves_space_above_bar() {
        let reserved_bottom = 6;
        let height = 20;
        let buffer = render_popup(80, height, reserved_bottom, &["a".to_string()]);

        // The reserved HUD strip (the bottom `reserved_bottom` rows) must stay untouched.
        for y in (height - reserved_bottom)..height {
            assert!(
                row_is_blank(&buffer, y),
                "reserved bottom row {y} was overwritten by the popup"
            );
        }

        // The popup itself must have rendered directly above the reserved strip
        // (its bottom border sits on the row just above the reserved area).
        let popup_bottom_row = height - reserved_bottom - 1;
        assert!(
            !row_is_blank(&buffer, popup_bottom_row),
            "popup was not rendered directly above the reserved strip"
        );
    }
}
