//! Main menu rendering

use crate::ui::state::{AppState, TypedScreen};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, Paragraph},
};
use rust_i18n::t;

/// Number of fixed, selectable menu entries after the scenario list: Review, Profile, Statistics, Quit.
///
/// Mirrors `handlers::menu::FIXED_MENU_ITEMS`. Duplicated here because `ui::render` and
/// `ui::state::handlers` are separate module trees (handlers stays private to `ui::state`);
/// keep the two literals in sync.
const FIXED_MENU_ITEMS: usize = 4;

/// Number of rendered rows appended after the scenario list: the `FIXED_MENU_ITEMS`
/// selectable entries plus the non-selectable separator row `build_menu_items` inserts
/// before them. Used for scroll-offset/scrollbar math, which operates on rendered rows —
/// NOT for user-facing item counts (see `FIXED_MENU_ITEMS` for those).
const RENDERED_FIXED_ROWS: usize = FIXED_MENU_ITEMS + 1;

/// Converts a logical selection index into its rendered-row index.
///
/// `selected_item` is a scenario index (`0..scenario_count`) or one of the fixed entries
/// (`scenario_count..scenario_count+FIXED_MENU_ITEMS`). `build_menu_items` inserts a
/// separator row between the two groups, so any fixed-entry selection sits one row further
/// down than its logical index suggests.
fn selection_to_row(selected_item: usize, scenario_count: usize) -> usize {
    selected_item + usize::from(selected_item >= scenario_count)
}

