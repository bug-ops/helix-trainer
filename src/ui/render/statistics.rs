//! Statistics screen rendering

use crate::constants::PROGRESS_BAR_WIDTH;
use crate::learning::{Analytics, MasteryLevel, MasterySummary, ScenarioMastery};
use crate::ui::state::AppState;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};
use rust_i18n::t;

/// Render the statistics screen showing detailed performance breakdown
pub(super) fn render_statistics_screen(frame: &mut Frame, state: &AppState) {
    let area = frame.area();

    // Main layout: title | content | instructions
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Length(3), // Title
            Constraint::Min(15),   // Content
            Constraint::Length(3), // Instructions
        ])
        .split(area);

    // Title
    let title = Paragraph::new(t!("statistics.title").to_string())
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(title, chunks[0]);

    // Content area
    render_statistics_content(frame, state, chunks[1]);

    // Instructions
    let instructions = Paragraph::new(t!("statistics.instructions").to_string())
        .style(Style::default().fg(Color::Gray))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(instructions, chunks[2]);
}

/// Render the main statistics content
#[expect(
    clippy::similar_names,
    reason = "parallel names mirror the values they bind"
)]
fn render_statistics_content(frame: &mut Frame, state: &AppState, area: ratatui::layout::Rect) {
    let content_block = Block::default().borders(Borders::ALL);
    let inner_area = content_block.inner(area);
    frame.render_widget(content_block, area);

    let mut lines = vec![];

    // Command Mastery Distribution section (from FSRS)
    render_command_mastery_section(&mut lines, state);

    lines.push(Line::from(""));

    // Review Status section (from FSRS scheduler)
    render_review_status_section(&mut lines, state);

    lines.push(Line::from(""));

    // Weak Commands section
    render_weak_commands_section(&mut lines, state);

    lines.push(Line::from(""));

    // Scenario Mastery section
    render_scenario_mastery_section(&mut lines, state);

    lines.push(Line::from(""));

    // Quest Statistics section
    render_quest_statistics(&mut lines, state);

    lines.push(Line::from(""));

    // Session History section
    render_session_history(&mut lines, state);

    lines.push(Line::from(""));

    // Arcade Mode Statistics section
    render_arcade_statistics(&mut lines, state);

    let stats = Paragraph::new(lines)
        .alignment(Alignment::Left)
        .style(Style::default().fg(Color::White));
    frame.render_widget(stats, inner_area);
}

/// Render command mastery distribution section using real FSRS data
fn render_command_mastery_section(lines: &mut Vec<Line<'static>>, state: &AppState) {
    lines.push(Line::from(vec![Span::styled(
        t!("statistics.mastery_header").to_string(),
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
    )]));
    lines.push(Line::from(""));

    let tracker = &state.progress.performance_tracker;
    let summary: MasterySummary = Analytics::get_mastery_summary(tracker);

    if summary.total_commands == 0 {
        lines.push(Line::from(vec![Span::styled(
            t!("statistics.no_commands").to_string(),
            Style::default().fg(Color::Gray),
        )]));
        return;
    }

    let total = summary.total_commands;

    for (level, color) in [
        (MasteryLevel::Master, Color::Yellow),
        (MasteryLevel::Advanced, Color::Cyan),
        (MasteryLevel::Intermediate, Color::Green),
        (MasteryLevel::Beginner, Color::Gray),
    ] {
        render_mastery_bar(
            lines,
            &mastery_level_label(level),
            mastery_level_count(&summary, level),
            total,
            color,
        );
    }

    // Additional metrics
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::raw(t!("statistics.avg_stability").to_string()),
        Span::styled(
            t!(
                "statistics.days_value",
                value = format!("{:.1}", summary.avg_stability)
            )
            .to_string(),
            Style::default().fg(Color::Cyan),
        ),
        Span::raw(t!("statistics.avg_difficulty").to_string()),
        Span::styled(
            format!("{:.1}/10", summary.avg_difficulty),
            Style::default().fg(Color::Magenta),
        ),
    ]));
}

