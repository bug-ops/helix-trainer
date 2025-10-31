//! Insert mode operations

use super::{HelixSimulator, Mode};
use crate::security::UserError;
use helix_core::{Selection, Transaction};

impl HelixSimulator {
    /// Append: move cursor one position right and enter insert mode
    pub(super) fn append(&mut self) -> Result<(), UserError> {
        // Move cursor one position to the right (after current character)
        let head = self.selection.primary().head;
        let new_pos = (head + 1).min(self.doc.len_chars());
        self.selection = Selection::point(new_pos);
        self.mode = Mode::Insert;
        Ok(())
    }

    /// Insert at line start: move to beginning of line and enter insert mode
    pub(super) fn insert_at_line_start(&mut self) -> Result<(), UserError> {
        // Move cursor to start of current line and enter insert mode
        let head = self.selection.primary().head;
        let current_line = self.doc.char_to_line(head);
        let line_start = self.doc.line_to_char(current_line);
        self.selection = Selection::point(line_start);
        self.mode = Mode::Insert;
        Ok(())
    }

    /// Append at line end: move to end of line and enter insert mode
    pub(super) fn append_at_line_end(&mut self) -> Result<(), UserError> {
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

    /// Open below: insert new line below current line and enter insert mode
    pub(super) fn open_below(&mut self) -> Result<(), UserError> {
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

    /// Open above: insert new line above current line and enter insert mode
    pub(super) fn open_above(&mut self) -> Result<(), UserError> {
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

    /// Replace character at cursor with the given character
    pub(super) fn replace_char(&mut self, ch: char) -> Result<(), UserError> {
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

    /// Change selection: delete current character and enter insert mode
    pub(super) fn change_selection(&mut self) -> Result<(), UserError> {
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

    /// Insert text at cursor position (only works in Insert mode)
    pub(super) fn insert_text(&mut self, text: &str) -> Result<(), UserError> {
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

    /// Delete character before cursor (only works in Insert mode)
    pub(super) fn backspace(&mut self) -> Result<(), UserError> {
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
}
