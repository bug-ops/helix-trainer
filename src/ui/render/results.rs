//! Results screen rendering

use crate::ui::state::AppState;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

/// Render the results screen showing scenario completion
pub(super) fn render_results_screen(frame: &mut Frame, state: &AppState) {
    let area = frame.area();

    if let Some(session) = &state.session
        && let Ok(feedback) = session.get_feedback()
    {
        // Layout: title | results | instructions
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(2)
            .constraints([
                Constraint::Length(3), // Title
                Constraint::Min(10),   // Results
                Constraint::Length(3), // Instructions
            ])
            .split(area);

        // Title
        let title_text = if feedback.success {
            "✓ Completed!"
        } else {
            "✗ Not Completed"
        };
        let title_color = if feedback.success {
            Color::Green
        } else {
            Color::Red
        };
        let title = Paragraph::new(title_text)
            .style(
                Style::default()
                    .fg(title_color)
                    .add_modifier(Modifier::BOLD),
            )
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        frame.render_widget(title, chunks[0]);

        // Results content
        let mut result_lines = vec![];

        // Rating and score
        result_lines.push(Line::from(vec![Span::styled(
            format!(
                "{} {}",
                feedback.rating.emoji(),
                feedback.rating.description()
            ),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )]));

        result_lines.push(Line::from(""));

        // Score
        result_lines.push(Line::from(vec![
            Span::raw("Score: "),
            Span::styled(
                format!("{}/{}", feedback.score, feedback.max_points),
                Style::default().fg(Color::Cyan),
            ),
        ]));

        // Actions
        let action_color = if feedback.is_optimal {
            Color::Green
        } else {
            Color::Yellow
        };
        result_lines.push(Line::from(vec![
            Span::raw("Actions: "),
            Span::styled(
                format!("{}", feedback.actions_taken),
                Style::default().fg(action_color),
            ),
            Span::raw(format!(" (optimal: {})", feedback.optimal_actions)),
        ]));

        // Duration
        result_lines.push(Line::from(vec![
            Span::raw("Time: "),
            Span::styled(
                format!("{:.1}s", feedback.duration.as_secs_f32()),
                Style::default().fg(Color::Blue),
            ),
        ]));

        // Hint if provided
        if let Some(hint) = &feedback.hint {
            result_lines.push(Line::from(""));
            result_lines.push(Line::from(vec![
                Span::styled("Tip: ", Style::default().fg(Color::Magenta)),
                Span::raw(hint),
            ]));
        }

        let results = Paragraph::new(result_lines)
            .block(Block::default().title("Performance").borders(Borders::ALL))
            .alignment(Alignment::Left)
            .style(Style::default().fg(Color::White));
        frame.render_widget(results, chunks[1]);

        // Instructions
        let instructions = Paragraph::new("[r] Retry  [m] Menu  [q] Quit")
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        frame.render_widget(instructions, chunks[2]);
    }
}
