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

    // Create layout: title | menu | instructions
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Length(3), // Title
            Constraint::Min(4),    // Menu items
            Constraint::Length(3), // Instructions
        ])
        .split(area);

    // Title
    let title = Paragraph::new(t!("menu.title").to_string())
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(title, chunks[0]);

    // Calculate visible area height for menu (excluding borders)
    let menu_height = chunks[1].height.saturating_sub(2) as usize; // -2 for borders
    let total_items = state.scenarios.len() + 1; // +1 for Quit option

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

    // Menu items - show all scenarios + Quit option
    let mut menu_items: Vec<ListItem> = state
        .scenarios
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

            let prefix = if selected { "> " } else { "  " };
            let display = format!("{}. {}", i + 1, scenario.name);
            ListItem::new(format!("{}{}", prefix, display)).style(style)
        })
        .collect();

    // Add Quit option at the end
    let quit_index = state.scenarios.len();
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
    frame.render_widget(menu, chunks[1]);

    // Draw scrollbar if needed
    if total_items > menu_height {
        let scrollbar_area = chunks[1];
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
    frame.render_widget(instructions, chunks[2]);
}
