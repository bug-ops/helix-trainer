//! Review session screen rendering

use crate::ui::state::{AppState, ReviewSessionState};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, Paragraph, Wrap},
};

/// Render the review session screen
pub(super) fn render_review_screen(frame: &mut Frame, state: &AppState) {
    let Some(session) = &state.game.review_session else {
        return;
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header with progress
            Constraint::Min(0),    // Content
            Constraint::Length(3), // Footer with controls
        ])
        .split(frame.area());

    // Header with progress
    render_header(frame, chunks[0], session);

    // Main content area
    render_review_content(frame, chunks[1], state, session);

    // Footer with controls
    render_footer(frame, chunks[2]);
}

/// Render the header with progress gauge
fn render_header(frame: &mut Frame, area: Rect, session: &ReviewSessionState) {
    let progress = (session.current_index + 1) as f64 / session.due_commands.len() as f64;
    let progress_text = format!(
        "Review Session - {}/{}",
        session.current_index + 1,
        session.due_commands.len()
    );

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
fn render_review_content(
    frame: &mut Frame,
    area: Rect,
    state: &AppState,
    session: &ReviewSessionState,
) {
    let Some(command) = &session.current_command else {
        return;
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(6), // Command info
            Constraint::Min(10),   // Practice area
            Constraint::Length(3), // Instructions
        ])
        .margin(2)
        .split(area);

    // Command info panel
    render_command_info(frame, chunks[0], state, command);

    // Practice area
    render_practice_area(frame, chunks[1]);

    // Instructions
    render_instructions(frame, chunks[2]);
}

/// Render command information panel
fn render_command_info(frame: &mut Frame, area: Rect, state: &AppState, command: &str) {
    let tracker = state.progress.performance_tracker.borrow();
    let perf = tracker.get_performance(command);

    let mastery_text = if let Some(p) = perf {
        format!("Mastery: {:?} ⭐", p.mastery_level)
    } else {
        "Mastery: New".to_string()
    };

    let next_review = if let Some(p) = perf {
        let now = chrono::Utc::now();
        let days = (p.due - now).num_days();
        if days > 0 {
            format!("Next: in {} days", days)
        } else {
            "Due: Now".to_string()
        }
    } else {
        "First time".to_string()
    };

    let success_rate = if let Some(p) = perf {
        format!("Success rate: {:.0}%", p.success_rate() * 100.0)
    } else {
        "Success rate: N/A".to_string()
    };

    let text = vec![
        Line::from(vec![
            Span::styled("Command: ", Style::default().fg(Color::Gray)),
            Span::styled(
                command,
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
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
        .block(Block::default().borders(Borders::ALL).title("Command Info"))
        .wrap(Wrap { trim: true });

    frame.render_widget(paragraph, area);
}

/// Render practice area (MVP: simple instructions)
fn render_practice_area(frame: &mut Frame, area: Rect) {
    let text = vec![
        Line::from(""),
        Line::from("Practice the command in your mind or on paper."),
        Line::from(""),
        Line::from("Think about:"),
        Line::from("  • What does this command do?"),
        Line::from("  • When would you use it?"),
        Line::from("  • What are common use cases?"),
        Line::from(""),
        Line::from(vec![
            Span::raw("Press "),
            Span::styled(
                "'s'",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" if you remember it correctly"),
        ]),
        Line::from(vec![
            Span::raw("Press "),
            Span::styled(
                "'f'",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            ),
            Span::raw(" if you couldn't remember"),
        ]),
    ];

    let paragraph = Paragraph::new(text)
        .block(Block::default().borders(Borders::ALL).title("Practice"))
        .alignment(Alignment::Left);

    frame.render_widget(paragraph, area);
}

/// Render instruction panel
fn render_instructions(frame: &mut Frame, area: Rect) {
    let text = Line::from(vec![
        Span::raw("Controls: "),
        Span::styled(
            "s",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" = Success  |  "),
        Span::styled(
            "f",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        ),
        Span::raw(" = Failed  |  "),
        Span::styled("Esc", Style::default().fg(Color::Gray)),
        Span::raw(" = Abandon"),
    ]);

    let paragraph = Paragraph::new(text)
        .block(Block::default().borders(Borders::ALL))
        .alignment(Alignment::Center);

    frame.render_widget(paragraph, area);
}

/// Render footer with session info
fn render_footer(frame: &mut Frame, area: Rect) {
    let text = Line::from(vec![Span::styled(
        "Review Session Active - Practice each command until you remember it",
        Style::default().fg(Color::Cyan),
    )]);

    let paragraph = Paragraph::new(text)
        .block(Block::default().borders(Borders::ALL))
        .alignment(Alignment::Center);

    frame.render_widget(paragraph, area);
}
