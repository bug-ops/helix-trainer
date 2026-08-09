//! End-game summary screen rendering
//!
//! Shown once every scenario in the curriculum has been completed at least
//! once. Takes `&EndGameSummaryData` directly rather than `&AppState` - the
//! snapshot is self-contained, which makes this the only screen renderer
//! unit-testable without constructing a full `AppState`.

use crate::gamification::XPCalculator;
use crate::ui::state::{EndGameSummaryData, NextStep};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, Paragraph, Wrap},
};
use rust_i18n::t;

use super::category_filters::category_display_name;

/// Render the end-game (curriculum-completion) summary screen.
pub(super) fn render_end_game_summary(frame: &mut Frame, data: &EndGameSummaryData) {
    let area = frame.area();

    // margin(1), not the Profile screen's margin(2): the content here is
    // taller than Profile's, and every row matters on an 80x24 terminal -
    // see `render_content`'s doc comment for the rest of the row budget.
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3), // Title
            Constraint::Min(15),   // Content
            Constraint::Length(3), // Instructions
        ])
        .split(area);

    let title = Paragraph::new(t!("end_game.title").to_string())
        .style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(title, chunks[0]);

    render_content(frame, data, chunks[1]);

    let instructions = Paragraph::new(t!("end_game.instructions").to_string())
        .style(Style::default().fg(Color::Gray))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(instructions, chunks[2]);
}

/// Render the level/XP gauge and the stats paragraph beneath it.
///
/// Unlike the Profile screen (which wraps its content in a bordered block),
/// this area has no border and no inner margin: at 80x24 the stats section
/// (`render_stats`) needs every row it can get so the next-steps suggestions,
/// the screen's one actionable element, stay above the fold. The category
/// breakdown, last in `render_stats`'s line order, is what scrolls off first
/// on a short terminal; that's an accepted trade-off, not a bug.
fn render_content(frame: &mut Frame, data: &EndGameSummaryData, area: Rect) {
    let content_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2), // Level and XP gauge
            Constraint::Min(10),   // Stats
        ])
        .split(area);

    render_level_gauge(frame, data, content_chunks[0]);
    render_stats(frame, data, content_chunks[1]);
}

