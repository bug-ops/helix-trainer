//! Statistics screen rendering

use crate::constants::PROGRESS_BAR_WIDTH;
use crate::learning::{Analytics, MasterySummary};
use crate::ui::state::AppState;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

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
    let title = Paragraph::new("DETAILED STATISTICS")
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
    let instructions = Paragraph::new(
        "Press 'p' for profile | Press 'a' for achievements | Press 'm' to return to menu",
    )
    .style(Style::default().fg(Color::Gray))
    .alignment(Alignment::Center)
    .block(Block::default().borders(Borders::ALL));
    frame.render_widget(instructions, chunks[2]);
}

/// Render the main statistics content
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
        "Command Mastery Distribution:",
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
    )]));
    lines.push(Line::from(""));

    let tracker = &state.progress.performance_tracker;
    let summary: MasterySummary = Analytics::get_mastery_summary(tracker);

    if summary.total_commands == 0 {
        lines.push(Line::from(vec![Span::styled(
            "  No commands tracked yet. Complete some scenarios to see your progress!",
            Style::default().fg(Color::Gray),
        )]));
        return;
    }

    let total = summary.total_commands;

    // Master level
    render_mastery_bar(lines, "Master", summary.master, total, Color::Yellow);

    // Advanced level
    render_mastery_bar(lines, "Advanced", summary.advanced, total, Color::Cyan);

    // Intermediate level
    render_mastery_bar(
        lines,
        "Intermediate",
        summary.intermediate,
        total,
        Color::Green,
    );

    // Beginner level
    render_mastery_bar(lines, "Beginner", summary.beginner, total, Color::Gray);

    // Additional metrics
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::raw("  Avg Stability: "),
        Span::styled(
            format!("{:.1} days", summary.avg_stability),
            Style::default().fg(Color::Cyan),
        ),
        Span::raw("  |  Avg Difficulty: "),
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
    let percentage = if total > 0 {
        (count as f32 / total as f32 * 100.0) as u32
    } else {
        0
    };

    // Create horizontal progress bar
    let filled = (percentage as usize * PROGRESS_BAR_WIDTH / 100).min(PROGRESS_BAR_WIDTH);
    let empty = PROGRESS_BAR_WIDTH - filled;
    let bar = format!("{}{}", "|".repeat(filled), ".".repeat(empty));

    lines.push(Line::from(vec![
        Span::raw(format!("  {:12}: ", label)),
        Span::styled(format!("{:3} ", count), Style::default().fg(Color::White)),
        Span::styled(bar, Style::default().fg(color)),
        Span::raw(format!("  {:3}%", percentage)),
    ]));
}

