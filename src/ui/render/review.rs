//! Review session screen rendering

use crate::grammar::commands::CMD_ESCAPE;
use crate::helix::registry::normal_registry;
use crate::ids::CommandId;
use crate::learning::ReviewSession;
use crate::ui::state::{AppState, ReviewData};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, Paragraph, Wrap},
};
use rust_i18n::t;
use std::borrow::Cow;

/// Get a human-readable description for a Helix command from registry
fn get_command_description(command: &str) -> Cow<'static, str> {
    normal_registry().metadata(command).map_or_else(
        || t!("review.default_description"),
        |m| Cow::Borrowed(m.summary()),
    )
}

/// Render the review session screen
pub(super) fn render_review_screen(frame: &mut Frame, state: &AppState, data: &ReviewData) {
    let session = &data.session;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header with progress
            Constraint::Min(0),    // Content
            Constraint::Length(3), // Footer with controls
        ])
        .horizontal_margin(2)
        .split(frame.area());

    // Header with progress
    render_header(frame, chunks[0], session);

    // Main content area
    render_review_content(frame, chunks[1], state, session);

    // Footer with controls
    render_footer(frame, chunks[2]);
}

/// Render the header with progress gauge
#[expect(
    clippy::cast_precision_loss,
    reason = "integer counters and scores stay far below 2^52, so the f64 value is exact"
)]
fn render_header(frame: &mut Frame, area: Rect, session: &ReviewSession) {
    let progress = session.position() as f64 / session.total() as f64;
    let progress_text = t!(
        "review.header",
        current = session.position(),
        total = session.total()
    )
    .to_string();

    let gauge = Gauge::default()
        .block(Block::default().borders(Borders::ALL))
        .gauge_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .label(progress_text)
        .ratio(progress);

    frame.render_widget(gauge, area);
}

/// Render the main review content area
fn render_review_content(frame: &mut Frame, area: Rect, state: &AppState, session: &ReviewSession) {
    let command = session.current();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(6), // Command info
            Constraint::Min(10),   // Practice area
            Constraint::Length(3), // Instructions
        ])
        .vertical_margin(2)
        .split(area);

    // Command info panel
    render_command_info(frame, chunks[0], state, command);

    // Practice area
    render_practice_area(frame, chunks[1]);

    // Instructions
    render_instructions(frame, chunks[2]);
}

