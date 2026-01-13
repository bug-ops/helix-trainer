//! Editor state snapshot for serialization and comparison.
//!
//! This module provides minimal types for capturing editor state:
//! - `EditorSnapshot`: Serializable snapshot of document and selection
//! - `EditorDisplay`: Zero-cost facade for UI rendering with row/col conversion
//!
//! These types exist to decouple serialization and UI concerns from the
//! helix-core primitives used internally by `HelixSimulator`.

use helix_core::{Range, Rope, Selection};
use serde::{Deserialize, Serialize};

/// Minimal serializable snapshot of editor state.
///
/// Used for:
/// - Scenario target state comparison
/// - Session state persistence (saves)
/// - Test assertions
///
/// Unlike `EditorState`, this is a thin data container without validation logic.
/// The actual editor state lives in `HelixSimulator` using helix-core primitives.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EditorSnapshot {
    /// Document content as string
    pub content: String,
    /// All selections in serializable format
    pub selections: Vec<SerializableRange>,
    /// Index of the primary selection
    pub primary_idx: usize,
}

/// Serializable representation of a helix_core::Range.
///
/// Stores anchor and head as char offsets (not row/col) for direct
/// compatibility with helix-core.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SerializableRange {
    /// Anchor position (start of selection, fixed point)
    pub anchor: usize,
    /// Head position (cursor position, moves during selection)
    pub head: usize,
}

impl SerializableRange {
    /// Create a new range from anchor and head positions.
    #[inline]
    pub fn new(anchor: usize, head: usize) -> Self {
        Self { anchor, head }
    }

    /// Create a point selection (cursor with no selection).
    #[inline]
    pub fn point(pos: usize) -> Self {
        Self {
            anchor: pos,
            head: pos,
        }
    }

    /// Convert to helix_core::Range.
    #[inline]
    pub fn to_range(self) -> Range {
        Range::new(self.anchor, self.head)
    }

    /// Create from helix_core::Range.
    #[inline]
    pub fn from_range(range: &Range) -> Self {
        Self {
            anchor: range.anchor,
            head: range.head,
        }
    }

    /// Check if this is a point selection (no actual selection).
    #[inline]
    pub fn is_point(&self) -> bool {
        self.anchor == self.head
    }
}

impl EditorSnapshot {
    /// Create a snapshot from content and a single cursor position (char offset).
    pub fn with_cursor(content: String, cursor_offset: usize) -> Self {
        Self {
            content,
            selections: vec![SerializableRange::point(cursor_offset)],
            primary_idx: 0,
        }
    }

    /// Create a snapshot from content and selections.
    pub fn with_selections(
        content: String,
        selections: Vec<SerializableRange>,
        primary_idx: usize,
    ) -> Self {
        Self {
            content,
            selections,
            primary_idx,
        }
    }

    /// Create from Rope and Selection (helix-core types).
    pub fn from_helix(doc: &Rope, selection: &Selection) -> Self {
        let content = doc.to_string();
        let selections: Vec<SerializableRange> = selection
            .ranges()
            .iter()
            .map(SerializableRange::from_range)
            .collect();
        let primary_idx = selection.primary_index();

        Self {
            content,
            selections,
            primary_idx,
        }
    }

    /// Convert selections to helix_core::Selection.
    ///
    /// # Panics
    ///
    /// Panics if selections is empty (Selection requires at least one range).
    pub fn to_helix_selection(&self) -> Selection {
        if self.selections.is_empty() {
            return Selection::point(0);
        }

        let ranges: Vec<Range> = self.selections.iter().map(|r| r.to_range()).collect();
        Selection::new(
            ranges.into(),
            self.primary_idx.min(self.selections.len() - 1),
        )
    }

    /// Get primary cursor position (head of primary selection).
    #[inline]
    pub fn cursor_offset(&self) -> usize {
        self.selections
            .get(self.primary_idx)
            .map(|r| r.head)
            .unwrap_or(0)
    }

    /// Check if content matches another snapshot (ignoring cursor/selection).
    #[inline]
    pub fn content_matches(&self, other: &EditorSnapshot) -> bool {
        self.content == other.content
    }

