//! Achievements screen rendering

use crate::gamification::AchievementEngine;
use crate::ui::state::{AppState, TypedScreen};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

/// Render the achievements screen, listing every achievement with its unlock status
///
/// Unlike the transient `Notification::Achievement` toast shown at unlock time, this
/// screen is a persistent view players can return to at any point to review their
/// full unlocked/locked achievement roster. The list scrolls (`j`/`k`/arrows) and clamps
/// its offset to the actual content height, so it degrades gracefully on small terminals
/// instead of silently truncating with no indicator.
pub(super) fn render_achievements_screen(frame: &mut Frame, state: &mut AppState) {
    let area = frame.area();

    // margin(1), not the Statistics/Profile screens' margin(2): at the minimum supported
    // terminal size (80x24), Length(3)+Min(15)+Length(3)=21 rows doesn't fit the 20 rows
    // margin(2) leaves, and the layout solver silently squeezes the Instructions section
    // to 2 rows (0 usable content height inside its border) - which would swallow the
    // achievement count + scroll position indicator without any visible sign of it.
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3), // Title
            Constraint::Min(15),   // Content
            Constraint::Length(3), // Instructions
        ])
        .split(area);

    let achievements = AchievementEngine::all_achievements();
    let unlocked_count = achievements
        .iter()
        .filter(|a| state.progress.profile.has_achievement(&a.id))
        .count();

    let title = Paragraph::new(format!(
        "ACHIEVEMENTS ({}/{})",
        unlocked_count,
        achievements.len()
    ))
    .style(
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )
    .alignment(Alignment::Center)
    .block(Block::default().borders(Borders::ALL));
    frame.render_widget(title, chunks[0]);

    let visible_range = render_achievements_content(frame, state, &achievements, chunks[1]);

    let scroll_hint = if let Some((first, last)) = visible_range {
        format!(" | {}-{}/{}", first, last, achievements.len())
    } else {
        String::new()
    };
    let instructions = Paragraph::new(format!(
        "'p': profile | 's': statistics | 'j'/'k': scroll | 'm': menu{}",
        scroll_hint
    ))
    .style(Style::default().fg(Color::Gray))
    .alignment(Alignment::Center)
    .block(Block::default().borders(Borders::ALL));
    frame.render_widget(instructions, chunks[2]);
}

/// Render the achievement list, one line per achievement, clamping and applying the
/// current scroll offset. Returns the 1-indexed (first, last) visible row range when the
/// list doesn't fully fit, for the instructions bar's position indicator.
fn render_achievements_content(
    frame: &mut Frame,
    state: &mut AppState,
    achievements: &[crate::gamification::Achievement],
    area: ratatui::layout::Rect,
) -> Option<(usize, usize)> {
    let content_block = Block::default().borders(Borders::ALL);
    let inner_area = content_block.inner(area);
    frame.render_widget(content_block, area);

    let TypedScreen::Achievements(ref mut data) = state.screen else {
        return None;
    };

    let visible_height = inner_area.height as usize;
    let total = achievements.len();
    let max_offset = total.saturating_sub(visible_height);
    data.scroll_offset = data.scroll_offset.min(max_offset);

    let profile = &state.progress.profile;
    let mut lines = Vec::with_capacity(total);
    for achievement in achievements {
        let unlocked = profile.has_achievement(&achievement.id);
        let (marker, name_style) = if unlocked {
            (
                "[X]",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            ("[ ]", Style::default().fg(Color::DarkGray))
        };
        let desc_style = if unlocked {
            Style::default().fg(Color::White)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        lines.push(Line::from(vec![
            Span::raw(format!("  {} ", marker)),
            Span::styled(format!("{:20}", achievement.name), name_style),
            Span::raw(" - "),
            Span::styled(achievement.description.clone(), desc_style),
        ]));
    }

    let scroll_offset = data.scroll_offset;
    let list = Paragraph::new(lines)
        .alignment(Alignment::Left)
        .style(Style::default().fg(Color::White))
        .scroll((scroll_offset as u16, 0));
    frame.render_widget(list, inner_area);

    if total > visible_height {
        let first = scroll_offset + 1;
        let last = (scroll_offset + visible_height).min(total);
        Some((first, last))
    } else {
        None
    }
}
