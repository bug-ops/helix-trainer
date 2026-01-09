//! Editor text rendering with cursor and selection

use super::helpers::{char_range_to_bytes, split_at_char_index};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};

/// Render editor text with cursor, selection and diff highlighting
///
/// Compares current state with target state and colors lines:
/// - Green: lines that match target
/// - Red: lines that differ from target
/// - Selection shown with blue background
/// - Cursor shown with inverse colors
pub(super) fn render_editor_with_diff<'a>(
    current: &'a crate::game::EditorState,
    target: &crate::game::EditorState,
) -> Vec<Line<'a>> {
    let current_content = current.content();
    let target_content = target.content();
    let cursor = current.cursor_position();
    let (cursor_line, cursor_col) = (cursor.row, cursor.col);
    let selection = current.selection();

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

            // Check if this line has selection
            if let Some(sel) = selection {
                let sel_start_line = sel.start.row;
                let sel_end_line = sel.end.row;

                // Skip if this is the end line but end_col is 0 (selection ends before this line)
                let line_has_selection = if line_idx == sel_end_line && sel.end.col == 0 {
                    false
                } else {
                    line_idx >= sel_start_line && line_idx <= sel_end_line
                };

                if line_has_selection {
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
                        line_text.chars().count()
                    };

                    // Get byte indices for selection range
                    let (start_byte, end_byte) =
                        char_range_to_bytes(line_text, line_start_col, line_end_col);

                    // Text before selection
                    if start_byte > 0 {
                        spans.push(Span::styled(
                            &line_text[..start_byte],
                            Style::default().fg(line_color),
                        ));
                    }

                    // Selected text with highlight
                    if start_byte < end_byte && end_byte <= line_text.len() {
                        spans.push(Span::styled(
                            &line_text[start_byte..end_byte],
                            Style::default()
                                .bg(super::SELECTION_BG_COLOR)
                                .fg(Color::White),
                        ));
                    }

                    // Text after selection
                    if end_byte < line_text.len() {
                        spans.push(Span::styled(
                            &line_text[end_byte..],
                            Style::default().fg(line_color),
                        ));
                    }

                    return Line::from(spans);
                }
            }

            // No selection - show cursor if on this line
            if line_idx == cursor_line {
                // This line contains the cursor
                let mut spans = Vec::new();

                // Split line at cursor position (zero-allocation)
                let (before_end, char_start, char_end, after_start) =
                    split_at_char_index(line_text, cursor_col);

                // Add text before cursor
                if before_end > 0 {
                    spans.push(Span::styled(
                        &line_text[..before_end],
                        Style::default().fg(line_color),
                    ));
                }

                // Add cursor character with inverse style
                // For empty lines, use a special block character that renders as a cursor
                // without causing line wrapping issues
                let cursor_char = &line_text[char_start..char_end];
                let cursor_display = if cursor_char.is_empty() {
                    "\u{2588}" // Full block character for cursor on empty line
                } else {
                    cursor_char
                };
                spans.push(Span::styled(
                    cursor_display,
                    Style::default()
                        .bg(Color::White)
                        .fg(Color::Black)
                        .add_modifier(Modifier::BOLD),
                ));

                // Add text after cursor
                if after_start < line_text.len() {
                    spans.push(Span::styled(
                        &line_text[after_start..],
                        Style::default().fg(line_color),
                    ));
                }

                Line::from(spans)
            } else {
                // Regular line without cursor
                Line::from(Span::styled(line_text, Style::default().fg(line_color)))
            }
        })
        .collect()
}

/// Render editor text with syntax highlighting and selection
///
/// Takes EditorState and returns `Vec<Line>` with syntax highlighting,
/// selection range highlighted using background color, and cursor shown if present.
pub(super) fn render_editor_with_selection(state: &crate::game::EditorState) -> Vec<Line<'static>> {
    let content = state.content();
    let cursor = state.cursor_position();
    let selection = state.selection();

    super::highlight::highlight_code_with_cursor(
        content,
        cursor.row,
        cursor.col,
        selection.as_ref(),
    )
}

/// Render a side-by-side editor view (current state vs target state)
///
/// This is the common layout used by both Training mode and Arcade mode.
/// Shows current state on the left with diff highlighting and cursor,
/// and target state on the right with selection highlighting.
///
/// # Arguments
///
/// * `frame` - Ratatui frame to render into
/// * `area` - Area to render the editor views
/// * `current_state` - Current editor state with cursor
/// * `target_state` - Target state to achieve
/// * `current_title` - Title for current state panel
/// * `target_title` - Title for target state panel
pub(super) fn render_editor_pair(
    frame: &mut Frame,
    area: Rect,
    current_state: &crate::game::EditorState,
    target_state: &crate::game::EditorState,
    current_title: &str,
    target_title: &str,
) {
    // Split into two columns
    let editor_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    // Current state with cursor and diff highlighting
    let current_lines = render_editor_with_diff(current_state, target_state);
    let current = Paragraph::new(current_lines)
        .block(
            Block::default()
                .title(current_title)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .wrap(Wrap { trim: false });
    frame.render_widget(current, editor_chunks[0]);

    // Target state with selection highlighting (if any)
    let target_lines = render_editor_with_selection(target_state);
    let target = Paragraph::new(target_lines)
        .block(
            Block::default()
                .title(target_title)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Green)),
        )
        .wrap(Wrap { trim: false });
    frame.render_widget(target, editor_chunks[1]);
}
