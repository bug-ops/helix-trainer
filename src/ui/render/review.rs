//! Review session screen rendering

use crate::helix::commands::CMD_ESCAPE;
use crate::ui::state::{AppState, ReviewSessionState};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, Paragraph, Wrap},
};

/// Get a human-readable description for a Helix command
fn get_command_description(command: &str) -> &'static str {
    match command {
        // Basic movement
        "h" => "Move cursor left",
        "j" => "Move cursor down",
        "k" => "Move cursor up",
        "l" => "Move cursor right",

        // Word movement
        "w" => "Move to next word start",
        "b" => "Move to previous word start",
        "e" => "Move to word end",
        "W" => "Move to next WORD start",
        "B" => "Move to previous WORD start",
        "E" => "Move to WORD end",

        // Line movement
        "0" => "Move to line start",
        "$" => "Move to line end",
        "^" => "Move to first non-blank character",
        "gg" => "Go to first line",
        "ge" => "Go to last line",
        "gh" => "Go to line start",
        "gl" => "Go to line end",
        "gs" => "Go to first non-whitespace",

        // Editing
        "d" => "Delete selection",
        "c" => "Change selection (delete and enter insert mode)",
        "i" => "Enter insert mode before cursor",
        "a" => "Enter insert mode after cursor",
        "I" => "Insert at line start",
        "A" => "Append at line end",
        "o" => "Open new line below",
        "O" => "Open new line above",
        "J" => "Join lines",
        "r" => "Replace character under cursor",
        "~" => "Switch case of selection",

        // Selection
        "x" => "Select entire line",
        "X" => "Extend line selection",
        "%" => "Select entire buffer",
        ";" => "Collapse selection to cursor",
        "v" => "Enter select mode",

        // Clipboard
        "y" => "Yank (copy) selection",
        "p" => "Paste after selection",
        "P" => "Paste before selection",
        "R" => "Replace selection with yanked text",

        // Undo/Redo
        "u" => "Undo last change",
        "U" => "Redo last undone change",

        // Search
        "/" => "Search forward",
        "?" => "Search backward",
        "n" => "Go to next search match",
        "N" => "Go to previous search match",
        "*" => "Search for word under cursor",

        // Find character
        "f" => "Find character forward",
        "F" => "Find character backward",
        "t" => "Till character forward",
        "T" => "Till character backward",

        // Indentation
        ">" => "Indent selection",
        "<" => "Dedent selection",

        // Match mode
        "m" => "Enter match mode",
        "mm" => "Jump to matching bracket",

        // View mode
        "z" => "Enter view mode",
        "zz" => "Center view on cursor",
        "zt" => "Align view top",
        "zb" => "Align view bottom",

        // Multi-cursor
        "," => "Keep only primary selection",
        "C" => "Copy selection to next line",

        // Comments
        "Ctrl-c" => "Toggle line comments",

        // Page movement
        "Ctrl-f" => "Page down",
        "Ctrl-b" => "Page up",
        "Ctrl-d" => "Half page down",
        "Ctrl-u" => "Half page up",

        // Other
        "." => "Repeat last command",
        "Escape" => "Return to normal mode",

        // Default fallback
        _ => "Helix command",
    }
}

/// Render the review session screen
pub(super) fn render_review_screen(frame: &mut Frame, state: &AppState) {
    let Some(session) = &state.game.review_session else {
        return;
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header with progress
            Constraint::Min(0),    // Content
            Constraint::Length(3), // Footer with controls
        ])
        .split(frame.area());

    // Header with progress
    render_header(frame, chunks[0], session);

    // Main content area
    render_review_content(frame, chunks[1], state, session);

    // Footer with controls
    render_footer(frame, chunks[2]);
}

/// Render the header with progress gauge
fn render_header(frame: &mut Frame, area: Rect, session: &ReviewSessionState) {
    let progress = (session.current_index + 1) as f64 / session.due_commands.len() as f64;
    let progress_text = format!(
        "Review Session - {}/{}",
        session.current_index + 1,
        session.due_commands.len()
    );

    let gauge = Gauge::default()
        .block(Block::default().borders(Borders::ALL))
        .gauge_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .label(progress_text)
        .ratio(progress);

    frame.render_widget(gauge, area);
}

/// Render the main review content area
fn render_review_content(
    frame: &mut Frame,
    area: Rect,
    state: &AppState,
    session: &ReviewSessionState,
) {
    let Some(command) = &session.current_command else {
        return;
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(6), // Command info
            Constraint::Min(10),   // Practice area
            Constraint::Length(3), // Instructions
        ])
        .margin(2)
        .split(area);

    // Command info panel
    render_command_info(frame, chunks[0], state, command);

    // Practice area
    render_practice_area(frame, chunks[1]);

    // Instructions
    render_instructions(frame, chunks[2]);
}

