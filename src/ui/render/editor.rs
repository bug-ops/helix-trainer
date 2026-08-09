//! Editor text rendering with cursor and selection

use super::helpers::find_surrounding_brackets;
use crate::game::PlayableScenario;
use crate::helix::SelectionBounds;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
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

    // Split into two columns of guaranteed-equal width. Percentage(50)/Percentage(50)
    // hands the leftover column to one side unpredictably on odd widths (#192). Equal
    // widths still matter with no wrapping (see below): they keep the truncation cutoff
    // and the horizontal-scroll offset identical in both panels, so columns stay aligned
    // between Current and Target. A middle spacer of the (0 or 1) leftover column keeps
    // both sides equal instead.
    let half_width = area.width / 2;
    let spare_width = area.width - half_width * 2;
    let editor_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(half_width),
            Constraint::Length(spare_width),
            Constraint::Length(half_width),
        ])
        .split(area);
    let current_area = editor_chunks[0];
    let target_area = editor_chunks[2];

    // Current state with multi-cursor and diff highlighting
    let current_lines = render_editor_with_diff(
        &current_content,
        &target_content,
        &cursors,
        &selections,
        preview,
    );
    // No `.wrap()`: Current/Target cursors sit on different rows by construction, so a row
    // whose length (including an appended past-EOL cursor block, see
    // `render_line_with_multi_cursor`) exactly fills the panel's inner width would soft-wrap
    // in one panel but not the other, desyncing every subsequent row between the two panels
    // (#333). Truncating instead guarantees one rendered row per logical line in both panels;
    // the horizontal scroll below keeps the truncated part reachable instead of hiding it
    // (and the cursor with it) with no indicator.
    //
    // `h_off` is derived from the Current panel's primary cursor column and applied to BOTH
    // panels identically, so a line that runs past the panel's inner width scrolls into view
    // around the cursor while keeping Current/Target columns aligned.
    let panel_inner_width = current_area.width.saturating_sub(2) as usize;
    let primary_cursor_col = cursors
        .iter()
        .find(|c| c.is_primary)
        .map(|c| c.col)
        .unwrap_or(0);
    let h_off: u16 = if primary_cursor_col < panel_inner_width {
        0
    } else {
        let raw = primary_cursor_col.saturating_sub(panel_inner_width) + 1;
        u16::try_from(raw).unwrap_or(u16::MAX)
    };

    let current = Paragraph::new(current_lines)
        .block(
            Block::default()
                .title(current_title)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .scroll((0, h_off));
    frame.render_widget(current, current_area);

    // Target state with syntax highlighting and multi-cursor
    let target_lines = super::highlight::highlight_code_with_multi_cursor(
        &target_content,
        &target_cursors,
        &target_selections,
    );
    let target = Paragraph::new(target_lines)
        .block(
            Block::default()
                .title(target_title)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Green)),
        )
        .scroll((0, h_off));
    frame.render_widget(target, target_area);
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

    // ==================== render_editor_pair panel-width regression tests (#192) ====================
    //
    // Percentage(50)/Percentage(50) used to hand the odd leftover column to one panel
    // unpredictably, so Current/Target wrapped the same line differently and desynced
    // their rendered rows. These tests pin the invariant that both panels always get
    // identical widths. Odd widths are the load-bearing case (see the odd-width test
    // below); even widths are a baseline sanity check only.

    fn render_editor_pair_buffer(area_width: u16) -> ratatui::buffer::Buffer {
        use crate::game::GameSession;
        use crate::testing::ScenarioBuilder;
        use ratatui::{Terminal, backend::TestBackend};

        let scenario = ScenarioBuilder::new()
            .id("panel_width_test")
            .setup_content("fn calculate_total(prices: &[f64]) -> f64 {\n")
            .target_content("fn calculate_total(prices: &[f64]) -> f64 {\n")
            .build();
        let session = GameSession::new(scenario).unwrap();

        let backend = TestBackend::new(area_width, 10);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                let area = frame.area();
                render_editor_pair(frame, area, &session, " Current ", " Target ", None);
            })
            .unwrap();
        terminal.backend().buffer().clone()
    }

    /// Scans the top border row for the `┌`/`┐` corners of both panels and returns
    /// their widths in left-to-right (Current, Target) order.
    fn top_border_panel_widths(buffer: &ratatui::buffer::Buffer) -> (u16, u16) {
        let width = buffer.area.width;
        let mut corners = Vec::new();
        for x in 0..width {
            let symbol = buffer[(x, 0)].symbol();
            if symbol == "┌" || symbol == "┐" {
                corners.push(x);
            }
        }
        assert_eq!(
            corners.len(),
            4,
            "expected exactly 2 top-left + 2 top-right border corners on row 0, found at columns {corners:?}"
        );
        let current_width = corners[1] - corners[0] + 1;
        let target_width = corners[3] - corners[2] + 1;
        (current_width, target_width)
    }

    #[test]
    fn test_render_editor_pair_panels_have_equal_width_at_odd_terminal_widths() {
        // ODD widths are the only case that distinguishes the fix from the old
        // Percentage(50)/Percentage(50) split: at EVEN widths both implementations
        // divide evenly and this assertion would pass either way (see critic finding
        // S1, .local/handoff/2026-08-09T17-42-20-critic.md) - it is the odd-width
        // case that actually exercises the leftover-column handling.
        for area_width in [41u16, 43, 81, 89, 91, 99, 121, 201] {
            assert!(area_width % 2 == 1, "test bug: {area_width} is not odd");
            let buffer = render_editor_pair_buffer(area_width);
            let (current_width, target_width) = top_border_panel_widths(&buffer);
            assert_eq!(
                current_width, target_width,
                "Current/Target panel widths differ at odd terminal width {area_width}: \
                 current={current_width}, target={target_width}"
            );
        }
    }

    #[test]
    fn test_render_editor_pair_panels_have_equal_width_at_even_terminal_widths() {
        // EVEN widths split evenly under both the old and new implementations, so this
        // is a baseline sanity check, not a regression test for #192 by itself -
        // the odd-width test above is what actually catches a reverted fix.
        for area_width in [20u16, 42, 80, 90, 100, 200] {
            assert!(area_width % 2 == 0, "test bug: {area_width} is not even");
            let buffer = render_editor_pair_buffer(area_width);
            let (current_width, target_width) = top_border_panel_widths(&buffer);
            assert_eq!(
                current_width, target_width,
                "Current/Target panel widths differ at even terminal width {area_width}: \
                 current={current_width}, target={target_width}"
            );
        }
    }

    #[test]
    fn test_render_editor_pair_odd_width_regression_91() {
        // Direct regression test for the exact live-tester repro from issue #192:
        // total terminal width 91 (editor area 89 cols) -> Current inner width 43,
        // Target inner width 42 under the old Percentage(50)/Percentage(50) split.
        let buffer = render_editor_pair_buffer(91);
        let (current_width, target_width) = top_border_panel_widths(&buffer);
        assert_eq!(
            current_width, target_width,
            "issue #192 regressed: Current={current_width}, Target={target_width} at width 91"
        );
    }

    // ==================== render_editor_pair cursor-past-EOL wrap desync regression (#333) ====================

    #[test]
    fn test_render_editor_pair_cursor_past_eol_at_panel_width_does_not_desync_rows() {
        use crate::game::{GameSession, SessionAfterAction};
        use crate::testing::ScenarioBuilder;
        use ratatui::{Terminal, backend::TestBackend};

        // Even area width so half_width divides exactly; inner content width per panel is
        // half_width - 2 (borders). Craft the first line to exactly fill that inner width so
        // appending the past-EOL cursor block character (see `render_line_with_multi_cursor`)
        // makes Current's row one cell wider than Target's - the exact condition that used
        // to make `Wrap` soft-wrap Current but not Target and desync every following row.
        let area_width = 42u16;
        let inner_width = (area_width / 2 - 2) as usize;
        let first_line = "x".repeat(inner_width);
        let content = format!("{first_line}\nsecond\n");

        let scenario = ScenarioBuilder::new()
            .id("panel_wrap_desync_test")
            .setup_content(content.clone())
            .target_content(content)
            .build();
        let session = GameSession::new(scenario).unwrap();

        // "A" (append at end of line) puts the cursor at col == line length, i.e. past-EOL.
        let session = match session.record_action("A".to_string()).unwrap() {
            SessionAfterAction::StillActive(session) => session,
            SessionAfterAction::Completed(_) => panic!("expected session to remain active"),
        };
        assert_eq!(session.all_cursors(), vec![(0, inner_width)]);

        let backend = TestBackend::new(area_width, 10);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                let area = frame.area();
                render_editor_pair(frame, area, &session, " Current ", " Target ", None);
            })
            .unwrap();
        let buffer = terminal.backend().buffer().clone();

        // Row 0 is the top border, row 1 is the first content line; "second" must land on
        // row 2 in BOTH panels. Before the fix, Current's soft-wrapped first line pushed
        // "second" down to row 3 in Current only, desyncing it from Target.
        let row_containing = |x_start: u16, x_end: u16, text: &str| -> Option<u16> {
            (0..buffer.area.height).find(|&y| {
                (x_start..x_end)
                    .map(|x| buffer[(x, y)].symbol())
                    .collect::<String>()
                    .contains(text)
            })
        };

        // At this exact-fit boundary the cursor sits one column past the visible window
        // (see the S1 horizontal-scroll fix below), so h_off == 1 here. The shared scroll
        // shifts every row uniformly, including "second", which loses its leading "s".
        let h_off = inner_width.saturating_sub(inner_width) + 1;
        let expected_second = &"second"[h_off.min("second".len())..];

        let half_width = area_width / 2;
        let current_second_row = row_containing(0, half_width, expected_second)
            .unwrap_or_else(|| panic!("{expected_second:?} not found in Current panel"));
        let target_second_row = row_containing(half_width, area_width, expected_second)
            .unwrap_or_else(|| panic!("{expected_second:?} not found in Target panel"));

        assert_eq!(
            current_second_row, 2,
            "Current panel wrapped the cursor-past-EOL line into an extra row"
        );
        assert_eq!(
            current_second_row, target_second_row,
            "Current/Target rows desynced: current has 'second' at row {current_second_row}, \
             target at row {target_second_row}"
        );
    }

    // ==================== render_editor_pair horizontal follow-cursor scroll (#333 S1) ====================

    #[test]
    fn test_render_editor_pair_long_line_scrolls_to_keep_cursor_visible() {
        use crate::game::{GameSession, SessionAfterAction};
        use crate::testing::ScenarioBuilder;
        use ratatui::{Terminal, backend::TestBackend};

        // First line is longer than the panel's inner width, so without horizontal scroll
        // the truncated tail (including a past-EOL cursor placed there) would be invisible.
        // The second line is a distinct marker long enough to survive the resulting shift
        // (the shared h_off scrolls every row, not just the cursor's).
        let area_width = 42u16;
        let inner_width = (area_width / 2 - 2) as usize;
        let first_line = "x".repeat(inner_width + 8);
        let second_line = "second_line_marker_0123456789";
        let content = format!("{first_line}\n{second_line}\n");

        let scenario = ScenarioBuilder::new()
            .id("panel_long_line_scroll_test")
            .setup_content(content.clone())
            .target_content(content)
            .build();
        let session = GameSession::new(scenario).unwrap();

        // "A" (append at end of line) puts the cursor past-EOL, beyond the inner width.
        let session = match session.record_action("A".to_string()).unwrap() {
            SessionAfterAction::StillActive(session) => session,
            SessionAfterAction::Completed(_) => panic!("expected session to remain active"),
        };
        let cursor_col = inner_width + 8;
        assert_eq!(session.all_cursors(), vec![(0, cursor_col)]);

        let backend = TestBackend::new(area_width, 10);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                let area = frame.area();
                render_editor_pair(frame, area, &session, " Current ", " Target ", None);
            })
            .unwrap();
        let buffer = terminal.backend().buffer().clone();

        // Rows must still stay in sync between the two panels even though the first line
        // overruns the panel width - no wrap means row count is unaffected by scrolling.
        // The shared h_off shifts the second line too, so search for its visible remainder
        // rather than the full marker text.
        let h_off = cursor_col - inner_width + 1;
        // Clip to the visible window: content past `inner_width` columns from `h_off` is
        // truncated (no wrap), so searching for more than that would never be found.
        let visible_end = (h_off + inner_width).min(second_line.len());
        let expected_second = &second_line[h_off.min(second_line.len())..visible_end];
        assert!(
            !expected_second.is_empty(),
            "test bug: second_line_marker too short to survive h_off={h_off}"
        );

        let half_width = area_width / 2;
        let row_containing = |x_start: u16, x_end: u16, text: &str| -> Option<u16> {
            (0..buffer.area.height).find(|&y| {
                (x_start..x_end)
                    .map(|x| buffer[(x, y)].symbol())
                    .collect::<String>()
                    .contains(text)
            })
        };
        let current_second_row = row_containing(0, half_width, expected_second)
            .unwrap_or_else(|| panic!("{expected_second:?} not found in Current panel"));
        let target_second_row = row_containing(half_width, area_width, expected_second)
            .unwrap_or_else(|| panic!("{expected_second:?} not found in Target panel"));
        assert_eq!(
            current_second_row, target_second_row,
            "Current/Target rows desynced with a long first line: current row \
             {current_second_row}, target row {target_second_row}"
        );

        // The past-EOL cursor block must actually be rendered on screen: with h_off =
        // cursor_col - inner_width + 1, its local column inside the panel is
        // cursor_col - h_off = inner_width - 1, the last visible column.
        let local_col = cursor_col - h_off;
        assert_eq!(local_col, inner_width - 1);
        let cursor_x = 1 + local_col as u16; // +1 for the left border
        let cursor_y = 1u16; // row 0 is the top border, row 1 is the first content line
        assert_eq!(
            buffer[(cursor_x, cursor_y)].symbol(),
            "\u{2588}",
            "past-EOL cursor block not visible at expected column {cursor_x} - \
             horizontal scroll did not bring it into view"
        );
    }
}
