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
    /// Performs order-independent selection comparison for multi-cursor scenarios.
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

        // Multi-selection comparison (order-independent)
        if self.selections.len() != other.selections.len() {
            return false;
        }

        // Sort both for order-independent comparison
        let mut self_sorted: Vec<_> = self.selections.clone();
        let mut other_sorted: Vec<_> = other.selections.clone();

        self_sorted.sort_by_key(|r| (r.anchor.min(r.head), r.anchor.max(r.head)));
        other_sorted.sort_by_key(|r| (r.anchor.min(r.head), r.anchor.max(r.head)));

        self_sorted == other_sorted
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
}
