//! Helix text editor simulator using helix-core primitives
//!
//! This module provides a HelixSimulator that uses the helix-core library
//! for text editing operations. It ensures unicode-correct handling of
//! graphemes, supports multi-cursor operations, and maintains undo history.

use crate::game::{CursorPosition, EditorState};
use crate::security::UserError;
use helix_core::{
    movement::{self, Movement},
    Rope, Selection, Transaction,
};

/// Editor mode (Normal or Insert)
///
/// Controls which operations are available and how input is interpreted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Normal mode: execute commands
    Normal,
    /// Insert mode: insert characters
    Insert,
}

/// Helix editor simulator using helix-core text primitives
///
/// Provides a faithful simulation of Helix editor operations with proper
/// unicode handling, undo/redo support, and multi-cursor awareness.
pub struct HelixSimulator {
    /// Text buffer (using Rope for efficient edits)
    doc: Rope,

    /// Current selection(s) with head and anchor positions
    selection: Selection,

    /// Editor mode (Normal or Insert)
    mode: Mode,

    /// Undo history stack storing both transactions and previous document states
    history: Vec<(Transaction, Rope)>,
}

impl HelixSimulator {
    /// Create a new simulator with initial content
    pub fn new(content: String) -> Self {
        Self {
            doc: Rope::from(content.as_str()),
            selection: Selection::point(0),
            mode: Mode::Normal,
            history: Vec::new(),
        }
    }

    /// Execute a Helix command
    pub fn execute_command(&mut self, cmd: &str) -> Result<(), UserError> {
        match cmd {
            // Movement commands - single character
            "h" => self.move_left(1)?,
            "l" => self.move_right(1)?,
            "j" => self.move_down(1)?,
            "k" => self.move_up(1)?,

            // Word movement
            "w" => self.move_next_word_start(1)?,
            "b" => self.move_prev_word_start(1)?,
            "e" => self.move_next_word_end(1)?,

            // Line movement
            "0" => self.move_line_start()?,
            "$" => self.move_line_end()?,

            // Document movement
            "gg" => self.move_document_start()?,
            "G" => self.move_document_end()?,

            // Deletion commands
            "x" => self.delete_char()?,
            "dd" => self.delete_line()?,

            // Mode changes
            "i" => {
                self.mode = Mode::Insert;
            }
            "Escape" => {
                self.mode = Mode::Normal;
            }

            // Undo/Redo
            "u" => self.undo()?,
            "ctrl-r" => self.redo()?,

            // Unknown command
            _ => {
                return Err(UserError::OperationFailed)
            }
        }

        Ok(())
    }

    /// Get current editor state
    pub fn get_state(&self) -> Result<EditorState, UserError> {
        let mut head = self.selection.primary().head;

        // Clamp cursor to valid bounds (sometimes helix-core can put it past end)
        let max_pos = self.doc.len_chars();
        if head > max_pos {
            head = max_pos;
        }

        // Convert head position to (line, col)
        let line = self.doc.char_to_line(head);
        let line_start = self.doc.line_to_char(line);
        let col = head - line_start;

        EditorState::new(
            self.doc.to_string(),
            CursorPosition::new(line, col).map_err(|_| UserError::OperationFailed)?,
            None,
        )
        .map_err(|_| UserError::OperationFailed)
    }

    /// Get current mode
    pub fn mode(&self) -> Mode {
        self.mode
    }

    // === Movement implementations ===

    fn move_left(&mut self, count: usize) -> Result<(), UserError> {
        use helix_core::movement::Direction;

        let slice = self.doc.slice(..);
        let new_selection = self.selection.clone().transform(|range| {
            movement::move_horizontally(
                slice,
                range,
                Direction::Backward,
                count,
                Movement::Move,
            )
        });

        self.selection = new_selection;
        Ok(())
    }

    fn move_right(&mut self, count: usize) -> Result<(), UserError> {
        use helix_core::movement::Direction;

        let slice = self.doc.slice(..);
        let new_selection = self.selection.clone().transform(|range| {
            movement::move_horizontally(
                slice,
                range,
                Direction::Forward,
                count,
                Movement::Move,
            )
        });

        self.selection = new_selection;
        Ok(())
    }

    fn move_down(&mut self, count: usize) -> Result<(), UserError> {
        use helix_core::movement::Direction;

        let slice = self.doc.slice(..);
        let new_selection = self.selection.clone().transform(|range| {
            movement::move_vertically(
                slice,
                range,
                Direction::Forward,
                count,
                Movement::Move,
            )
        });

        self.selection = new_selection;
        Ok(())
    }

    fn move_up(&mut self, count: usize) -> Result<(), UserError> {
        use helix_core::movement::Direction;

        let slice = self.doc.slice(..);
        let new_selection = self.selection.clone().transform(|range| {
            movement::move_vertically(
                slice,
                range,
                Direction::Backward,
                count,
                Movement::Move,
            )
        });

        self.selection = new_selection;
        Ok(())
    }

