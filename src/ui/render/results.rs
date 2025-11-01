//! Results screen rendering

use crate::{game::PerformanceRating, ui::state::AppState};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};
use rust_i18n::t;

/// Get localized description for performance rating
fn rating_description(rating: &PerformanceRating) -> String {
    match rating {
        PerformanceRating::Perfect => t!("results.rating_perfect").to_string(),
        PerformanceRating::Excellent => t!("results.rating_excellent").to_string(),
        PerformanceRating::Good => t!("results.rating_good").to_string(),
        PerformanceRating::Fair => t!("results.rating_fair").to_string(),
        PerformanceRating::Poor => t!("results.rating_poor").to_string(),
    }
}

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
            t!("results.completed").to_string()
        } else {
            t!("results.abandoned").to_string()
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
                rating_description(&feedback.rating)
            ),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )]));

        result_lines.push(Line::from(""));

        // Score
        result_lines.push(Line::from(vec![
            Span::raw(format!("{}: ", t!("results.score"))),
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
            Span::raw(format!("{}: ", t!("results.your_actions"))),
            Span::styled(
                format!("{}", feedback.actions_taken),
                Style::default().fg(action_color),
            ),
            Span::raw(format!(
                " ({}: {})",
                t!("results.optimal_actions"),
                feedback.optimal_actions
            )),
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
            .block(
                Block::default()
                    .title(t!("results.performance").to_string())
                    .borders(Borders::ALL),
            )
            .alignment(Alignment::Left)
            .style(Style::default().fg(Color::White));
        frame.render_widget(results, chunks[1]);

        // Instructions
        let instructions = Paragraph::new(t!("results.instructions").to_string())
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        frame.render_widget(instructions, chunks[2]);
    }
}
