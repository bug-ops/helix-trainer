//! Popup rendering (hints, success, key history)

use super::helpers::{centered_popup, inner_rect, popup_block};
use crate::ui::state::AppState;
use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};
use tui_big_text::{BigText, PixelSize};

/// Render a centered hint popup
pub(super) fn render_hint_popup(frame: &mut Frame, state: &AppState) {
    let area = frame.area();

    // Calculate popup dimensions with constraints
    let popup_width = 70.min(area.width.saturating_sub(4));
    let popup_height = 10.min(area.height.saturating_sub(4));

    // Create centered popup area
    let popup_area = centered_popup(area, popup_width, popup_height);

    // Render popup background with border
    let background = popup_block(Some("Hint"), Color::White);
    frame.render_widget(&background, popup_area);

    // Render hint text inside popup
    if let Some(hint) = &state.current_hint {
        let inner = inner_rect(popup_area);

        let hint_text = Paragraph::new(hint.as_str())
            .wrap(Wrap { trim: true })
            .style(Style::default().fg(Color::Yellow))
            .alignment(Alignment::Center);

        frame.render_widget(hint_text, inner);
    }
}

/// Render key history popup showing last 5 keys pressed with large text
pub(super) fn render_key_history_popup(frame: &mut Frame, state: &AppState) {
    let area = frame.area();

    // Show last 5 keys
    let max_keys = 5;

    // Build text from recent keys
    let mut key_text = String::new();
    for (idx, key) in state.key_history.iter().take(max_keys).enumerate() {
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
    let area = frame.area();

    // Create centered popup area
    let popup_area = centered_popup(area, 40, 7);

    // Success message
    let success_text = vec![
        Line::from(""),
        Line::from(Span::styled(
            "SUCCESS!",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Scenario completed!",
            Style::default().fg(Color::White),
        )),
        Line::from(""),
    ];

    let success_paragraph = Paragraph::new(success_text)
        .alignment(Alignment::Center)
        .block(popup_block(None, Color::Green));

    frame.render_widget(success_paragraph, popup_area);
}