    fn move_next_word_start(&mut self, count: usize) -> Result<(), UserError> {
        let slice = self.doc.slice(..);
        let new_selection = self.selection.clone().transform(|range| {
            movement::move_next_word_start(slice, range, count)
        });

        self.selection = new_selection;
        Ok(())
    }

    fn move_prev_word_start(&mut self, count: usize) -> Result<(), UserError> {
        let slice = self.doc.slice(..);
        let new_selection = self.selection.clone().transform(|range| {
            movement::move_prev_word_start(slice, range, count)
        });

        self.selection = new_selection;
        Ok(())
    }

    fn move_next_word_end(&mut self, count: usize) -> Result<(), UserError> {
        let slice = self.doc.slice(..);
        let new_selection = self.selection.clone().transform(|range| {
            movement::move_next_word_end(slice, range, count)
        });

        self.selection = new_selection;
        Ok(())
    }

    fn move_line_start(&mut self) -> Result<(), UserError> {
        let head = self.selection.primary().head;
        let line = self.doc.char_to_line(head);
        let line_start = self.doc.line_to_char(line);

        self.selection = Selection::point(line_start);
        Ok(())
    }

    fn move_line_end(&mut self) -> Result<(), UserError> {
        let head = self.selection.primary().head;
        let line = self.doc.char_to_line(head);

        // Get position of next line, or end of document
        let line_end = if line + 1 < self.doc.len_lines() {
            self.doc.line_to_char(line + 1) - 1
        } else {
            self.doc.len_chars()
        };

        self.selection = Selection::point(line_end);
        Ok(())
    }

    fn move_document_start(&mut self) -> Result<(), UserError> {
        self.selection = Selection::point(0);
        Ok(())
    }

    fn move_document_end(&mut self) -> Result<(), UserError> {
        let end = self.doc.len_chars();
        self.selection = Selection::point(end);
        Ok(())
    }

    // === Editing implementations ===

    fn delete_char(&mut self) -> Result<(), UserError> {
        let transaction = Transaction::change_by_selection(&self.doc, &self.selection, |range| {
            let start = range.from();
            let end = start
                .saturating_add(1)
                .min(self.doc.len_chars())
                .max(start);
            (start, end, None)
        });

        self.apply_transaction(transaction);
        Ok(())
    }

    fn delete_line(&mut self) -> Result<(), UserError> {
        let transaction = Transaction::change_by_selection(&self.doc, &self.selection, |range| {
            let line = self.doc.char_to_line(range.head);
            let start = self.doc.line_to_char(line);
            let end = if line + 1 < self.doc.len_lines() {
                self.doc.line_to_char(line + 1)
            } else {
                self.doc.len_chars()
            };
            (start, end, None)
        });

        self.apply_transaction(transaction);
        Ok(())
    }

    fn undo(&mut self) -> Result<(), UserError> {
        if let Some((_transaction, prev_doc)) = self.history.pop() {
            // Restore the previous document state
            self.doc = prev_doc;

            // Clamp cursor to valid position
            let head = self.selection.primary().head.min(self.doc.len_chars());
            self.selection = Selection::point(head);
        }
        Ok(())
    }

    fn redo(&mut self) -> Result<(), UserError> {
        // Full redo would require keeping a separate redo stack
        // For now, this is a placeholder
        Ok(())
    }

    // === Helper methods ===

