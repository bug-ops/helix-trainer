//! Main menu rendering

use crate::ui::layout::{FIXED_MENU_ITEMS, RENDERED_FIXED_ROWS, clamp_menu_view, menu_chunks};
use crate::ui::state::{AppState, TypedScreen};
use crate::ui::text;
use ratatui::{
    Frame,
    layout::Alignment,
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, Paragraph},
};
use rust_i18n::t;

/// Build menu items with scenario list and navigation options
///
/// Returns list of menu items with proper styling and indicators
fn build_menu_items(state: &AppState, selected_item: usize) -> Vec<ListItem<'_>> {
    let filtered_scenarios = state.game.scenario_collection.filtered();
    let profile = &state.progress.profile;

    let mut menu_items: Vec<ListItem> = filtered_scenarios
        .iter()
        .enumerate()
        .map(|(i, scenario)| {
            let selected = i == selected_item;
            let style = if selected {
                Style::default()
                    .bg(super::SELECTION_BG_COLOR)
                    .fg(Color::White)
            } else {
                Style::default().fg(Color::White)
            };

            // Get difficulty indicator
            let difficulty_indicator = scenario.metadata.difficulty.map_or("  ", |d| match d {
                crate::config::Difficulty::Beginner => "🟢",
                crate::config::Difficulty::Intermediate => "🟡",
                crate::config::Difficulty::Advanced => "🔴",
            });

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
            ListItem::new(format!("{prefix}{display}")).style(style)
        })
        .collect();

    let scenario_count = state.game.scenario_collection.count();

    // Add separator line
    menu_items.push(ListItem::new("─".repeat(30)).style(Style::default().fg(Color::DarkGray)));

    // Add Review Commands option
    let review_index = scenario_count;
    let review_selected = review_index == selected_item;
    let due_count = state
        .progress
        .scheduler
        .get_due_reviews(&state.progress.performance_tracker)
        .len();
    let review_style = if review_selected {
        Style::default()
            .bg(super::SELECTION_BG_COLOR)
            .fg(Color::White)
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
                format!(" [{due_count}]")
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
            .bg(super::SELECTION_BG_COLOR)
            .fg(Color::White)
    } else {
        Style::default().fg(Color::Cyan)
    };
    let profile_prefix = if profile_selected { "> " } else { "  " };
    menu_items
        .push(ListItem::new(format!("{profile_prefix}View Profile (p)")).style(profile_style));

    // Add Statistics option
    let stats_index = scenario_count + 2;
    let stats_selected = stats_index == selected_item;
    let stats_style = if stats_selected {
        Style::default()
            .bg(super::SELECTION_BG_COLOR)
            .fg(Color::White)
    } else {
        Style::default().fg(Color::Cyan)
    };
    let stats_prefix = if stats_selected { "> " } else { "  " };
    menu_items.push(ListItem::new(format!("{stats_prefix}Statistics (s)")).style(stats_style));

    // Add Quit option at the end
    let quit_index = scenario_count + 3;
    let quit_selected = quit_index == selected_item;
    let quit_style = if quit_selected {
        Style::default()
            .bg(super::SELECTION_BG_COLOR)
            .fg(Color::White)
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
            (scroll_offset * scrollbar_height) / total_items.saturating_sub(visible_height).max(1)
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
            let y = area.y + 1 + u16::try_from(y).unwrap_or(u16::MAX); // +1 for top border

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

/// Render the main menu screen as a background (for overlay popups)
///
/// This renders a simplified version of the menu without selection highlighting,
/// used when `CategoryFilters` or other popups need to show the menu behind them.
pub(super) fn render_main_menu_background(frame: &mut Frame, state: &AppState) {
    let area = frame.area();

    // Create layout: header | title | menu | quests | instructions
    let chunks = menu_chunks(area);

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
    let scenario_count = state.game.scenario_collection.count();
    // display_total is the user-facing item count (title, instructions threshold);
    // row_total is the rendered row count (includes the separator) for scroll/scrollbar math.
    let display_total = scenario_count + FIXED_MENU_ITEMS;
    let row_total = scenario_count + RENDERED_FIXED_ROWS;

    // Use default scroll offset (0) for background render - no selection
    let scroll_offset = 0;

    // Build menu items with no selection (usize::MAX ensures nothing matches)
    let all_items = build_menu_items(state, usize::MAX);

    // Apply scroll offset by skipping items
    let visible_items: Vec<ListItem> = all_items
        .into_iter()
        .skip(scroll_offset)
        .take(menu_height)
        .collect();

    // Add scroll indicator to title if list is scrollable
    let menu_title = if row_total > menu_height {
        let first_visible = scroll_offset + 1;
        let last_visible = (scroll_offset + menu_height).min(display_total);
        t!(
            "menu.main_menu_with_scroll",
            first = first_visible,
            last = last_visible,
            total = display_total
        )
        .to_string()
    } else {
        t!("menu.main_menu_total", total = display_total).to_string()
    };

    let menu = List::new(visible_items)
        .block(Block::default().title(menu_title).borders(Borders::ALL))
        .style(Style::default().fg(Color::White));
    frame.render_widget(menu, chunks[2]);

    // Render quest panel
    render_quest_panel(frame, chunks[3], state);

    // Draw scrollbar if needed
    if row_total > menu_height {
        render_scrollbar(frame, chunks[2], scroll_offset, row_total, menu_height);
    }

    // Instructions
    let instructions = if display_total > 9 {
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

/// Render the main menu screen
pub(super) fn render_main_menu(frame: &mut Frame, state: &AppState) {
    let TypedScreen::Menu(menu_data) = &state.screen else {
        return; // Wrong screen type
    };

    let area = frame.area();

    // Create layout: header | title | menu | quests | instructions
    let chunks = menu_chunks(area);

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
    let scenario_count = state.game.scenario_collection.count();
    // display_total is the user-facing item count (title, instructions threshold);
    // row_total is the rendered row count (includes the separator) for scroll/scrollbar math.
    let display_total = scenario_count + FIXED_MENU_ITEMS;
    let row_total = scenario_count + RENDERED_FIXED_ROWS;

    // Read-only defensive clamp: `AppState::normalize_view` already clamped against the
    // tracked viewport, but the frame may be a different size (e.g. before the first resize
    // event) and a render must never index out of the list. The row space is the rendered
    // one (see `selection_to_row`), matching `row_total` and the skip() below.
    let (selected_item, scroll_offset) = clamp_menu_view(
        menu_data.selected_item,
        menu_data.scroll_offset,
        scenario_count,
        menu_height,
    );

    // Build menu items
    let all_items = build_menu_items(state, selected_item);

    // Apply scroll offset by skipping items
    let visible_items: Vec<ListItem> = all_items
        .into_iter()
        .skip(scroll_offset)
        .take(menu_height)
        .collect();

    // Add scroll indicator to title if list is scrollable
    let menu_title = if row_total > menu_height {
        let first_visible = scroll_offset + 1;
        let last_visible = (scroll_offset + menu_height).min(display_total);
        t!(
            "menu.main_menu_with_scroll",
            first = first_visible,
            last = last_visible,
            total = display_total
        )
        .to_string()
    } else {
        t!("menu.main_menu_total", total = display_total).to_string()
    };

    let menu = List::new(visible_items)
        .block(Block::default().title(menu_title).borders(Borders::ALL))
        .style(Style::default().fg(Color::White));
    frame.render_widget(menu, chunks[2]);

    // Render quest panel
    render_quest_panel(frame, chunks[3], state);

    // Draw scrollbar if needed
    if row_total > menu_height {
        render_scrollbar(frame, chunks[2], scroll_offset, row_total, menu_height);
    }

    // Instructions
    let instructions = if display_total > 9 {
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
    let profile = &state.progress.profile;

    let progress = profile.level_progress();

    let header_text = format!(
        "Level {} ⭐  🔥 {} days   XP: {}/{} ({}%)",
        profile.level,
        profile.current_streak,
        progress.earned(),
        progress.span(),
        progress.percent()
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
    let profile = &state.progress.profile;
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
                icon,
                text::quest_description(quest),
                progress_str,
                quest.xp_reward
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

/// Format quest progress string using `ProgressTracker` trait
fn format_quest_progress(quest_type: &crate::gamification::QuestType) -> String {
    use crate::gamification::QuestType;
    use crate::learning::ProgressTracker;

    // SpeedRun has no incremental progress display
    if matches!(quest_type, QuestType::SpeedRun { .. }) {
        return String::new();
    }

    // Use ProgressTracker trait for consistent progress formatting
    format!(" ({}/{})", quest_type.current(), quest_type.target())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Regression test for #311: end-to-end through the real update()/render_main_menu()
    // pipeline, not just the scroll-math helpers directly.
    mod quit_reachable_end_to_end {
        use super::*;
        use crate::testing::update_executing_effects as update;
        use crate::testing::{ScenarioBuilder, test_app_state_with_scenarios};
        use crate::ui::state::{MenuData, Message, TypedScreen};
        use ratatui::Terminal;
        use ratatui::backend::TestBackend;

        #[test]
        fn test_quit_visible_after_scrolling_down_through_long_menu() {
            let scenarios: Vec<_> = (0..30)
                .map(|i| {
                    ScenarioBuilder::new()
                        .id(format!("scenario_{i}"))
                        .setup_content("line 1\n")
                        .target_content("line 2\n")
                        .optimal_count(1)
                        .build()
                })
                .collect();
            let scenario_count = scenarios.len();
            let mut state = test_app_state_with_scenarios(scenarios);
            state.screen = TypedScreen::Menu(MenuData::default());

            let backend = TestBackend::new(80, 24);
            let mut terminal = Terminal::new(backend).unwrap();

            let mut last_text = String::new();
            for _ in 0..(scenario_count + FIXED_MENU_ITEMS) {
                update(&mut state, Message::MenuDown).unwrap();
                terminal.draw(|f| render_main_menu(f, &state)).unwrap();
                let buffer = terminal.backend().buffer().clone();
                last_text = buffer
                    .content
                    .iter()
                    .map(ratatui::buffer::Cell::symbol)
                    .collect();
            }

            let TypedScreen::Menu(menu_data) = &state.screen else {
                panic!("expected TypedScreen::Menu");
            };
            assert_eq!(
                menu_data.selected_item,
                scenario_count + FIXED_MENU_ITEMS - 1
            );
            assert!(
                last_text.contains("Quit"),
                "Quit not visible after scrolling to the end of a {scenario_count}-scenario menu: {last_text}"
            );
        }
    }
}