/// Render a horizontal bar for mastery level distribution
fn render_mastery_bar(
    lines: &mut Vec<Line<'static>>,
    label: &str,
    count: usize,
    total: usize,
    color: Color,
) {
    let percentage = count
        .saturating_mul(100)
        .checked_div(total)
        .map_or(0, |pct| u32::try_from(pct).unwrap_or(u32::MAX));

    // Create horizontal progress bar
    let filled = (percentage as usize * PROGRESS_BAR_WIDTH / 100).min(PROGRESS_BAR_WIDTH);
    let empty = PROGRESS_BAR_WIDTH - filled;
    let bar = format!("{}{}", "|".repeat(filled), ".".repeat(empty));

    lines.push(Line::from(vec![
        Span::raw(format!("  {label:12}: ")),
        Span::styled(format!("{count:3} "), Style::default().fg(Color::White)),
        Span::styled(bar, Style::default().fg(color)),
        Span::raw(format!("  {percentage:3}%")),
    ]));
}

/// Render review status section using real FSRS scheduler data
fn render_review_status_section(lines: &mut Vec<Line<'static>>, state: &AppState) {
    lines.push(Line::from(vec![Span::styled(
        t!("statistics.review_header").to_string(),
        Style::default()
            .fg(Color::Blue)
            .add_modifier(Modifier::BOLD),
    )]));
    lines.push(Line::from(""));

    let tracker = &state.progress.performance_tracker;
    let scheduler = &state.progress.scheduler;

    // Get due reviews
    let due_commands = scheduler.get_due_reviews(tracker);
    let due_count = due_commands.len();

    // Get total tracked commands
    let total_commands = Analytics::total_commands(tracker);

    // Get average success rate
    let avg_success_rate = Analytics::avg_success_rate(tracker);

    // Due for review
    let due_color = if due_count > 5 {
        Color::Red
    } else if due_count > 0 {
        Color::Yellow
    } else {
        Color::Green
    };

    lines.push(Line::from(vec![
        Span::raw(t!("statistics.due_for_review").to_string()),
        Span::styled(format!("{due_count}"), Style::default().fg(due_color)),
        Span::styled(
            if due_count > 5 {
                t!("statistics.due_many").to_string()
            } else if due_count > 0 {
                t!("statistics.due_some").to_string()
            } else {
                t!("statistics.due_none").to_string()
            },
            Style::default().fg(Color::Gray),
        ),
    ]));

    // Total commands tracked
    lines.push(Line::from(vec![
        Span::raw(t!("statistics.commands_tracked").to_string()),
        Span::styled(
            format!("{total_commands}"),
            Style::default().fg(Color::Cyan),
        ),
    ]));

    // Average success rate
    let success_color = if avg_success_rate >= 0.8 {
        Color::Green
    } else if avg_success_rate >= 0.6 {
        Color::Yellow
    } else {
        Color::Red
    };

    lines.push(Line::from(vec![
        Span::raw(t!("statistics.avg_success_rate").to_string()),
        Span::styled(
            format!("{:.0}%", avg_success_rate * 100.0),
            Style::default().fg(success_color),
        ),
    ]));
}

/// Render weak commands section showing commands that need practice
fn render_weak_commands_section(lines: &mut Vec<Line<'static>>, state: &AppState) {
    lines.push(Line::from(vec![Span::styled(
        t!("statistics.weak_header").to_string(),
        Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
    )]));
    lines.push(Line::from(""));

    let tracker = &state.progress.performance_tracker;
    let weak_commands = tracker.weak_commands();

    if weak_commands.is_empty() {
        lines.push(Line::from(vec![Span::styled(
            t!("statistics.no_weak").to_string(),
            Style::default().fg(Color::Green),
        )]));
        return;
    }

    // Show top 5 weak commands with their stats
    for (i, cmd) in weak_commands.iter().take(5).enumerate() {
        if let Some(perf) = tracker.performance(cmd) {
            let success_rate = perf.success_rate();
            let rate_color = if success_rate < 0.5 {
                Color::Red
            } else if success_rate < 0.7 {
                Color::Yellow
            } else {
                Color::Gray
            };

            lines.push(Line::from(vec![
                Span::raw(format!("  {}. ", i + 1)),
                Span::styled(format!("{cmd:10}"), Style::default().fg(Color::White)),
                Span::raw(" - "),
                Span::styled(
                    t!(
                        "statistics.weak_success",
                        pct = format!("{:.0}", success_rate * 100.0)
                    )
                    .to_string(),
                    Style::default().fg(rate_color),
                ),
                Span::raw(", "),
                Span::styled(
                    t!("statistics.weak_lapses", count = perf.lapses()).to_string(),
                    Style::default().fg(if perf.lapses() > 2 {
                        Color::Red
                    } else {
                        Color::Gray
                    }),
                ),
            ]));
        }
    }

    if weak_commands.len() > 5 {
        lines.push(Line::from(vec![Span::styled(
            t!("statistics.weak_more", count = weak_commands.len() - 5).to_string(),
            Style::default().fg(Color::Gray),
        )]));
    }
}

