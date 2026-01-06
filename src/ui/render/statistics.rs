//! Statistics screen rendering

use crate::constants::PROGRESS_BAR_WIDTH;
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
    let instructions =
        Paragraph::new("Press 'p' to return to profile | Press 'm' to return to menu")
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

    // Get profile data
    let profile = &state.progress.profile;
    let scenarios_completed = profile.scenarios_completed;
    let perfect_scenarios = profile.perfect_scenarios;

    // Calculate performance breakdown
    // For now, we'll use simplified calculations since we don't track all rating types yet
    // In the future, this should come from PerformanceTracker
    let total = scenarios_completed.max(1); // Avoid division by zero
    let perfect_count = perfect_scenarios;

    // Estimate other ratings (placeholder logic - in real implementation, these should be tracked)
    // For demonstration purposes, we'll distribute the remaining scenarios
    let remaining = total.saturating_sub(perfect_count);
    let excellent_count = (remaining as f32 * 0.4) as u32;
    let good_count = (remaining as f32 * 0.3) as u32;
    let fair_count = (remaining as f32 * 0.2) as u32;
    let poor_count = remaining.saturating_sub(excellent_count + good_count + fair_count);

    let mut lines = vec![];

    // Performance Breakdown section
    lines.push(Line::from(vec![Span::styled(
        "📈 Performance Breakdown:",
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
    )]));
    lines.push(Line::from(""));

    // Perfect
    render_rating_bar(&mut lines, "Perfect", perfect_count, total, Color::Green);

    // Excellent
    render_rating_bar(&mut lines, "Excellent", excellent_count, total, Color::Cyan);

    // Good
    render_rating_bar(&mut lines, "Good", good_count, total, Color::Yellow);

    // Fair
    render_rating_bar(&mut lines, "Fair", fair_count, total, Color::Magenta);

    // Poor
    render_rating_bar(&mut lines, "Poor", poor_count, total, Color::Red);

    lines.push(Line::from(""));

    // Quest Statistics section
    render_quest_statistics(&mut lines, state);

    lines.push(Line::from(""));

    // Session History section
    render_session_history(&mut lines, state);

    let stats = Paragraph::new(lines)
        .alignment(Alignment::Left)
        .style(Style::default().fg(Color::White));
    frame.render_widget(stats, inner_area);
}

/// Render a horizontal bar chart for a performance rating
fn render_rating_bar(
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
    let bar = format!("{}{}", "█".repeat(filled), "░".repeat(empty));

    lines.push(Line::from(vec![
        Span::raw(format!("  {:10}: ", label)),
        Span::styled(format!("{:3} ", count), Style::default().fg(Color::White)),
        Span::styled(bar, Style::default().fg(color)),
        Span::raw(format!("  {:3}%", percentage)),
    ]));
}

/// Render quest statistics section
fn render_quest_statistics(lines: &mut Vec<Line<'static>>, state: &AppState) {
    lines.push(Line::from(vec![Span::styled(
        "🎯 Quest Statistics:",
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
    let profile = &state.progress.profile;
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
        "⏱️  Session History:",
        Style::default()
            .fg(Color::Blue)
            .add_modifier(Modifier::BOLD),
    )]));
    lines.push(Line::from(""));

    // Calculate average score (placeholder - should be tracked properly)
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
        Span::raw("  Total Time Played: "),
        Span::styled(time_str, Style::default().fg(Color::Yellow)),
    ]));
}
