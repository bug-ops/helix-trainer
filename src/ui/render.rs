//! Pure rendering functions for the TUI
//!
//! This module contains all rendering logic for the terminal user interface.
//! All functions are pure (no side effects) and take an immutable reference
//! to the application state.

use crate::ui::state::{AppState, Screen};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
};
use tui_big_text::{BigText, PixelSize};

/// Main render function dispatches to screen-specific renderers
///
/// This is the entry point for all rendering. It's pure and has no side effects.
///
/// # Arguments
///
/// * `frame` - The ratatui frame to render to
/// * `state` - The application state (immutable)
pub fn render(frame: &mut Frame, state: &AppState) {
    match state.screen {
        Screen::MainMenu => render_main_menu(frame, state),
        Screen::Task => render_task_screen(frame, state),
        Screen::Results => render_results_screen(frame, state),
    }
}

/// Render the main menu screen
fn render_main_menu(frame: &mut Frame, state: &AppState) {
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
    let title = Paragraph::new("Helix Keybindings Trainer")
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(title, chunks[0]);

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
    menu_items.push(ListItem::new(format!("{}Quit", quit_prefix)).style(quit_style));

    let menu = List::new(menu_items)
        .block(Block::default().title("Main Menu").borders(Borders::ALL))
        .style(Style::default().fg(Color::White));
    frame.render_widget(menu, chunks[1]);

    // Instructions
    let instructions = Paragraph::new("↑/↓: Navigate | Enter: Select | q: Quit")
        .style(Style::default().fg(Color::Gray))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(instructions, chunks[2]);
}

/// Render the task screen where user plays a scenario
fn render_task_screen(frame: &mut Frame, state: &AppState) {
    let area = frame.area();

    if let Some(session) = &state.session {
        let scenario = session.scenario();

        // Layout: title | description | editor view | stats | instructions
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Length(3), // Title
                Constraint::Length(4), // Description
                Constraint::Min(8),    // Editor view
                Constraint::Length(3), // Stats
                Constraint::Length(3), // Instructions
            ])
            .split(area);

        // Title
        let title = Paragraph::new(format!("Scenario: {}", scenario.name))
            .style(
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            )
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        frame.render_widget(title, chunks[0]);

        // Description
        let description = Paragraph::new(scenario.description.as_str())
            .wrap(Wrap { trim: true })
            .style(Style::default().fg(Color::White))
            .block(Block::default().title("Task").borders(Borders::ALL));
        frame.render_widget(description, chunks[1]);

        // Editor view - split into current and target
        let editor_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(chunks[2]);

        // Current state with cursor and diff highlighting
        let current_state = session.current_state();
        let target_state = session.target_state();
        let current_lines = render_editor_with_diff(current_state, target_state);
        let current = Paragraph::new(current_lines)
            .block(
                Block::default()
                    .title("Current State (with cursor)")
                    .borders(Borders::ALL),
            )
            .wrap(Wrap { trim: false });
        frame.render_widget(current, editor_chunks[0]);

        // Target state with selection highlighting (if any)
        let target_state = session.target_state();
        let target_lines = render_editor_with_selection(target_state);
        let target = Paragraph::new(target_lines)
            .block(
                Block::default()
                    .title("Target State (goal)")
                    .borders(Borders::ALL),
            )
            .wrap(Wrap { trim: false });
        frame.render_widget(target, editor_chunks[1]);

        // Stats with mode indicator and progress
        let optimal = scenario.scoring.optimal_count;
        let actions = session.action_count();
        let elapsed = session.elapsed();
        let elapsed_secs = elapsed.as_secs_f32();
        let mode = session.mode_name();
        let progress = session.completion_progress();

        // Color code mode: green for Normal, yellow for Insert
        let mode_color = if mode == "NORMAL" {
            Color::Green
        } else {
            Color::Yellow
        };

        // Color code progress: green if 100%, yellow if >50%, red otherwise
        let progress_color = if progress == 100 {
            Color::Green
        } else if progress > 50 {
            Color::Yellow
        } else {
            Color::Red
        };

        // Create colored mode indicator
        let mode_span = Span::styled(
            format!("Mode: {} ", mode),
            Style::default().fg(mode_color).add_modifier(Modifier::BOLD),
        );

        // Create colored progress indicator
        let progress_span = Span::styled(
            format!("| Progress: {}% ", progress),
            Style::default()
                .fg(progress_color)
                .add_modifier(Modifier::BOLD),
        );

        // Create rest of stats
        let rest_of_stats = if actions <= optimal {
            format!(
                "| Actions: {} (optimal: {}) | Time: {:.1}s",
                actions, optimal, elapsed_secs
            )
        } else {
            format!(
                "| Actions: {} (optimal: {}) - {} extra | Time: {:.1}s",
                actions,
                optimal,
                actions - optimal,
                elapsed_secs
            )
        };
        let rest_span = Span::styled(rest_of_stats, Style::default().fg(Color::White));

        let stats = Paragraph::new(Line::from(vec![mode_span, progress_span, rest_span]))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        frame.render_widget(stats, chunks[3]);

        // Instructions with hint indicator and last command
        let hint_indicator = if state.show_hint_panel && state.current_hint.is_some() {
            " [h: Next Hint] "
        } else {
            " [h: Show Hint] "
        };

        let last_cmd_text = if let Some(cmd) = &state.last_command {
            format!(" Last: {} |", cmd)
        } else {
            String::new()
        };

        let instructions = Paragraph::new(format!(
            "{}{}| Esc: Abandon | Ctrl-c: Quit",
            hint_indicator, last_cmd_text
        ))
        .style(Style::default().fg(Color::Gray))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
        frame.render_widget(instructions, chunks[4]);

        // Render hint panel if visible
        if state.show_hint_panel {
            render_hint_popup(frame, state);
        }

        // Show key history popup if visible
        if state.show_key_history {
            render_key_history_popup(frame, state);
        }

        // Show success message if scenario just completed
        if state.completion_time.is_some() {
            render_success_popup(frame);
        }
    }
}