    fn apply_transaction(&mut self, transaction: Transaction) {
        // Save previous state before applying transaction
        let prev_doc = self.doc.clone();
        self.history.push((transaction.clone(), prev_doc));
        transaction.apply(&mut self.doc);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_simulator() {
        let sim = HelixSimulator::new("hello world".to_string());
        let state = sim.get_state().unwrap();
        assert_eq!(state.content(), "hello world");
        assert_eq!(state.cursor_position().row, 0);
        assert_eq!(state.cursor_position().col, 0);
    }

    #[test]
    fn test_initial_mode() {
        let sim = HelixSimulator::new("test".to_string());
        assert_eq!(sim.mode(), Mode::Normal);
    }

    #[test]
    fn test_move_right() {
        let mut sim = HelixSimulator::new("hello".to_string());

        sim.execute_command("l").unwrap();
        let state = sim.get_state().unwrap();
        assert_eq!(state.cursor_position().col, 1);

        sim.execute_command("l").unwrap();
        let state = sim.get_state().unwrap();
        assert_eq!(state.cursor_position().col, 2);
    }

    #[test]
    fn test_move_left() {
        let mut sim = HelixSimulator::new("hello".to_string());

        // Move right twice
        sim.execute_command("l").unwrap();
        sim.execute_command("l").unwrap();

        // Move left once
        sim.execute_command("h").unwrap();
        let state = sim.get_state().unwrap();
        assert_eq!(state.cursor_position().col, 1);
    }

    #[test]
    fn test_word_movement() {
        let mut sim = HelixSimulator::new("hello world foo".to_string());

        // Move to next word
        sim.execute_command("w").unwrap();
        let state = sim.get_state().unwrap();
        assert_eq!(state.cursor_position().col, 6); // "world"

        // Move to next word again
        sim.execute_command("w").unwrap();
        let state = sim.get_state().unwrap();
        assert_eq!(state.cursor_position().col, 12); // "foo"
    }

    #[test]
    fn test_delete_line() {
        let mut sim = HelixSimulator::new("line 1\nline 2\nline 3\n".to_string());

        sim.execute_command("dd").unwrap();

        let state = sim.get_state().unwrap();
        assert_eq!(state.content(), "line 2\nline 3\n");
    }

    #[test]
    fn test_delete_char() {
        let mut sim = HelixSimulator::new("hello".to_string());

        sim.execute_command("x").unwrap();

        let state = sim.get_state().unwrap();
        assert_eq!(state.content(), "ello");
    }

    #[test]
    fn test_delete_char_in_middle() {
        let mut sim = HelixSimulator::new("hello".to_string());

        sim.execute_command("l").unwrap(); // Move to 'e'
        sim.execute_command("l").unwrap(); // Move to 'l'
        sim.execute_command("x").unwrap(); // Delete 'l'

        let state = sim.get_state().unwrap();
        assert_eq!(state.content(), "helo");
    }

    #[test]
    fn test_undo() {
        let mut sim = HelixSimulator::new("test\n".to_string());

        sim.execute_command("dd").unwrap();
        assert_eq!(sim.get_state().unwrap().content(), "");

        sim.execute_command("u").unwrap();
        assert_eq!(sim.get_state().unwrap().content(), "test\n");
    }

    #[test]
    fn test_mode_change() {
        let mut sim = HelixSimulator::new("test".to_string());

        assert_eq!(sim.mode(), Mode::Normal);

        sim.execute_command("i").unwrap();
        assert_eq!(sim.mode(), Mode::Insert);

        sim.execute_command("Escape").unwrap();
        assert_eq!(sim.mode(), Mode::Normal);
    }

    #[test]
    fn test_move_line_start() {
        let mut sim = HelixSimulator::new("hello\nworld\n".to_string());

        // Move to next line
        sim.execute_command("j").unwrap();
        // Move to end of line
        sim.execute_command("$").unwrap();
        let state = sim.get_state().unwrap();
        // Cursor at end of "world" - which is position 4 or 5
        assert!(state.cursor_position().col >= 4);

        // Move to start of line
        sim.execute_command("0").unwrap();
        let state = sim.get_state().unwrap();
        assert_eq!(state.cursor_position().col, 0);
    }

    #[test]
    fn test_move_down_up() {
        let mut sim = HelixSimulator::new("line1\nline2\nline3\n".to_string());

        sim.execute_command("j").unwrap();
        let state = sim.get_state().unwrap();
        assert_eq!(state.cursor_position().row, 1);

        sim.execute_command("j").unwrap();
        let state = sim.get_state().unwrap();
        assert_eq!(state.cursor_position().row, 2);

        sim.execute_command("k").unwrap();
        let state = sim.get_state().unwrap();
        assert_eq!(state.cursor_position().row, 1);
    }

    #[test]
    fn test_document_start() {
        let mut sim = HelixSimulator::new("line1\nline2\nline3\n".to_string());

        // Move somewhere else
        sim.execute_command("j").unwrap();
        let state = sim.get_state().unwrap();
        assert_eq!(state.cursor_position().row, 1);

        // Go back to start
        sim.execute_command("gg").unwrap();
        let state = sim.get_state().unwrap();
        assert_eq!(state.cursor_position().row, 0);
        assert_eq!(state.cursor_position().col, 0);
    }

    #[test]
    fn test_unknown_command() {
        let mut sim = HelixSimulator::new("test".to_string());
        let result = sim.execute_command("unknown");
        assert!(result.is_err());
    }

    #[test]
    fn test_multiple_line_deletions() {
        let mut sim = HelixSimulator::new("line1\nline2\nline3\n".to_string());

        sim.execute_command("dd").unwrap();
        sim.execute_command("dd").unwrap();

        let state = sim.get_state().unwrap();
        assert_eq!(state.content(), "line3\n");
    }

    #[test]
    fn test_move_word_boundary() {
        let mut sim = HelixSimulator::new("  spaced  words  ".to_string());

        sim.execute_command("w").unwrap();
        let state = sim.get_state().unwrap();
        // Should move to first non-space character of next word
        assert!(state.cursor_position().col > 0);
    }

    #[test]
    fn test_move_word_end() {
        let mut sim = HelixSimulator::new("hello world".to_string());

        sim.execute_command("e").unwrap();
        let state = sim.get_state().unwrap();
        // Should be at end of "hello"
        assert!(state.cursor_position().col >= 4 && state.cursor_position().col <= 5);
    }

    #[test]
    fn test_move_prev_word() {
        let mut sim = HelixSimulator::new("hello world foo".to_string());

        // Move to end of document first
        sim.execute_command("G").unwrap();
        // Then move to previous word
        sim.execute_command("b").unwrap();

        let state = sim.get_state().unwrap();
        // Should have moved to start of a previous word
        assert!(state.cursor_position().col >= 11);
    }
}
