//! Editor state representation and management.
//!
//! This module defines the EditorState type which represents the state of a text editor
//! at a given moment, including file content, cursor position, and optional selection.
//!
//! All operations validate against security limits and ensure bounds correctness.
//!
//! # Examples
//!
//! ```ignore
//! use helix_trainer::game::EditorState;
//! use helix_trainer::game::CursorPosition;
//!
//! let content = "line 1\nline 2\nline 3\n".to_string();
//! let cursor = CursorPosition::new(1, 0)?;
//! let state = EditorState::new(content, cursor, None)?;
//!
//! assert_eq!(state.line_count(), 3);
//! assert_eq!(state.current_line(), Some("line 2"));
//! # Ok::<(), helix_trainer::security::SecurityError>(())
//! ```

use crate::security::{self, SecurityError};
use serde::{Deserialize, Serialize};

/// Represents the state of the text editor at a given moment.
///
/// Stores complete file content, cursor position, and optional text selection.
/// All operations validate bounds to ensure the state remains consistent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EditorState {
    /// The complete file content
    content: String,
    /// Cursor position as (row, col) - both 0-indexed
    cursor_pos: CursorPosition,
    /// Optional selection range (start_row, start_col, end_row, end_col)
    selection: Option<Selection>,
}

/// Cursor position with validated bounds.
///
/// Represents a position in the text as (row, column) coordinates, both 0-indexed.
/// Validation ensures cursor positions stay within reasonable bounds relative to content size.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CursorPosition {
    /// Row index (0-indexed)
    pub row: usize,
    /// Column index (0-indexed)
    pub col: usize,
}

impl CursorPosition {
    /// Create a new cursor position with validation.
    ///
    /// # Errors
    ///
    /// Returns `InvalidCursorPosition` if row or col exceeds reasonable bounds.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use helix_trainer::game::CursorPosition;
    ///
    /// let pos = CursorPosition::new(0, 5)?;
    /// assert_eq!(pos.row, 0);
    /// assert_eq!(pos.col, 5);
    /// # Ok::<(), helix_trainer::security::SecurityError>(())
    /// ```
    pub fn new(row: usize, col: usize) -> Result<Self, SecurityError> {
        // Use a reasonable default max content size for initial validation
        const DEFAULT_MAX_CONTENT: usize = 100_000;
        security::arithmetic::validate_cursor_position(row, col, DEFAULT_MAX_CONTENT)?;
        Ok(Self { row, col })
    }

    /// Create from array `[row, col]`.
    ///
    /// # Errors
    ///
    /// Returns `InvalidCursorPosition` if position is invalid.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use helix_trainer::game::CursorPosition;
    ///
    /// let pos = CursorPosition::from_array([1, 3])?;
    /// assert_eq!(pos.to_array(), [1, 3]);
    /// # Ok::<(), helix_trainer::security::SecurityError>(())
    /// ```
    pub fn from_array(pos: [usize; 2]) -> Result<Self, SecurityError> {
        Self::new(pos[0], pos[1])
    }

    /// Convert to array `[row, col]`.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use helix_trainer::game::CursorPosition;
    ///
    /// let pos = CursorPosition::new(1, 5)?;
    /// assert_eq!(pos.to_array(), [1, 5]);
    /// # Ok::<(), helix_trainer::security::SecurityError>(())
    /// ```
    pub fn to_array(&self) -> [usize; 2] {
        [self.row, self.col]
    }
}

/// Text selection range.
///
/// Represents a selection of text between two cursor positions.
/// The selection is stored as-is (start position may come after end position).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Selection {
    /// Starting position of the selection
    pub start: CursorPosition,
    /// Ending position of the selection
    pub end: CursorPosition,
}

