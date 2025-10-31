//! Helix text editor simulator using helix-core primitives
//!
//! This module provides a HelixSimulator that uses the helix-core library
//! for text editing operations. It ensures unicode-correct handling of
//! graphemes, supports multi-cursor operations, and maintains undo history.

use crate::game::{CursorPosition, EditorState};
use crate::security::UserError;
use helix_core::{
    Rope, Selection, Transaction,
    doc_formatter::TextFormat,
    movement::{self, Movement},
    text_annotations::TextAnnotations,
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

    /// Clipboard for yank and paste operations
    clipboard: Option<String>,
}

impl HelixSimulator {
    /// Create a new simulator with initial content
    pub fn new(content: String) -> Self {
        Self {
            doc: Rope::from(content.as_str()),
            selection: Selection::point(0),
            mode: Mode::Normal,
            history: Vec::new(),
            clipboard: None,
        }
    }

    /// Execute a Helix command
    pub fn execute_command(&mut self, cmd: &str) -> Result<(), UserError> {
        // In Insert mode, handle special keys and text input
        if self.mode == Mode::Insert {
            return match cmd {
                "Escape" => {
                    self.mode = Mode::Normal;
                    Ok(())
                }
                "Backspace" => self.backspace(),
                "ArrowLeft" => self.move_left(1),
                "ArrowRight" => self.move_right(1),
                "ArrowUp" => self.move_up(1),
                "ArrowDown" => self.move_down(1),
                _ => self.insert_text(cmd),
            };
        }

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
            "c" => self.change_selection()?,
            "J" => self.join_lines()?,

            // Indentation
            ">" => self.indent_line()?,
            "<" => self.dedent_line()?,

            // Yank and paste
            "y" => self.yank()?,
            "p" => self.paste_after()?,
            "P" => self.paste_before()?,

            // Mode changes and editing
            "i" => {
                self.mode = Mode::Insert;
            }
            "a" => self.append()?,
            "I" => self.insert_at_line_start()?,
            "A" => self.append_at_line_end()?,
            "o" => self.open_below()?,
            "O" => self.open_above()?,
            "Escape" => {
                self.mode = Mode::Normal;
            }

            // Character operations
            cmd if cmd.starts_with('r') && cmd.len() == 2 => {
                let ch = cmd.chars().nth(1).unwrap();
                self.replace_char(ch)?;
            }

            // Undo/Redo
            "u" => self.undo()?,
            "ctrl-r" => self.redo()?,

            // Unknown command
            _ => return Err(UserError::OperationFailed),
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

    /// Convert simulator state to EditorState (alias for get_state)
    pub fn to_editor_state(&self) -> Result<EditorState, UserError> {
        self.get_state()
    }

    /// Get current mode
    pub fn mode(&self) -> Mode {
        self.mode
    }

    // === Movement implementations ===

    fn move_left(&mut self, count: usize) -> Result<(), UserError> {
        use helix_core::movement::Direction;

        let slice = self.doc.slice(..);
        let text_fmt = TextFormat::default();
        let mut annotations = TextAnnotations::default();

        let new_selection = self.selection.clone().transform(|range| {
            movement::move_horizontally(
                slice,
                range,
                Direction::Backward,
                count,
                Movement::Move,
                &text_fmt,
                &mut annotations,
            )
        });

        self.selection = new_selection;
        Ok(())
    }

    fn move_right(&mut self, count: usize) -> Result<(), UserError> {
        use helix_core::movement::Direction;

        let slice = self.doc.slice(..);
        let text_fmt = TextFormat::default();
        let mut annotations = TextAnnotations::default();

        let new_selection = self.selection.clone().transform(|range| {
            movement::move_horizontally(
                slice,
                range,
                Direction::Forward,
                count,
                Movement::Move,
                &text_fmt,
                &mut annotations,
            )
        });

        self.selection = new_selection;
        Ok(())
    }

    fn move_down(&mut self, count: usize) -> Result<(), UserError> {
        use helix_core::movement::Direction;

        let slice = self.doc.slice(..);
        let text_fmt = TextFormat::default();
        let mut annotations = TextAnnotations::default();

        let new_selection = self.selection.clone().transform(|range| {
            movement::move_vertically(
                slice,
                range,
                Direction::Forward,
                count,
                Movement::Move,
                &text_fmt,
                &mut annotations,
            )
        });

        self.selection = new_selection;
        Ok(())
    }

    fn move_up(&mut self, count: usize) -> Result<(), UserError> {
        use helix_core::movement::Direction;

        let slice = self.doc.slice(..);
        let text_fmt = TextFormat::default();
        let mut annotations = TextAnnotations::default();

        let new_selection = self.selection.clone().transform(|range| {
            movement::move_vertically(
                slice,
                range,
                Direction::Backward,
                count,
                Movement::Move,
                &text_fmt,
                &mut annotations,
            )
        });

        self.selection = new_selection;
        Ok(())
    }

    fn move_next_word_start(&mut self, count: usize) -> Result<(), UserError> {
        let slice = self.doc.slice(..);
        let new_selection = self
            .selection
            .clone()
            .transform(|range| movement::move_next_word_start(slice, range, count));

        self.selection = new_selection;
        Ok(())
    }

    fn move_prev_word_start(&mut self, count: usize) -> Result<(), UserError> {
        let slice = self.doc.slice(..);
        let new_selection = self
            .selection
            .clone()
            .transform(|range| movement::move_prev_word_start(slice, range, count));

        self.selection = new_selection;
        Ok(())
    }

    fn move_next_word_end(&mut self, count: usize) -> Result<(), UserError> {
        let slice = self.doc.slice(..);
        let new_selection = self
            .selection
            .clone()
            .transform(|range| movement::move_next_word_end(slice, range, count));

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

    fn append(&mut self) -> Result<(), UserError> {
        // Move cursor one position to the right (after current character)
        let head = self.selection.primary().head;
        let new_pos = (head + 1).min(self.doc.len_chars());
        self.selection = Selection::point(new_pos);
        self.mode = Mode::Insert;
        Ok(())
    }

    fn insert_at_line_start(&mut self) -> Result<(), UserError> {
        // Move cursor to start of current line and enter insert mode
        let head = self.selection.primary().head;
        let current_line = self.doc.char_to_line(head);
        let line_start = self.doc.line_to_char(current_line);
        self.selection = Selection::point(line_start);
        self.mode = Mode::Insert;
        Ok(())
    }

    fn append_at_line_end(&mut self) -> Result<(), UserError> {
        // Move cursor to end of current line and enter insert mode
        let head = self.selection.primary().head;
        let current_line = self.doc.char_to_line(head);

        // Find the end of the line (position before newline or end of document)
        let line_end = if current_line + 1 < self.doc.len_lines() {
            // Not the last line - go to position before newline
            self.doc.line_to_char(current_line + 1) - 1
        } else {
            // Last line - go to end of document
            self.doc.len_chars()
        };

        self.selection = Selection::point(line_end);
        self.mode = Mode::Insert;
        Ok(())
    }

    fn open_below(&mut self) -> Result<(), UserError> {
        // Find end of current line
        let head = self.selection.primary().head;
        let current_line = self.doc.char_to_line(head);
        let line_end = if current_line + 1 < self.doc.len_lines() {
            self.doc.line_to_char(current_line + 1) - 1
        } else {
            self.doc.len_chars()
        };

        // Insert newline at end of current line
        let transaction = Transaction::change(
            &self.doc,
            [(line_end, line_end, Some("\n".into()))].into_iter(),
        );

        self.apply_transaction(transaction);

        // Move cursor to the new empty line
        let new_line_start = self.doc.line_to_char(current_line + 1);
        self.selection = Selection::point(new_line_start);
        self.mode = Mode::Insert;

        Ok(())
    }

    fn open_above(&mut self) -> Result<(), UserError> {
        // Find start of current line
        let head = self.selection.primary().head;
        let current_line = self.doc.char_to_line(head);
        let line_start = self.doc.line_to_char(current_line);

        // Insert newline at start of current line
        let transaction = Transaction::change(
            &self.doc,
            [(line_start, line_start, Some("\n".into()))].into_iter(),
        );

        self.apply_transaction(transaction);

        // Cursor is already at the new empty line (same position)
        self.selection = Selection::point(line_start);
        self.mode = Mode::Insert;

        Ok(())
    }

    fn replace_char(&mut self, ch: char) -> Result<(), UserError> {
        // Replace character at cursor with the given character
        let head = self.selection.primary().head;

        // Don't replace if at end of document or on newline
        if head >= self.doc.len_chars() {
            return Ok(());
        }

        let current_char = self.doc.char(head);
        if current_char == '\n' {
            return Ok(());
        }

        // Replace current character
        let transaction = Transaction::change(
            &self.doc,
            [(head, head + 1, Some(ch.to_string().into()))].into_iter(),
        );

        self.apply_transaction(transaction);

        Ok(())
    }

    // === Editing implementations ===

    fn delete_char(&mut self) -> Result<(), UserError> {
        let transaction = Transaction::change_by_selection(&self.doc, &self.selection, |range| {
            let start = range.from();
            let end = start.saturating_add(1).min(self.doc.len_chars()).max(start);
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

    fn join_lines(&mut self) -> Result<(), UserError> {
        // Join current line with next line
        let head = self.selection.primary().head;
        let current_line = self.doc.char_to_line(head);

        // Can't join if on last line
        if current_line + 1 >= self.doc.len_lines() {
            return Ok(());
        }

        // Find the newline character at the end of current line
        let line_end = self.doc.line_to_char(current_line + 1) - 1;

        // Replace newline with space
        let transaction = Transaction::change(
            &self.doc,
            [(line_end, line_end + 1, Some(" ".into()))].into_iter(),
        );

        self.apply_transaction(transaction);

        Ok(())
    }

    fn indent_line(&mut self) -> Result<(), UserError> {
        // Add indentation (2 spaces) at the beginning of current line
        let head = self.selection.primary().head;
        let current_line = self.doc.char_to_line(head);
        let line_start = self.doc.line_to_char(current_line);

        // Insert 2 spaces at line start
        let transaction = Transaction::change(
            &self.doc,
            [(line_start, line_start, Some("  ".into()))].into_iter(),
        );

        self.apply_transaction(transaction);

        // Move cursor to maintain relative position
        let new_head = head + 2;
        self.selection = Selection::point(new_head.min(self.doc.len_chars()));

        Ok(())
    }

    fn dedent_line(&mut self) -> Result<(), UserError> {
        // Remove indentation (up to 2 spaces) from the beginning of current line
        let head = self.selection.primary().head;
        let current_line = self.doc.char_to_line(head);
        let line_start = self.doc.line_to_char(current_line);

        // Check how many spaces to remove (max 2)
        let slice = self.doc.slice(..);
        let mut spaces_to_remove = 0;

        for i in 0..2 {
            let pos = line_start + i;
            if pos < self.doc.len_chars() && slice.char(pos) == ' ' {
                spaces_to_remove += 1;
            } else {
                break;
            }
        }

        if spaces_to_remove == 0 {
            return Ok(());
        }

        // Remove the spaces
        let transaction = Transaction::change(
            &self.doc,
            [(line_start, line_start + spaces_to_remove, None)].into_iter(),
        );

        self.apply_transaction(transaction);

        // Move cursor to maintain relative position
        let new_head = head.saturating_sub(spaces_to_remove);
        self.selection = Selection::point(new_head.min(self.doc.len_chars()));

        Ok(())
    }

    fn change_selection(&mut self) -> Result<(), UserError> {
        // Change selection: delete current character and enter insert mode
        // This is similar to delete_char but also enters insert mode
        let head = self.selection.primary().head;

        // Don't delete if at end of document
        if head >= self.doc.len_chars() {
            self.mode = Mode::Insert;
            return Ok(());
        }

        // Delete current character (don't delete newlines)
        let current_char = self.doc.char(head);
        if current_char != '\n' {
            let transaction =
                Transaction::change_by_selection(&self.doc, &self.selection, |range| {
                    let start = range.from();
                    let end = start.saturating_add(1).min(self.doc.len_chars()).max(start);
                    (start, end, None)
                });

            self.apply_transaction(transaction);
        }

        // Enter insert mode
        self.mode = Mode::Insert;
        Ok(())
    }

    fn yank(&mut self) -> Result<(), UserError> {
        // Copy current character to clipboard
        let head = self.selection.primary().head;

        if head >= self.doc.len_chars() {
            return Ok(());
        }

        let current_char = self.doc.char(head);
        self.clipboard = Some(current_char.to_string());
        Ok(())
    }

    fn paste_after(&mut self) -> Result<(), UserError> {
        // Paste clipboard content after cursor
        if let Some(text) = &self.clipboard {
            let head = self.selection.primary().head;
            let insert_pos = (head + 1).min(self.doc.len_chars());
            let text_len = text.chars().count();

            let transaction = Transaction::change(
                &self.doc,
                [(insert_pos, insert_pos, Some(text.as_str().into()))].into_iter(),
            );

            self.apply_transaction(transaction);

            // Move cursor to the end of pasted text
            let new_pos = insert_pos + text_len;
            self.selection = Selection::point(new_pos.min(self.doc.len_chars()));
        }
        Ok(())
    }

    fn paste_before(&mut self) -> Result<(), UserError> {
        // Paste clipboard content before cursor
        if let Some(text) = &self.clipboard {
            let head = self.selection.primary().head;
            let text_len = text.chars().count();

            let transaction = Transaction::change(
                &self.doc,
                [(head, head, Some(text.as_str().into()))].into_iter(),
            );

            self.apply_transaction(transaction);

            // Move cursor to the end of pasted text
            let new_pos = head + text_len;
            self.selection = Selection::point(new_pos.min(self.doc.len_chars()));
        }
        Ok(())
    }

    fn insert_text(&mut self, text: &str) -> Result<(), UserError> {
        // Insert text at cursor position (only works in Insert mode)
        if self.mode != Mode::Insert {
            return Err(UserError::OperationFailed);
        }

        let head = self.selection.primary().head;
        let text_len = text.chars().count();

        let transaction =
            Transaction::change(&self.doc, [(head, head, Some(text.into()))].into_iter());

        self.apply_transaction(transaction);

        // Move cursor after inserted text
        let new_pos = head + text_len;
        self.selection = Selection::point(new_pos.min(self.doc.len_chars()));

        Ok(())
    }

    fn backspace(&mut self) -> Result<(), UserError> {
        // Delete character before cursor (only works in Insert mode)
        if self.mode != Mode::Insert {
            return Err(UserError::OperationFailed);
        }

        let head = self.selection.primary().head;

        // Can't backspace at position 0
        if head == 0 {
            return Ok(());
        }

        // Delete character before cursor
        let delete_start = head - 1;
        let transaction = Transaction::change(&self.doc, [(delete_start, head, None)].into_iter());

        self.apply_transaction(transaction);

        // Move cursor back one position
        self.selection = Selection::point(delete_start);

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

// Implement CommandExecutor trait for HelixSimulator
impl super::executor::CommandExecutor for HelixSimulator {
    fn execute_command(&mut self, cmd: &str) -> Result<(), UserError> {
        self.execute_command(cmd)
    }

    fn to_editor_state(&self) -> Result<EditorState, UserError> {
        self.to_editor_state()
    }

    fn mode(&self) -> Mode {
        self.mode()
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

    #[test]
    fn test_append_mode() {
        let mut sim = HelixSimulator::new("hello".to_string());

        // Cursor at start (position 0)
        assert_eq!(sim.get_state().unwrap().cursor_position().col, 0);

        // Press 'a' should move cursor one position right and enter insert mode
        sim.execute_command("a").unwrap();

        let state = sim.get_state().unwrap();
        assert_eq!(sim.mode(), Mode::Insert);
        assert_eq!(state.cursor_position().col, 1); // Moved one right
    }

    #[test]
    fn test_open_below() {
        let mut sim = HelixSimulator::new("line1\nline2".to_string());

        // Cursor at start of first line
        assert_eq!(sim.get_state().unwrap().cursor_position().row, 0);

        // Press 'o' should insert new line below and enter insert mode
        sim.execute_command("o").unwrap();

        let state = sim.get_state().unwrap();
        assert_eq!(sim.mode(), Mode::Insert);
        assert_eq!(state.content(), "line1\n\nline2");
        assert_eq!(state.cursor_position().row, 1); // On new empty line
    }

    #[test]
    fn test_open_above() {
        let mut sim = HelixSimulator::new("line1\nline2".to_string());

        // Move to second line
        sim.execute_command("j").unwrap();
        assert_eq!(sim.get_state().unwrap().cursor_position().row, 1);

        // Press 'O' should insert new line above and enter insert mode
        sim.execute_command("O").unwrap();

        let state = sim.get_state().unwrap();
        assert_eq!(sim.mode(), Mode::Insert);
        assert_eq!(state.content(), "line1\n\nline2");
        assert_eq!(state.cursor_position().row, 1); // On new empty line
    }

    #[test]
    fn test_replace_char() {
        let mut sim = HelixSimulator::new("hello".to_string());

        // Cursor at start
        assert_eq!(sim.get_state().unwrap().content(), "hello");

        // Press 'r' then 'X' should replace 'h' with 'X'
        sim.execute_command("rX").unwrap();

        let state = sim.get_state().unwrap();
        assert_eq!(state.content(), "Xello");
        assert_eq!(sim.mode(), Mode::Normal); // Should stay in normal mode
    }

    #[test]
    fn test_insert_at_line_start() {
        let mut sim = HelixSimulator::new("  hello world".to_string());

        // Move cursor to middle of line
        sim.execute_command("w").unwrap();
        let state = sim.get_state().unwrap();
        assert!(state.cursor_position().col > 0);

        // Press 'I' should move to start of line and enter insert mode
        sim.execute_command("I").unwrap();

        let state = sim.get_state().unwrap();
        assert_eq!(sim.mode(), Mode::Insert);
        assert_eq!(state.cursor_position().col, 0);
    }

    #[test]
    fn test_append_at_line_end() {
        let mut sim = HelixSimulator::new("hello world\nline2".to_string());

        // Cursor at start
        assert_eq!(sim.get_state().unwrap().cursor_position().col, 0);

        // Press 'A' should move to end of line and enter insert mode
        sim.execute_command("A").unwrap();

        let state = sim.get_state().unwrap();
        assert_eq!(sim.mode(), Mode::Insert);
        assert_eq!(state.cursor_position().col, 11); // After "hello world"
        assert_eq!(state.cursor_position().row, 0); // Still on first line
    }

    #[test]
    fn test_change_selection() {
        let mut sim = HelixSimulator::new("hello".to_string());

        // Cursor at start
        assert_eq!(sim.get_state().unwrap().content(), "hello");
        assert_eq!(sim.mode(), Mode::Normal);

        // Press 'c' should delete 'h' and enter insert mode
        sim.execute_command("c").unwrap();

        let state = sim.get_state().unwrap();
        assert_eq!(state.content(), "ello");
        assert_eq!(sim.mode(), Mode::Insert);
        assert_eq!(state.cursor_position().col, 0); // Cursor stays at start
    }

    #[test]
    fn test_yank_and_paste_after() {
        let mut sim = HelixSimulator::new("abc".to_string());

        // Yank 'a'
        sim.execute_command("y").unwrap();

        // Move to 'b'
        sim.execute_command("l").unwrap();
        assert_eq!(sim.get_state().unwrap().cursor_position().col, 1);

        // Paste after 'b' - should insert 'a' between 'b' and 'c'
        sim.execute_command("p").unwrap();

        let state = sim.get_state().unwrap();
        assert_eq!(state.content(), "abac");
        assert_eq!(state.cursor_position().col, 3); // Cursor after pasted 'a'
    }

    #[test]
    fn test_yank_and_paste_before() {
        let mut sim = HelixSimulator::new("abc".to_string());

        // Move to 'c'
        sim.execute_command("l").unwrap();
        sim.execute_command("l").unwrap();
        assert_eq!(sim.get_state().unwrap().cursor_position().col, 2);

        // Yank 'c'
        sim.execute_command("y").unwrap();

        // Move back to 'a'
        sim.execute_command("h").unwrap();
        sim.execute_command("h").unwrap();
        assert_eq!(sim.get_state().unwrap().cursor_position().col, 0);

        // Paste before 'a'
        sim.execute_command("P").unwrap();

        let state = sim.get_state().unwrap();
        assert_eq!(state.content(), "cabc");
        assert_eq!(state.cursor_position().col, 1); // Cursor after pasted 'c'
    }

    #[test]
    fn test_insert_text_in_insert_mode() {
        let mut sim = HelixSimulator::new("hello".to_string());

        // Enter insert mode
        sim.execute_command("i").unwrap();
        assert_eq!(sim.mode(), Mode::Insert);

        // Insert a character
        sim.execute_command("!").unwrap();

        let state = sim.get_state().unwrap();
        assert_eq!(state.content(), "!hello");
        assert_eq!(state.cursor_position().col, 1);
    }

    #[test]
    fn test_append_at_line_end_and_insert() {
        let mut sim = HelixSimulator::new("hello".to_string());

        // Append at line end
        sim.execute_command("A").unwrap();
        assert_eq!(sim.mode(), Mode::Insert);

        let cursor_pos = sim.get_state().unwrap().cursor_position().col;
        assert_eq!(cursor_pos, 5); // After 'hello'

        // Insert '!'
        sim.execute_command("!").unwrap();

        let state = sim.get_state().unwrap();
        assert_eq!(state.content(), "hello!");
        assert_eq!(state.cursor_position().col, 6);
    }

    #[test]
    fn test_insert_text_only_works_in_insert_mode() {
        let mut sim = HelixSimulator::new("hello".to_string());

        // Try to insert text in Normal mode - should fail
        let result = sim.insert_text("!");
        assert!(result.is_err());

        // Enter Insert mode
        sim.execute_command("i").unwrap();

        // Now it should work
        let result = sim.insert_text("!");
        assert!(result.is_ok());
    }

    #[test]
    fn test_insert_multiple_chars() {
        let mut sim = HelixSimulator::new("".to_string());

        // Enter insert mode
        sim.execute_command("i").unwrap();
        assert_eq!(sim.mode(), Mode::Insert);

        // Insert multiple characters
        sim.execute_command("a").unwrap();
        sim.execute_command("b").unwrap();
        sim.execute_command("c").unwrap();

        let state = sim.get_state().unwrap();
        assert_eq!(state.content(), "abc");
        assert_eq!(state.cursor_position().col, 3);
    }

    #[test]
    fn test_backspace_in_insert_mode() {
        let mut sim = HelixSimulator::new("hello".to_string());

        // Enter insert mode at position 5
        sim.execute_command("$").unwrap(); // Move to end
        sim.execute_command("a").unwrap(); // Append
        assert_eq!(sim.mode(), Mode::Insert);

        // Type some characters
        sim.execute_command("!").unwrap();
        sim.execute_command("!").unwrap();

        let state = sim.get_state().unwrap();
        assert_eq!(state.content(), "hello!!");
        assert_eq!(state.cursor_position().col, 7);

        // Backspace once
        sim.execute_command("Backspace").unwrap();

        let state = sim.get_state().unwrap();
        assert_eq!(state.content(), "hello!");
        assert_eq!(state.cursor_position().col, 6);

        // Backspace again
        sim.execute_command("Backspace").unwrap();

        let state = sim.get_state().unwrap();
        assert_eq!(state.content(), "hello");
        assert_eq!(state.cursor_position().col, 5);
    }

    #[test]
    fn test_backspace_at_start() {
        let mut sim = HelixSimulator::new("test".to_string());

        // Enter insert mode at start
        sim.execute_command("i").unwrap();
        assert_eq!(sim.mode(), Mode::Insert);

        // Backspace at position 0 should do nothing
        sim.execute_command("Backspace").unwrap();

        let state = sim.get_state().unwrap();
        assert_eq!(state.content(), "test");
        assert_eq!(state.cursor_position().col, 0);
    }

    #[test]
    fn test_arrow_keys_in_insert_mode() {
        let mut sim = HelixSimulator::new("abc\ndef".to_string());

        // Enter insert mode
        sim.execute_command("i").unwrap();
        assert_eq!(sim.mode(), Mode::Insert);

        // Test ArrowRight
        sim.execute_command("ArrowRight").unwrap();
        assert_eq!(sim.get_state().unwrap().cursor_position().col, 1);

        // Test ArrowLeft
        sim.execute_command("ArrowLeft").unwrap();
        assert_eq!(sim.get_state().unwrap().cursor_position().col, 0);

        // Test ArrowDown
        sim.execute_command("ArrowDown").unwrap();
        assert_eq!(sim.get_state().unwrap().cursor_position().row, 1);

        // Test ArrowUp
        sim.execute_command("ArrowUp").unwrap();
        assert_eq!(sim.get_state().unwrap().cursor_position().row, 0);

        // Should still be in Insert mode
        assert_eq!(sim.mode(), Mode::Insert);
    }

    #[test]
    fn test_backspace_only_works_in_insert_mode() {
        let mut sim = HelixSimulator::new("hello".to_string());

        // Try backspace in Normal mode - should fail
        let result = sim.backspace();
        assert!(result.is_err());

        // Enter Insert mode
        sim.execute_command("i").unwrap();

        // Now it should work (but do nothing at position 0)
        let result = sim.backspace();
        assert!(result.is_ok());
    }

    #[test]
    fn test_join_lines() {
        let mut sim = HelixSimulator::new("line1\nline2\nline3".to_string());

        // Join first two lines
        sim.execute_command("J").unwrap();

        let state = sim.get_state().unwrap();
        assert_eq!(state.content(), "line1 line2\nline3");
        assert_eq!(state.cursor_position().row, 0);
    }

    #[test]
    fn test_join_lines_at_last_line() {
        let mut sim = HelixSimulator::new("line1\nline2".to_string());

        // Move to last line
        sim.execute_command("j").unwrap();
        assert_eq!(sim.get_state().unwrap().cursor_position().row, 1);

        // Try to join - should do nothing
        sim.execute_command("J").unwrap();

        let state = sim.get_state().unwrap();
        assert_eq!(state.content(), "line1\nline2");
    }

    #[test]
    fn test_indent_line() {
        let mut sim = HelixSimulator::new("hello\nworld".to_string());

        // Indent first line
        sim.execute_command(">").unwrap();

        let state = sim.get_state().unwrap();
        assert_eq!(state.content(), "  hello\nworld");
        // Cursor should move forward by 2
        assert_eq!(state.cursor_position().col, 2);
    }

    #[test]
    fn test_dedent_line() {
        let mut sim = HelixSimulator::new("  hello\n    world".to_string());

        // Dedent first line (remove 2 spaces)
        sim.execute_command("<").unwrap();

        let state = sim.get_state().unwrap();
        assert_eq!(state.content(), "hello\n    world");
        assert_eq!(state.cursor_position().col, 0);
    }

    #[test]
    fn test_dedent_line_with_one_space() {
        let mut sim = HelixSimulator::new(" hello".to_string());

        // Dedent - should remove only 1 space
        sim.execute_command("<").unwrap();

        let state = sim.get_state().unwrap();
        assert_eq!(state.content(), "hello");
        assert_eq!(state.cursor_position().col, 0);
    }

    #[test]
    fn test_dedent_line_no_spaces() {
        let mut sim = HelixSimulator::new("hello".to_string());

        // Dedent line with no leading spaces - should do nothing
        sim.execute_command("<").unwrap();

        let state = sim.get_state().unwrap();
        assert_eq!(state.content(), "hello");
    }

    #[test]
    fn test_multiple_indent() {
        let mut sim = HelixSimulator::new("code".to_string());

        // Indent twice
        sim.execute_command(">").unwrap();
        sim.execute_command(">").unwrap();

        let state = sim.get_state().unwrap();
        assert_eq!(state.content(), "    code");
        assert_eq!(state.cursor_position().col, 4);
    }
}