/// Render the results screen showing scenario completion
fn render_results_screen(frame: &mut Frame, state: &AppState) {
    let area = frame.area();

    if let Some(session) = &state.session
        && let Ok(feedback) = session.get_feedback()
    {
        // Layout: title | results | instructions
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(2)
            .constraints([
                Constraint::Length(3), // Title
                Constraint::Min(10),   // Results
                Constraint::Length(3), // Instructions
            ])
            .split(area);

        // Title
        let title_text = if feedback.success {
            "✓ Completed!"
        } else {
            "✗ Not Completed"
        };
        let title_color = if feedback.success {
            Color::Green
        } else {
            Color::Red
        };
        let title = Paragraph::new(title_text)
            .style(
                Style::default()
                    .fg(title_color)
                    .add_modifier(Modifier::BOLD),
            )
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        frame.render_widget(title, chunks[0]);

        // Results content
        let mut result_lines = vec![];

        // Rating and score
        result_lines.push(Line::from(vec![Span::styled(
            format!(
                "{} {}",
                feedback.rating.emoji(),
                feedback.rating.description()
            ),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )]));

        result_lines.push(Line::from(""));

        // Score
        result_lines.push(Line::from(vec![
            Span::raw("Score: "),
            Span::styled(
                format!("{}/{}", feedback.score, feedback.max_points),
                Style::default().fg(Color::Cyan),
            ),
        ]));

        // Actions
        let action_color = if feedback.is_optimal {
            Color::Green
        } else {
            Color::Yellow
        };
        result_lines.push(Line::from(vec![
            Span::raw("Actions: "),
            Span::styled(
                format!("{}", feedback.actions_taken),
                Style::default().fg(action_color),
            ),
            Span::raw(format!(" (optimal: {})", feedback.optimal_actions)),
        ]));

        // Duration
        result_lines.push(Line::from(vec![
            Span::raw("Time: "),
            Span::styled(
                format!("{:.1}s", feedback.duration.as_secs_f32()),
                Style::default().fg(Color::Blue),
            ),
        ]));

        // Hint if provided
        if let Some(hint) = &feedback.hint {
            result_lines.push(Line::from(""));
            result_lines.push(Line::from(vec![
                Span::styled("Tip: ", Style::default().fg(Color::Magenta)),
                Span::raw(hint),
            ]));
        }

        let results = Paragraph::new(result_lines)
            .block(Block::default().title("Performance").borders(Borders::ALL))
            .alignment(Alignment::Left)
            .style(Style::default().fg(Color::White));
        frame.render_widget(results, chunks[1]);

        // Instructions
        let instructions = Paragraph::new("[r] Retry  [m] Menu  [q] Quit")
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        frame.render_widget(instructions, chunks[2]);
    }
}