    /// Check if this snapshot matches another (content + selections).
    ///
    /// Performs order-independent and direction-independent selection comparison.
    /// This handles the case where helix-core might have (anchor=27, head=12) while
    /// the TOML target specifies (anchor=12, head=27) - both represent the same range.
    pub fn matches(&self, other: &EditorSnapshot) -> bool {
        // Content must match
        if self.content != other.content {
            return false;
        }

        // If target has no selections or only point selections, just compare content
        let other_has_real_selections = other.selections.iter().any(|s| !s.is_point());
        if !other_has_real_selections && other.selections.len() <= 1 {
            // Simple cursor comparison for single-cursor scenarios
            return self.cursor_offset() == other.cursor_offset();
        }

        // Multi-selection comparison (order-independent and direction-independent)
        if self.selections.len() != other.selections.len() {
            return false;
        }

        // Normalize both sets to (min, max) tuples for comparison
        // This ignores selection direction (anchor vs head)
        let normalize = |r: &SerializableRange| (r.anchor.min(r.head), r.anchor.max(r.head));

        let mut self_normalized: Vec<_> = self.selections.iter().map(normalize).collect();
        let mut other_normalized: Vec<_> = other.selections.iter().map(normalize).collect();

        self_normalized.sort();
        other_normalized.sort();

        self_normalized == other_normalized
    }
}

/// Selection bounds for UI rendering.
///
/// Simple struct holding row/col coordinates for start and end of selection.
/// Used to decouple UI rendering from helix-core types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SelectionBounds {
    /// Start row (0-indexed)
    pub start_row: usize,
    /// Start column (0-indexed)
    pub start_col: usize,
    /// End row (0-indexed)
    pub end_row: usize,
    /// End column (0-indexed)
    pub end_col: usize,
}

impl SelectionBounds {
    /// Create new selection bounds.
    #[inline]
    pub fn new(start_row: usize, start_col: usize, end_row: usize, end_col: usize) -> Self {
        Self {
            start_row,
            start_col,
            end_row,
            end_col,
        }
    }
}

/// Zero-cost display facade for UI rendering.
///
/// Provides row/col conversion from char offsets without copying data.
/// Used only at the UI boundary for rendering cursor positions.
pub struct EditorDisplay<'a> {
    doc: &'a Rope,
    selection: &'a Selection,
}

impl<'a> EditorDisplay<'a> {
    /// Create a new display facade.
    pub fn new(doc: &'a Rope, selection: &'a Selection) -> Self {
        Self { doc, selection }
    }

    /// Get primary cursor position as (row, col).
    ///
    /// This is the only place where row/col conversion happens.
    pub fn cursor_position(&self) -> (usize, usize) {
        let head = self.selection.primary().head;
        self.char_to_row_col(head)
    }

    /// Get all cursor positions as (row, col) pairs.
    pub fn all_cursor_positions(&self) -> Vec<(usize, usize)> {
        self.selection
            .ranges()
            .iter()
            .map(|range| self.char_to_row_col(range.head))
            .collect()
    }

    /// Get primary selection bounds as ((start_row, start_col), (end_row, end_col)).
    ///
    /// Returns None if primary selection is a point (no actual selection).
    pub fn primary_selection_bounds(&self) -> Option<((usize, usize), (usize, usize))> {
        let range = self.selection.primary();
        if range.anchor == range.head {
            return None;
        }

        let anchor_pos = self.char_to_row_col(range.anchor);
        let head_pos = self.char_to_row_col(range.head);

        // Normalize to (start, end)
        if range.anchor < range.head {
            Some((anchor_pos, head_pos))
        } else {
            Some((head_pos, anchor_pos))
        }
    }

    /// Get primary selection as SelectionBounds for UI rendering.
    ///
    /// Returns None if primary selection is a point (no actual selection).
    /// This is the preferred method for UI code.
    pub fn selection(&self) -> Option<SelectionBounds> {
        let range = self.selection.primary();
        if range.anchor == range.head {
            return None;
        }

        let anchor_pos = self.char_to_row_col(range.anchor);
        let head_pos = self.char_to_row_col(range.head);

        // Normalize to (start, end)
        if range.anchor < range.head {
            Some(SelectionBounds::new(
                anchor_pos.0,
                anchor_pos.1,
                head_pos.0,
                head_pos.1,
            ))
        } else {
            Some(SelectionBounds::new(
                head_pos.0,
                head_pos.1,
                anchor_pos.0,
                anchor_pos.1,
            ))
        }
    }