/// Render the level/XP progress gauge, matching the Profile screen's style.
fn render_level_gauge(frame: &mut Frame, data: &EndGameSummaryData, area: Rect) {
    let current_level_xp = XPCalculator::xp_for_level(data.level);
    let next_level_xp = XPCalculator::xp_for_level(data.level.saturating_add(1));
    let xp_in_level = data.total_xp.saturating_sub(current_level_xp);
    let xp_needed = next_level_xp.saturating_sub(current_level_xp);
    let progress = if xp_needed == 0 {
        1.0
    } else {
        xp_in_level as f64 / xp_needed as f64
    };

    let label = t!("end_game.level", level = data.level, xp = data.total_xp).to_string();
    let gauge = Gauge::default()
        .block(Block::default())
        .gauge_style(
            Style::default()
                .fg(Color::Yellow)
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .label(label)
        .ratio(progress.clamp(0.0, 1.0));
    frame.render_widget(gauge, area);
}

/// Journey-length copy, selected per R9: never render a bare "0 days".
fn journey_line(days: i64) -> String {
    match days {
        0 => t!("end_game.journey_same_day").to_string(),
        1 => t!("end_game.journey_one_day").to_string(),
        n => t!("end_game.journey_days", days = n).to_string(),
    }
}

/// Map a suggested next step to its localized display line.
fn next_step_line(step: &NextStep) -> String {
    match step {
        NextStep::DueReviews(n) => t!("end_game.next_due_reviews", count = n).to_string(),
        NextStep::PendingQuests(n) => t!("end_game.next_pending_quests", count = n).to_string(),
        NextStep::ImperfectScenarios(n) => t!("end_game.next_imperfect", count = n).to_string(),
        NextStep::ArcadeMode => t!("end_game.next_arcade").to_string(),
    }
}

/// Render the body: banner, journey/completions/mastery stats, review note,
/// next-step suggestions, and category breakdown.
///
/// Next-steps and the review note render *above* the category breakdown
/// (which grows one row per category and is the least essential section)
/// so the actionable content stays visible on short terminals even when the
/// breakdown itself scrolls out of the fixed content area.
fn render_stats(frame: &mut Frame, data: &EndGameSummaryData, area: Rect) {
    // Fully perfected (imperfect == 0) gets a stronger highlight than partial progress.
    let perfected_style = if data.imperfect == 0 {
        Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Yellow)
    };

    let mut lines = vec![
        Line::from(vec![Span::styled(
            t!("end_game.banner", total = data.scenarios_total).to_string(),
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(Span::styled(
            journey_line(data.journey_days),
            Style::default().fg(Color::Gray),
        )),
        Line::from(""),
        Line::from(vec![
            Span::raw("  "),
            Span::styled(
                t!("end_game.total_completions", count = data.total_completions).to_string(),
                Style::default().fg(Color::Cyan),
            ),
        ]),
        Line::from(vec![
            Span::raw("  "),
            Span::styled(
                t!(
                    "end_game.perfected",
                    perfected = data.perfected,
                    total = data.scenarios_total
                )
                .to_string(),
                perfected_style,
            ),
        ]),
        Line::from(vec![
            Span::raw("  "),
            Span::styled(
                t!(
                    "end_game.command_success_rate",
                    pct = (data.command_success_rate * 100.0).round() as u32
                )
                .to_string(),
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(vec![
            Span::raw("  "),
            Span::styled(
                t!("end_game.commands_mastered", count = data.commands_mastered).to_string(),
                Style::default().fg(Color::White),
            ),
        ]),
    ];

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        t!("end_game.review_note").to_string(),
        Style::default().fg(Color::Gray),
    )));

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        t!("end_game.next_steps_header").to_string(),
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
    )));
    for step in &data.next_steps {
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(next_step_line(step), Style::default().fg(Color::White)),
        ]));
    }

    if !data.category_breakdown.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            t!("end_game.category_header").to_string(),
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
        )));
        for (category, perfected, total) in &data.category_breakdown {
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(
                    t!(
                        "end_game.category_row",
                        category = category_display_name(category),
                        perfected = perfected,
                        total = total
                    )
                    .to_string(),
                    Style::default().fg(Color::White),
                ),
            ]));
        }
    }

    let stats = Paragraph::new(lines)
        .alignment(Alignment::Left)
        .style(Style::default().fg(Color::White))
        .wrap(Wrap { trim: true });
    frame.render_widget(stats, area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ScenarioCategory;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    fn sample_data() -> EndGameSummaryData {
        EndGameSummaryData {
            scenarios_total: 136,
            perfected: 100,
            imperfect: 36,
            total_completions: 250,
            total_xp: 5000,
            level: 10,
            command_success_rate: 0.87,
            journey_days: 42,
            commands_mastered: 30,
            category_breakdown: vec![(ScenarioCategory::Movement, 20, 25)],
            next_steps: vec![NextStep::DueReviews(3), NextStep::ArcadeMode],
        }
    }

    #[test]
    fn test_render_end_game_summary_does_not_panic() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let data = sample_data();

        terminal
            .draw(|f| render_end_game_summary(f, &data))
            .unwrap();

        let buffer = terminal.backend().buffer().clone();
        let text: String = buffer.content.iter().map(|cell| cell.symbol()).collect();
        assert!(text.contains("136"));
    }

    /// Regression test: at 80x24 the `Paragraph` used to have no `Wrap` and the
    /// category breakdown rendered before next-steps, pushing the "What's next"
    /// header and every next-step line off the bottom of the fixed content area.
    #[test]
    fn test_render_end_game_summary_next_steps_visible_at_80x24() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let data = sample_data();

        terminal
            .draw(|f| render_end_game_summary(f, &data))
            .unwrap();

        let buffer = terminal.backend().buffer().clone();
        let text: String = buffer.content.iter().map(|cell| cell.symbol()).collect();
        assert!(
            text.contains("What's next"),
            "next-steps header must be visible at 80x24, got: {text}"
        );
    }

    #[test]
    fn test_render_end_game_summary_empty_category_breakdown_does_not_panic() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut data = sample_data();
        data.category_breakdown = vec![];

        terminal
            .draw(|f| render_end_game_summary(f, &data))
            .unwrap();
    }

    #[test]
    fn test_render_end_game_summary_small_terminal_does_not_panic() {
        let backend = TestBackend::new(20, 5);
        let mut terminal = Terminal::new(backend).unwrap();
        let data = sample_data();

        terminal
            .draw(|f| render_end_game_summary(f, &data))
            .unwrap();
    }

    #[test]
    fn test_journey_line_same_day() {
        assert_eq!(journey_line(0), "Completed in a single day");
    }

    #[test]
    fn test_journey_line_one_day() {
        assert_eq!(journey_line(1), "A 1-day journey");
    }

    #[test]
    fn test_journey_line_n_days() {
        assert_eq!(journey_line(42), "A 42-day journey");
    }
}