/// Render a centered hint popup
fn render_hint_popup(frame: &mut Frame, state: &AppState) {
    let area = frame.area();

    // Calculate popup dimensions with constraints
    let popup_width = 70.min(area.width.saturating_sub(4));
    let popup_height = 10.min(area.height.saturating_sub(4));

    // Create centered popup area
    let popup_area = centered_popup(area, popup_width, popup_height);

    // Render popup background with border
    let background = popup_block(Some("Hint"), Color::White);
    frame.render_widget(&background, popup_area);

    // Render hint text inside popup
    if let Some(hint) = &state.current_hint {
        let inner = inner_rect(popup_area);

        let hint_text = Paragraph::new(hint.as_str())
            .wrap(Wrap { trim: true })
            .style(Style::default().fg(Color::Yellow))
            .alignment(Alignment::Center);

        frame.render_widget(hint_text, inner);
    }
}

/// Render editor text with cursor and diff highlighting
///
/// Compares current state with target state and colors lines:
/// - Green: lines that match target
/// - Red: lines that differ from target
/// - Cursor shown with inverse colors
fn render_editor_with_diff<'a>(
    current: &'a crate::game::EditorState,
    target: &crate::game::EditorState,
) -> Vec<Line<'a>> {
    let current_content = current.content();
    let target_content = target.content();
    let cursor = current.cursor_position();
    let (cursor_line, cursor_col) = (cursor.row, cursor.col);

    let current_lines: Vec<&str> = current_content.lines().collect();
    let target_lines: Vec<&str> = target_content.lines().collect();

    current_lines
        .iter()
        .enumerate()
        .map(|(line_idx, &line_text)| {
            // Determine if this line matches target
            let matches_target = target_lines
                .get(line_idx)
                .map(|&target_line| target_line == line_text)
                .unwrap_or(false);

            // Choose color based on match
            let line_color = if matches_target {
                Color::Green
            } else {
                Color::Red
            };

            if line_idx == cursor_line {
                // This line contains the cursor
                let mut spans = Vec::new();

                // Split line at cursor position (zero-allocation)
                let (before_end, char_start, char_end, after_start) =
                    split_at_char_index(line_text, cursor_col);

                // Add text before cursor
                if before_end > 0 {
                    spans.push(Span::styled(&line_text[..before_end], Style::default().fg(line_color)));
                }

                // Add cursor character with inverse style
                let cursor_char = &line_text[char_start..char_end];
                let cursor_display = if cursor_char.is_empty() { " " } else { cursor_char };
                spans.push(Span::styled(
                    cursor_display,
                    Style::default()
                        .bg(Color::White)
                        .fg(Color::Black)
                        .add_modifier(Modifier::BOLD),
                ));

                // Add text after cursor
                if after_start < line_text.len() {
                    spans.push(Span::styled(&line_text[after_start..], Style::default().fg(line_color)));
                }

                Line::from(spans)
            } else {
                // Regular line without cursor
                Line::from(Span::styled(
                    line_text.to_string(),
                    Style::default().fg(line_color),
                ))
            }
        })
        .collect()
}