/// Render review status section using real FSRS scheduler data
fn render_review_status_section(lines: &mut Vec<Line<'static>>, state: &AppState) {
    lines.push(Line::from(vec![Span::styled(
        "Review Status (FSRS):",
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
        Span::raw("  Due for Review: "),
        Span::styled(format!("{}", due_count), Style::default().fg(due_color)),
        Span::styled(
            if due_count > 5 {
                " (practice recommended!)"
            } else if due_count > 0 {
                " (some practice needed)"
            } else {
                " (all caught up!)"
            },
            Style::default().fg(Color::Gray),
        ),
    ]));

    // Total commands tracked
    lines.push(Line::from(vec![
        Span::raw("  Commands Tracked: "),
        Span::styled(
            format!("{}", total_commands),
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
        Span::raw("  Avg Success Rate: "),
        Span::styled(
            format!("{:.0}%", avg_success_rate * 100.0),
            Style::default().fg(success_color),
        ),
    ]));
}

/// Render weak commands section showing commands that need practice
fn render_weak_commands_section(lines: &mut Vec<Line<'static>>, state: &AppState) {
    lines.push(Line::from(vec![Span::styled(
        "Commands Needing Practice:",
        Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
    )]));
    lines.push(Line::from(""));

    let tracker = &state.progress.performance_tracker;
    let weak_commands = tracker.weak_commands();

    if weak_commands.is_empty() {
        lines.push(Line::from(vec![Span::styled(
            "  No weak commands detected. Great job!",
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
                Span::styled(format!("{:10}", cmd), Style::default().fg(Color::White)),
                Span::raw(" - "),
                Span::styled(
                    format!("{:.0}% success", success_rate * 100.0),
                    Style::default().fg(rate_color),
                ),
                Span::raw(", "),
                Span::styled(
                    format!("{} lapses", perf.lapses),
                    Style::default().fg(if perf.lapses > 2 {
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
            format!("  ... and {} more", weak_commands.len() - 5),
            Style::default().fg(Color::Gray),
        )]));
    }
}

/// Render scenario mastery section using profile's scenario history
fn render_scenario_mastery_section(lines: &mut Vec<Line<'static>>, state: &AppState) {
    lines.push(Line::from(vec![Span::styled(
        "Scenario Mastery:",
        Style::default()
            .fg(Color::Magenta)
            .add_modifier(Modifier::BOLD),
    )]));
    lines.push(Line::from(""));

    let profile = &state.progress.profile;
    let stats = profile.scenario_history.mastery_stats();

    let total = stats.learning + stats.proficient + stats.mastered;

    if total == 0 {
        lines.push(Line::from(vec![Span::styled(
            "  No scenarios attempted yet. Start practicing!",
            Style::default().fg(Color::Gray),
        )]));
        return;
    }

    // Mastered scenarios
    render_scenario_mastery_bar(lines, "Mastered", stats.mastered, total, Color::Yellow);

    // Proficient scenarios
    render_scenario_mastery_bar(lines, "Proficient", stats.proficient, total, Color::Cyan);

    // Learning scenarios
    render_scenario_mastery_bar(lines, "Learning", stats.learning, total, Color::Gray);

    // Total scenarios attempted
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::raw("  Total Scenarios Attempted: "),
        Span::styled(format!("{}", total), Style::default().fg(Color::White)),
    ]));
}

/// Render a horizontal bar for scenario mastery distribution
fn render_scenario_mastery_bar(
    lines: &mut Vec<Line<'static>>,
    label: &str,
    count: u32,
    total: u32,
    color: Color,
) {
    let percentage = if total > 0 {
        (count as f32 / total as f32 * 100.0) as u32
    } else {
        0
    };

    // Create horizontal progress bar
    let filled = (percentage as usize * PROGRESS_BAR_WIDTH / 100).min(PROGRESS_BAR_WIDTH);
    let empty = PROGRESS_BAR_WIDTH - filled;
    let bar = format!("{}{}", "|".repeat(filled), ".".repeat(empty));

    lines.push(Line::from(vec![
        Span::raw(format!("  {:12}: ", label)),
        Span::styled(format!("{:3} ", count), Style::default().fg(Color::White)),
        Span::styled(bar, Style::default().fg(color)),
        Span::raw(format!("  {:3}%", percentage)),
    ]));
}

/// Render quest statistics section
fn render_quest_statistics(lines: &mut Vec<Line<'static>>, state: &AppState) {
    lines.push(Line::from(vec![Span::styled(
        "Quest Statistics:",
        Style::default()
            .fg(Color::Magenta)
            .add_modifier(Modifier::BOLD),
    )]));
    lines.push(Line::from(""));

    // Get quest data
    let profile = &state.progress.profile;
    let completed_today = profile.daily_quests.iter().filter(|q| q.completed).count();

    lines.push(Line::from(vec![
        Span::raw("  Daily Quests Completed: "),
        Span::styled(
            format!("{}", completed_today),
            Style::default().fg(Color::Green),
        ),
    ]));

    // Streak information
    let current_streak = profile.current_streak;
    let longest_streak = profile.longest_streak;

    lines.push(Line::from(vec![
        Span::raw("  Current Streak: "),
        Span::styled(
            format!("{} days", current_streak),
            Style::default().fg(Color::Yellow),
        ),
    ]));

    lines.push(Line::from(vec![
        Span::raw("  Longest Streak: "),
        Span::styled(
            format!("{} days", longest_streak),
            Style::default().fg(Color::Cyan),
        ),
    ]));
}

/// Render session history section
fn render_session_history(lines: &mut Vec<Line<'static>>, state: &AppState) {
    lines.push(Line::from(vec![Span::styled(
        "Session History:",
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
        Span::raw("  Average Score: "),
        Span::styled(format!("{}%", avg_score), Style::default().fg(Color::Cyan)),
    ]));

    // Total scenarios completed
    lines.push(Line::from(vec![
        Span::raw("  Scenarios Completed: "),
        Span::styled(
            format!("{}", scenarios_completed),
            Style::default().fg(Color::Green),
        ),
    ]));

    // Perfect completions
    lines.push(Line::from(vec![
        Span::raw("  Perfect Completions: "),
        Span::styled(
            format!("{}", perfect_scenarios),
            Style::default().fg(Color::Yellow),
        ),
    ]));

    // Session time
    let session_duration = state.progress.session_start_time.elapsed();
    let minutes = session_duration.as_secs() / 60;
    let hours = minutes / 60;
    let remaining_minutes = minutes % 60;

    let time_str = if hours > 0 {
        format!("{}h {}m", hours, remaining_minutes)
    } else {
        format!("{}m", minutes)
    };

    lines.push(Line::from(vec![
        Span::raw("  Current Session Time: "),
        Span::styled(time_str, Style::default().fg(Color::Yellow)),
    ]));
}

/// Render arcade mode statistics section
fn render_arcade_statistics(lines: &mut Vec<Line<'static>>, state: &AppState) {
    lines.push(Line::from(vec![Span::styled(
        "Arcade Mode:",
        Style::default()
            .fg(Color::Magenta)
            .add_modifier(Modifier::BOLD),
    )]));
    lines.push(Line::from(""));

    let profile = &state.progress.profile;

    // High Score
    lines.push(Line::from(vec![
        Span::raw("  High Score: "),
        Span::styled(
            format!("{}", profile.minigame_high_score),
            Style::default().fg(Color::Yellow),
        ),
    ]));

    // Best Streak
    lines.push(Line::from(vec![
        Span::raw("  Best Streak: "),
        Span::styled(
            format!("{}", profile.minigame_best_streak),
            Style::default().fg(Color::Cyan),
        ),
    ]));

    // Games Played
    lines.push(Line::from(vec![
        Span::raw("  Games Played: "),
        Span::styled(
            format!("{}", profile.minigame_games_played),
            Style::default().fg(Color::Green),
        ),
    ]));
}
