//! Main menu rendering

use crate::ui::state::{AppState, TypedScreen};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, Paragraph},
};
use rust_i18n::t;

/// Calculate menu scroll offset to keep selected item visible
///
/// Returns (scroll_offset, visible_count) tuple
fn calculate_menu_scroll(
    selected: usize,
    current_offset: usize,
    visible_height: usize,
    total_items: usize,
) -> (usize, usize) {
    let mut scroll_offset = current_offset;

    // Adjust scroll offset to keep selected item visible
    if selected < scroll_offset {
        // Selected item is above visible area - scroll up
        scroll_offset = selected;
    } else if selected >= scroll_offset + visible_height {
        // Selected item is below visible area - scroll down
        scroll_offset = selected.saturating_sub(visible_height - 1);
    }

    // Clamp scroll offset to valid range
    let max_offset = total_items.saturating_sub(visible_height);
    scroll_offset = scroll_offset.min(max_offset);

    (scroll_offset, visible_height)
}

/// Build menu items with scenario list and navigation options
///
/// Returns list of menu items with proper styling and indicators
fn build_menu_items(state: &AppState, selected_item: usize) -> Vec<ListItem<'_>> {
    let filtered_scenarios = state.game.scenario_collection.get_filtered();
    let profile = state.progress.profile.borrow();

    let mut menu_items: Vec<ListItem> = filtered_scenarios
        .iter()
        .enumerate()
        .map(|(i, scenario)| {
            let selected = i == selected_item;
            let style = if selected {
                Style::default()
                    .bg(Color::Blue)
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            // Get difficulty indicator
            let difficulty_indicator = scenario
                .metadata
                .as_ref()
                .and_then(|m| m.difficulty)
                .map(|d| match d {
                    crate::config::Difficulty::Beginner => "🟢",
                    crate::config::Difficulty::Intermediate => "🟡",
                    crate::config::Difficulty::Advanced => "🔴",
                })
                .unwrap_or("  ");

            // Get completion status
            let completion_indicator = if profile.scenario_history.get(&scenario.id).is_some() {
                "✅"
            } else {
                "  "
            };

            let prefix = if selected { "> " } else { "  " };
            let display = format!(
                "{}. {} {} {}",
                i + 1,
                difficulty_indicator,
                scenario.name,
                completion_indicator
            );
            ListItem::new(format!("{}{}", prefix, display)).style(style)
        })
        .collect();

    let scenario_count = state.game.scenario_collection.count();

    // Add separator line
    menu_items.push(ListItem::new("─".repeat(30)).style(Style::default().fg(Color::DarkGray)));

    // Add Review Commands option
    let review_index = scenario_count;
    let review_selected = review_index == selected_item;
    let due_count = state.progress.scheduler.get_due_reviews().len();
    let review_style = if review_selected {
        Style::default()
            .bg(Color::Blue)
            .fg(Color::White)
            .add_modifier(Modifier::BOLD)
    } else if due_count > 0 {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Gray)
    };
    let review_prefix = if review_selected { "> " } else { "  " };
    menu_items.push(
        ListItem::new(format!(
            "{}Review Commands (r){}",
            review_prefix,
            if due_count > 0 {
                format!(" [{}]", due_count)
            } else {
                String::new()
            }
        ))
        .style(review_style),
    );

    // Add View Profile option
    let profile_index = scenario_count + 1;
    let profile_selected = profile_index == selected_item;
    let profile_style = if profile_selected {
        Style::default()
            .bg(Color::Blue)
            .fg(Color::White)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Cyan)
    };
    let profile_prefix = if profile_selected { "> " } else { "  " };
    menu_items
        .push(ListItem::new(format!("{}View Profile (p)", profile_prefix)).style(profile_style));

    // Add Statistics option
    let stats_index = scenario_count + 2;
    let stats_selected = stats_index == selected_item;
    let stats_style = if stats_selected {
        Style::default()
            .bg(Color::Blue)
            .fg(Color::White)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Cyan)
    };
    let stats_prefix = if stats_selected { "> " } else { "  " };
    menu_items.push(ListItem::new(format!("{}Statistics (s)", stats_prefix)).style(stats_style));

    // Add Quit option at the end
    let quit_index = scenario_count + 3;
    let quit_selected = quit_index == selected_item;
    let quit_style = if quit_selected {
        Style::default()
            .bg(Color::Blue)
            .fg(Color::White)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Red)
    };
    let quit_prefix = if quit_selected { "> " } else { "  " };
    menu_items.push(ListItem::new(format!("{}{}", quit_prefix, t!("menu.quit"))).style(quit_style));

    menu_items
}