/// Render editor text with selection highlighted
///
/// Takes EditorState and returns Vec<Line> with selection range
/// highlighted using background color and cursor shown if present.
fn render_editor_with_selection<'a>(state: &'a crate::game::EditorState) -> Vec<Line<'a>> {
    let content = state.content();
    let cursor = state.cursor_position();
    let (cursor_line, cursor_col) = (cursor.row, cursor.col);
    let selection = state.selection();

    content
        .lines()
        .enumerate()
        .map(|(line_idx, line_text)| {
            if let Some(sel) = selection {
                // Check if this line has selection
                let sel_start_line = sel.start.row;
                let sel_end_line = sel.end.row;

                if line_idx >= sel_start_line && line_idx <= sel_end_line {
                    // This line contains selection
                    let mut spans = Vec::new();

                    // Determine selection range for this line
                    let line_start_col = if line_idx == sel_start_line {
                        sel.start.col
                    } else {
                        0
                    };

                    let line_end_col = if line_idx == sel_end_line {
                        sel.end.col
                    } else {
                        line_text.len()
                    };

                    // Get byte indices for selection range (zero-allocation)
                    let (start_byte, end_byte) = char_range_to_bytes(line_text, line_start_col, line_end_col);

                    // Text before selection
                    if start_byte > 0 {
                        spans.push(Span::styled(&line_text[..start_byte], Style::default().fg(Color::Yellow)));
                    }

                    // Selected text with highlight
                    if start_byte < end_byte {
                        spans.push(Span::styled(
                            &line_text[start_byte..end_byte],
                            Style::default()
                                .bg(Color::Blue)
                                .fg(Color::White)
                                .add_modifier(Modifier::BOLD),
                        ));
                    }

                    // Text after selection
                    if end_byte < line_text.len() {
                        spans.push(Span::styled(&line_text[end_byte..], Style::default().fg(Color::Yellow)));
                    }

                    return Line::from(spans);
                }
            }

            // No selection on this line - show with cursor if applicable
            if line_idx == cursor_line {
                let mut spans = Vec::new();

                // Split line at cursor position (zero-allocation)
                let (before_end, char_start, char_end, after_start) =
                    split_at_char_index(line_text, cursor_col);

                if before_end > 0 {
                    spans.push(Span::styled(&line_text[..before_end], Style::default().fg(Color::Yellow)));
                }

                let cursor_char = &line_text[char_start..char_end];
                let cursor_display = if cursor_char.is_empty() { " " } else { cursor_char };
                spans.push(Span::styled(
                    cursor_display,
                    Style::default()
                        .bg(Color::White)
                        .fg(Color::Black)
                        .add_modifier(Modifier::BOLD),
                ));

                if after_start < line_text.len() {
                    spans.push(Span::styled(&line_text[after_start..], Style::default().fg(Color::Yellow)));
                }

                Line::from(spans)
            } else {
                // Regular line
                Line::from(Span::styled(
                    line_text.to_string(),
                    Style::default().fg(Color::Yellow),
                ))
            }
        })
        .collect()
}

/// Render key history popup showing last 5 keys pressed with large text
fn render_key_history_popup(frame: &mut Frame, state: &AppState) {
    let area = frame.area();

    // Show last 5 keys
    let max_keys = 5;

    // Build text from recent keys
    let mut key_text = String::new();
    for (idx, key) in state.key_history.iter().take(max_keys).enumerate() {
        if idx > 0 {
            key_text.push(' ');
        }
        key_text.push_str(key);
    }

    // Calculate required dimensions before consuming key_text
    // Each character in Full size is approximately 4 cells wide, plus spacing
    let chars_count = key_text.chars().count();
    let popup_width = ((chars_count * 5).max(30) as u16).min(area.width.saturating_sub(4));
    let text_height = 8;
    let popup_height = text_height + 2; // +2 for borders

    // Create BigText widget with large font and cyan color
    let big_text = BigText::builder()
        .pixel_size(PixelSize::Full)
        .style(Style::default().fg(Color::Cyan))
        .lines(vec![key_text.into()])
        .centered()
        .build();

    // Position in bottom right corner
    let popup_x = area.width.saturating_sub(popup_width + 2);
    let popup_y = area.height.saturating_sub(popup_height + 2);

    let popup_area = Rect {
        x: popup_x,
        y: popup_y,
        width: popup_width,
        height: popup_height,
    };

    // Render with border
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .style(Style::default().bg(Color::Black));

    let inner_area = block.inner(popup_area);
    frame.render_widget(block, popup_area);
    frame.render_widget(big_text, inner_area);
}

/// Render success popup when scenario is completed
fn render_success_popup(frame: &mut Frame) {
    let area = frame.area();

    // Create centered popup area
    let popup_area = centered_popup(area, 40, 7);

    // Success message
    let success_text = vec![
        Line::from(""),
        Line::from(Span::styled(
            "SUCCESS!",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Scenario completed!",
            Style::default().fg(Color::White),
        )),
        Line::from(""),
    ];

    let success_paragraph = Paragraph::new(success_text)
        .alignment(Alignment::Center)
        .block(popup_block(None, Color::Green));

    frame.render_widget(success_paragraph, popup_area);
}

// ============================================================================
// Helper functions for common rendering patterns
// ============================================================================

/// Split a string at a character index without allocating
///
/// Returns byte indices (start, char_byte_pos, end) for efficient slicing.
/// This avoids collecting chars into new Strings.
///
/// # Arguments
///
/// * `s` - The string to split
/// * `char_idx` - Character index (not byte index)
///
/// # Returns
///
/// Tuple of (before_end_byte, char_start_byte, char_end_byte, after_start_byte)
fn split_at_char_index(s: &str, char_idx: usize) -> (usize, usize, usize, usize) {
    let mut char_indices = s.char_indices();

    // Find the byte position of the character at char_idx
    let char_byte_start = char_indices
        .nth(char_idx)
        .map(|(idx, _)| idx)
        .unwrap_or(s.len());

    // Find the end byte position of the character (start of next char or end of string)
    let char_byte_end = char_indices
        .next()
        .map(|(idx, _)| idx)
        .unwrap_or(s.len());

    (char_byte_start, char_byte_start, char_byte_end, char_byte_end)
}