/// Render command information panel
fn render_command_info(frame: &mut Frame, area: Rect, state: &AppState, command: &str) {
    let tracker = &state.progress.performance_tracker;
    let perf = tracker.get_performance(command);

    // Get command description
    let description = get_command_description(command);

    let mastery_text = if let Some(p) = perf {
        format!("Mastery: {:?} ⭐", p.mastery_level)
    } else {
        "Mastery: New".to_string()
    };

    let next_review = if let Some(p) = perf {
        let now = chrono::Utc::now();
        let days = (p.due - now).num_days();
        if days > 0 {
            format!("Next: in {} days", days)
        } else {
            "Due: Now".to_string()
        }
    } else {
        "First time".to_string()
    };

    let success_rate = if let Some(p) = perf {
        format!("Success rate: {:.0}%", p.success_rate() * 100.0)
    } else {
        "Success rate: N/A".to_string()
    };

    let text = vec![
        Line::from(vec![
            Span::styled("Command: ", Style::default().fg(Color::Gray)),
            Span::styled(
                command,
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![Span::styled(
            description,
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::ITALIC),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled(&mastery_text, Style::default().fg(Color::Yellow)),
            Span::raw("  "),
            Span::styled(&next_review, Style::default().fg(Color::Gray)),
        ]),
        Line::from(Span::styled(
            &success_rate,
            Style::default().fg(Color::Gray),
        )),
    ];

    let paragraph = Paragraph::new(text)
        .block(Block::default().borders(Borders::ALL).title("Command Info"))
        .wrap(Wrap { trim: true });

    frame.render_widget(paragraph, area);
}

/// Render practice area (MVP: simple instructions)
fn render_practice_area(frame: &mut Frame, area: Rect) {
    let text = vec![
        Line::from(""),
        Line::from("Practice the command in your mind or on paper."),
        Line::from(""),
        Line::from("Think about:"),
        Line::from("  • What does this command do?"),
        Line::from("  • When would you use it?"),
        Line::from("  • What are common use cases?"),
        Line::from(""),
        Line::from(vec![
            Span::raw("Press "),
            Span::styled(
                "'s'",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" if you remember it correctly"),
        ]),
        Line::from(vec![
            Span::raw("Press "),
            Span::styled(
                "'f'",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            ),
            Span::raw(" if you couldn't remember"),
        ]),
    ];

    let paragraph = Paragraph::new(text)
        .block(Block::default().borders(Borders::ALL).title("Practice"))
        .alignment(Alignment::Left);

    frame.render_widget(paragraph, area);
}

/// Render instruction panel
fn render_instructions(frame: &mut Frame, area: Rect) {
    let text = Line::from(vec![
        Span::raw("Controls: "),
        Span::styled(
            "s",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" = Success  |  "),
        Span::styled(
            "f",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        ),
        Span::raw(" = Failed  |  "),
        Span::styled(CMD_ESCAPE, Style::default().fg(Color::Gray)),
        Span::raw(" = Abandon"),
    ]);

    let paragraph = Paragraph::new(text)
        .block(Block::default().borders(Borders::ALL))
        .alignment(Alignment::Center);

    frame.render_widget(paragraph, area);
}

/// Render footer with session info
fn render_footer(frame: &mut Frame, area: Rect) {
    let text = Line::from(vec![Span::styled(
        "Review Session Active - Practice each command until you remember it",
        Style::default().fg(Color::Cyan),
    )]);

    let paragraph = Paragraph::new(text)
        .block(Block::default().borders(Borders::ALL))
        .alignment(Alignment::Center);

    frame.render_widget(paragraph, area);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_command_description_basic_movement() {
        assert_eq!(get_command_description("h"), "Move cursor left");
        assert_eq!(get_command_description("j"), "Move cursor down");
        assert_eq!(get_command_description("k"), "Move cursor up");
        assert_eq!(get_command_description("l"), "Move cursor right");
    }

    #[test]
    fn test_get_command_description_word_movement() {
        assert_eq!(get_command_description("w"), "Move to next word start");
        assert_eq!(get_command_description("b"), "Move to previous word start");
        assert_eq!(get_command_description("e"), "Move to word end");
        assert_eq!(get_command_description("W"), "Move to next WORD start");
        assert_eq!(get_command_description("B"), "Move to previous WORD start");
        assert_eq!(get_command_description("E"), "Move to WORD end");
    }

    #[test]
    fn test_get_command_description_editing() {
        assert_eq!(get_command_description("d"), "Delete selection");
        assert_eq!(
            get_command_description("c"),
            "Change selection (delete and enter insert mode)"
        );
        assert_eq!(
            get_command_description("i"),
            "Enter insert mode before cursor"
        );
        assert_eq!(
            get_command_description("a"),
            "Enter insert mode after cursor"
        );
        assert_eq!(get_command_description("o"), "Open new line below");
        assert_eq!(get_command_description("O"), "Open new line above");
    }

    #[test]
    fn test_get_command_description_selection() {
        assert_eq!(get_command_description("x"), "Select entire line");
        assert_eq!(get_command_description("X"), "Extend line selection");
        assert_eq!(get_command_description("%"), "Select entire buffer");
        assert_eq!(get_command_description(";"), "Collapse selection to cursor");
        assert_eq!(get_command_description("v"), "Enter select mode");
    }

    #[test]
    fn test_get_command_description_clipboard() {
        assert_eq!(get_command_description("y"), "Yank (copy) selection");
        assert_eq!(get_command_description("p"), "Paste after selection");
        assert_eq!(get_command_description("P"), "Paste before selection");
    }

    #[test]
    fn test_get_command_description_unknown_command_fallback() {
        assert_eq!(get_command_description("unknown_cmd"), "Helix command");
        assert_eq!(get_command_description(""), "Helix command");
        assert_eq!(get_command_description("xyz123"), "Helix command");
    }
}