/// Render scenario mastery section using profile's scenario history
#[expect(
    clippy::similar_names,
    reason = "parallel names mirror the values they bind"
)]
fn render_scenario_mastery_section(lines: &mut Vec<Line<'static>>, state: &AppState) {
    lines.push(Line::from(vec![Span::styled(
        t!("statistics.scenario_header").to_string(),
        Style::default()
            .fg(Color::Magenta)
            .add_modifier(Modifier::BOLD),
    )]));
    lines.push(Line::from(""));

    let profile = &state.progress.profile;
    let stats = profile.scenario_history.mastery_stats();

    let total = stats.learning + stats.proficient + stats.mastered;
    let [mastered, proficient, learning, total_count] =
        [stats.mastered, stats.proficient, stats.learning, total]
            .map(|n| usize::try_from(n).unwrap_or(usize::MAX));

    if total == 0 {
        lines.push(Line::from(vec![Span::styled(
            t!("statistics.no_scenarios").to_string(),
            Style::default().fg(Color::Gray),
        )]));
        return;
    }

    for (level, count, color) in [
        (ScenarioMastery::Mastered, mastered, Color::Yellow),
        (ScenarioMastery::Proficient, proficient, Color::Cyan),
        (ScenarioMastery::Learning, learning, Color::Gray),
    ] {
        render_mastery_bar(
            lines,
            &scenario_mastery_label(level),
            count,
            total_count,
            color,
        );
    }

    // Total scenarios attempted
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::raw(t!("statistics.scenarios_attempted").to_string()),
        Span::styled(format!("{total}"), Style::default().fg(Color::White)),
    ]));
}

/// Render quest statistics section
fn render_quest_statistics(lines: &mut Vec<Line<'static>>, state: &AppState) {
    lines.push(Line::from(vec![Span::styled(
        t!("statistics.quest_header").to_string(),
        Style::default()
            .fg(Color::Magenta)
            .add_modifier(Modifier::BOLD),
    )]));
    lines.push(Line::from(""));

    // Get quest data
    let profile = &state.progress.profile;
    let completed_today = profile.completed_quests_today().count();

    lines.push(Line::from(vec![
        Span::raw(t!("statistics.quests_completed").to_string()),
        Span::styled(
            format!("{completed_today}"),
            Style::default().fg(Color::Green),
        ),
    ]));

    // Streak information
    let current_streak = profile.current_streak;
    let longest_streak = profile.longest_streak;

    lines.push(Line::from(vec![
        Span::raw(t!("statistics.current_streak").to_string()),
        Span::styled(
            t!("statistics.days_value", value = current_streak).to_string(),
            Style::default().fg(Color::Yellow),
        ),
    ]));

    lines.push(Line::from(vec![
        Span::raw(t!("statistics.longest_streak").to_string()),
        Span::styled(
            t!("statistics.days_value", value = longest_streak).to_string(),
            Style::default().fg(Color::Cyan),
        ),
    ]));
}

/// Render session history section
fn render_session_history(lines: &mut Vec<Line<'static>>, state: &AppState) {
    lines.push(Line::from(vec![Span::styled(
        t!("statistics.session_header").to_string(),
        Style::default()
            .fg(Color::Blue)
            .add_modifier(Modifier::BOLD),
    )]));
    lines.push(Line::from(""));

    // Calculate average score from profile data
    let profile = &state.progress.profile;
    let scenarios_completed = profile.scenarios_completed;
    let perfect_scenarios = profile.perfect_scenarios;

    let avg_score = if scenarios_completed > 0 {
        // Rough estimate: perfect = 100%, others average ~75%
        let perfect_total = perfect_scenarios * 100;
        let other_total = scenarios_completed.saturating_sub(perfect_scenarios) * 75;
        (perfect_total + other_total) / scenarios_completed
    } else {
        0
    };

    lines.push(Line::from(vec![
        Span::raw(t!("statistics.average_score").to_string()),
        Span::styled(format!("{avg_score}%"), Style::default().fg(Color::Cyan)),
    ]));

    // Total scenarios completed
    lines.push(Line::from(vec![
        Span::raw(t!("statistics.scenarios_completed").to_string()),
        Span::styled(
            format!("{scenarios_completed}"),
            Style::default().fg(Color::Green),
        ),
    ]));

    // Perfect completions
    lines.push(Line::from(vec![
        Span::raw(t!("statistics.perfect_completions").to_string()),
        Span::styled(
            format!("{perfect_scenarios}"),
            Style::default().fg(Color::Yellow),
        ),
    ]));

    // Session time
    let session_duration = state
        .progress
        .monotonic_now()
        .duration_since(state.progress.session_start_time);
    let minutes = session_duration.as_secs() / 60;
    let hours = minutes / 60;
    let remaining_minutes = minutes % 60;

    let time_str = if hours > 0 {
        t!(
            "statistics.duration_hours",
            hours = hours,
            minutes = remaining_minutes
        )
        .to_string()
    } else {
        t!("statistics.duration_minutes", minutes = minutes).to_string()
    };

    lines.push(Line::from(vec![
        Span::raw(t!("statistics.session_time").to_string()),
        Span::styled(time_str, Style::default().fg(Color::Yellow)),
    ]));
}