impl Selection {
    /// Create a new selection with validation.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use helix_trainer::game::{Selection, CursorPosition};
    ///
    /// let start = CursorPosition::new(0, 0)?;
    /// let end = CursorPosition::new(0, 5)?;
    /// let sel = Selection::new(start, end);
    /// # Ok::<(), helix_trainer::security::SecurityError>(())
    /// ```
    pub fn new(start: CursorPosition, end: CursorPosition) -> Self {
        Self { start, end }
    }

    /// Check if selection is empty (start == end).
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use helix_trainer::game::{Selection, CursorPosition};
    ///
    /// let pos = CursorPosition::new(0, 0)?;
    /// let sel = Selection::new(pos, pos);
    /// assert!(sel.is_empty());
    /// # Ok::<(), helix_trainer::security::SecurityError>(())
    /// ```
    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }

    /// Get the normalized selection (start <= end).
    ///
    /// Returns `(start, end)` where start comes before or at the same position as end.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use helix_trainer::game::{Selection, CursorPosition};
    ///
    /// let start = CursorPosition::new(2, 5)?;
    /// let end = CursorPosition::new(1, 3)?;
    /// let sel = Selection::new(start, end);
    /// let (norm_start, norm_end) = sel.normalized();
    /// assert_eq!(norm_start, end);
    /// # Ok::<(), helix_trainer::security::SecurityError>(())
    /// ```
    pub fn normalized(&self) -> (CursorPosition, CursorPosition) {
        if self.start.row < self.end.row
            || (self.start.row == self.end.row && self.start.col <= self.end.col)
        {
            (self.start, self.end)
        } else {
            (self.end, self.start)
        }
    }
}

impl EditorState {
    /// Create a new editor state with validation.
    ///
    /// Validates that:
    /// - Content size is within limits
    /// - Cursor position is within content bounds
    /// - Selection (if present) is within content bounds
    ///
    /// # Errors
    ///
    /// Returns `SecurityError` if validation fails:
    /// - `ContentTooLarge` if content exceeds maximum size
    /// - `InvalidInput` if cursor is out of bounds
    /// - `InvalidInput` if selection is out of bounds
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use helix_trainer::game::{EditorState, CursorPosition};
    ///
    /// let cursor = CursorPosition::new(0, 0)?;
    /// let state = EditorState::new("line 1\nline 2\n".to_string(), cursor, None)?;
    /// assert_eq!(state.line_count(), 2);
    /// # Ok::<(), helix_trainer::security::SecurityError>(())
    /// ```
    pub fn new(
        content: String,
        cursor_pos: CursorPosition,
        selection: Option<Selection>,
    ) -> Result<Self, SecurityError> {
        // Validate content size
        security::sanitizer::sanitize_content(&content)?;

        // Validate cursor position is within content bounds
        let line_count = content.lines().count().max(1);
        if cursor_pos.row >= line_count {
            return Err(SecurityError::InvalidInput(format!(
                "Cursor row {} exceeds line count {}",
                cursor_pos.row, line_count
            )));
        }

        // Validate column position
        if let Some(line) = content.lines().nth(cursor_pos.row) {
            if cursor_pos.col > line.len() {
                return Err(SecurityError::InvalidInput(format!(
                    "Cursor col {} exceeds line length {}",
                    cursor_pos.col,
                    line.len()
                )));
            }
        }

        // Validate selection if present
        if let Some(sel) = selection {
            Self::validate_selection_bounds(&content, &sel)?;
        }

        Ok(Self {
            content,
            cursor_pos,
            selection,
        })
    }

    /// Create from scenario setup data.
    ///
    /// Convenience constructor that takes setup data from scenario TOML format.
    ///
    /// # Errors
    ///
    /// Returns `SecurityError` if content or cursor position is invalid.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use helix_trainer::game::EditorState;
    ///
    /// let state = EditorState::from_setup("line 1\nline 2\n", [1, 0])?;
    /// assert_eq!(state.cursor_position().row, 1);
    /// # Ok::<(), helix_trainer::security::SecurityError>(())
    /// ```
    pub fn from_setup(
        file_content: &str,
        cursor_position: [usize; 2],
    ) -> Result<Self, SecurityError> {
        let cursor = CursorPosition::from_array(cursor_position)?;
        Self::new(file_content.to_string(), cursor, None)
    }

