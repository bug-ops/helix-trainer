//! Mode selection screen rendering

use crate::ui::state::{AppState, TypedScreen};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
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

    // Render mode selection items using Paragraph for reliable display
    let block = Block::default()
        .title(" Select Mode ")
        .borders(Borders::ALL);
    let inner = block.inner(chunks[1]);
    frame.render_widget(block, chunks[1]);

    // Render each mode option with icons
    render_mode_option(
        frame,
        Rect::new(inner.x, inner.y + 1, inner.width, 2),
        "Training Mode",
        "Manual scenario selection with detailed feedback",
        mode_data.selected_mode == 0,
        "📚",
    );

    render_mode_option(
        frame,
        Rect::new(inner.x, inner.y + 4, inner.width, 2),
        "Arcade Mode",
        "Fast-paced mini-games with time pressure",
        mode_data.selected_mode == 1,
        "🎮",
    );

    // Instructions
    let instructions = Paragraph::new("↑/↓ or j/k: Navigate  |  Enter: Select  |  q: Quit")
        .style(Style::default().fg(Color::Gray))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(instructions, chunks[2]);
}

/// Render a single mode option with icon
fn render_mode_option(
    frame: &mut Frame,
    area: Rect,
    name: &str,
    description: &str,
    selected: bool,
    icon: &str,
) {
    let (prefix, style) = if selected {
        (
            " > ",
            Style::default()
                .bg(super::SELECTION_BG_COLOR)
                .fg(Color::White),
        )
    } else {
        ("   ", Style::default().fg(Color::White))
    };

    let lines = vec![
        Line::from(vec![
            Span::styled(prefix, style),
            Span::styled(format!("{} {}", icon, name), style),
        ]),
        Line::from(vec![
            Span::raw("      "),
            Span::styled(description, Style::default().fg(Color::Gray)),
        ]),
    ];

    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, area);
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
