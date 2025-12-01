//! Popup rendering (hints, success, key history, notifications)

use super::helpers::{centered_popup, inner_rect, popup_block};
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
    let popup_width = 70.min(area.width.saturating_sub(4));
    let popup_height = 10.min(area.height.saturating_sub(4));

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
/// Used by both training mode and arcade mode.
pub(super) fn render_key_history_popup(frame: &mut Frame, key_history: &[String]) {
    if key_history.is_empty() {
        return;
    }

    let area = frame.area();

    // Show last 5 keys
    let max_keys = 5;

    // Build text from recent keys
    let mut key_text = String::new();
    for (idx, key) in key_history.iter().take(max_keys).enumerate() {
        if idx > 0 {
            key_text.push(' ');
        }
        key_text.push_str(key);
    }

    // Calculate required dimensions before consuming key_text
    // Each character in Full size is approximately 4 cells wide, plus spacing
    let chars_count = key_text.chars().count();
    let popup_width = ((chars_count * 5).max(30) as u16).min(area.width.saturating_sub(4));
    let text_height = 8;
    let popup_height = text_height + 2; // +2 for borders

    // Create BigText widget with large font and cyan color
    let big_text = BigText::builder()
        .pixel_size(PixelSize::Full)
        .style(Style::default().fg(Color::Cyan))
        .lines(vec![key_text.into()])
        .centered()
        .build();

    // Position in bottom right corner
    let popup_x = area.width.saturating_sub(popup_width + 2);
    let popup_y = area.height.saturating_sub(popup_height + 2);

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
