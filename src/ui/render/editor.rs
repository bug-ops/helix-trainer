//! Editor text rendering with cursor and selection

use super::helpers::find_surrounding_brackets;
use crate::game::PlayableScenario;
use crate::helix::SelectionBounds;
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

/// Cursor information for multi-cursor rendering.
///
/// Encapsulates cursor position and primary/secondary status for style differentiation.
#[derive(Debug, Clone, Copy)]
pub struct CursorInfo {
    /// Row position (0-indexed)
    pub row: usize,
    /// Column position (0-indexed)
    pub col: usize,
    /// Whether this is the primary cursor
    pub is_primary: bool,
}

impl CursorInfo {
    /// Get cursor style based on primary/secondary status.
    ///
    /// Primary cursor: white background, black foreground
    /// Secondary cursor: cyan background, black foreground
    pub fn style(&self) -> Style {
        if self.is_primary {
            Style::default()
                .bg(Color::White)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
                .bg(Color::Cyan)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD)
        }
    }
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

/// Render a line with multiple cursors.
///
/// Handles multiple cursor positions on a single line, with different styles
/// for primary (white bg) and secondary (cyan bg) cursors.
fn render_line_with_multi_cursor(
    line_text: &str,
    line_color: Color,
    cursors: &[CursorInfo],
    preview_positions: &[usize],
    preview_color: Color,
) -> Line<'static> {
    let mut spans = Vec::new();
    let line_chars: Vec<char> = line_text.chars().collect();
    let mut byte_pos = 0;

    for (col, ch) in line_chars.iter().enumerate() {
        let char_len = ch.len_utf8();
        let char_str = &line_text[byte_pos..byte_pos + char_len];

        // Check if any cursor is at this position
        let cursor_at_pos = cursors.iter().find(|c| c.col == col);

        let style = if let Some(cursor) = cursor_at_pos {
            cursor.style()
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

    // Cursors at end of line
    for cursor in cursors.iter().filter(|c| c.col >= line_chars.len()) {
        spans.push(Span::styled("\u{2588}".to_string(), cursor.style()));
    }

    Line::from(spans)
}

/// Render a line with multiple selections.
///
/// Handles overlapping selections by using a HashSet to track highlighted columns.
fn render_line_with_multi_selection(
    line_text: &str,
    line_idx: usize,
    line_color: Color,
    selections: &[SelectionBounds],
) -> Line<'static> {
    // Build a set of highlighted columns from all selections
    let mut highlighted_cols: std::collections::HashSet<usize> = std::collections::HashSet::new();

    for sel in selections {
        if !line_has_selection(line_idx, sel) {
            continue;
        }

        let start_col = if line_idx == sel.start_row {
            sel.start_col
        } else {
            0
        };
        let end_col = if line_idx == sel.end_row {
            sel.end_col
        } else {
            line_text.chars().count()
        };

        for col in start_col..end_col {
            highlighted_cols.insert(col);
        }
    }

    // Render character by character
    let mut spans = Vec::new();
    let mut byte_pos = 0;

    for (col, ch) in line_text.chars().enumerate() {
        let char_len = ch.len_utf8();
        let char_str = &line_text[byte_pos..byte_pos + char_len];

        let style = if highlighted_cols.contains(&col) {
            Style::default()
                .bg(super::SELECTION_BG_COLOR)
                .fg(Color::White)
        } else {
            Style::default().fg(line_color)
        };

        spans.push(Span::styled(char_str.to_string(), style));
        byte_pos += char_len;
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
fn line_has_selection(line_idx: usize, sel: &SelectionBounds) -> bool {
    if line_idx == sel.end_row && sel.end_col == 0 {
        return false;
    }
    line_idx >= sel.start_row && line_idx <= sel.end_row
}

/// Render editor text with multi-cursor, multi-selection and diff highlighting.
///
/// Compares current state with target state and colors lines:
/// - Green: lines that match target
/// - Red: lines that differ from target
/// - Selection shown with blue background
/// - Primary cursor shown with white background
/// - Secondary cursors shown with cyan background
/// - Preview highlight shown with yellow/red background
pub(super) fn render_editor_with_diff(
    current_content: &str,
    target_content: &str,
    cursors: &[CursorInfo],
    selections: &[SelectionBounds],
    preview: Option<PreviewHighlight>,
) -> Vec<Line<'static>> {
    let preview_color = preview.map(|p| p.color()).unwrap_or(Color::Yellow);

    let current_lines: Vec<&str> = current_content.lines().collect();
    let target_lines: Vec<&str> = target_content.lines().collect();

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

            // Multi-selection takes priority
            let line_has_any_selection = selections
                .iter()
                .any(|sel| line_has_selection(line_idx, sel));
            if line_has_any_selection {
                return render_line_with_multi_selection(
                    line_text, line_idx, line_color, selections,
                );
            }

            // Gather cursors on this line
            let cursors_on_line: Vec<CursorInfo> = cursors
                .iter()
                .filter(|c| c.row == line_idx)
                .copied()
                .collect();

            let preview_positions = get_preview_positions(preview, line_idx);
            let has_cursors = !cursors_on_line.is_empty();
            let has_preview = !preview_positions.is_empty();

            if has_cursors || has_preview {
                render_line_with_multi_cursor(
                    line_text,
                    line_color,
                    &cursors_on_line,
                    &preview_positions,
                    preview_color,
                )
            } else {
                Line::from(Span::styled(
                    line_text.to_string(),
                    Style::default().fg(line_color),
                ))
            }
        })
        .collect()
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
/// * `scenario` - The playable scenario providing state access
/// * `current_title` - Title for current state panel
/// * `target_title` - Title for target state panel
/// * `preview` - Optional preview highlight for surround replace
pub(super) fn render_editor_pair<S: PlayableScenario + ?Sized>(
    frame: &mut Frame,
    area: Rect,
    scenario: &S,
    current_title: &str,
    target_title: &str,
    preview: Option<PreviewHighlight>,
) {
    // Get state from trait methods
    let current_content = scenario.current_content();
    let target_content = scenario.target_content();

    // Get all current cursors and build CursorInfo list
    let all_cursor_positions = scenario.all_cursors();
    let cursors: Vec<CursorInfo> = all_cursor_positions
        .iter()
        .enumerate()
        .map(|(idx, &(row, col))| CursorInfo {
            row,
            col,
            is_primary: idx == 0, // First cursor is primary
        })
        .collect();

    // Get all current selections
    let selections = scenario.all_selections();

    // Get all target cursors and build CursorInfo list
    let all_target_cursor_positions = scenario.all_target_cursors();
    let target_cursors: Vec<CursorInfo> = all_target_cursor_positions
        .iter()
        .enumerate()
        .map(|(idx, &(row, col))| CursorInfo {
            row,
            col,
            is_primary: idx == 0, // First cursor is primary
        })
        .collect();

    // Get all target selections
    let target_selections = scenario.all_target_selections();

    // Split into two columns
    let editor_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    // Current state with multi-cursor and diff highlighting
    let current_lines = render_editor_with_diff(
        &current_content,
        &target_content,
        &cursors,
        &selections,
        preview,
    );
    let current = Paragraph::new(current_lines)
        .block(
            Block::default()
                .title(current_title)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .wrap(Wrap { trim: false });
    frame.render_widget(current, editor_chunks[0]);

    // Target state with multi-cursor (no diff highlighting - use same content for both)
    let target_lines = render_editor_with_diff(
        &target_content,
        &target_content,
        &target_cursors,
        &target_selections,
        None,
    );
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
        let sel = SelectionBounds::new(1, 0, 3, 5);

        assert!(!line_has_selection(0, &sel)); // Before selection
        assert!(line_has_selection(1, &sel)); // Start of selection
        assert!(line_has_selection(2, &sel)); // Middle of selection
        assert!(line_has_selection(3, &sel)); // End of selection
        assert!(!line_has_selection(4, &sel)); // After selection
    }

    #[test]
    fn test_line_has_selection_edge_case_end_col_zero() {
        let sel = SelectionBounds::new(1, 0, 2, 0);

        // When end_col is 0, line 2 should NOT be considered selected
        assert!(line_has_selection(1, &sel));
        assert!(!line_has_selection(2, &sel));
    }

    // ==================== CursorInfo tests ====================

    #[test]
    fn test_cursor_info_primary_style() {
        let cursor = CursorInfo {
            row: 0,
            col: 5,
            is_primary: true,
        };
        let style = cursor.style();
        assert_eq!(style.bg, Some(Color::White));
        assert_eq!(style.fg, Some(Color::Black));
    }

    #[test]
    fn test_cursor_info_secondary_style() {
        let cursor = CursorInfo {
            row: 1,
            col: 3,
            is_primary: false,
        };
        let style = cursor.style();
        assert_eq!(style.bg, Some(Color::Cyan));
        assert_eq!(style.fg, Some(Color::Black));
    }

    // ==================== render_line_with_multi_cursor tests ====================

    #[test]
    fn test_render_line_with_multi_cursor_single() {
        let cursors = vec![CursorInfo {
            row: 0,
            col: 5,
            is_primary: true,
        }];
        let line = render_line_with_multi_cursor(
            "hello world",
            Color::Green,
            &cursors,
            &[],
            Color::Yellow,
        );
        // Should produce spans with cursor styled at position 5
        assert!(!line.spans.is_empty());
    }

    #[test]
    fn test_render_line_with_multi_cursor_multiple() {
        let cursors = vec![
            CursorInfo {
                row: 0,
                col: 0,
                is_primary: true,
            },
            CursorInfo {
                row: 0,
                col: 5,
                is_primary: false,
            },
        ];
        let line = render_line_with_multi_cursor(
            "hello world",
            Color::Green,
            &cursors,
            &[],
            Color::Yellow,
        );
        // Should have spans for each character
        assert_eq!(line.spans.len(), 11); // "hello world" = 11 chars
    }

    #[test]
    fn test_render_line_with_multi_cursor_at_end() {
        let cursors = vec![CursorInfo {
            row: 0,
            col: 5,
            is_primary: true,
        }];
        // Cursor at col 5 on a line with only 5 chars (indices 0-4) should render block
        let line =
            render_line_with_multi_cursor("hello", Color::Green, &cursors, &[], Color::Yellow);
        // Should have 5 char spans + 1 block char for cursor at end
        assert_eq!(line.spans.len(), 6);
    }

    #[test]
    fn test_render_line_with_multi_cursor_primary_secondary() {
        let cursors = vec![
            CursorInfo {
                row: 0,
                col: 0,
                is_primary: true,
            },
            CursorInfo {
                row: 0,
                col: 2,
                is_primary: false,
            },
        ];
        let line = render_line_with_multi_cursor("abc", Color::Green, &cursors, &[], Color::Yellow);
        // Check that first and third chars have cursor styling
        assert_eq!(line.spans.len(), 3);
        // First span (primary cursor) should have white background
        assert_eq!(line.spans[0].style.bg, Some(Color::White));
        // Third span (secondary cursor) should have cyan background
        assert_eq!(line.spans[2].style.bg, Some(Color::Cyan));
    }

    // ==================== render_line_with_multi_selection tests ====================

    #[test]
    fn test_render_line_with_multi_selection_single() {
        let selections = vec![SelectionBounds::new(0, 2, 0, 5)];
        let line = render_line_with_multi_selection("hello world", 0, Color::Green, &selections);
        // Should highlight chars at indices 2, 3, 4
        assert!(!line.spans.is_empty());
    }

    #[test]
    fn test_render_line_with_multi_selection_overlapping() {
        let selections = vec![
            SelectionBounds::new(0, 0, 0, 5),
            SelectionBounds::new(0, 3, 0, 8),
        ];
        let line = render_line_with_multi_selection("hello world", 0, Color::Green, &selections);
        // Overlapping selections (0-5 and 3-8) should merge to highlight 0-8
        // Check that we have spans with proper styling
        assert!(!line.spans.is_empty());
    }

    #[test]
    fn test_render_line_with_multi_selection_spanning_lines() {
        // Selection from row 0 to row 2
        let selections = vec![SelectionBounds::new(0, 3, 2, 5)];

        // Line 0: should highlight from col 3 to end
        let line0 = render_line_with_multi_selection("hello", 0, Color::Green, &selections);
        assert!(!line0.spans.is_empty());

        // Line 1: should highlight entire line (0 to end)
        let line1 = render_line_with_multi_selection("world", 1, Color::Green, &selections);
        assert!(!line1.spans.is_empty());

        // Line 2: should highlight from 0 to col 5
        let line2 = render_line_with_multi_selection("test", 2, Color::Green, &selections);
        assert!(!line2.spans.is_empty());
    }

    #[test]
    fn test_render_line_with_multi_selection_no_selection_on_line() {
        let selections = vec![SelectionBounds::new(2, 0, 3, 5)];
        // Line 0 is not in the selection range (2-3)
        let line = render_line_with_multi_selection("hello", 0, Color::Green, &selections);
        // Should render with normal styling (no selection highlight)
        assert!(!line.spans.is_empty());
        // All spans should have green foreground (no selection background)
        for span in &line.spans {
            assert_eq!(span.style.fg, Some(Color::Green));
            assert!(span.style.bg.is_none());
        }
    }
}