/// Render arcade mode statistics section
fn render_arcade_statistics(lines: &mut Vec<Line<'static>>, state: &AppState) {
    lines.push(Line::from(vec![Span::styled(
        t!("statistics.arcade_header").to_string(),
        Style::default()
            .fg(Color::Magenta)
            .add_modifier(Modifier::BOLD),
    )]));
    lines.push(Line::from(""));

    let profile = &state.progress.profile;

    // High Score
    lines.push(Line::from(vec![
        Span::raw(t!("statistics.high_score").to_string()),
        Span::styled(
            format!("{}", profile.minigame_high_score),
            Style::default().fg(Color::Yellow),
        ),
    ]));

    // Best Streak
    lines.push(Line::from(vec![
        Span::raw(t!("statistics.best_streak").to_string()),
        Span::styled(
            format!("{}", profile.minigame_best_streak),
            Style::default().fg(Color::Cyan),
        ),
    ]));

    // Games Played
    lines.push(Line::from(vec![
        Span::raw(t!("statistics.games_played").to_string()),
        Span::styled(
            format!("{}", profile.minigame_games_played),
            Style::default().fg(Color::Green),
        ),
    ]));
}

fn mastery_level_label(level: MasteryLevel) -> String {
    match level {
        MasteryLevel::Master => t!("mastery.master"),
        MasteryLevel::Advanced => t!("mastery.advanced"),
        MasteryLevel::Intermediate => t!("mastery.intermediate"),
        MasteryLevel::Beginner => t!("mastery.beginner"),
    }
    .to_string()
}

fn mastery_level_count(summary: &MasterySummary, level: MasteryLevel) -> usize {
    match level {
        MasteryLevel::Master => summary.master,
        MasteryLevel::Advanced => summary.advanced,
        MasteryLevel::Intermediate => summary.intermediate,
        MasteryLevel::Beginner => summary.beginner,
    }
}

fn scenario_mastery_label(level: ScenarioMastery) -> String {
    match level {
        ScenarioMastery::Mastered => t!("mastery.mastered"),
        ScenarioMastery::Proficient => t!("mastery.proficient"),
        ScenarioMastery::Learning => t!("mastery.learning"),
    }
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bar_text(count: usize, total: usize) -> String {
        let mut lines = Vec::new();
        render_mastery_bar(&mut lines, "Label", count, total, Color::Gray);
        lines
            .iter()
            .flat_map(|line| line.spans.iter())
            .map(|span| span.content.as_ref())
            .collect()
    }

    #[test]
    fn test_mastery_bar_fills_proportionally() {
        let text = bar_text(1, 2);
        assert!(text.contains(" 50%"), "{text}");
        assert_eq!(text.matches('|').count(), PROGRESS_BAR_WIDTH / 2);
    }

    #[test]
    fn test_mastery_bar_with_empty_total_is_zero_percent() {
        assert!(bar_text(0, 0).contains("  0%"));
    }

    #[test]
    fn test_mastery_bar_survives_extreme_counts() {
        let text = bar_text(usize::MAX, usize::MAX);
        assert!(text.contains('%'), "{text}");
    }

    #[test]
    fn test_tier_labels_match_domain_names() {
        use crate::learning::ProgressionTier;

        for level in [
            MasteryLevel::Master,
            MasteryLevel::Advanced,
            MasteryLevel::Intermediate,
            MasteryLevel::Beginner,
        ] {
            assert_eq!(mastery_level_label(level), level.name());
        }
        for level in [
            ScenarioMastery::Mastered,
            ScenarioMastery::Proficient,
            ScenarioMastery::Learning,
        ] {
            assert_eq!(scenario_mastery_label(level), level.name());
        }
    }
}