/// Get byte indices for a character range without allocating
///
/// Returns (start_byte, end_byte) for a range of characters.
///
/// # Arguments
///
/// * `s` - The string to analyze
/// * `start_char` - Starting character index
/// * `end_char` - Ending character index (exclusive)
///
/// # Returns
///
/// Tuple of (start_byte_index, end_byte_index)
fn char_range_to_bytes(s: &str, start_char: usize, end_char: usize) -> (usize, usize) {
    let mut char_indices = s.char_indices().enumerate();

    let start_byte = char_indices
        .find_map(|(idx, (byte_pos, _))| (idx == start_char).then_some(byte_pos))
        .unwrap_or(s.len());

    let end_byte = char_indices
        .find_map(|(idx, (byte_pos, _))| (idx == end_char).then_some(byte_pos))
        .unwrap_or(s.len());

    (start_byte, end_byte)
}

/// Calculate centered popup area with given dimensions
///
/// # Arguments
///
/// * `parent` - The parent area to center within
/// * `width` - Desired popup width
/// * `height` - Desired popup height
///
/// # Returns
///
/// A Rect centered within the parent area
fn centered_popup(parent: Rect, width: u16, height: u16) -> Rect {
    let popup_x = (parent.width.saturating_sub(width)) / 2;
    let popup_y = (parent.height.saturating_sub(height)) / 2;

    Rect {
        x: popup_x,
        y: popup_y,
        width,
        height,
    }
}

/// Create a standard popup block with borders
///
/// # Arguments
///
/// * `title` - Optional title for the popup
/// * `border_color` - Color for the border
///
/// # Returns
///
/// A Block with standard styling
fn popup_block(title: Option<&str>, border_color: Color) -> Block<'_> {
    let mut block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .style(Style::default().bg(Color::Black));

    if let Some(t) = title {
        block = block.title(t);
    }

    block
}

/// Calculate the inner area of a rect (excluding borders)
///
/// # Arguments
///
/// * `outer` - The outer rect with borders
///
/// # Returns
///
/// The inner rect without borders
fn inner_rect(outer: Rect) -> Rect {
    Rect {
        x: outer.x + 1,
        y: outer.y + 1,
        width: outer.width.saturating_sub(2),
        height: outer.height.saturating_sub(2),
    }
}

#[cfg(test)]
#[allow(unused_variables)] // Test backends don't use all variables
mod tests {
    use super::*;
    use crate::config::{ScoringConfig, Setup, Solution, TargetState};
    use crate::ui::state::AppState;

    fn create_test_scenario() -> crate::config::Scenario {
        crate::config::Scenario {
            id: "test_001".to_string(),
            name: "Test Scenario".to_string(),
            description: "A test scenario for rendering".to_string(),
            setup: Setup {
                file_content: "line 1\n".to_string(),
                cursor_position: (0, 0),
            },
            target: TargetState {
                file_content: "line 2\n".to_string(),
                cursor_position: (0, 0),
                selection: None,
            },
            solution: Solution {
                commands: vec!["dd".to_string()],
                description: "Delete line".to_string(),
            },
            alternatives: vec![],
            hints: vec!["Test hint".to_string()],
            scoring: ScoringConfig {
                optimal_count: 1,
                max_points: 100,
                tolerance: 0,
            },
        }
    }

    #[test]
    fn test_main_menu_items_count() {
        let items = AppState::menu_items();
        assert_eq!(items.len(), 2);
    }

    #[test]
    fn test_render_does_not_panic_on_empty_state() {
        use ratatui::Terminal;
        use ratatui::backend::TestBackend;

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        let state = AppState::new(vec![]);

        terminal
            .draw(|f| {
                render(f, &state);
            })
            .unwrap();
    }

    #[test]
    fn test_render_task_screen_with_session() {
        use ratatui::Terminal;
        use ratatui::backend::TestBackend;

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        let scenario = create_test_scenario();
        let mut state = AppState::new(vec![scenario]);

        // Create a session
        crate::ui::update(&mut state, crate::ui::Message::StartScenario(0)).unwrap();

        terminal
            .draw(|f| {
                render(f, &state);
            })
            .unwrap();
    }
}
