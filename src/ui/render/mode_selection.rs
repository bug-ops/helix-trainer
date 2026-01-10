//! Mode selection screen rendering

use crate::ui::state::{AppState, MiniGameModeSelection, TypedScreen};
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
        mode_data.selected_mode == 1 && mode_data.minigame_mode_selection.is_none(),
        "🎮",
    );

    // Render mini-game mode submenu if active
    if let Some(ref selection) = mode_data.minigame_mode_selection {
        render_minigame_mode_submenu(frame, inner, selection);
    }

    // Instructions - different when submenu is open
    let instructions_text = if mode_data.minigame_mode_selection.is_some() {
        "↑/↓ j/k: Navigate  |  Enter: Start  |  Esc: Back  |  q: Quit"
    } else {
        "↑/↓ j/k: Navigate  |  Enter: Select  |  r: Review  |  p: Profile  |  s: Stats  |  q: Quit"
    };
    let instructions = Paragraph::new(instructions_text)
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

/// Render the mini-game mode submenu (Arcade/Survival/Challenge)
fn render_minigame_mode_submenu(
    frame: &mut Frame,
    parent_area: Rect,
    selection: &MiniGameModeSelection,
) {
    use crate::minigame::{ArcadeConfig, ChallengeConfig, MiniGameMode, SurvivalConfig};

    // Position submenu to the right of "Arcade Mode" option
    let submenu_width = 45;
    let submenu_height = 11;
    let submenu_x = parent_area.x + 20;
    let submenu_y = parent_area.y + 3;

    let submenu_area = Rect::new(
        submenu_x.min(parent_area.right().saturating_sub(submenu_width)),
        submenu_y,
        submenu_width.min(parent_area.width),
        submenu_height.min(parent_area.height.saturating_sub(3)),
    );

    // Clear background
    let clear = Block::default().style(Style::default().bg(Color::Black));
    frame.render_widget(clear, submenu_area);

    // Submenu border
    let block = Block::default()
        .title(" Select Game Mode ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));
    let inner = block.inner(submenu_area);
    frame.render_widget(block, submenu_area);

    // Mode options with descriptions
    let modes: [(MiniGameMode, &str); 3] = [
        (MiniGameMode::Arcade(ArcadeConfig::default()), "🎮"),
        (MiniGameMode::Survival(SurvivalConfig::default()), "💀"),
        (MiniGameMode::Challenge(ChallengeConfig::for_today()), "📅"),
    ];

    for (idx, (mode, icon)) in modes.iter().enumerate() {
        let is_selected = idx == selection.selected_index;
        let y_offset = (idx * 3) as u16;

        if inner.y + y_offset >= inner.bottom() {
            break;
        }

        let (prefix, name_style) = if is_selected {
            (
                " > ",
                Style::default()
                    .bg(super::SELECTION_BG_COLOR)
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            ("   ", Style::default().fg(Color::White))
        };

        // Mode name line
        let name_line = Line::from(vec![
            Span::styled(prefix, name_style),
            Span::styled(format!("{} {}", icon, mode.name()), name_style),
        ]);

        // Description line
        let desc_line = Line::from(vec![
            Span::raw("      "),
            Span::styled(mode.description(), Style::default().fg(Color::DarkGray)),
        ]);

        let paragraph = Paragraph::new(vec![name_line, desc_line]);
        let item_area = Rect::new(inner.x, inner.y + y_offset, inner.width, 2);
        frame.render_widget(paragraph, item_area);
    }
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
