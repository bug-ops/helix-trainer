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

    // Get feedback from ResultsData (guaranteed to exist via TypedScreen)
    if let crate::ui::state::TypedScreen::Results(results_data) = &state.screen {
        let feedback = &results_data.feedback;
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
        render_progression_panel(frame, state, results_data, middle_chunks[1]);

        // Instructions
        let instructions = Paragraph::new(t!("results.instructions").to_string())
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        frame.render_widget(instructions, chunks[2]);
    }
}

/// Render XP breakdown and quest progress panel
fn render_progression_panel(
    frame: &mut Frame,
    state: &AppState,
    results_data: &crate::ui::state::ResultsData,
    area: ratatui::layout::Rect,
) {
    let mut lines = vec![];

    // XP Breakdown section
    if let Some(xp) = &results_data.xp_breakdown {
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

        // Mastery scaling
        if xp.mastery_multiplier < 1.0 {
            let mastery_text = if let Some((mastery, _)) = &results_data.scenario_mastery {
                format!("  {} {}: ", mastery.emoji(), mastery.display_name())
            } else {
                "  Mastery: ".to_string()
            };

            let reduction_pct = ((1.0 - xp.mastery_multiplier) * 100.0) as u32;
            lines.push(Line::from(vec![
                Span::raw(mastery_text),
                Span::styled(
                    format!("-{}%", reduction_pct),
                    Style::default().fg(Color::Red),
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
    if !results_data.quest_changes.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(""));
        lines.push(Line::from(vec![Span::styled(
            "Quest Progress",
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
        )]));
        lines.push(Line::from(""));

        for change in &results_data.quest_changes {
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

    // Current level and progress section
    lines.push(Line::from(""));
    lines.push(Line::from(""));
    lines.push(Line::from(vec![Span::styled(
        "Your Stats",
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
    )]));
    lines.push(Line::from(""));

    let (level, xp_in_level, xp_needed_for_level, total_xp, scenarios, perfect, streak) = {
        let profile = state.progress.profile.borrow();
        let current_level_xp = crate::gamification::XPCalculator::xp_for_level(profile.level);
        let next_level_xp = crate::gamification::XPCalculator::xp_for_level(profile.level + 1);
        let xp_in_level = profile.total_xp.saturating_sub(current_level_xp);
        let xp_needed = next_level_xp.saturating_sub(current_level_xp);
        (
            profile.level,
            xp_in_level,
            xp_needed,
            profile.total_xp,
            profile.scenarios_completed,
            profile.perfect_scenarios,
            profile.current_streak,
        )
    };

    // Level with XP progress
    lines.push(Line::from(vec![
        Span::raw("  Level: "),
        Span::styled(
            format!("{}", level),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("  ({}/{} XP)", xp_in_level, xp_needed_for_level),
            Style::default().fg(Color::Gray),
        ),
    ]));

    // Total XP with earned XP from this scenario
    let earned_xp = results_data.xp_breakdown.as_ref().map(|xp| xp.total_xp);
    lines.push(Line::from(vec![
        Span::raw("  Total XP: "),
        Span::styled(format!("{}", total_xp), Style::default().fg(Color::Yellow)),
        if let Some(earned) = earned_xp {
            Span::styled(
                format!("  (+{})", earned),
                Style::default().fg(Color::Green),
            )
        } else {
            Span::raw("")
        },
    ]));

    // Scenarios completed
    lines.push(Line::from(vec![
        Span::raw("  Scenarios: "),
        Span::styled(format!("{}", scenarios), Style::default().fg(Color::Green)),
        Span::styled(
            format!(" ({} perfect)", perfect),
            Style::default().fg(Color::Gray),
        ),
    ]));

    // Current streak
    if streak > 0 {
        lines.push(Line::from(vec![
            Span::raw("  Streak: "),
            Span::styled(
                format!("{} days", streak),
                Style::default().fg(Color::Magenta),
            ),
            Span::raw(" 🔥"),
        ]));
    }

    let panel = Paragraph::new(lines)
        .block(Block::default().title("Progression").borders(Borders::ALL))
        .alignment(Alignment::Left)
        .style(Style::default().fg(Color::White));
    frame.render_widget(panel, area);
}
