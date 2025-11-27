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
        // Layout: title | horizontal(results | xp & quests) | instructions
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(2)
            .constraints([
                Constraint::Length(3), // Title
                Constraint::Min(10),   // Results + XP
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

        // Split middle area into two columns: performance | progression
        let middle_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(chunks[1]);

        // Left panel: Performance results
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
        frame.render_widget(results, middle_chunks[0]);

        // Right panel: XP breakdown and quest progress
        render_progression_panel(frame, state, middle_chunks[1]);

        // Instructions
        let instructions = Paragraph::new(t!("results.instructions").to_string())
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        frame.render_widget(instructions, chunks[2]);
    }
}

/// Render XP breakdown and quest progress panel
fn render_progression_panel(frame: &mut Frame, state: &AppState, area: ratatui::layout::Rect) {
    let mut lines = vec![];

    // XP Breakdown section
    if let Some(xp) = &state.xp_breakdown {
        lines.push(Line::from(vec![Span::styled(
            "XP Earned",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
        )]));
        lines.push(Line::from(""));

        // Base XP
        lines.push(Line::from(vec![
            Span::raw("  Base: "),
            Span::styled(
                format!("+{}", xp.base_xp),
                Style::default().fg(Color::Green),
            ),
        ]));

        // Perfect bonus
        if xp.perfect_bonus > 0 {
            lines.push(Line::from(vec![
                Span::raw("  Perfect: "),
                Span::styled(
                    format!("+{}", xp.perfect_bonus),
                    Style::default().fg(Color::Cyan),
                ),
            ]));
        }

        // First today bonus
        if xp.first_today_bonus > 0 {
            lines.push(Line::from(vec![
                Span::raw("  First today: "),
                Span::styled(
                    format!("+{}", xp.first_today_bonus),
                    Style::default().fg(Color::Cyan),
                ),
            ]));
        }

        // Quest bonuses
        for (desc, bonus_xp) in &xp.quest_bonuses {
            lines.push(Line::from(vec![
                Span::raw("  Quest ("),
                Span::styled(desc.clone(), Style::default().fg(Color::Magenta)),
                Span::raw("): "),
                Span::styled(format!("+{}", bonus_xp), Style::default().fg(Color::Cyan)),
            ]));
        }

        // Total
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled(
                "  Total XP: ",
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("+{}", xp.total_xp),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));
    }

    // Quest progress changes
    if !state.quest_progress_changes.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(""));
        lines.push(Line::from(vec![Span::styled(
            "Quest Progress",
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
        )]));
        lines.push(Line::from(""));

        for change in &state.quest_progress_changes {
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(
                    change.quest_description.clone(),
                    Style::default().fg(Color::White),
                ),
            ]));
            lines.push(Line::from(vec![
                Span::raw("    "),
                Span::styled(
                    format!("{}", change.old_progress),
                    Style::default().fg(Color::Gray),
                ),
                Span::raw(" → "),
                Span::styled(
                    format!("{}", change.new_progress),
                    Style::default().fg(Color::Green),
                ),
            ]));
        }
    }

    // Current level and progress
    lines.push(Line::from(""));
    lines.push(Line::from(""));
    let (level, total_xp, xp_for_next) = {
        let profile = state.profile.borrow();
        let current_level_xp = crate::gamification::XPCalculator::xp_for_level(profile.level);
        let xp_in_level = profile.total_xp - current_level_xp;
        (profile.level, xp_in_level, profile.xp_for_next_level())
    };
    lines.push(Line::from(vec![
        Span::styled(
            format!("Level {}", level),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(format!("  ({}/{})", total_xp, xp_for_next)),
    ]));

    let panel = Paragraph::new(lines)
        .block(Block::default().title("Progression").borders(Borders::ALL))
        .alignment(Alignment::Left)
        .style(Style::default().fg(Color::White));
    frame.render_widget(panel, area);
}