    /// Get the file content.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use helix_trainer::game::{EditorState, CursorPosition};
    ///
    /// let cursor = CursorPosition::new(0, 0)?;
    /// let state = EditorState::new("test\n".to_string(), cursor, None)?;
    /// assert_eq!(state.content(), "test\n");
    /// # Ok::<(), helix_trainer::security::SecurityError>(())
    /// ```
    pub fn content(&self) -> &str {
        &self.content
    }

    /// Get the cursor position.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use helix_trainer::game::{EditorState, CursorPosition};
    ///
    /// let cursor = CursorPosition::new(1, 3)?;
    /// let state = EditorState::new("line 1\nline 2\n".to_string(), cursor, None)?;
    /// assert_eq!(state.cursor_position().row, 1);
    /// # Ok::<(), helix_trainer::security::SecurityError>(())
    /// ```
    pub fn cursor_position(&self) -> CursorPosition {
        self.cursor_pos
    }

    /// Get the selection.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use helix_trainer::game::{EditorState, CursorPosition, Selection};
    ///
    /// let cursor = CursorPosition::new(0, 0)?;
    /// let state = EditorState::new("test\n".to_string(), cursor, None)?;
    /// assert!(state.selection().is_none());
    /// # Ok::<(), helix_trainer::security::SecurityError>(())
    /// ```
    pub fn selection(&self) -> Option<Selection> {
        self.selection
    }

    /// Get number of lines in the content.
    ///
    /// Empty content is treated as having 1 line.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use helix_trainer::game::{EditorState, CursorPosition};
    ///
    /// let cursor = CursorPosition::new(0, 0)?;
    /// let state = EditorState::new("line 1\nline 2\nline 3\n".to_string(), cursor, None)?;
    /// assert_eq!(state.line_count(), 3);
    /// # Ok::<(), helix_trainer::security::SecurityError>(())
    /// ```
    pub fn line_count(&self) -> usize {
        self.content.lines().count().max(1)
    }

    /// Get a specific line by index (0-indexed).
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use helix_trainer::game::{EditorState, CursorPosition};
    ///
    /// let cursor = CursorPosition::new(0, 0)?;
    /// let state = EditorState::new("line 1\nline 2\nline 3\n".to_string(), cursor, None)?;
    /// assert_eq!(state.line(1), Some("line 2"));
    /// # Ok::<(), helix_trainer::security::SecurityError>(())
    /// ```
    pub fn line(&self, index: usize) -> Option<&str> {
        self.content.lines().nth(index)
    }

    /// Get the current line where cursor is positioned.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use helix_trainer::game::{EditorState, CursorPosition};
    ///
    /// let cursor = CursorPosition::new(1, 0)?;
    /// let state = EditorState::new("line 1\nline 2\nline 3\n".to_string(), cursor, None)?;
    /// assert_eq!(state.current_line(), Some("line 2"));
    /// # Ok::<(), helix_trainer::security::SecurityError>(())
    /// ```
    pub fn current_line(&self) -> Option<&str> {
        self.line(self.cursor_pos.row)
    }

    /// Set new content with validation.
    ///
    /// After updating content, the method automatically:
    /// - Adjusts cursor if it's out of bounds
    /// - Clears selection if it's invalid
    ///
    /// # Errors
    ///
    /// Returns `SecurityError` if content validation fails.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use helix_trainer::game::{EditorState, CursorPosition};
    ///
    /// let cursor = CursorPosition::new(0, 0)?;
    /// let mut state = EditorState::new("line 1\n".to_string(), cursor, None)?;
    /// state.set_content("new content\n".to_string())?;
    /// assert_eq!(state.content(), "new content\n");
    /// # Ok::<(), helix_trainer::security::SecurityError>(())
    /// ```
    pub fn set_content(&mut self, new_content: String) -> Result<(), SecurityError> {
        // Validate new content
        security::sanitizer::sanitize_content(&new_content)?;

        // Update content
        self.content = new_content;

        // Adjust cursor if it's now out of bounds
        self.clamp_cursor_to_bounds()?;

        // Clear selection if it's now invalid
        if let Some(sel) = self.selection {
            if Self::validate_selection_bounds(&self.content, &sel).is_err() {
                self.selection = None;
            }
        }

        Ok(())
    }