/// Render command information panel
fn render_command_info(frame: &mut Frame, area: Rect, state: &AppState, command: &CommandId) {
    let tracker = &state.progress.performance_tracker;
    let perf = tracker.performance(command);

    // Get command description
    let description = get_command_description(command.as_str());

    let mastery_text = perf.map_or_else(
        || t!("review.mastery_new").to_string(),
        |p| {
            t!(
                "review.mastery",
                level = format!("{:?} ⭐", p.mastery_level())
            )
            .to_string()
        },
    );

    let next_review = perf.map_or_else(
        || t!("review.first_time").to_string(),
        |p| {
            // `state: &AppState` is in scope here, so use the injected clock rather than
            // the wall clock directly.
            let now = state.progress.now();
            let days = (p.due() - now).num_days();
            if days > 0 {
                t!("review.next_in_days", days = days).to_string()
            } else {
                t!("review.due_now").to_string()
            }
        },
    );

    let success_rate = perf.map_or_else(
        || t!("review.success_rate_na").to_string(),
        |p| {
            t!(
                "review.success_rate",
                percent = format!("{:.0}", p.success_rate() * 100.0)
            )
            .to_string()
        },
    );

    let text = vec![
        Line::from(vec![
            Span::styled(t!("review.command_label"), Style::default().fg(Color::Gray)),
            Span::styled(
                command.as_str(),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![Span::styled(
            description,
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::ITALIC),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled(&mastery_text, Style::default().fg(Color::Yellow)),
            Span::raw("  "),
            Span::styled(&next_review, Style::default().fg(Color::Gray)),
        ]),
        Line::from(Span::styled(
            &success_rate,
            Style::default().fg(Color::Gray),
        )),
    ];

    let paragraph = Paragraph::new(text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(t!("review.command_info_title")),
        )
        .wrap(Wrap { trim: true });

    frame.render_widget(paragraph, area);
}

/// Render practice area (MVP: simple instructions)
fn render_practice_area(frame: &mut Frame, area: Rect) {
    let text = vec![
        Line::from(""),
        Line::from(t!("review.practice_intro")),
        Line::from(""),
        Line::from(t!("review.practice_think")),
        Line::from(t!("review.practice_what")),
        Line::from(t!("review.practice_when")),
        Line::from(t!("review.practice_cases")),
        Line::from(""),
        Line::from(vec![
            Span::raw(t!("review.press")),
            Span::styled(
                "'s'",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(t!("review.remembered_suffix")),
        ]),
        Line::from(vec![
            Span::raw(t!("review.press")),
            Span::styled(
                "'f'",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            ),
            Span::raw(t!("review.forgot_suffix")),
        ]),
    ];

    let paragraph = Paragraph::new(text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(t!("review.practice_title")),
        )
        .alignment(Alignment::Left);

    frame.render_widget(paragraph, area);
}

/// Render instruction panel
fn render_instructions(frame: &mut Frame, area: Rect) {
    let text = Line::from(vec![
        Span::raw(t!("review.controls")),
        Span::styled(
            "s",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(t!("review.success_label")),
        Span::styled(
            "f",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        ),
        Span::raw(t!("review.failed_label")),
        Span::styled(CMD_ESCAPE, Style::default().fg(Color::Gray)),
        Span::raw(t!("review.abandon_label")),
    ]);

    let paragraph = Paragraph::new(text)
        .block(Block::default().borders(Borders::ALL))
        .alignment(Alignment::Center);

    frame.render_widget(paragraph, area);
}

/// Render footer with session info
fn render_footer(frame: &mut Frame, area: Rect) {
    let text = Line::from(vec![Span::styled(
        t!("review.footer"),
        Style::default().fg(Color::Cyan),
    )]);

    let paragraph = Paragraph::new(text)
        .block(Block::default().borders(Borders::ALL))
        .alignment(Alignment::Center);

    frame.render_widget(paragraph, area);
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::learning::ReviewSession;
    use crate::testing::{cid, empty_test_app_state};
    use ratatui::{Terminal, backend::TestBackend};
    use std::time::Instant;

    fn render(width: u16, height: u16) -> Terminal<TestBackend> {
        let state = empty_test_app_state();
        let session = ReviewSession::start(vec![cid("h"), cid("j")], Instant::now())
            .expect("fixture has commands");
        let data = ReviewData { session };
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal
            .draw(|frame| render_review_screen(frame, &state, &data))
            .unwrap();
        terminal
    }

    fn first_non_blank_columns(terminal: &Terminal<TestBackend>) -> Vec<usize> {
        let buffer = terminal.backend().buffer();
        let width = usize::from(buffer.area.width);
        buffer
            .content()
            .chunks(width)
            .filter_map(|row| row.iter().position(|cell| cell.symbol() != " "))
            .collect()
    }

    /// #586: the header, the body panels and the bottom bar share one margin
    #[test]
    fn test_every_box_starts_at_the_screen_margin() {
        let columns = first_non_blank_columns(&render(130, 45));
        assert!(!columns.is_empty());
        assert!(columns.iter().all(|&column| column == 2), "{columns:?}");
    }

    #[test]
    fn test_narrow_terminal_does_not_panic() {
        for (width, height) in [(1, 1), (3, 3), (4, 10), (20, 5)] {
            render(width, height);
        }
    }

    #[test]
    fn test_get_command_description_uses_registry() {
        // Verify that known commands return descriptions from registry
        let fallback = t!("review.default_description");
        let desc = get_command_description("h");
        assert_ne!(desc, fallback);
        assert!(!desc.is_empty());

        let desc = get_command_description("w");
        assert_ne!(desc, fallback);

        let desc = get_command_description("d");
        assert_ne!(desc, fallback);
    }

    #[test]
    fn test_get_command_description_unknown_command_fallback() {
        assert_eq!(get_command_description("unknown_cmd"), "Helix command");
        assert_eq!(get_command_description(""), "Helix command");
        assert_eq!(get_command_description("xyz123"), "Helix command");
    }
}
