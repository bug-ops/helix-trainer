//! Editor text rendering with cursor and selection

use super::helpers::{char_range_to_bytes, split_at_char_index};
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

/// Render editor text with cursor and diff highlighting
///
/// Compares current state with target state and colors lines:
/// - Green: lines that match target
/// - Red: lines that differ from target
/// - Cursor shown with inverse colors
pub(super) fn render_editor_with_diff<'a>(
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
                    spans.push(Span::styled(
                        &line_text[..before_end],
                        Style::default().fg(line_color),
                    ));
                }

                // Add cursor character with inverse style
                let cursor_char = &line_text[char_start..char_end];
                let cursor_display = if cursor_char.is_empty() {
                    " "
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

/// Render editor text with selection highlighted
///
/// Takes EditorState and returns Vec<Line> with selection range
/// highlighted using background color and cursor shown if present.
pub(super) fn render_editor_with_selection<'a>(
    state: &'a crate::game::EditorState,
) -> Vec<Line<'a>> {
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
                    let (start_byte, end_byte) =
                        char_range_to_bytes(line_text, line_start_col, line_end_col);

                    // Text before selection
                    if start_byte > 0 {
                        spans.push(Span::styled(
                            &line_text[..start_byte],
                            Style::default().fg(Color::Yellow),
                        ));
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
                        spans.push(Span::styled(
                            &line_text[end_byte..],
                            Style::default().fg(Color::Yellow),
                        ));
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
                    spans.push(Span::styled(
                        &line_text[..before_end],
                        Style::default().fg(Color::Yellow),
                    ));
                }

                let cursor_char = &line_text[char_start..char_end];
                let cursor_display = if cursor_char.is_empty() {
                    " "
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

                if after_start < line_text.len() {
                    spans.push(Span::styled(
                        &line_text[after_start..],
                        Style::default().fg(Color::Yellow),
                    ));
                }

                Line::from(spans)
            } else {
                // Regular line
                Line::from(Span::styled(line_text, Style::default().fg(Color::Yellow)))
            }
        })
        .collect()
}