    /// Get all selection bounds.
    pub fn all_selection_bounds(&self) -> Vec<((usize, usize), (usize, usize))> {
        self.selection
            .ranges()
            .iter()
            .filter(|range| range.anchor != range.head)
            .map(|range| {
                let anchor_pos = self.char_to_row_col(range.anchor);
                let head_pos = self.char_to_row_col(range.head);
                if range.anchor < range.head {
                    (anchor_pos, head_pos)
                } else {
                    (head_pos, anchor_pos)
                }
            })
            .collect()
    }

    /// Get document content for display.
    ///
    /// Note: This allocates a String. For large documents, consider
    /// iterating over chunks instead.
    #[inline]
    pub fn content(&self) -> String {
        self.doc.to_string()
    }

    /// Get number of lines in document.
    #[inline]
    pub fn line_count(&self) -> usize {
        self.doc.len_lines()
    }

    /// Get a specific line by index.
    pub fn line(&self, idx: usize) -> Option<String> {
        if idx < self.doc.len_lines() {
            Some(self.doc.line(idx).to_string())
        } else {
            None
        }
    }

    /// Convert char offset to (row, col).
    fn char_to_row_col(&self, char_idx: usize) -> (usize, usize) {
        let clamped = char_idx.min(self.doc.len_chars());
        let line = self.doc.char_to_line(clamped);
        let line_start = self.doc.line_to_char(line);
        let col = clamped - line_start;
        (line, col)
    }
}

impl EditorSnapshot {
    /// Create a snapshot from scenario config with row/col cursor position.
    ///
    /// Converts row/col coordinates to char offsets using the content.
    /// This is the entry point for loading scenarios into the helix-core representation.
    ///
    /// # Arguments
    /// * `content` - Document content
    /// * `cursor_row` - Row (0-indexed)
    /// * `cursor_col` - Column (0-indexed)
    pub fn from_row_col(content: String, cursor_row: usize, cursor_col: usize) -> Self {
        let char_offset = row_col_to_char_offset(&content, cursor_row, cursor_col);
        Self::with_cursor(content, char_offset)
    }

    /// Create a snapshot from scenario config with row/col selection.
    ///
    /// Converts row/col coordinates to char offsets.
    ///
    /// # Arguments
    /// * `content` - Document content
    /// * `sel` - Selection as [start_row, start_col, end_row, end_col]
    pub fn from_row_col_selection(content: String, sel: [usize; 4]) -> Self {
        let anchor = row_col_to_char_offset(&content, sel[0], sel[1]);
        let head = row_col_to_char_offset(&content, sel[2], sel[3]);
        Self::with_selections(content, vec![SerializableRange::new(anchor, head)], 0)
    }

    /// Create from scenario config with multiple cursors (point selections).
    pub fn from_multi_cursor(content: String, cursors: &[[usize; 2]]) -> Self {
        if cursors.is_empty() {
            return Self::with_cursor(content, 0);
        }

        let selections: Vec<SerializableRange> = cursors
            .iter()
            .map(|c| {
                let offset = row_col_to_char_offset(&content, c[0], c[1]);
                SerializableRange::point(offset)
            })
            .collect();

        Self::with_selections(content, selections, 0)
    }

    /// Create from scenario config with multiple selections.
    pub fn from_multi_selection(content: String, selections: &[[usize; 4]]) -> Self {
        if selections.is_empty() {
            return Self::with_cursor(content, 0);
        }

        let ranges: Vec<SerializableRange> = selections
            .iter()
            .map(|s| {
                let anchor = row_col_to_char_offset(&content, s[0], s[1]);
                let head = row_col_to_char_offset(&content, s[2], s[3]);
                SerializableRange::new(anchor, head)
            })
            .collect();

        Self::with_selections(content, ranges, 0)
    }