/// Calculate menu scroll offset to keep selected item visible
///
/// Both `selected` and `total_items` must be in the same coordinate space as the rendered
/// row list (see `selection_to_row`) — this function has no way to detect a space mismatch.
///
/// Returns (scroll_offset, visible_count) tuple
fn calculate_menu_scroll(
    selected: usize,
    current_offset: usize,
    visible_height: usize,
    total_items: usize,
) -> (usize, usize) {
    // No rows are visible (e.g. a very short terminal collapses the menu area to 0 height) -
    // `visible_height - 1` below would underflow (#321). Return the caller's offset
    // unchanged rather than resetting to 0: nothing is visible to scroll, so there is
    // nothing to correct, and resetting would discard the caller's scroll position for
    // this transient 0-height frame.
    if visible_height == 0 {
        return (current_offset, 0);
    }

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
            .bg(super::SELECTION_BG_COLOR)
            .fg(Color::White)
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
            .bg(super::SELECTION_BG_COLOR)
            .fg(Color::White)
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

/// Render the main menu screen as a background (for overlay popups)
///
/// This renders a simplified version of the menu without selection highlighting,
/// used when CategoryFilters or other popups need to show the menu behind them.
pub(super) fn render_main_menu_background(frame: &mut Frame, state: &AppState) {
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
    let scenario_count = state.game.scenario_collection.count();
    // display_total is the user-facing item count (title, instructions threshold);
    // row_total is the rendered row count (includes the separator) for scroll/scrollbar math.
    let display_total = scenario_count + FIXED_MENU_ITEMS;
    let row_total = scenario_count + RENDERED_FIXED_ROWS;

    // Get mutable access to menu_data for adjusting scroll
    let TypedScreen::Menu(menu_data) = &mut state.screen else {
        unreachable!("Already checked above")
    };

    // A filter change may have shrunk the scenario list since selected_item was last set
    // (e.g. toggling a category filter while a later item was selected); clamp here, the
    // single point every path that can select a menu item funnels through before display.
    menu_data.selected_item = menu_data.selected_item.min(display_total.saturating_sub(1));

    // Calculate scroll offset (in rendered-row space, matching row_total and the skip()
    // below — build_menu_items' separator row shifts fixed entries one row past their
    // logical index, see selection_to_row)
    let selected_row = selection_to_row(menu_data.selected_item, scenario_count);
    let (scroll_offset, _) = calculate_menu_scroll(
        selected_row,
        menu_data.scroll_offset,
        menu_height,
        row_total,
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
    use crate::gamification::XPCalculator;

    let profile = &state.progress.profile;

    // Calculate XP progress within current level
    let current_level_xp = XPCalculator::xp_for_level(profile.level);
    let next_level_xp = XPCalculator::xp_for_level(profile.level + 1);
    let xp_in_level = profile.total_xp.saturating_sub(current_level_xp);
    let xp_needed = next_level_xp.saturating_sub(current_level_xp);
    let progress_pct = (profile.xp_progress() * 100.0) as u8;

    let header_text = format!(
        "Level {} ⭐  🔥 {} days   XP: {}/{} ({}%)",
        profile.level, profile.current_streak, xp_in_level, xp_needed, progress_pct
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

/// Format quest progress string using ProgressTracker trait
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

        #[test]
        fn test_calculate_menu_scroll_zero_visible_height_does_not_underflow() {
            // Regression test for #321: at very small terminal heights the menu layout can
            // yield a 0-height visible area, and `visible_height - 1` would underflow.
            // Nothing is visible, so the caller's offset must be preserved rather than reset.
            let (offset, visible) = calculate_menu_scroll(5, 3, 0, 20);
            assert_eq!(offset, 3);
            assert_eq!(visible, 0);
        }
    }

    // Regression tests for #311: Quit must be scrollable into view.
    //
    // `calculate_menu_scroll` operates purely on whatever index space it's given, so a test
    // that only checks its output in isolation (as `calculate_menu_scroll_tests` above does)
    // cannot catch a caller passing it the wrong space. These tests instead combine
    // `selection_to_row` with `calculate_menu_scroll` exactly as `render_main_menu` does, and
    // assert on the property that actually matters: the selected item's *rendered row* must
    // fall inside the visible window after scrolling. A caller that skips `selection_to_row`
    // (passing the logical `scenario_count + FIXED_MENU_ITEMS - 1` index straight through, as
    // the original code and the first, inert `RENDERED_FIXED_ROWS`-only fix both did) computes
    // an offset that is exactly one row short of this for every case below.
    mod quit_reachability_tests {
        use super::*;

        fn assert_quit_row_visible(scenario_count: usize, visible_height: usize) {
            let quit_selected_item = scenario_count + FIXED_MENU_ITEMS - 1;
            let row_total = scenario_count + RENDERED_FIXED_ROWS;

            // Ground truth, independent of `selection_to_row` (the function under test):
            // `build_menu_items` always lays out scenario rows, then 1 separator row, then
            // the 4 fixed entries, so Quit's physical row is always `row_total - 1`. Deriving
            // this from `selection_to_row` instead would make the assertion vacuous against a
            // broken (e.g. identity) `selection_to_row`, since `calculate_menu_scroll` always
            // makes whatever row it's given visible.
            let true_quit_row = row_total - 1;

            let scroll_input_row = selection_to_row(quit_selected_item, scenario_count);
            let (offset, _) = calculate_menu_scroll(scroll_input_row, 0, visible_height, row_total);

            assert!(
                offset <= true_quit_row && true_quit_row < offset + visible_height,
                "Quit's true rendered row {true_quit_row} not inside visible window \
                 [{offset}, {}) for scenario_count={scenario_count}, visible_height={visible_height} \
                 (selection_to_row produced {scroll_input_row})",
                offset + visible_height
            );
        }

        #[test]
        fn test_quit_reachable_critic_counterexample_10_5() {
            assert_quit_row_visible(10, 5);
        }

        #[test]
        fn test_quit_reachable_critic_counterexample_20_10() {
            assert_quit_row_visible(20, 10);
        }

        #[test]
        fn test_quit_reachable_critic_counterexample_50_12() {
            assert_quit_row_visible(50, 12);
        }

        #[test]
        fn test_quit_reachable_large_scenario_count() {
            assert_quit_row_visible(200, 20);
        }

        #[test]
        fn test_quit_reachable_visible_height_of_one() {
            // Degenerate but valid: a single visible row must still be able to show Quit.
            assert_quit_row_visible(10, 1);
        }
    }

    // Regression test for #311: end-to-end through the real update()/render_main_menu()
    // pipeline, not just the scroll-math helpers directly.
    mod quit_reachable_end_to_end {
        use super::*;
        use crate::testing::{ScenarioBuilder, test_app_state_with_scenarios};
        use crate::ui::state::{MenuData, Message, TypedScreen, update};
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
                terminal.draw(|f| render_main_menu(f, &mut state)).unwrap();
                let buffer = terminal.backend().buffer().clone();
                last_text = buffer.content.iter().map(|c| c.symbol()).collect();
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
