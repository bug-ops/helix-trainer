//! Editor text rendering with cursor and selection

use super::helpers::{char_range_to_bytes, find_surrounding_brackets};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};

/// Type of preview highlight
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreviewType {
    /// Surround replace - yellow highlight for brackets to be replaced
    Replace,
    /// Surround delete - red highlight for brackets to be deleted
    Delete,
}

/// Preview highlight for surround operations
///
/// Contains positions of brackets that will be modified and the operation type
#[derive(Debug, Clone, Copy)]
pub struct PreviewHighlight {
    pub open_row: usize,
    pub open_col: usize,
    pub close_row: usize,
    pub close_col: usize,
    pub preview_type: PreviewType,
}

impl PreviewHighlight {
    /// Create preview highlight by finding surrounding brackets
    pub fn from_surround_char(
        content: &str,
        cursor_row: usize,
        cursor_col: usize,
        bracket_char: char,
        preview_type: PreviewType,
    ) -> Option<Self> {
        let (open_row, open_col, close_row, close_col) =
            find_surrounding_brackets(content, cursor_row, cursor_col, bracket_char)?;
        Some(Self {
            open_row,
            open_col,
            close_row,
            close_col,
            preview_type,
        })
    }

    /// Get the highlight color for this preview type
    pub fn color(&self) -> Color {
        match self.preview_type {
            PreviewType::Replace => Color::Yellow,
            PreviewType::Delete => Color::Red,
        }
    }
}

/// Render a line with selection highlighting
fn render_line_with_selection<'a>(
    line_text: &'a str,
    line_idx: usize,
    line_color: Color,
    sel: &crate::game::Selection,
) -> Line<'a> {
    let mut spans = Vec::new();

    let line_start_col = if line_idx == sel.start.row {
        sel.start.col
    } else {
        0
    };

    let line_end_col = if line_idx == sel.end.row {
        sel.end.col
    } else {
        line_text.chars().count()
    };

    let (start_byte, end_byte) = char_range_to_bytes(line_text, line_start_col, line_end_col);

    if start_byte > 0 {
        spans.push(Span::styled(
            &line_text[..start_byte],
            Style::default().fg(line_color),
        ));
    }

    if start_byte < end_byte && end_byte <= line_text.len() {
        spans.push(Span::styled(
            &line_text[start_byte..end_byte],
            Style::default()
                .bg(super::SELECTION_BG_COLOR)
                .fg(Color::White),
        ));
    }

    if end_byte < line_text.len() {
        spans.push(Span::styled(
            &line_text[end_byte..],
            Style::default().fg(line_color),
        ));
    }

    Line::from(spans)
}

/// Render a line with cursor and/or preview highlights
fn render_line_with_highlights(
    line_text: &str,
    line_color: Color,
    cursor_col: Option<usize>,
    preview_positions: &[usize],
    preview_color: Color,
) -> Line<'static> {
    let mut spans = Vec::new();
    let line_chars: Vec<char> = line_text.chars().collect();
    let mut byte_pos = 0;

    for (col, ch) in line_chars.iter().enumerate() {
        let char_len = ch.len_utf8();
        let char_str = &line_text[byte_pos..byte_pos + char_len];

        let style = if cursor_col == Some(col) {
            Style::default()
                .bg(Color::White)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD)
        } else if preview_positions.contains(&col) {
            Style::default()
                .bg(preview_color)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(line_color)
        };

        spans.push(Span::styled(char_str.to_string(), style));
        byte_pos += char_len;
    }

    // Cursor at end of line
    if let Some(col) = cursor_col
        && col >= line_chars.len()
    {
        spans.push(Span::styled(
            "\u{2588}".to_string(),
            Style::default()
                .bg(Color::White)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
        ));
    }

    Line::from(spans)
}

/// Get preview highlight positions for a specific line
fn get_preview_positions(preview: Option<PreviewHighlight>, line_idx: usize) -> Vec<usize> {
    let Some(p) = preview else {
        return Vec::new();
    };

    let mut positions = Vec::new();
    if line_idx == p.open_row {
        positions.push(p.open_col);
    }
    if line_idx == p.close_row {
        positions.push(p.close_col);
    }
    positions.sort();
    positions
}

