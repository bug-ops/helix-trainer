//! Main menu rendering

use crate::ui::state::AppState;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, Paragraph},
};
use rust_i18n::t;

/// Render the main menu screen
pub(super) fn render_main_menu(frame: &mut Frame, state: &mut AppState) {
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

    // NEW: Render profile header
    render_profile_header(frame, chunks[0], state);

    // Title (updated index from 0 to 1)
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
    let menu_height = chunks[2].height.saturating_sub(2) as usize; // -2 for borders (updated index)
    let total_items = state.scenario_collection.count() + 3; // +3 for Profile, Statistics, Quit

    // Adjust scroll offset to keep selected item visible
    if state.selected_menu_item < state.menu_scroll_offset {
        // Selected item is above visible area - scroll up
        state.menu_scroll_offset = state.selected_menu_item;
    } else if state.selected_menu_item >= state.menu_scroll_offset + menu_height {
        // Selected item is below visible area - scroll down
        state.menu_scroll_offset = state.selected_menu_item.saturating_sub(menu_height - 1);
    }

    // Clamp scroll offset to valid range
    let max_offset = total_items.saturating_sub(menu_height);
    state.menu_scroll_offset = state.menu_scroll_offset.min(max_offset);

    // Menu items - show filtered scenarios + Quit option with indicators
    let filtered_scenarios = state.scenario_collection.get_filtered();
    let profile = state.profile.borrow();

    let mut menu_items: Vec<ListItem> = filtered_scenarios
        .iter()
        .enumerate()
        .map(|(i, scenario)| {
            let selected = i == state.selected_menu_item;
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

    let scenario_count = state.scenario_collection.count();

    // Add separator line
    menu_items.push(ListItem::new("─".repeat(30)).style(Style::default().fg(Color::DarkGray)));

    // Add Review Commands option
    let review_index = scenario_count;
    let review_selected = review_index == state.selected_menu_item;
    let due_count = state.scheduler.get_due_reviews().len();
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
    let profile_selected = profile_index == state.selected_menu_item;
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
    let stats_selected = stats_index == state.selected_menu_item;
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
    let quit_selected = quit_index == state.selected_menu_item;
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

    // Apply scroll offset by skipping items
    let visible_items: Vec<ListItem> = menu_items
        .into_iter()
        .skip(state.menu_scroll_offset)
        .take(menu_height)
        .collect();

    // Add scroll indicator to title if list is scrollable
    let menu_title = if total_items > menu_height {
        let first_visible = state.menu_scroll_offset + 1;
        let last_visible = (state.menu_scroll_offset + menu_height).min(total_items);
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
        let scrollbar_area = chunks[2];
        let scrollbar_height = scrollbar_area.height.saturating_sub(2) as usize; // -2 for borders

        if scrollbar_height > 0 {
            // Calculate scrollbar position
            let scrollbar_pos = if total_items > 1 {
                (state.menu_scroll_offset * scrollbar_height) / (total_items - menu_height).max(1)
            } else {
                0
            };

            // Calculate scrollbar thumb size (proportional to visible items)
            let thumb_size = ((menu_height * scrollbar_height) / total_items).max(1);

            // Draw scrollbar on the right edge
            for y in 0..scrollbar_height {
                let is_thumb = y >= scrollbar_pos && y < scrollbar_pos + thumb_size;
                let symbol = if is_thumb { "█" } else { "│" };
                let style = if is_thumb {
                    Style::default().fg(Color::Cyan)
                } else {
                    Style::default().fg(Color::DarkGray)
                };

                let x = scrollbar_area.x + scrollbar_area.width - 2; // -2 to be inside border
                let y = scrollbar_area.y + 1 + y as u16; // +1 for top border

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
    frame.render_widget(instructions, chunks[4]); // Updated index to 4 (after quest panel)
}

/// Render the profile header showing level, XP, and streak
fn render_profile_header(frame: &mut Frame, area: ratatui::layout::Rect, state: &AppState) {
    let profile = state.profile.borrow();

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
    let profile = state.profile.borrow();
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
