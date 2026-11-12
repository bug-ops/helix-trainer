//! Achievements screen rendering

use crate::gamification::AchievementEngine;
use crate::ui::layout::{achievement_chunks, clamp_achievement_scroll};
use crate::ui::state::{AppState, TypedScreen};
use crate::ui::text;
use ratatui::{
    Frame,
    layout::Alignment,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};
use rust_i18n::t;

/// Render the achievements screen, listing every achievement with its unlock status
///
/// Unlike the transient `Notification::Achievement` toast shown at unlock time, this
/// screen is a persistent view players can return to at any point to review their
/// full unlocked/locked achievement roster. The list scrolls (`j`/`k`/arrows) and clamps
/// its offset to the actual content height, so it degrades gracefully on small terminals
/// instead of silently truncating with no indicator.
pub(super) fn render_achievements_screen(frame: &mut Frame, state: &AppState) {
    let area = frame.area();

    let chunks = achievement_chunks(area);

    let achievements = AchievementEngine::all_achievements();
    let unlocked_count = achievements
        .iter()
        .filter(|a| state.progress.profile.has_achievement(&a.id))
        .count();

    let title = Paragraph::new(
        t!(
            "achievements.title",
            unlocked = unlocked_count,
            total = achievements.len()
        )
        .to_string(),
    )
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
        t!(
            "achievements.scroll_hint",
            first = first,
            last = last,
            total = achievements.len()
        )
        .to_string()
    } else {
        String::new()
    };
    let instructions =
        Paragraph::new(t!("achievements.instructions", scroll_hint = scroll_hint).to_string())
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
    frame.render_widget(instructions, chunks[2]);
}

/// Render the achievement list, one line per achievement, clamping and applying the
/// current scroll offset. Returns the 1-indexed (first, last) visible row range when the
/// list doesn't fully fit, for the instructions bar's position indicator.
#[expect(
    clippy::similar_names,
    reason = "parallel names mirror the values they bind"
)]
fn render_achievements_content(
    frame: &mut Frame,
    state: &AppState,
    achievements: &[crate::gamification::Achievement],
    area: ratatui::layout::Rect,
) -> Option<(usize, usize)> {
    let content_block = Block::default().borders(Borders::ALL);
    let inner_area = content_block.inner(area);
    frame.render_widget(content_block, area);

    let TypedScreen::Achievements(data) = &state.screen else {
        return None;
    };

    // Read-only defensive clamp; `AppState::normalize_view` clamps the stored offset
    let visible_height = inner_area.height as usize;
    let total = achievements.len();
    let scroll_offset = clamp_achievement_scroll(data.scroll_offset, total, visible_height);

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
            Span::raw(format!("  {marker} ")),
            Span::styled(
                format!("{:20}", text::achievement_name(achievement.id)),
                name_style,
            ),
            Span::raw(" - "),
            Span::styled(text::achievement_description(achievement.id), desc_style),
        ]));
    }

    let list = Paragraph::new(lines)
        .alignment(Alignment::Left)
        .style(Style::default().fg(Color::White))
        .scroll((u16::try_from(scroll_offset).unwrap_or(u16::MAX), 0));
    frame.render_widget(list, inner_area);

    if total > visible_height {
        let first = scroll_offset + 1;
        let last = (scroll_offset + visible_height).min(total);
        Some((first, last))
    } else {
        None
    }
}