/// Render scrollbar for menu navigation
///
/// Draws a visual scrollbar on the right edge of the menu area
fn render_scrollbar(
    frame: &mut Frame,
    area: ratatui::layout::Rect,
    scroll_offset: usize,
    total_items: usize,
    visible_height: usize,
) {
    let scrollbar_height = area.height.saturating_sub(2) as usize; // -2 for borders

    if scrollbar_height > 0 {
        // Calculate scrollbar position
        let scrollbar_pos = if total_items > 1 {
            (scroll_offset * scrollbar_height) / (total_items - visible_height).max(1)
        } else {
            0
        };

        // Calculate scrollbar thumb size (proportional to visible items)
        let thumb_size = ((visible_height * scrollbar_height) / total_items).max(1);

        // Draw scrollbar on the right edge
        for y in 0..scrollbar_height {
            let is_thumb = y >= scrollbar_pos && y < scrollbar_pos + thumb_size;
            let symbol = if is_thumb { "█" } else { "│" };
            let style = if is_thumb {
                Style::default().fg(Color::Cyan)
            } else {
                Style::default().fg(Color::DarkGray)
            };

            let x = area.x + area.width - 2; // -2 to be inside border
            let y = area.y + 1 + y as u16; // +1 for top border

            frame.render_widget(
                Paragraph::new(symbol).style(style),
                ratatui::layout::Rect {
                    x,
                    y,
                    width: 1,
                    height: 1,
                },
            );
        }
    }
}

/// Render the main menu screen
pub(super) fn render_main_menu(frame: &mut Frame, state: &mut AppState) {
    // Extract MenuData from TypedScreen::Menu (early check)
    if !matches!(state.screen, TypedScreen::Menu(_)) {
        return; // Wrong screen type
    };

    let area = frame.area();

    // Create layout: header | title | menu | quests | instructions
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Length(1), // Header with level/XP/streak
            Constraint::Length(3), // Title
            Constraint::Min(4),    // Menu items
            Constraint::Length(6), // Quest panel
            Constraint::Length(3), // Instructions
        ])
        .split(area);

    // Render profile header
    render_profile_header(frame, chunks[0], state);

    // Title
    let title = Paragraph::new(t!("menu.title").to_string())
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(title, chunks[1]);

    // Calculate visible area height for menu (excluding borders)
    let menu_height = chunks[2].height.saturating_sub(2) as usize;
    let total_items = state.game.scenario_collection.count() + 4; // +4 for Review, Profile, Statistics, Quit

    // Get mutable access to menu_data for adjusting scroll
    let TypedScreen::Menu(menu_data) = &mut state.screen else {
        unreachable!("Already checked above")
    };

    // Calculate scroll offset
    let (scroll_offset, _) = calculate_menu_scroll(
        menu_data.selected_item,
        menu_data.scroll_offset,
        menu_height,
        total_items,
    );
    menu_data.scroll_offset = scroll_offset;

    // Get copies of the values we need (to release the borrow)
    let selected_item = menu_data.selected_item;

    // Build menu items
    let all_items = build_menu_items(state, selected_item);

    // Apply scroll offset by skipping items
    let visible_items: Vec<ListItem> = all_items
        .into_iter()
        .skip(scroll_offset)
        .take(menu_height)
        .collect();

    // Add scroll indicator to title if list is scrollable
    let menu_title = if total_items > menu_height {
        let first_visible = scroll_offset + 1;
        let last_visible = (scroll_offset + menu_height).min(total_items);
        t!(
            "menu.main_menu_with_scroll",
            first = first_visible,
            last = last_visible,
            total = total_items
        )
        .to_string()
    } else {
        t!("menu.main_menu_total", total = total_items).to_string()
    };

    let menu = List::new(visible_items)
        .block(Block::default().title(menu_title).borders(Borders::ALL))
        .style(Style::default().fg(Color::White));
    frame.render_widget(menu, chunks[2]);

    // Render quest panel
    render_quest_panel(frame, chunks[3], state);

    // Draw scrollbar if needed
    if total_items > menu_height {
        render_scrollbar(frame, chunks[2], scroll_offset, total_items, menu_height);
    }

    // Instructions
    let instructions = if total_items > 9 {
        Paragraph::new(t!("menu.instructions_with_numbers").to_string())
    } else {
        Paragraph::new(t!("menu.instructions").to_string())
    };

    let instructions = instructions
        .style(Style::default().fg(Color::Gray))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(instructions, chunks[4]);
}

/// Render the profile header showing level, XP, and streak
fn render_profile_header(frame: &mut Frame, area: ratatui::layout::Rect, state: &AppState) {
    let profile = state.progress.profile.borrow();

    let next_level_xp = crate::gamification::XPCalculator::xp_for_level(profile.level + 1);
    let progress_pct = (profile.xp_progress() * 100.0) as u8;

    let header_text = format!(
        "Level {} ⭐  🔥 {} days   XP: {}/{} ({}%)",
        profile.level, profile.current_streak, profile.total_xp, next_level_xp, progress_pct
    );

    let header = Paragraph::new(header_text)
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .alignment(Alignment::Center);

    frame.render_widget(header, area);
}