    /// Move cursor to a new position with validation.
    ///
    /// # Errors
    ///
    /// Returns `InvalidInput` if the new position is out of bounds.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use helix_trainer::game::{EditorState, CursorPosition};
    ///
    /// let cursor = CursorPosition::new(0, 0)?;
    /// let mut state = EditorState::new("line 1\nline 2\n".to_string(), cursor, None)?;
    /// let new_pos = CursorPosition::new(1, 3)?;
    /// state.move_cursor(new_pos)?;
    /// assert_eq!(state.cursor_position(), new_pos);
    /// # Ok::<(), helix_trainer::security::SecurityError>(())
    /// ```
    pub fn move_cursor(&mut self, new_pos: CursorPosition) -> Result<(), SecurityError> {
        // Validate new position is within bounds
        let line_count = self.line_count();
        if new_pos.row >= line_count {
            return Err(SecurityError::InvalidInput(format!(
                "Cannot move cursor to row {} (only {} lines)",
                new_pos.row, line_count
            )));
        }

        if let Some(line) = self.line(new_pos.row) {
            if new_pos.col > line.len() {
                return Err(SecurityError::InvalidInput(format!(
                    "Cannot move cursor to col {} (line length is {})",
                    new_pos.col,
                    line.len()
                )));
            }
        }

        self.cursor_pos = new_pos;
        Ok(())
    }

    /// Set the selection.
    ///
    /// Validates that selection bounds are within content bounds.
    ///
    /// # Errors
    ///
    /// Returns `InvalidInput` if selection is out of bounds.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use helix_trainer::game::{EditorState, CursorPosition, Selection};
    ///
    /// let cursor = CursorPosition::new(0, 0)?;
    /// let mut state = EditorState::new("test\n".to_string(), cursor, None)?;
    ///
    /// let start = CursorPosition::new(0, 0)?;
    /// let end = CursorPosition::new(0, 4)?;
    /// let sel = Selection::new(start, end);
    /// state.set_selection(Some(sel))?;
    /// # Ok::<(), helix_trainer::security::SecurityError>(())
    /// ```
    pub fn set_selection(&mut self, selection: Option<Selection>) -> Result<(), SecurityError> {
        if let Some(sel) = selection {
            Self::validate_selection_bounds(&self.content, &sel)?;
        }
        self.selection = selection;
        Ok(())
    }

    /// Check if this state matches another state (for completion checking).
    ///
    /// Compares both content and cursor position. Selection is not compared.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use helix_trainer::game::{EditorState, CursorPosition};
    ///
    /// let cursor = CursorPosition::new(0, 0)?;
    /// let state1 = EditorState::new("test\n".to_string(), cursor, None)?;
    /// let state2 = EditorState::new("test\n".to_string(), cursor, None)?;
    /// assert!(state1.matches(&state2));
    /// # Ok::<(), helix_trainer::security::SecurityError>(())
    /// ```
    pub fn matches(&self, other: &EditorState) -> bool {
        self.content == other.content && self.cursor_pos == other.cursor_pos
    }

    /// Check if content matches another state (ignoring cursor and selection).
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use helix_trainer::game::{EditorState, CursorPosition};
    ///
    /// let cursor1 = CursorPosition::new(0, 0)?;
    /// let cursor2 = CursorPosition::new(0, 1)?;
    /// let state1 = EditorState::new("test\n".to_string(), cursor1, None)?;
    /// let state2 = EditorState::new("test\n".to_string(), cursor2, None)?;
    /// assert!(state1.content_matches(&state2));
    /// # Ok::<(), helix_trainer::security::SecurityError>(())
    /// ```
    pub fn content_matches(&self, other: &EditorState) -> bool {
        self.content == other.content
    }

