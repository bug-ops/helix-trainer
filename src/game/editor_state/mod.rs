//! Editor state representation and management.
//!
//! This module defines the EditorState type which represents the state of a text editor
//! at a given moment, including file content, cursor position, and selections.
//!
//! Supports multiple selections (multi-cursor) with a primary selection for backward
//! compatibility. All operations validate against security limits and ensure bounds correctness.
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

#[cfg(test)]
mod tests;

use crate::security::{self, SecurityError};
use serde::{Deserialize, Serialize};

/// Represents the state of the text editor at a given moment.
///
/// Stores complete file content, cursor position, and text selections.
/// Supports multiple selections (multi-cursor) with a primary selection.
/// All operations validate bounds to ensure the state remains consistent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EditorState {
    /// The complete file content
    content: String,
    /// Cursor position as (row, col) - both 0-indexed
    cursor_pos: CursorPosition,
    /// All selections (empty Vec means no selection)
    selections: Vec<Selection>,
    /// Index of the primary selection in selections Vec
    primary_idx: usize,
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
        if let Some(line) = content.lines().nth(cursor_pos.row)
            && cursor_pos.col > line.len()
        {
            return Err(SecurityError::InvalidInput(format!(
                "Cursor col {} exceeds line length {}",
                cursor_pos.col,
                line.len()
            )));
        }

        // Validate selection if present
        if let Some(sel) = &selection {
            Self::validate_selection_bounds(&content, sel)?;
        }

        // Convert Option<Selection> to Vec<Selection> for backward compatibility
        let selections = selection.into_iter().collect();

        Ok(Self {
            content,
            cursor_pos,
            selections,
            primary_idx: 0,
        })
    }

    /// Create a new editor state with multiple selections.
    ///
    /// Validates that:
    /// - Content size is within limits
    /// - Cursor position is within content bounds
    /// - All selections are within content bounds
    ///
    /// # Errors
    ///
    /// Returns `SecurityError` if validation fails.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use helix_trainer::game::{EditorState, CursorPosition, Selection};
    ///
    /// let cursor = CursorPosition::new(0, 0)?;
    /// let sel1 = Selection::new(
    ///     CursorPosition::new(0, 0)?,
    ///     CursorPosition::new(0, 5)?,
    /// );
    /// let sel2 = Selection::new(
    ///     CursorPosition::new(1, 0)?,
    ///     CursorPosition::new(1, 5)?,
    /// );
    /// let state = EditorState::with_selections(
    ///     "hello world\nfoo bar".to_string(),
    ///     cursor,
    ///     vec![sel1, sel2],
    ///     0,
    /// )?;
    /// assert_eq!(state.selections().len(), 2);
    /// # Ok::<(), helix_trainer::security::SecurityError>(())
    /// ```
    pub fn with_selections(
        content: String,
        cursor_pos: CursorPosition,
        selections: Vec<Selection>,
        primary_idx: usize,
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
        if let Some(line) = content.lines().nth(cursor_pos.row)
            && cursor_pos.col > line.len()
        {
            return Err(SecurityError::InvalidInput(format!(
                "Cursor col {} exceeds line length {}",
                cursor_pos.col,
                line.len()
            )));
        }

        // Validate all selections
        for sel in &selections {
            Self::validate_selection_bounds(&content, sel)?;
        }

        // Validate primary_idx
        if !selections.is_empty() && primary_idx >= selections.len() {
            return Err(SecurityError::InvalidInput(format!(
                "Primary selection index {} exceeds selection count {}",
                primary_idx,
                selections.len()
            )));
        }

        Ok(Self {
            content,
            cursor_pos,
            selections,
            primary_idx,
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
    /// let state = EditorState::from_setup("line 1\nline 2\n", [1, 0], None)?;
    /// assert_eq!(state.cursor_position().row, 1);
    /// # Ok::<(), helix_trainer::security::SecurityError>(())
    /// ```
    pub fn from_setup(
        file_content: &str,
        cursor_position: [usize; 2],
        selection: Option<[usize; 4]>,
    ) -> Result<Self, SecurityError> {
        let cursor = CursorPosition::from_array(cursor_position)?;
        let sel = selection.map(|s| Selection {
            start: CursorPosition {
                row: s[0],
                col: s[1],
            },
            end: CursorPosition {
                row: s[2],
                col: s[3],
            },
        });
        Self::new(file_content.to_string(), cursor, sel)
    }

    /// Create EditorState from target configuration with optional selection
    ///
    /// # Arguments
    /// * `file_content` - The content string
    /// * `cursor_position` - Array [row, col]
    /// * `selection` - Optional selection range [start_row, start_col, end_row, end_col]
    pub fn from_target(
        file_content: &str,
        cursor_position: [usize; 2],
        selection: Option<[usize; 4]>,
    ) -> Result<Self, SecurityError> {
        let cursor = CursorPosition::from_array(cursor_position)?;
        let sel = selection.map(|s| Selection {
            start: CursorPosition {
                row: s[0],
                col: s[1],
            },
            end: CursorPosition {
                row: s[2],
                col: s[3],
            },
        });
        Self::new(file_content.to_string(), cursor, sel)
    }

    /// Create EditorState from scenario setup with multi-cursor support.
    ///
    /// Handles both single-cursor and multi-cursor formats:
    /// - Single cursor: uses `cursor_position` and optional `selection`
    /// - Multi-cursor: uses `cursors` or `selections` arrays
    ///
    /// # Errors
    ///
    /// Returns `SecurityError` if content or cursor positions are invalid.
    pub fn from_scenario_setup(
        file_content: &str,
        cursor_position: Option<(usize, usize)>,
        selection: Option<[usize; 4]>,
        cursors: Option<&[[usize; 2]]>,
        selections: Option<&[[usize; 4]]>,
    ) -> Result<Self, SecurityError> {
        Self::from_multi_cursor_config(
            file_content,
            cursor_position,
            selection,
            cursors,
            selections,
        )
    }

    /// Create EditorState from scenario target with multi-cursor support.
    ///
    /// Handles both single-cursor and multi-cursor formats:
    /// - Single cursor: uses `cursor_position` and optional `selection`
    /// - Multi-cursor: uses `cursors` or `selections` arrays
    ///
    /// # Errors
    ///
    /// Returns `SecurityError` if content or cursor positions are invalid.
    pub fn from_scenario_target(
        file_content: &str,
        cursor_position: Option<(usize, usize)>,
        selection: Option<[usize; 4]>,
        cursors: Option<&[[usize; 2]]>,
        selections: Option<&[[usize; 4]]>,
    ) -> Result<Self, SecurityError> {
        Self::from_multi_cursor_config(
            file_content,
            cursor_position,
            selection,
            cursors,
            selections,
        )
    }

    /// Internal helper for creating EditorState from multi-cursor configuration.
    fn from_multi_cursor_config(
        file_content: &str,
        cursor_position: Option<(usize, usize)>,
        selection: Option<[usize; 4]>,
        cursors: Option<&[[usize; 2]]>,
        selections_arr: Option<&[[usize; 4]]>,
    ) -> Result<Self, SecurityError> {
        // Pre-compute line count for validation
        let line_count = file_content.lines().count().max(1);

        // Multi-selection format takes priority
        if let Some(sels) = selections_arr {
            // Early validation: check all positions are within bounds before allocating
            for (idx, sel) in sels.iter().enumerate() {
                if sel[0] >= line_count || sel[2] >= line_count {
                    return Err(SecurityError::InvalidInput(format!(
                        "Selection {} row out of bounds (max {})",
                        idx,
                        line_count.saturating_sub(1)
                    )));
                }
            }

            let selections: Vec<Selection> = sels
                .iter()
                .map(|s| Selection {
                    start: CursorPosition {
                        row: s[0],
                        col: s[1],
                    },
                    end: CursorPosition {
                        row: s[2],
                        col: s[3],
                    },
                })
                .collect();

            // Use first selection's end as cursor position
            let cursor = if let Some(first) = selections.first() {
                first.end
            } else {
                CursorPosition { row: 0, col: 0 }
            };

            return Self::with_selections(file_content.to_string(), cursor, selections, 0);
        }

        // Multi-cursor format (point selections)
        if let Some(curs) = cursors {
            // Early validation: check all positions are within bounds before allocating
            for (idx, cur) in curs.iter().enumerate() {
                if cur[0] >= line_count {
                    return Err(SecurityError::InvalidInput(format!(
                        "Cursor {} row {} out of bounds (max {})",
                        idx,
                        cur[0],
                        line_count.saturating_sub(1)
                    )));
                }
            }

            let selections: Vec<Selection> = curs
                .iter()
                .map(|c| {
                    let pos = CursorPosition {
                        row: c[0],
                        col: c[1],
                    };
                    Selection {
                        start: pos,
                        end: pos,
                    }
                })
                .collect();

            let cursor = if let Some(first) = curs.first() {
                CursorPosition {
                    row: first[0],
                    col: first[1],
                }
            } else {
                CursorPosition { row: 0, col: 0 }
            };

            return Self::with_selections(file_content.to_string(), cursor, selections, 0);
        }

        // Single cursor format (backward compatible)
        let pos = cursor_position.unwrap_or((0, 0));
        let cursor = CursorPosition::from_array([pos.0, pos.1])?;
        let sel = selection.map(|s| Selection {
            start: CursorPosition {
                row: s[0],
                col: s[1],
            },
            end: CursorPosition {
                row: s[2],
                col: s[3],
            },
        });
        Self::new(file_content.to_string(), cursor, sel)
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

    /// Get the cursor position as (row, col) tuple.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use helix_trainer::game::{EditorState, CursorPosition};
    ///
    /// let cursor = CursorPosition::new(1, 3)?;
    /// let state = EditorState::new("line 1\nline 2\n".to_string(), cursor, None)?;
    /// let (row, col) = state.cursor_position();
    /// assert_eq!(row, 1);
    /// # Ok::<(), helix_trainer::security::SecurityError>(())
    /// ```
    pub fn cursor_position(&self) -> (usize, usize) {
        (self.cursor_pos.row, self.cursor_pos.col)
    }

    /// Get the primary selection (backward compatible).
    ///
    /// Returns the primary selection if any selections exist, otherwise `None`.
    /// For multi-cursor support, use `selections()` to get all selections.
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
        self.selections.get(self.primary_idx).copied()
    }

    /// Get all selections.
    ///
    /// Returns a slice of all selections. Empty slice means no selections.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use helix_trainer::game::{EditorState, CursorPosition, Selection};
    ///
    /// let cursor = CursorPosition::new(0, 0)?;
    /// let sel1 = Selection::new(
    ///     CursorPosition::new(0, 0)?,
    ///     CursorPosition::new(0, 5)?,
    /// );
    /// let sel2 = Selection::new(
    ///     CursorPosition::new(1, 0)?,
    ///     CursorPosition::new(1, 5)?,
    /// );
    /// let state = EditorState::with_selections(
    ///     "hello world\nfoo bar".to_string(),
    ///     cursor,
    ///     vec![sel1, sel2],
    ///     0,
    /// )?;
    /// assert_eq!(state.selections().len(), 2);
    /// # Ok::<(), helix_trainer::security::SecurityError>(())
    /// ```
    pub fn selections(&self) -> &[Selection] {
        &self.selections
    }

    /// Get the index of the primary selection.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use helix_trainer::game::{EditorState, CursorPosition, Selection};
    ///
    /// let cursor = CursorPosition::new(0, 0)?;
    /// let sel = Selection::new(
    ///     CursorPosition::new(0, 0)?,
    ///     CursorPosition::new(0, 5)?,
    /// );
    /// let state = EditorState::with_selections(
    ///     "hello world".to_string(),
    ///     cursor,
    ///     vec![sel],
    ///     0,
    /// )?;
    /// assert_eq!(state.primary_selection_idx(), 0);
    /// # Ok::<(), helix_trainer::security::SecurityError>(())
    /// ```
    pub fn primary_selection_idx(&self) -> usize {
        self.primary_idx
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

        // Pre-compute line count once to avoid O(n*m) complexity in retain loop
        let line_count = self.content.lines().count().max(1);

        // Remove invalid selections
        self.selections.retain(|sel| {
            let (start, end) = sel.normalized();
            start.row < line_count && end.row < line_count
        });

        // Reset primary_idx if it's now out of bounds
        if !self.selections.is_empty() && self.primary_idx >= self.selections.len() {
            self.primary_idx = 0;
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

        if let Some(line) = self.line(new_pos.row)
            && new_pos.col > line.len()
        {
            return Err(SecurityError::InvalidInput(format!(
                "Cannot move cursor to col {} (line length is {})",
                new_pos.col,
                line.len()
            )));
        }

        self.cursor_pos = new_pos;
        Ok(())
    }

    /// Set the primary selection (backward compatible).
    ///
    /// Replaces all selections with a single selection, or clears all if `None`.
    /// For multi-cursor support, use `set_selections()` or `add_selection()`.
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
        if let Some(sel) = &selection {
            Self::validate_selection_bounds(&self.content, sel)?;
        }
        self.selections = selection.into_iter().collect();
        self.primary_idx = 0;
        Ok(())
    }

    /// Set all selections.
    ///
    /// Replaces all selections with the provided list.
    ///
    /// # Errors
    ///
    /// Returns `InvalidInput` if any selection is out of bounds or primary_idx is invalid.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use helix_trainer::game::{EditorState, CursorPosition, Selection};
    ///
    /// let cursor = CursorPosition::new(0, 0)?;
    /// let mut state = EditorState::new("hello world\nfoo bar".to_string(), cursor, None)?;
    ///
    /// let sel1 = Selection::new(
    ///     CursorPosition::new(0, 0)?,
    ///     CursorPosition::new(0, 5)?,
    /// );
    /// let sel2 = Selection::new(
    ///     CursorPosition::new(1, 0)?,
    ///     CursorPosition::new(1, 3)?,
    /// );
    /// state.set_selections(vec![sel1, sel2], 0)?;
    /// assert_eq!(state.selections().len(), 2);
    /// # Ok::<(), helix_trainer::security::SecurityError>(())
    /// ```
    pub fn set_selections(
        &mut self,
        selections: Vec<Selection>,
        primary_idx: usize,
    ) -> Result<(), SecurityError> {
        // Validate all selections
        for sel in &selections {
            Self::validate_selection_bounds(&self.content, sel)?;
        }

        // Validate primary_idx
        if !selections.is_empty() && primary_idx >= selections.len() {
            return Err(SecurityError::InvalidInput(format!(
                "Primary selection index {} exceeds selection count {}",
                primary_idx,
                selections.len()
            )));
        }

        self.selections = selections;
        self.primary_idx = primary_idx;
        Ok(())
    }

    /// Add a selection to the list.
    ///
    /// Appends a new selection to the existing selections.
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
    /// let sel1 = Selection::new(
    ///     CursorPosition::new(0, 0)?,
    ///     CursorPosition::new(0, 5)?,
    /// );
    /// let mut state = EditorState::new("hello world\nfoo bar".to_string(), cursor, Some(sel1))?;
    ///
    /// let sel2 = Selection::new(
    ///     CursorPosition::new(1, 0)?,
    ///     CursorPosition::new(1, 3)?,
    /// );
    /// state.add_selection(sel2)?;
    /// assert_eq!(state.selections().len(), 2);
    /// # Ok::<(), helix_trainer::security::SecurityError>(())
    /// ```
    pub fn add_selection(&mut self, selection: Selection) -> Result<(), SecurityError> {
        Self::validate_selection_bounds(&self.content, &selection)?;
        self.selections.push(selection);
        Ok(())
    }

    /// Set the primary selection index.
    ///
    /// # Errors
    ///
    /// Returns `InvalidInput` if index exceeds number of selections.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use helix_trainer::game::{EditorState, CursorPosition, Selection};
    ///
    /// let cursor = CursorPosition::new(0, 0)?;
    /// let sel1 = Selection::new(
    ///     CursorPosition::new(0, 0)?,
    ///     CursorPosition::new(0, 5)?,
    /// );
    /// let sel2 = Selection::new(
    ///     CursorPosition::new(1, 0)?,
    ///     CursorPosition::new(1, 3)?,
    /// );
    /// let mut state = EditorState::with_selections(
    ///     "hello world\nfoo bar".to_string(),
    ///     cursor,
    ///     vec![sel1, sel2],
    ///     0,
    /// )?;
    /// state.set_primary_selection(1)?;
    /// assert_eq!(state.primary_selection_idx(), 1);
    /// # Ok::<(), helix_trainer::security::SecurityError>(())
    /// ```
    pub fn set_primary_selection(&mut self, index: usize) -> Result<(), SecurityError> {
        if self.selections.is_empty() {
            return Err(SecurityError::InvalidInput(
                "Cannot set primary selection when no selections exist".to_string(),
            ));
        }
        if index >= self.selections.len() {
            return Err(SecurityError::InvalidInput(format!(
                "Primary selection index {} exceeds selection count {}",
                index,
                self.selections.len()
            )));
        }
        self.primary_idx = index;
        Ok(())
    }

    /// Check if this state matches another state (for completion checking).
    ///
    /// Compares content, cursor position, and selections (if target has any).
    /// When the target state has selections, both content and all selections must match
    /// (order-independent comparison). When the target has no selections, only content
    /// and cursor are compared.
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
        // Content must always match
        if self.content != other.content {
            return false;
        }

        // Early return for single-cursor scenarios (99% of cases)
        // This avoids unnecessary Vec clones for the common case
        if other.selections.is_empty() && self.selections.is_empty() {
            return self.cursor_pos == other.cursor_pos;
        }

        // If target has selections, check all selections match (order-independent)
        if !other.selections.is_empty() {
            if self.selections.len() != other.selections.len() {
                return false;
            }

            // Sort both selection lists for order-independent comparison
            let mut self_sorted = self.selections.clone();
            let mut other_sorted = other.selections.clone();
            self_sorted.sort_by(selection_cmp);
            other_sorted.sort_by(selection_cmp);

            return self_sorted == other_sorted;
        }

        // If target has no selection, check cursor position
        self.cursor_pos == other.cursor_pos
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
        if let Some(line) = self.line(self.cursor_pos.row)
            && self.cursor_pos.col > line.len()
        {
            self.cursor_pos.col = line.len();
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
            selections: Vec::new(),
            primary_idx: 0,
        }
    }
}

/// Compare two selections for sorting (by start position, then by end position).
fn selection_cmp(a: &Selection, b: &Selection) -> std::cmp::Ordering {
    let (a_start, a_end) = a.normalized();
    let (b_start, b_end) = b.normalized();

    (a_start.row, a_start.col, a_end.row, a_end.col).cmp(&(
        b_start.row,
        b_start.col,
        b_end.row,
        b_end.col,
    ))
}