/// Check if a line has selection (accounting for edge cases)
fn line_has_selection(line_idx: usize, sel: &crate::game::Selection) -> bool {
    if line_idx == sel.end.row && sel.end.col == 0 {
        return false;
    }
    line_idx >= sel.start.row && line_idx <= sel.end.row
}

/// Render editor text with cursor, selection and diff highlighting
///
/// Compares current state with target state and colors lines:
/// - Green: lines that match target
/// - Red: lines that differ from target
/// - Selection shown with blue background
/// - Cursor shown with inverse colors
/// - Preview highlight shown with yellow/red background
pub(super) fn render_editor_with_diff<'a>(
    current: &'a crate::game::EditorState,
    target: &crate::game::EditorState,
    preview: Option<PreviewHighlight>,
) -> Vec<Line<'a>> {
    let cursor = current.cursor_position();
    let selection = current.selection();
    let preview_color = preview.map(|p| p.color()).unwrap_or(Color::Yellow);

    let current_lines: Vec<&str> = current.content().lines().collect();
    let target_lines: Vec<&str> = target.content().lines().collect();

    current_lines
        .iter()
        .enumerate()
        .map(|(line_idx, &line_text)| {
            let matches_target = target_lines
                .get(line_idx)
                .map(|&t| t == line_text)
                .unwrap_or(false);

            let line_color = if matches_target {
                Color::Green
            } else {
                Color::Red
            };

            // Selection takes priority
            if let Some(sel) = selection
                && line_has_selection(line_idx, &sel)
            {
                return render_line_with_selection(line_text, line_idx, line_color, &sel);
            }

            let preview_positions = get_preview_positions(preview, line_idx);
            let has_cursor = line_idx == cursor.row;
            let has_preview = !preview_positions.is_empty();

            if has_cursor || has_preview {
                let cursor_col = if has_cursor { Some(cursor.col) } else { None };
                render_line_with_highlights(
                    line_text,
                    line_color,
                    cursor_col,
                    &preview_positions,
                    preview_color,
                )
            } else {
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
/// * `preview` - Optional preview highlight for surround replace
pub(super) fn render_editor_pair(
    frame: &mut Frame,
    area: Rect,
    current_state: &crate::game::EditorState,
    target_state: &crate::game::EditorState,
    current_title: &str,
    target_title: &str,
    preview: Option<PreviewHighlight>,
) {
    // Split into two columns
    let editor_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    // Current state with cursor and diff highlighting
    let current_lines = render_editor_with_diff(current_state, target_state, preview);
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

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== PreviewHighlight::color tests ====================

    #[test]
    fn test_preview_highlight_color_replace() {
        let preview = PreviewHighlight {
            open_row: 0,
            open_col: 0,
            close_row: 0,
            close_col: 5,
            preview_type: PreviewType::Replace,
        };

        assert_eq!(preview.color(), Color::Yellow);
    }

    #[test]
    fn test_preview_highlight_color_delete() {
        let preview = PreviewHighlight {
            open_row: 0,
            open_col: 0,
            close_row: 0,
            close_col: 5,
            preview_type: PreviewType::Delete,
        };

        assert_eq!(preview.color(), Color::Red);
    }

    // ==================== get_preview_positions tests ====================

    #[test]
    fn test_get_preview_positions_open_bracket() {
        let preview = PreviewHighlight {
            open_row: 0,
            open_col: 5,
            close_row: 2,
            close_col: 10,
            preview_type: PreviewType::Replace,
        };

        // Line 0 should contain the open bracket position
        let positions = get_preview_positions(Some(preview), 0);
        assert_eq!(positions, vec![5]);
    }

    #[test]
    fn test_get_preview_positions_close_bracket() {
        let preview = PreviewHighlight {
            open_row: 0,
            open_col: 5,
            close_row: 2,
            close_col: 10,
            preview_type: PreviewType::Replace,
        };

        // Line 2 should contain the close bracket position
        let positions = get_preview_positions(Some(preview), 2);
        assert_eq!(positions, vec![10]);
    }

    #[test]
    fn test_get_preview_positions_both_on_same_line() {
        let preview = PreviewHighlight {
            open_row: 0,
            open_col: 3,
            close_row: 0,
            close_col: 8,
            preview_type: PreviewType::Delete,
        };

        // Line 0 should contain both bracket positions, sorted
        let positions = get_preview_positions(Some(preview), 0);
        assert_eq!(positions, vec![3, 8]);
    }

    #[test]
    fn test_get_preview_positions_none() {
        // When preview is None, should return empty vec
        let positions = get_preview_positions(None, 0);
        assert!(positions.is_empty());
    }

    #[test]
    fn test_get_preview_positions_middle_line() {
        let preview = PreviewHighlight {
            open_row: 0,
            open_col: 5,
            close_row: 3,
            close_col: 10,
            preview_type: PreviewType::Replace,
        };

        // Line 1 (middle line) should have no positions
        let positions = get_preview_positions(Some(preview), 1);
        assert!(positions.is_empty());

        // Line 2 (middle line) should also have no positions
        let positions = get_preview_positions(Some(preview), 2);
        assert!(positions.is_empty());
    }

    // ==================== PreviewHighlight::from_surround_char tests ====================

    #[test]
    fn test_preview_highlight_from_surround_char_found() {
        let content = "fn test(arg) { }";
        // Cursor inside parentheses
        let preview =
            PreviewHighlight::from_surround_char(content, 0, 8, '(', PreviewType::Replace);

        assert!(preview.is_some());
        let p = preview.unwrap();
        assert_eq!(p.open_col, 7);
        assert_eq!(p.close_col, 11);
        assert_eq!(p.preview_type, PreviewType::Replace);
    }

    #[test]
    fn test_preview_highlight_from_surround_char_not_found() {
        let content = "no brackets here";
        let preview = PreviewHighlight::from_surround_char(content, 0, 5, '(', PreviewType::Delete);

        assert!(preview.is_none());
    }

    #[test]
    fn test_preview_highlight_from_surround_char_delete_type() {
        let content = "[item]";
        let preview = PreviewHighlight::from_surround_char(content, 0, 2, '[', PreviewType::Delete);

        assert!(preview.is_some());
        let p = preview.unwrap();
        assert_eq!(p.preview_type, PreviewType::Delete);
        assert_eq!(p.color(), Color::Red);
    }

    // ==================== PreviewType equality tests ====================

    #[test]
    fn test_preview_type_equality() {
        assert_eq!(PreviewType::Replace, PreviewType::Replace);
        assert_eq!(PreviewType::Delete, PreviewType::Delete);
        assert_ne!(PreviewType::Replace, PreviewType::Delete);
    }

    // ==================== line_has_selection tests ====================

    #[test]
    fn test_line_has_selection_within_range() {
        let sel = crate::game::Selection::new(
            crate::game::CursorPosition { row: 1, col: 0 },
            crate::game::CursorPosition { row: 3, col: 5 },
        );

        assert!(!line_has_selection(0, &sel)); // Before selection
        assert!(line_has_selection(1, &sel)); // Start of selection
        assert!(line_has_selection(2, &sel)); // Middle of selection
        assert!(line_has_selection(3, &sel)); // End of selection
        assert!(!line_has_selection(4, &sel)); // After selection
    }

    #[test]
    fn test_line_has_selection_edge_case_end_col_zero() {
        let sel = crate::game::Selection::new(
            crate::game::CursorPosition { row: 1, col: 0 },
            crate::game::CursorPosition { row: 2, col: 0 },
        );

        // When end.col is 0, line 2 should NOT be considered selected
        assert!(line_has_selection(1, &sel));
        assert!(!line_has_selection(2, &sel));
    }
}