    /// Clamp cursor to valid bounds after content change.
    fn clamp_cursor_to_bounds(&mut self) -> Result<(), SecurityError> {
        let line_count = self.line_count();

        // Clamp row
        if self.cursor_pos.row >= line_count {
            self.cursor_pos.row = line_count.saturating_sub(1);
        }

        // Clamp column
        if let Some(line) = self.line(self.cursor_pos.row) {
            if self.cursor_pos.col > line.len() {
                self.cursor_pos.col = line.len();
            }
        }

        // Revalidate after clamping
        security::arithmetic::validate_cursor_position(
            self.cursor_pos.row,
            self.cursor_pos.col,
            self.content.len(),
        )?;

        Ok(())
    }

    /// Validate selection is within content bounds.
    fn validate_selection_bounds(
        content: &str,
        selection: &Selection,
    ) -> Result<(), SecurityError> {
        let line_count = content.lines().count().max(1);
        let (start, end) = selection.normalized();

        // Validate start position
        if start.row >= line_count {
            return Err(SecurityError::InvalidInput(format!(
                "Selection start row {} exceeds line count {}",
                start.row, line_count
            )));
        }

        // Validate end position
        if end.row >= line_count {
            return Err(SecurityError::InvalidInput(format!(
                "Selection end row {} exceeds line count {}",
                end.row, line_count
            )));
        }

        Ok(())
    }
}

