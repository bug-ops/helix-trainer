//! Mode selection screen rendering

use crate::ui::state::{AppState, TypedScreen};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, Paragraph},
};

/// Render the mode selection screen
pub(super) fn render_mode_selection(frame: &mut Frame, state: &AppState) {
    // Extract ModeSelectionData from TypedScreen::ModeSelection
    let TypedScreen::ModeSelection(mode_data) = &state.screen else {
        return; // Wrong screen type
    };

    let area = frame.area();

    // Create layout: title | modes | instructions
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Length(5), // Title with welcome message
            Constraint::Min(8),    // Mode selection items
            Constraint::Length(3), // Instructions
        ])
        .split(area);

    // Title with welcome message
    let title_text = [
        "HELIX TRAINER".to_string(),
        String::new(),
        "Choose your training mode:".to_string(),
    ]
    .join("\n");

    let title = Paragraph::new(title_text)
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(title, chunks[0]);

    // Build mode selection items
    let modes = [
        (
            "Training Mode",
            "Manual scenario selection with detailed feedback",
        ),
        (
            "Arcade Mode",
            "Fast-paced mini-games with progressive difficulty",
        ),
    ];

    let mode_items: Vec<ListItem> = modes
        .iter()
        .enumerate()
        .map(|(i, (name, description))| {
            let selected = i == mode_data.selected_mode;
            let style = if selected {
                Style::default()
                    .bg(Color::Blue)
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            let prefix = if selected { " > " } else { "   " };
            let icon = if i == 0 { "📚" } else { "🎮" };

            let text = format!("{}{} {}\n      {}", prefix, icon, name, description);
            ListItem::new(text).style(style)
        })
        .collect();

    let mode_list = List::new(mode_items)
        .block(
            Block::default()
                .title(" Select Mode ")
                .borders(Borders::ALL),
        )
        .style(Style::default().fg(Color::White));
    frame.render_widget(mode_list, chunks[1]);

    // Instructions
    let instructions = Paragraph::new("↑/↓ or j/k: Navigate  |  Enter: Select  |  q: Quit")
        .style(Style::default().fg(Color::Gray))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(instructions, chunks[2]);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gamification::{ProfileStorage, UserProfile};
    use crate::learning::PerformanceTracker;
    use crate::ui::state::{ModeSelectionData, TypedScreen};
    use ratatui::{Terminal, backend::TestBackend};

    #[test]
    fn test_render_mode_selection_no_panic() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut state = AppState::new(
            vec![],
            UserProfile::new(),
            ProfileStorage::new(),
            PerformanceTracker::new(),
        );
        state.screen = TypedScreen::ModeSelection(ModeSelectionData::default());

        let result = terminal.draw(|f| render_mode_selection(f, &state));
        assert!(result.is_ok());
    }

    #[test]
    fn test_render_mode_selection_wrong_screen() {
        use crate::ui::state::MenuData;

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut state = AppState::new(
            vec![],
            UserProfile::new(),
            ProfileStorage::new(),
            PerformanceTracker::new(),
        );
        state.screen = TypedScreen::Menu(MenuData::default());

        // Should not panic when called with wrong screen
        let result = terminal.draw(|f| render_mode_selection(f, &state));
        assert!(result.is_ok());
    }
}
