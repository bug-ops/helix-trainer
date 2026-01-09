//! Profile screen rendering

use crate::learning::Analytics;
use crate::ui::state::AppState;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, Paragraph},
};

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
    let title = Paragraph::new("PLAYER PROFILE")
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
    let instructions =
        Paragraph::new("Press 's' for detailed statistics | Press 'm' to return to menu")
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
    let xp_progress = profile.xp_progress();
    let xp_for_next = profile.xp_for_next_level();
    let scenarios_completed = profile.scenarios_completed;
    let perfect_scenarios = profile.perfect_scenarios;

    // Calculate current XP in level
    let current_level_xp = crate::gamification::XPCalculator::xp_for_level(level);
    let xp_in_level = total_xp - current_level_xp;

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
    render_level_section(
        frame,
        level,
        xp_in_level,
        xp_for_next,
        xp_progress,
        content_chunks[0],
    );

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
    xp_in_level: u64,
    xp_for_next: u64,
    xp_progress: f64,
    area: ratatui::layout::Rect,
) {
    let level_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(2)])
        .split(area);

    // Level text
    let level_text = Paragraph::new(Line::from(vec![
        Span::styled(
            format!("Level {} ", level),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("⭐", Style::default().fg(Color::Yellow)),
    ]))
    .alignment(Alignment::Center);
    frame.render_widget(level_text, level_chunks[0]);

    // XP progress bar
    let xp_label = format!("{}/{} XP", xp_in_level, xp_for_next);
    let gauge = Gauge::default()
        .block(Block::default())
        .gauge_style(
            Style::default()
                .fg(Color::Yellow)
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .label(xp_label)
        .ratio(xp_progress.clamp(0.0, 1.0));
    frame.render_widget(gauge, level_chunks[1]);
}

/// Render statistics and quest information
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
        "📊 Statistics:",
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
    )]));
    lines.push(Line::from(""));

    lines.push(Line::from(vec![
        Span::raw("  Total Scenarios: "),
        Span::styled(
            format!("{}", scenarios_completed),
            Style::default().fg(Color::Cyan),
        ),
    ]));

    lines.push(Line::from(vec![
        Span::raw("  Perfect Runs: "),
        Span::styled(
            format!("{}", perfect_scenarios),
            Style::default().fg(Color::Green),
        ),
    ]));

    lines.push(Line::from(vec![
        Span::raw("  Total XP: "),
        Span::styled(format!("{}", total_xp), Style::default().fg(Color::Yellow)),
    ]));

    lines.push(Line::from(""));

    // Quest section
    let profile = &state.progress.profile;
    let active_quests = profile.daily_quests.len();
    let completed_today = profile.daily_quests.iter().filter(|q| q.completed).count();

    lines.push(Line::from(vec![Span::styled(
        "🎯 Daily Quests:",
        Style::default()
            .fg(Color::Magenta)
            .add_modifier(Modifier::BOLD),
    )]));
    lines.push(Line::from(""));

    lines.push(Line::from(vec![
        Span::raw("  Active Quests: "),
        Span::styled(
            format!("{}/3", active_quests),
            Style::default().fg(Color::Cyan),
        ),
    ]));

    lines.push(Line::from(vec![
        Span::raw("  Completed Today: "),
        Span::styled(
            format!("{}", completed_today),
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
        "Learning Progress:",
        Style::default()
            .fg(Color::Blue)
            .add_modifier(Modifier::BOLD),
    )]));
    lines.push(Line::from(""));

    lines.push(Line::from(vec![
        Span::raw("  Commands Learned: "),
        Span::styled(
            format!("{}", summary.total_commands),
            Style::default().fg(Color::Cyan),
        ),
    ]));

    lines.push(Line::from(vec![
        Span::raw("  Commands Mastered: "),
        Span::styled(
            format!("{}", summary.master),
            Style::default().fg(Color::Yellow),
        ),
    ]));

    if due_count > 0 {
        lines.push(Line::from(vec![
            Span::raw("  Due for Review: "),
            Span::styled(format!("{}", due_count), Style::default().fg(Color::Yellow)),
            Span::styled(" (press 'r' in menu)", Style::default().fg(Color::Gray)),
        ]));
    } else {
        lines.push(Line::from(vec![Span::styled(
            "  All caught up!",
            Style::default().fg(Color::Green),
        )]));
    }

    let stats = Paragraph::new(lines)
        .alignment(Alignment::Left)
        .style(Style::default().fg(Color::White));
    frame.render_widget(stats, area);
}