impl Default for EditorState {
    fn default() -> Self {
        Self {
            content: String::new(),
            cursor_pos: CursorPosition { row: 0, col: 0 },
            selection: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_valid_state() {
        let cursor = CursorPosition::new(0, 0).unwrap();
        let state = EditorState::new("line 1\nline 2\n".to_string(), cursor, None);
        assert!(state.is_ok());
    }

    #[test]
    fn test_cursor_out_of_bounds_rejected() {
        let cursor = CursorPosition::new(10, 0).unwrap();
        let state = EditorState::new("line 1\n".to_string(), cursor, None);
        assert!(state.is_err());
    }

    #[test]
    fn test_cursor_column_out_of_bounds() {
        let cursor = CursorPosition::new(0, 100).unwrap();
        let state = EditorState::new("short\n".to_string(), cursor, None);
        assert!(state.is_err());
    }

    #[test]
    fn test_from_setup() {
        let state = EditorState::from_setup("line 1\nline 2\n", [1, 0]);
        assert!(state.is_ok());
        let state = state.unwrap();
        assert_eq!(state.cursor_position().row, 1);
        assert_eq!(state.cursor_position().col, 0);
    }

    #[test]
    fn test_line_count() {
        let cursor = CursorPosition::new(0, 0).unwrap();
        let state = EditorState::new("line 1\nline 2\nline 3\n".to_string(), cursor, None).unwrap();
        assert_eq!(state.line_count(), 3);
    }

    #[test]
    fn test_current_line() {
        let cursor = CursorPosition::new(1, 0).unwrap();
        let state = EditorState::new("line 1\nline 2\nline 3\n".to_string(), cursor, None).unwrap();
        assert_eq!(state.current_line(), Some("line 2"));
    }

    #[test]
    fn test_set_content_adjusts_cursor() {
        let cursor = CursorPosition::new(2, 5).unwrap();
        let mut state =
            EditorState::new("line 1\nline 2\nline 3\n".to_string(), cursor, None).unwrap();

        // Set content with fewer lines
        state.set_content("only one line\n".to_string()).unwrap();

        // Cursor should be clamped to line 0
        assert_eq!(state.cursor_position().row, 0);
    }

    #[test]
    fn test_move_cursor() {
        let cursor = CursorPosition::new(0, 0).unwrap();
        let mut state = EditorState::new("line 1\nline 2\n".to_string(), cursor, None).unwrap();

        let new_pos = CursorPosition::new(1, 3).unwrap();
        state.move_cursor(new_pos).unwrap();

        assert_eq!(state.cursor_position(), new_pos);
    }

    #[test]
    fn test_move_cursor_out_of_bounds_rejected() {
        let cursor = CursorPosition::new(0, 0).unwrap();
        let mut state = EditorState::new("line 1\n".to_string(), cursor, None).unwrap();

        let invalid_pos = CursorPosition::new(10, 0).unwrap();
        let result = state.move_cursor(invalid_pos);

        assert!(result.is_err());
    }

    #[test]
    fn test_state_matches() {
        let cursor = CursorPosition::new(0, 0).unwrap();
        let state1 = EditorState::new("test\n".to_string(), cursor, None).unwrap();
        let state2 = EditorState::new("test\n".to_string(), cursor, None).unwrap();

        assert!(state1.matches(&state2));
    }

    #[test]
    fn test_content_matches() {
        let cursor1 = CursorPosition::new(0, 0).unwrap();
        let cursor2 = CursorPosition::new(0, 1).unwrap();

        let state1 = EditorState::new("test\n".to_string(), cursor1, None).unwrap();
        let state2 = EditorState::new("test\n".to_string(), cursor2, None).unwrap();

        assert!(state1.content_matches(&state2));
        assert!(!state1.matches(&state2)); // Different cursor positions
    }

    #[test]
    fn test_selection_creation() {
        let start = CursorPosition::new(0, 0).unwrap();
        let end = CursorPosition::new(0, 5).unwrap();
        let selection = Selection::new(start, end);

        assert!(!selection.is_empty());
    }

    #[test]
    fn test_selection_normalized() {
        let start = CursorPosition::new(2, 5).unwrap();
        let end = CursorPosition::new(1, 3).unwrap();
        let selection = Selection::new(start, end);

        let (norm_start, norm_end) = selection.normalized();
        assert_eq!(norm_start, end); // End comes first
        assert_eq!(norm_end, start);
    }

    #[test]
    fn test_empty_content_handling() {
        let cursor = CursorPosition::new(0, 0).unwrap();
        let state = EditorState::new(String::new(), cursor, None).unwrap();
        assert_eq!(state.line_count(), 1); // Empty file still has 1 line
    }

    #[test]
    fn test_selection_empty() {
        let pos = CursorPosition::new(0, 0).unwrap();
        let selection = Selection::new(pos, pos);
        assert!(selection.is_empty());
    }

    #[test]
    fn test_set_selection_with_valid_bounds() {
        let cursor = CursorPosition::new(0, 0).unwrap();
        let mut state = EditorState::new("test content\n".to_string(), cursor, None).unwrap();

        let start = CursorPosition::new(0, 0).unwrap();
        let end = CursorPosition::new(0, 4).unwrap();
        let sel = Selection::new(start, end);

        assert!(state.set_selection(Some(sel)).is_ok());
        assert_eq!(state.selection(), Some(sel));
    }

    #[test]
    fn test_set_selection_clear() {
        let cursor = CursorPosition::new(0, 0).unwrap();
        let start = CursorPosition::new(0, 0).unwrap();
        let end = CursorPosition::new(0, 4).unwrap();
        let sel = Selection::new(start, end);

        let mut state = EditorState::new("test\n".to_string(), cursor, Some(sel)).unwrap();
        assert!(state.selection().is_some());

        assert!(state.set_selection(None).is_ok());
        assert!(state.selection().is_none());
    }

    #[test]
    fn test_default_editor_state() {
        let state = EditorState::default();
        assert_eq!(state.content(), "");
        assert_eq!(state.cursor_position().row, 0);
        assert_eq!(state.cursor_position().col, 0);
        assert!(state.selection().is_none());
    }
}
