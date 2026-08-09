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
/// Used by both training mode and arcade mode. `bounds` is the outer `Rect`
/// of the Target editor panel (as returned by `render_editor_pair`); the
/// popup is anchored to the bottom-right corner of `bounds`'s *inner* area
/// (i.e. inset from `bounds`'s own border) and its width is clipped to fit
/// within that inner area, so it can never draw over the Target panel's
/// border - regardless of how wide the key text grows. If the inner area is
/// smaller than the minimum legible popup size, rendering is skipped for
/// that frame instead of drawing illegibly clipped glyphs.
pub(super) fn render_key_history_popup(frame: &mut Frame, key_history: &[String], bounds: Rect) {
    if key_history.is_empty() {
        return;
    }

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

    let safe_area = inner_rect(bounds);

    // HalfHeight halves the glyph height (BIG_TEXT_HEIGHT_LINES) so the popup fits
    // within the Target panel's inner area instead of spilling past its border.
    let popup_height = BIG_TEXT_HEIGHT_LINES / 2 + 2; // +2 for borders

    // The panel's inner area is too small to fit even a minimum-width,
    // minimum-height popup - skip rather than render illegibly clipped
    // glyphs. Never shrink `popup_height`/`KEY_HISTORY_MIN_WIDTH` below what
    // `tui-big-text` needs to stay readable.
    if safe_area.width < KEY_HISTORY_MIN_WIDTH || safe_area.height < popup_height {
        return;
    }

    // Calculate required dimensions before consuming key_text
    // PixelSize::HalfHeight only halves the glyph height, not the width - each
    // character (including the space separators) is still BIG_TEXT_CHAR_WIDTH_CELLS wide
    let chars_count = key_text.chars().count();
    let popup_width =
        ((chars_count * BIG_TEXT_CHAR_WIDTH_CELLS).max(KEY_HISTORY_MIN_WIDTH as usize) as u16)
            .min(safe_area.width);

    // Create BigText widget with large font and cyan color
    let big_text = BigText::builder()
        .pixel_size(PixelSize::HalfHeight)
        .style(Style::default().fg(Color::Cyan))
        .lines(vec![key_text.into()])
        .centered()
        .build();

    // Anchor to the bottom-right corner of the panel's inner area - both
    // dimensions were already clipped to `safe_area`, so this rect is always
    // a subset of `safe_area` and never touches `bounds`'s border.
    let popup_x = safe_area.x + safe_area.width.saturating_sub(popup_width);
    let popup_y = safe_area.y + safe_area.height.saturating_sub(popup_height);

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

    fn render_popup(width: u16, height: u16, bounds: Rect, keys: &[String]) -> Buffer {
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                // Simulate the Target panel's own border, matching what
                // `render_editor_pair` renders into `bounds` in production.
                frame.render_widget(Block::default().borders(Borders::ALL), bounds);
                render_key_history_popup(frame, keys, bounds);
            })
            .unwrap();
        terminal.backend().buffer().clone()
    }

    fn render_bounds_only(width: u16, height: u16, bounds: Rect) -> Buffer {
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                frame.render_widget(Block::default().borders(Borders::ALL), bounds);
            })
            .unwrap();
        terminal.backend().buffer().clone()
    }

    /// Assert every cell on `bounds`'s own border rectangle is unchanged
    /// between the two buffers - i.e. the popup never drew over it.
    fn assert_border_unchanged(bounds: Rect, baseline: &Buffer, other: &Buffer) {
        for x in bounds.x..bounds.x + bounds.width {
            assert_eq!(
                baseline[(x, bounds.y)],
                other[(x, bounds.y)],
                "top border cell ({x}, {}) was overwritten",
                bounds.y
            );
            let bottom_y = bounds.y + bounds.height - 1;
            assert_eq!(
                baseline[(x, bottom_y)],
                other[(x, bottom_y)],
                "bottom border cell ({x}, {bottom_y}) was overwritten"
            );
        }
        for y in bounds.y..bounds.y + bounds.height {
            assert_eq!(
                baseline[(bounds.x, y)],
                other[(bounds.x, y)],
                "left border cell ({}, {y}) was overwritten",
                bounds.x
            );
            let right_x = bounds.x + bounds.width - 1;
            assert_eq!(
                baseline[(right_x, y)],
                other[(right_x, y)],
                "right border cell ({right_x}, {y}) was overwritten"
            );
        }
    }

    #[test]
    fn test_render_key_history_popup_empty_history_skips_render() {
        let bounds = Rect::new(10, 5, 40, 12);
        let buffer = render_popup(80, 20, bounds, &[]);
        let baseline = render_bounds_only(80, 20, bounds);
        assert_eq!(
            buffer, baseline,
            "empty key history must render nothing beyond the panel border"
        );
    }

    #[test]
    fn test_render_key_history_popup_skips_when_bounds_too_small() {
        // Inner area (after the panel's own border) is only 1x1 - too small
        // to fit a bordered popup without touching `bounds`'s own border.
        let bounds = Rect::new(10, 5, 3, 3);
        let buffer = render_popup(80, 20, bounds, &["a".to_string()]);
        let baseline = render_bounds_only(80, 20, bounds);
        assert_eq!(
            buffer, baseline,
            "popup must not render when bounds leave no room for it"
        );
    }

    /// Regression test: an inner area narrower than `KEY_HISTORY_MIN_WIDTH`
    /// (but still >= 3 cells) must be skipped entirely rather than rendered
    /// as an illegibly squished/clipped popup - the popup's minimum legible
    /// size must never be silently shrunk below what `tui-big-text` needs.
    #[test]
    fn test_render_key_history_popup_skips_rather_than_render_illegibly_narrow() {
        // Inner width is well above 3 cells but well below KEY_HISTORY_MIN_WIDTH.
        let bounds = Rect::new(10, 5, 20, 12);
        assert!(inner_rect(bounds).width < KEY_HISTORY_MIN_WIDTH);

        let buffer = render_popup(80, 20, bounds, &["a".to_string()]);
        let baseline = render_bounds_only(80, 20, bounds);
        assert_eq!(
            buffer, baseline,
            "popup must be skipped, not squished, when narrower than the minimum legible width"
        );
    }

    /// Regression test: an inner area shorter than the minimum popup height
    /// (but still >= 3 cells) must be skipped entirely rather than rendered
    /// with a vertically clipped glyph.
    #[test]
    fn test_render_key_history_popup_skips_rather_than_render_illegibly_short() {
        let min_height = BIG_TEXT_HEIGHT_LINES / 2 + 2;
        // Inner height is well above 3 cells but below the minimum popup height.
        let bounds = Rect::new(10, 5, 40, min_height);
        assert!(inner_rect(bounds).height < min_height);

        let buffer = render_popup(80, 20, bounds, &["a".to_string()]);
        let baseline = render_bounds_only(80, 20, bounds);
        assert_eq!(
            buffer, baseline,
            "popup must be skipped, not vertically clipped, when shorter than the minimum legible height"
        );
    }

    /// Regression test for #364: a growing, multi-key history (e.g. while
    /// typing a regex pattern, recording a macro, or using a named
    /// register) must never draw over the Target panel's border, no matter
    /// how wide the rendered key text gets.
    #[test]
    fn test_render_key_history_popup_never_overwrites_panel_border() {
        let bounds = Rect::new(10, 5, 40, 12);
        let wide_keys: Vec<String> = vec![
            "regex-pattern".to_string(),
            "Space".to_string(),
            "⌫".to_string(),
            "↵".to_string(),
            "Esc".to_string(),
        ];

        let buffer = render_popup(80, 20, bounds, &wide_keys);
        let baseline = render_bounds_only(80, 20, bounds);

        assert_border_unchanged(bounds, &baseline, &buffer);
        // Sanity: the popup did actually render something inside the panel.
        assert_ne!(
            buffer, baseline,
            "popup should have rendered inside the panel bounds"
        );
    }

    #[test]
    fn test_render_key_history_popup_stays_inside_bounds_inner_area() {
        let bounds = Rect::new(10, 5, 40, 12);
        let inner = inner_rect(bounds);
        let buffer = render_popup(80, 20, bounds, &["Esc".to_string(), "Space".to_string()]);

        // Every non-blank cell outside `bounds` entirely must remain blank,
        // and nothing should render between `bounds`'s edge and its inner area.
        for y in 0..20u16 {
            for x in 0..80u16 {
                let inside_inner = x >= inner.x
                    && x < inner.x + inner.width
                    && y >= inner.y
                    && y < inner.y + inner.height;
                let inside_bounds = x >= bounds.x
                    && x < bounds.x + bounds.width
                    && y >= bounds.y
                    && y < bounds.y + bounds.height;
                if inside_bounds && !inside_inner {
                    // Border ring of `bounds` - already covered by the
                    // border-unchanged test; skip here.
                    continue;
                }
                if !inside_inner {
                    let symbol = buffer[(x, y)].symbol();
                    assert!(
                        symbol == " " || symbol.is_empty(),
                        "unexpected content at ({x}, {y}) outside the panel's inner area: {symbol:?}"
                    );
                }
            }
        }
    }
}
