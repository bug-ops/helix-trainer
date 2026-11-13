//! Mode selection screen rendering

use crate::ui::state::{AppState, MiniGameModeKind, MiniGameModeSelection, TypedScreen};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};
use rust_i18n::t;

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
        t!("mode_selection.logo").to_string(),
        String::new(),
        t!("mode_selection.prompt").to_string(),
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
        .title(t!("mode_selection.select_mode_title").to_string())
        .borders(Borders::ALL);
    let inner = block.inner(chunks[1]);
    frame.render_widget(block, chunks[1]);

    // Render each mode option with icons
    render_mode_option(
        frame,
        Rect::new(inner.x, inner.y + 1, inner.width, 2),
        &t!("mode_selection.training_name"),
        &t!("mode_selection.training_description"),
        mode_data.selected_mode == crate::ui::state::TopMode::Training,
        "📚",
    );

    render_mode_option(
        frame,
        Rect::new(inner.x, inner.y + 4, inner.width, 2),
        &t!("mode_selection.arcade_name"),
        &t!("mode_selection.arcade_description"),
        mode_data.selected_mode == crate::ui::state::TopMode::Arcade
            && mode_data.minigame_mode_selection.is_none(),
        "🎮",
    );

    // Render mini-game mode submenu if active
    if let Some(ref selection) = mode_data.minigame_mode_selection {
        render_minigame_mode_submenu(
            frame,
            inner,
            selection,
            state.progress.today(),
            state.progress.profile.challenge_progress(),
        );
    }

    // Instructions - different when submenu is open
    let instructions_text = if mode_data.minigame_mode_selection.is_some() {
        t!("mode_selection.instructions_submenu")
    } else {
        t!("mode_selection.instructions")
    };
    let instructions = Paragraph::new(instructions_text.to_string())
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
            Span::styled(format!("{icon} {name}"), style),
        ]),
        Line::from(vec![
            Span::raw("      "),
            Span::styled(description, Style::default().fg(Color::Gray)),
        ]),
    ];

    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, area);
}

pub(super) fn mode_label(mode: &crate::minigame::MiniGameMode) -> String {
    use crate::minigame::MiniGameMode;
    match mode {
        MiniGameMode::Arcade(_) => t!("mode_selection.mode_arcade"),
        MiniGameMode::Survival(_) => t!("mode_selection.mode_survival"),
        MiniGameMode::Challenge(_) => t!("mode_selection.mode_challenge"),
    }
    .to_string()
}

/// Render the mini-game mode submenu (Arcade/Survival/Challenge)
fn render_minigame_mode_submenu(
    frame: &mut Frame,
    parent_area: Rect,
    selection: &MiniGameModeSelection,
    today: crate::time::PlayerDay,
    challenge_progress: &crate::gamification::ChallengeProgress,
) {
    const ICONS: [&str; MiniGameModeSelection::MODE_COUNT] = ["🎮", "💀", "📅"];
    // Vertical stride per entry (1 content row + 1 blank spacer row); also used
    // below to derive submenu_height, keeping the two in lockstep.
    const ENTRY_STRIDE: u16 = 2;

    // Position submenu to the right of "Arcade Mode" option
    let submenu_width = 45;
    // Exact inner height needed for MODE_COUNT entries at ENTRY_STRIDE spacing,
    // plus 2 for the top/bottom border. Derived from MODE_COUNT so an added mode
    // grows the submenu instead of silently clipping (see #385).
    let submenu_height = (u16::try_from(MiniGameModeSelection::MODE_COUNT).unwrap_or(u16::MAX) - 1)
        * ENTRY_STRIDE
        + 1
        + 2;
    let submenu_x = parent_area.x + 20;
    let submenu_y = parent_area.y + 3;

    let submenu_area = Rect::new(
        submenu_x.min(parent_area.right().saturating_sub(submenu_width)),
        submenu_y,
        submenu_width.min(parent_area.width),
        submenu_height.min(parent_area.height.saturating_sub(3)),
    );

    // Clear whatever the parent screen already painted here — Block styling
    // alone only restyles cells and leaves stale symbols behind (#385 S1).
    frame.render_widget(Clear, submenu_area);
    let clear = Block::default().style(Style::default().bg(Color::Black));
    frame.render_widget(clear, submenu_area);

    // Submenu border
    let block = Block::default()
        .title(t!("mode_selection.select_game_mode_title").to_string())
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));
    let inner = block.inner(submenu_area);
    frame.render_widget(block, submenu_area);

    // Mode options with descriptions. Modes come from `MiniGameModeSelection::all_modes`
    // (the same single source of truth `selected_mode`/`mode_name`/`mode_description`
    // use) so this menu can't silently omit or misdescribe a mode; icons are a
    // render-only concern paired on by index.
    let modes = MiniGameModeSelection::all_modes(today);

    for (idx, ((mode, kind), icon)) in modes
        .iter()
        .zip(MiniGameModeKind::ALL)
        .zip(ICONS.iter())
        .enumerate()
    {
        let is_selected = kind == selection.selected;
        let y_offset = u16::try_from(idx).unwrap_or(u16::MAX) * ENTRY_STRIDE;

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

        // Mode name line — for Challenge mode, append today's remaining
        // attempts (FR-006) so the cap is visible before the player is
        // blocked. Entries are single-line (no description) so all three
        // modes fit within the minimum supported terminal height (80x24).
        let name_text = if mode.is_challenge() {
            format!(
                "{} {} ({}/{})",
                icon,
                mode_label(mode),
                challenge_progress.attempts_remaining(today),
                crate::constants::CHALLENGE_MAX_ATTEMPTS
            )
        } else {
            format!("{} {}", icon, mode_label(mode))
        };
        let name_line = Line::from(vec![
            Span::styled(prefix, name_style),
            Span::styled(name_text, name_style),
        ]);

        let paragraph = Paragraph::new(name_line);
        let item_area = Rect::new(inner.x, inner.y + y_offset, inner.width, 1);
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
            ProfileStorage::for_test(),
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
            ProfileStorage::for_test(),
            PerformanceTracker::new(),
        );
        state.screen = TypedScreen::Menu(MenuData::default());

        // Should not panic when called with wrong screen
        let result = terminal.draw(|f| render_mode_selection(f, &state));
        assert!(result.is_ok());
    }
}
