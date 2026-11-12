//! Profile screen rendering

use crate::gamification::LevelProgress;
use crate::learning::Analytics;
use crate::ui::state::AppState;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, Paragraph},
};
use rust_i18n::t;

/// Render the profile screen showing user level, XP, and statistics
pub(super) fn render_profile_screen(frame: &mut Frame, state: &AppState) {
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
    let title = Paragraph::new(t!("profile.screen_title").to_string())
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(title, chunks[0]);

    // Content area
    render_profile_content(frame, state, chunks[1]);

    // Instructions
    let instructions = Paragraph::new(t!("profile.instructions").to_string())
        .style(Style::default().fg(Color::Gray))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(instructions, chunks[2]);
}

/// Render the main profile content (level, XP, stats, quests)
fn render_profile_content(frame: &mut Frame, state: &AppState, area: ratatui::layout::Rect) {
    let content_block = Block::default().borders(Borders::ALL);
    let inner_area = content_block.inner(area);
    frame.render_widget(content_block, area);

    // Get profile data
    let profile = &state.progress.profile;
    let level = profile.level;
    let total_xp = profile.total_xp;
    let progress = profile.level_progress();
    let scenarios_completed = profile.scenarios_completed;
    let perfect_scenarios = profile.perfect_scenarios;

    // Split content into two sections
    let content_chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3), // Level and XP bar
            Constraint::Min(8),    // Stats and quests
        ])
        .split(inner_area);

    // Level and XP progress bar
    render_level_section(frame, level, progress, content_chunks[0]);

    // Statistics and quests
    render_stats_section(
        frame,
        state,
        scenarios_completed,
        perfect_scenarios,
        total_xp,
        content_chunks[1],
    );
}

/// Render level and XP progress bar
fn render_level_section(
    frame: &mut Frame,
    level: u32,
    progress: LevelProgress,
    area: ratatui::layout::Rect,
) {
    let level_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(2)])
        .split(area);

    // Level text
    let level_text = Paragraph::new(Line::from(vec![
        Span::styled(
            t!("profile.level", level = level).to_string(),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("⭐", Style::default().fg(Color::Yellow)),
    ]))
    .alignment(Alignment::Center);
    frame.render_widget(level_text, level_chunks[0]);

    // XP progress bar
    let xp_label = t!(
        "profile.xp_label",
        earned = progress.earned(),
        span = progress.span()
    )
    .to_string();
    let gauge = Gauge::default()
        .block(Block::default())
        .gauge_style(
            Style::default()
                .fg(Color::Yellow)
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .label(xp_label)
        .ratio(progress.ratio());
    frame.render_widget(gauge, level_chunks[1]);
}

/// Render statistics and quest information
#[expect(
    clippy::too_many_lines,
    reason = "one flat sequence of steps; splitting would only add indirection"
)]
#[expect(
    clippy::similar_names,
    reason = "parallel names mirror the values they bind"
)]
fn render_stats_section(
    frame: &mut Frame,
    state: &AppState,
    scenarios_completed: u32,
    perfect_scenarios: u32,
    total_xp: u64,
    area: ratatui::layout::Rect,
) {
    let mut lines = vec![];

    // Statistics section
    lines.push(Line::from(vec![Span::styled(
        t!("profile.statistics_heading").to_string(),
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
    )]));
    lines.push(Line::from(""));

    lines.push(Line::from(vec![
        Span::raw(t!("profile.total_scenarios").to_string()),
        Span::styled(
            format!("{scenarios_completed}"),
            Style::default().fg(Color::Cyan),
        ),
    ]));

    lines.push(Line::from(vec![
        Span::raw(t!("profile.perfect_runs").to_string()),
        Span::styled(
            format!("{perfect_scenarios}"),
            Style::default().fg(Color::Green),
        ),
    ]));

    lines.push(Line::from(vec![
        Span::raw(t!("profile.total_xp").to_string()),
        Span::styled(format!("{total_xp}"), Style::default().fg(Color::Yellow)),
    ]));

    lines.push(Line::from(""));

    // Quest section
    let profile = &state.progress.profile;
    let total_quests = profile.daily_quests.len();
    let completed_today = profile
        .daily_quests
        .iter()
        .filter(|q| q.is_completed())
        .count();
    let active_quests = total_quests.saturating_sub(completed_today);

    lines.push(Line::from(vec![Span::styled(
        t!("profile.daily_quests_heading").to_string(),
        Style::default()
            .fg(Color::Magenta)
            .add_modifier(Modifier::BOLD),
    )]));
    lines.push(Line::from(""));

    lines.push(Line::from(vec![
        Span::raw(t!("profile.active_quests").to_string()),
        Span::styled(
            format!("{active_quests}/{total_quests}"),
            Style::default().fg(Color::Cyan),
        ),
    ]));

    lines.push(Line::from(vec![
        Span::raw(t!("profile.completed_today").to_string()),
        Span::styled(
            format!("{completed_today}"),
            Style::default().fg(Color::Green),
        ),
    ]));

    lines.push(Line::from(""));

    // Learning Progress section (FSRS)
    let tracker = &state.progress.performance_tracker;
    let scheduler = &state.progress.scheduler;
    let summary = Analytics::get_mastery_summary(tracker);
    let due_count = scheduler.get_due_reviews(tracker).len();

    lines.push(Line::from(vec![Span::styled(
        t!("profile.learning_heading").to_string(),
        Style::default()
            .fg(Color::Blue)
            .add_modifier(Modifier::BOLD),
    )]));
    lines.push(Line::from(""));

    lines.push(Line::from(vec![
        Span::raw(t!("profile.commands_learned").to_string()),
        Span::styled(
            format!("{}", summary.total_commands),
            Style::default().fg(Color::Cyan),
        ),
    ]));

    lines.push(Line::from(vec![
        Span::raw(t!("profile.commands_mastered").to_string()),
        Span::styled(
            format!("{}", summary.master),
            Style::default().fg(Color::Yellow),
        ),
    ]));

    if due_count > 0 {
        lines.push(Line::from(vec![
            Span::raw(t!("profile.due_for_review").to_string()),
            Span::styled(format!("{due_count}"), Style::default().fg(Color::Yellow)),
            Span::styled(
                t!("profile.due_hint").to_string(),
                Style::default().fg(Color::Gray),
            ),
        ]));
    } else {
        lines.push(Line::from(vec![Span::styled(
            t!("profile.all_caught_up").to_string(),
            Style::default().fg(Color::Green),
        )]));
    }

    let stats = Paragraph::new(lines)
        .alignment(Alignment::Left)
        .style(Style::default().fg(Color::White));
    frame.render_widget(stats, area);
}
