//! Insert mode operations

use super::commands::clipboard::yank_to_register;
use super::{HelixSimulator, InsertMode, NormalMode};
use crate::security::UserError;
use helix_core::{Selection, Transaction};

// Operations that prepare for entering insert mode (Normal mode methods)
impl HelixSimulator<NormalMode> {
    /// Append: move cursor after current character (prepare for insert mode)
    pub fn append(&mut self) -> Result<(), UserError> {
        let range = self.selection.primary();
        let head = range.head;
        let anchor = range.anchor;

        // In Helix, when head > anchor (forward selection), the visual cursor
        // is on head-1. Append should insert AFTER the visual cursor position.
        // - If head > anchor: visual cursor at head-1, insert at head (no change needed)
        // - If head <= anchor (point or backward): visual cursor at head, insert at head+1
        let new_pos = if head > anchor {
            // Forward selection: head already points after visual cursor
            head.min(self.doc.len_chars())
        } else {
            // Point or backward: need to move one right
            (head + 1).min(self.doc.len_chars())
        };

        self.selection = Selection::point(new_pos);
        Ok(())
    }

    /// Insert at line start: move to beginning of line (prepare for insert mode)
    pub fn insert_at_line_start(&mut self) -> Result<(), UserError> {
        // Move cursor to start of current line
        let head = self.selection.primary().head;
        let current_line = self.doc.char_to_line(head);
        let line_start = self.doc.line_to_char(current_line);
        self.selection = Selection::point(line_start);
        Ok(())
    }

    /// Append at line end: move to end of line (prepare for insert mode)
    pub fn append_at_line_end(&mut self) -> Result<(), UserError> {
        // Move cursor to end of current line
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
        Ok(())
    }

    /// Open below: insert new line below current line (prepare for insert mode)
    pub fn open_below(&mut self) -> Result<(), UserError> {
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

        Ok(())
    }

    /// Open above: insert new line above current line (prepare for insert mode)
    pub fn open_above(&mut self) -> Result<(), UserError> {
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

        Ok(())
    }

    /// Replace character at cursor with the given character
    pub fn replace_char(&mut self, ch: char) -> Result<(), UserError> {
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

    /// Change selection: delete the full selection and write it to the
    /// default register (prepare for insert mode; Helix 'c').
    pub fn change_selection(&mut self) -> Result<(), UserError> {
        self.change_selection_from_register(None)
    }

    /// Change selection, yanking to the given register (`"<reg>c`).
    ///
    /// `register: None` addresses the unnamed register, matching plain
    /// `c`. See [`Self::change_selection`].
    pub fn change_selection_from_register(
        &mut self,
        register: Option<char>,
    ) -> Result<(), UserError> {
        yank_to_register(self, register)?;
        self.change_selection_impl()
    }

    /// Change selection without yanking: delete the full selection, leaving
    /// registers untouched (prepare for insert mode; Helix 'Alt-c').
    pub fn change_selection_noyank(&mut self) -> Result<(), UserError> {
        self.change_selection_impl()
    }

    /// Delete the full active selection (respecting multi-range selections,
    /// each mapped to its own correct post-deletion position) and leave the
    /// mapped selection for `enter_insert_mode` to collapse - it reduces any
    /// selection to a single (correctly positioned) cursor at the dispatch
    /// call site, since this simulator's Insert mode only ever carries one
    /// cursor, matching every other insert-entry command (`a`, `i`, `o`,
    /// `O`, ...).
    ///
    /// A linewise selection (every range spans whole lines including their
    /// trailing newline, as produced by `x`) is handled differently,
    /// matching upstream Helix: instead of leaving a mid-line insertion
    /// point where the deleted text used to start, each range is replaced
    /// with a blank line and the cursor lands on it (`Open::Above`), so
    /// `xc` behaves like `O` rather than a plain insert at the deletion
    /// point.
    fn change_selection_impl(&mut self) -> Result<(), UserError> {
        self.selection = if super::commands::editing::is_selection_linewise(self) {
            super::commands::editing::change_selection_linewise(self)
        } else {
            super::commands::editing::delete_active_selection(self)
        };
        Ok(())
    }
}

// Operations only available in Insert mode
impl HelixSimulator<InsertMode> {
    /// Insert text at cursor position
    ///
    /// Reads/writes only `selection.primary()`, overwriting the whole
    /// selection with a single point - see `enter_insert_mode`'s doc
    /// comment for why every Insert-mode command is single-cursor, not just
    /// mode entry.
    pub fn insert_text(&mut self, text: &str) -> Result<(), UserError> {
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

    /// Delete character before cursor (backspace)
    ///
    /// Same single-cursor caveat as [`Self::insert_text`].
    pub fn backspace(&mut self) -> Result<(), UserError> {
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