/// Render the quest panel showing daily quests
fn render_quest_panel(frame: &mut Frame, area: ratatui::layout::Rect, state: &AppState) {
    let profile = state.progress.profile.borrow();
    let quests = &profile.daily_quests;

    let completed_count = quests.iter().filter(|q| q.is_completed()).count();
    let title = format!(" Daily Quests ({}/{}) ", completed_count, quests.len());

    // Build quest lines
    let quest_lines: Vec<String> = quests
        .iter()
        .map(|quest| {
            let icon = if quest.is_completed() { "✓" } else { "⏺" };
            let progress_str = format_quest_progress(&quest.quest_type);

            // Format: "✓ Delete 3 lines (3/3)    +25 XP"
            format!(
                " {} {}{}    +{} XP",
                icon, quest.description, progress_str, quest.xp_reward
            )
        })
        .collect();

    let quest_text = quest_lines.join("\n");

    // Use colors: green for completed, yellow for in-progress
    let panel = Paragraph::new(quest_text)
        .style(Style::default().fg(Color::White))
        .block(Block::default().title(title).borders(Borders::ALL));

    frame.render_widget(panel, area);
}

/// Format quest progress string based on quest type
fn format_quest_progress(quest_type: &crate::gamification::QuestType) -> String {
    use crate::gamification::QuestType;

    match quest_type {
        QuestType::CommandPractice {
            current, target, ..
        } => format!(" ({}/{})", current, target),
        QuestType::ScenarioCompletion { current, target } => format!(" ({}/{})", current, target),
        QuestType::TimeInvested {
            current_minutes,
            target_minutes,
        } => format!(" ({}/{})", current_minutes, target_minutes),
        QuestType::Exploration {
            commands_used,
            target_commands,
        } => format!(" ({}/{})", commands_used.len(), target_commands),
        QuestType::SpeedRun { .. } => String::new(), // No progress display for speed runs
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Unit tests for calculate_menu_scroll()
    mod calculate_menu_scroll_tests {
        use super::*;

        #[test]
        fn test_calculate_menu_scroll_selected_at_top() {
            // Selected item is 0, already at top
            let (offset, _) = calculate_menu_scroll(0, 0, 10, 20);
            assert_eq!(offset, 0);
        }

        #[test]
        fn test_calculate_menu_scroll_selected_at_bottom() {
            // Selected item is 19 (last), visible height 10
            // Should scroll to 10 so items 10-19 are visible
            let (offset, _) = calculate_menu_scroll(19, 0, 10, 20);
            assert_eq!(offset, 10);
        }

        #[test]
        fn test_calculate_menu_scroll_selected_in_middle() {
            // Selected item is 5, already in visible range [0, 10)
            let (offset, _) = calculate_menu_scroll(5, 0, 10, 20);
            assert_eq!(offset, 0);
        }

        #[test]
        fn test_calculate_menu_scroll_selected_above_visible() {
            // Selected item is 3, current offset 5
            // Should scroll up to show item 3
            let (offset, _) = calculate_menu_scroll(3, 5, 10, 20);
            assert_eq!(offset, 3);
        }

        #[test]
        fn test_calculate_menu_scroll_selected_below_visible() {
            // Selected item is 15, current offset 0
            // Visible range is [0, 10), should scroll to show 15
            let (offset, _) = calculate_menu_scroll(15, 0, 10, 20);
            assert_eq!(offset, 6); // 15 - 10 + 1 = 6
        }

        #[test]
        fn test_calculate_menu_scroll_all_items_visible() {
            // Only 5 items total, visible height 10
            // Should not scroll
            let (offset, _) = calculate_menu_scroll(4, 0, 10, 5);
            assert_eq!(offset, 0);
        }

        #[test]
        fn test_calculate_menu_scroll_zero_items() {
            // Edge case: 0 items
            let (offset, _) = calculate_menu_scroll(0, 0, 10, 0);
            assert_eq!(offset, 0);
        }

        #[test]
        fn test_calculate_menu_scroll_one_item() {
            // Edge case: 1 item
            let (offset, _) = calculate_menu_scroll(0, 0, 10, 1);
            assert_eq!(offset, 0);
        }

        #[test]
        fn test_calculate_menu_scroll_max_offset_clamped() {
            // Selected item is beyond total items - should clamp offset
            // Max offset is total_items - visible_height = 20 - 10 = 10
            let (offset, _) = calculate_menu_scroll(19, 15, 10, 20);
            assert_eq!(offset, 10);
        }

        #[test]
        fn test_calculate_menu_scroll_maintains_visibility() {
            // Scrolling down gradually
            let (offset, _) = calculate_menu_scroll(9, 0, 10, 20);
            assert_eq!(offset, 0); // Still visible in [0, 10)

            let (offset, _) = calculate_menu_scroll(10, 0, 10, 20);
            assert_eq!(offset, 1); // Need to scroll to keep 10 visible
        }

        #[test]
        fn test_calculate_menu_scroll_returns_visible_height() {
            let (_, visible) = calculate_menu_scroll(5, 0, 10, 20);
            assert_eq!(visible, 10);
        }
    }
}