    /// Create from scenario Setup or TargetState config.
    ///
    /// Handles all cursor/selection formats (single, multi-cursor, multi-selection).
    pub fn from_scenario_config(
        content: String,
        cursor_position: Option<(usize, usize)>,
        selection: Option<[usize; 4]>,
        cursors: Option<&[[usize; 2]]>,
        selections: Option<&[[usize; 4]]>,
    ) -> Self {
        // Multi-selection takes priority
        if let Some(sels) = selections {
            return Self::from_multi_selection(content, sels);
        }

        // Multi-cursor next
        if let Some(curs) = cursors {
            return Self::from_multi_cursor(content, curs);
        }

        // Single selection
        if let Some(sel) = selection {
            return Self::from_row_col_selection(content, sel);
        }

        // Single cursor (or default)
        let (row, col) = cursor_position.unwrap_or((0, 0));
        Self::from_row_col(content, row, col)
    }
}

/// Convert (row, col) to char offset in content.
///
/// This is the central conversion function for row/col -> char offset.
/// Used when loading scenarios from TOML format.
fn row_col_to_char_offset(content: &str, row: usize, col: usize) -> usize {
    let mut char_offset = 0;
    for (line_idx, line) in content.lines().enumerate() {
        if line_idx == row {
            // Clamp column to line length
            let safe_col = col.min(line.chars().count());
            return char_offset + safe_col;
        }
        // Add line length plus newline
        char_offset += line.chars().count() + 1;
    }
    // If row exceeds line count, return end of content
    content.chars().count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serializable_range_point() {
        let range = SerializableRange::point(5);
        assert!(range.is_point());
        assert_eq!(range.anchor, 5);
        assert_eq!(range.head, 5);
    }

    #[test]
    fn test_serializable_range_selection() {
        let range = SerializableRange::new(0, 10);
        assert!(!range.is_point());
        assert_eq!(range.anchor, 0);
        assert_eq!(range.head, 10);
    }

    #[test]
    fn test_snapshot_with_cursor() {
        let snap = EditorSnapshot::with_cursor("hello".to_string(), 3);
        assert_eq!(snap.content, "hello");
        assert_eq!(snap.cursor_offset(), 3);
        assert_eq!(snap.selections.len(), 1);
        assert!(snap.selections[0].is_point());
    }

    #[test]
    fn test_snapshot_matches_content_only() {
        let snap1 = EditorSnapshot::with_cursor("hello".to_string(), 0);
        let snap2 = EditorSnapshot::with_cursor("hello".to_string(), 3);

        // Different cursor positions but same content
        assert!(snap1.content_matches(&snap2));
    }

    #[test]
    fn test_snapshot_matches_exact() {
        let snap1 = EditorSnapshot::with_cursor("hello".to_string(), 3);
        let snap2 = EditorSnapshot::with_cursor("hello".to_string(), 3);

        assert!(snap1.matches(&snap2));
    }

    #[test]
    fn test_snapshot_matches_different_cursor() {
        let snap1 = EditorSnapshot::with_cursor("hello".to_string(), 0);
        let snap2 = EditorSnapshot::with_cursor("hello".to_string(), 3);

        // Single-cursor scenarios: cursor must match
        assert!(!snap1.matches(&snap2));
    }

    #[test]
    fn test_snapshot_matches_multi_selection_order_independent() {
        let snap1 = EditorSnapshot::with_selections(
            "hello world".to_string(),
            vec![SerializableRange::new(0, 5), SerializableRange::new(6, 11)],
            0,
        );
        let snap2 = EditorSnapshot::with_selections(
            "hello world".to_string(),
            vec![SerializableRange::new(6, 11), SerializableRange::new(0, 5)],
            1,
        );

        // Order-independent comparison for multi-selection
        assert!(snap1.matches(&snap2));
    }

    #[test]
    fn test_display_cursor_position() {
        let rope = Rope::from("line 1\nline 2\nline 3");
        let selection = Selection::point(7); // Start of "line 2"

        let display = EditorDisplay::new(&rope, &selection);
        let (row, col) = display.cursor_position();

        assert_eq!(row, 1);
        assert_eq!(col, 0);
    }

    #[test]
    fn test_display_cursor_position_middle_of_line() {
        let rope = Rope::from("hello world");
        let selection = Selection::point(6); // On 'w'

        let display = EditorDisplay::new(&rope, &selection);
        let (row, col) = display.cursor_position();

        assert_eq!(row, 0);
        assert_eq!(col, 6);
    }

    #[test]
    fn test_display_selection_bounds() {
        let rope = Rope::from("hello world");
        let selection = Selection::single(0, 5); // "hello"

        let display = EditorDisplay::new(&rope, &selection);
        let bounds = display.primary_selection_bounds();

        assert!(bounds.is_some());
        let ((sr, sc), (er, ec)) = bounds.unwrap();
        assert_eq!((sr, sc), (0, 0));
        assert_eq!((er, ec), (0, 5));
    }

    #[test]
    fn test_display_no_selection_for_point() {
        let rope = Rope::from("hello");
        let selection = Selection::point(3);

        let display = EditorDisplay::new(&rope, &selection);
        let bounds = display.primary_selection_bounds();

        assert!(bounds.is_none());
    }

    #[test]
    fn test_snapshot_from_helix() {
        let rope = Rope::from("test content");
        let selection = Selection::single(0, 4);

        let snap = EditorSnapshot::from_helix(&rope, &selection);

        assert_eq!(snap.content, "test content");
        assert_eq!(snap.selections.len(), 1);
        assert_eq!(snap.selections[0].anchor, 0);
        assert_eq!(snap.selections[0].head, 4);
    }

    #[test]
    fn test_snapshot_to_helix_selection() {
        let snap = EditorSnapshot::with_selections(
            "test".to_string(),
            vec![SerializableRange::new(0, 2), SerializableRange::new(2, 4)],
            1,
        );

        let selection = snap.to_helix_selection();

        assert_eq!(selection.ranges().len(), 2);
        assert_eq!(selection.primary_index(), 1);
    }

    // ==================== row_col_to_char_offset tests ====================

    #[test]
    fn test_row_col_to_char_offset_first_line() {
        let content = "hello\nworld";
        assert_eq!(row_col_to_char_offset(content, 0, 0), 0);
        assert_eq!(row_col_to_char_offset(content, 0, 3), 3);
        assert_eq!(row_col_to_char_offset(content, 0, 5), 5);
    }

    #[test]
    fn test_row_col_to_char_offset_second_line() {
        let content = "hello\nworld";
        // Second line starts at char 6 (5 chars + 1 newline)
        assert_eq!(row_col_to_char_offset(content, 1, 0), 6);
        assert_eq!(row_col_to_char_offset(content, 1, 3), 9);
    }

    #[test]
    fn test_row_col_to_char_offset_clamps_column() {
        let content = "hi\nworld";
        // Line 0 has 2 chars, so col 10 should clamp to 2
        assert_eq!(row_col_to_char_offset(content, 0, 10), 2);
    }

    #[test]
    fn test_row_col_to_char_offset_beyond_lines() {
        let content = "hello";
        // Row 5 doesn't exist, should return end of content
        assert_eq!(row_col_to_char_offset(content, 5, 0), 5);
    }

    // ==================== EditorSnapshot::from_row_col tests ====================

    #[test]
    fn test_snapshot_from_row_col() {
        let snap = EditorSnapshot::from_row_col("line 1\nline 2".to_string(), 1, 2);

        assert_eq!(snap.content, "line 1\nline 2");
        // Line 1 starts at char 7, plus col 2 = char 9
        assert_eq!(snap.cursor_offset(), 9);
    }

    #[test]
    fn test_snapshot_from_row_col_selection() {
        let snap = EditorSnapshot::from_row_col_selection(
            "hello world".to_string(),
            [0, 0, 0, 5], // Select "hello"
        );

        assert_eq!(snap.selections.len(), 1);
        assert_eq!(snap.selections[0].anchor, 0);
        assert_eq!(snap.selections[0].head, 5);
    }

    #[test]
    fn test_snapshot_from_multi_cursor() {
        let snap = EditorSnapshot::from_multi_cursor("hello\nworld".to_string(), &[[0, 0], [1, 0]]);

        assert_eq!(snap.selections.len(), 2);
        assert_eq!(snap.selections[0].anchor, 0);
        assert_eq!(snap.selections[0].head, 0);
        assert_eq!(snap.selections[1].anchor, 6);
        assert_eq!(snap.selections[1].head, 6);
    }

    #[test]
    fn test_snapshot_from_multi_selection() {
        let snap = EditorSnapshot::from_multi_selection(
            "hello world".to_string(),
            &[[0, 0, 0, 5], [0, 6, 0, 11]],
        );

        assert_eq!(snap.selections.len(), 2);
        assert_eq!(snap.selections[0].anchor, 0);
        assert_eq!(snap.selections[0].head, 5);
        assert_eq!(snap.selections[1].anchor, 6);
        assert_eq!(snap.selections[1].head, 11);
    }

    #[test]
    fn test_snapshot_from_scenario_config_single_cursor() {
        let snap = EditorSnapshot::from_scenario_config(
            "test".to_string(),
            Some((0, 2)),
            None,
            None,
            None,
        );

        assert_eq!(snap.cursor_offset(), 2);
        assert_eq!(snap.selections.len(), 1);
    }

    #[test]
    fn test_snapshot_from_scenario_config_multi_selection_priority() {
        // Multi-selection should take priority over cursor_position
        let snap = EditorSnapshot::from_scenario_config(
            "hello world".to_string(),
            Some((0, 0)),                         // cursor
            None,                                 // selection
            None,                                 // cursors
            Some(&[[0, 0, 0, 5], [0, 6, 0, 11]]), // multi-selections
        );

        // Should use multi-selections, not cursor_position
        assert_eq!(snap.selections.len(), 2);
    }

    #[test]
    fn test_scenario_line_selection_conversion() {
        // Reproduce the select_line_001 scenario issue
        // Target: selection = [1, 0, 2, 0] means row 1 col 0 to row 2 col 0
        let content = "fn main() {\n    let x = 1;\n    let y = 2;\n}";

        // Target snapshot from TOML config
        let target_snap = EditorSnapshot::from_scenario_config(
            content.to_string(),
            Some((1, 0)),       // cursor_position
            Some([1, 0, 2, 0]), // selection
            None,               // cursors
            None,               // selections
        );

        // Calculate what the char offsets should be:
        // "fn main() {\n" = 12 chars (11 + 1 newline)
        // Row 1: "    let x = 1;\n" = 15 chars (14 + 1 newline)
        // Row 1 starts at char 12
        // Row 2 starts at char 12 + 15 = 27

        assert_eq!(target_snap.selections.len(), 1);
        assert_eq!(
            target_snap.selections[0].anchor, 12,
            "anchor should be start of row 1"
        );
        assert_eq!(
            target_snap.selections[0].head, 27,
            "head should be start of row 2"
        );
    }

    #[test]
    fn test_scenario_line_selection_matching() {
        // Test that a simulator state matches the target
        let content = "fn main() {\n    let x = 1;\n    let y = 2;\n}";

        // Target: line selection from row 1 to row 2
        let target = EditorSnapshot::from_scenario_config(
            content.to_string(),
            Some((1, 0)),
            Some([1, 0, 2, 0]),
            None,
            None,
        );

        // Current: same selection (as if we executed 'x' on line 1)
        // Row 1 starts at char 12, Row 2 starts at char 27
        let current = EditorSnapshot::with_selections(
            content.to_string(),
            vec![SerializableRange::new(12, 27)],
            0,
        );

        assert!(current.matches(&target), "Same selection should match");
    }

    #[test]
    fn test_helix_select_line_matches_toml_target() {
        // Verify that helix-core 'x' command result matches TOML target
        // even when anchor/head are in different order
        let content = "fn main() {\n    let x = 1;\n    let y = 2;\n}";

        use crate::helix::HelixSimulator;
        let mut sim = HelixSimulator::new(content.to_string());

        // Position at row 1, col 6 (char 18)
        sim.selection = helix_core::Selection::point(18);

        // Execute select line command
        use crate::helix::registry::normal_registry;
        normal_registry().execute(&mut sim, "x").unwrap();

        let current_snap = sim.to_snapshot();

        // Target from TOML: selection = [1, 0, 2, 0]
        let target_snap = EditorSnapshot::from_scenario_config(
            content.to_string(),
            Some((1, 0)),
            Some([1, 0, 2, 0]),
            None,
            None,
        );

        // Helix produces (anchor=27, head=12), TOML produces (anchor=12, head=27)
        // These should match because they represent the same range
        assert!(current_snap.matches(&target_snap));
    }
}
