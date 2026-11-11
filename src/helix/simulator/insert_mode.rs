//! Insert mode operations

use super::commands::clipboard::yank_to_register;
use super::{HelixSimulator, InsertMode, NormalMode};
use crate::helix::commands::{
    CMD_INSERT_ALT_BACKSPACE, CMD_INSERT_DELETE_CHAR_FORWARD, CMD_INSERT_DELETE_KEY,
    CMD_INSERT_DELETE_TO_LINE_END, CMD_INSERT_DELETE_TO_LINE_START,
    CMD_INSERT_DELETE_WORD_BACKWARD, CMD_INSERT_DELETE_WORD_FORWARD, CMD_INSERT_TAB,
};
use crate::security::UserError;
use helix_core::{
    Range, Rope, Selection, Transaction, graphemes::next_grapheme_boundary,
    line_ending::line_end_char_index, movement,
};
use helix_stdx::rope::RopeSliceExt;

/// An Insert-mode editing key beyond typed characters, `Backspace` and the
/// arrow keys.
///
/// This is a closed set, so a modified key or `Tab` is never mistaken for
/// literal text. Each variant mirrors the Helix 25.07.1 default binding.
///
/// # Examples
///
/// ```
/// use helix_trainer::helix::simulator::InsertCommand;
///
/// assert_eq!(InsertCommand::parse("Ctrl-w"), Some(InsertCommand::DeleteWordBackward));
/// assert_eq!(InsertCommand::parse("Alt-Backspace"), Some(InsertCommand::DeleteWordBackward));
/// assert_eq!(InsertCommand::parse("\t"), Some(InsertCommand::Indent));
/// assert_eq!(InsertCommand::parse("x"), None);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InsertCommand {
    /// `Ctrl-w` / `Alt-Backspace`: delete the word before the cursor.
    DeleteWordBackward,
    /// `Ctrl-u`: delete back to the first non-blank (or line start).
    DeleteToLineStart,
    /// `Ctrl-k`: delete to the end of the line (the newline when already there).
    DeleteToLineEnd,
    /// `Ctrl-d` / `Delete`: delete the character after the cursor.
    DeleteCharForward,
    /// `Alt-d`: delete the word after the cursor.
    DeleteWordForward,
    /// `Tab`: insert one indent unit of the document language at the cursor.
    ///
    /// ASSUMPTION: `hx` jumps to the end of the parent syntax node when the
    /// cursor is not in leading whitespace; the simulator has no syntax tree
    /// and always inserts the unit.
    Indent,
}

impl InsertCommand {
    /// Parse a canonical command string, returning `None` for anything else.
    pub fn parse(cmd: &str) -> Option<Self> {
        match cmd {
            CMD_INSERT_DELETE_WORD_BACKWARD | CMD_INSERT_ALT_BACKSPACE => {
                Some(Self::DeleteWordBackward)
            }
            CMD_INSERT_DELETE_TO_LINE_START => Some(Self::DeleteToLineStart),
            CMD_INSERT_DELETE_TO_LINE_END => Some(Self::DeleteToLineEnd),
            CMD_INSERT_DELETE_CHAR_FORWARD | CMD_INSERT_DELETE_KEY => Some(Self::DeleteCharForward),
            CMD_INSERT_DELETE_WORD_FORWARD => Some(Self::DeleteWordForward),
            CMD_INSERT_TAB => Some(Self::Indent),
            _ => None,
        }
    }

    /// The canonical command string, accepted back by [`Self::parse`].
    pub fn as_str(self) -> &'static str {
        match self {
            Self::DeleteWordBackward => CMD_INSERT_DELETE_WORD_BACKWARD,
            Self::DeleteToLineStart => CMD_INSERT_DELETE_TO_LINE_START,
            Self::DeleteToLineEnd => CMD_INSERT_DELETE_TO_LINE_END,
            Self::DeleteCharForward => CMD_INSERT_DELETE_CHAR_FORWARD,
            Self::DeleteWordForward => CMD_INSERT_DELETE_WORD_FORWARD,
            Self::Indent => CMD_INSERT_TAB,
        }
    }
}

/// The run of spaces and tabs at the start of `line`.
fn leading_whitespace(doc: &Rope, line: usize) -> String {
    doc.line(line)
        .chars()
        .take_while(|ch| matches!(ch, ' ' | '\t'))
        .collect()
}

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

    /// Open below: insert new line below the selection's last line, carrying
    /// that line's indentation (prepare for insert mode)
    pub fn open_below(&mut self) -> Result<(), UserError> {
        let range = self.selection.primary();
        let last_char = if range.anchor == range.head {
            range.head
        } else {
            range.to().saturating_sub(1).max(range.from())
        };
        let current_line = self.doc.char_to_line(last_char.min(self.doc.len_chars()));
        let line_end = line_end_char_index(&self.doc.slice(..), current_line);
        let indent = leading_whitespace(&self.doc, current_line);

        let transaction = Transaction::change(
            &self.doc,
            [(line_end, line_end, Some(format!("\n{indent}").into()))].into_iter(),
        );
        self.apply_transaction(transaction);

        let cursor = line_end + 1 + indent.chars().count();
        self.selection = Selection::point(cursor);
        self.auto_indent_end = Some(cursor);
        Ok(())
    }

    /// Open above: insert new line above the selection's first line, carrying
    /// that line's indentation (prepare for insert mode)
    pub fn open_above(&mut self) -> Result<(), UserError> {
        let head = self.selection.primary().from();
        let current_line = self.doc.char_to_line(head.min(self.doc.len_chars()));
        let line_start = self.doc.line_to_char(current_line);
        let indent = leading_whitespace(&self.doc, current_line);

        let transaction = Transaction::change(
            &self.doc,
            [(line_start, line_start, Some(format!("{indent}\n").into()))].into_iter(),
        );
        self.apply_transaction(transaction);

        let cursor = line_start + indent.chars().count();
        self.selection = Selection::point(cursor);
        self.auto_indent_end = Some(cursor);
        Ok(())
    }

    /// Replace every character of every selection range with `ch` (Helix `r`)
    ///
    /// A point range is the character under the cursor; a cursor at the end
    /// of the document changes nothing. Line endings inside a range are
    /// replaced too, as in Helix. The selection is kept.
    pub fn replace_char(&mut self, ch: char) -> Result<(), UserError> {
        let len_chars = self.doc.len_chars();
        let transaction = Transaction::change_by_selection(&self.doc, &self.selection, |range| {
            let (from, to) = if range.anchor == range.head {
                (range.head, (range.head + 1).min(len_chars))
            } else {
                (range.from(), range.to())
            };
            if from >= to {
                return (from, to, None);
            }
            let replacement: String = self.doc.slice(from..to).graphemes().map(|_| ch).collect();
            (from, to, Some(replacement.into()))
        });

        let mapped = self.selection.clone().map(transaction.changes());
        self.apply_transaction(transaction);
        if self.doc.len_chars() != len_chars {
            self.selection = mapped;
        }
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
        self.change_selection_impl();
        Ok(())
    }

    /// Change selection without yanking: delete the full selection, leaving
    /// registers untouched (prepare for insert mode; Helix 'Alt-c').
    pub fn change_selection_noyank(&mut self) -> Result<(), UserError> {
        self.change_selection_impl();
        Ok(())
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
    fn change_selection_impl(&mut self) {
        self.selection = if super::commands::editing::is_selection_linewise(self) {
            super::commands::editing::change_selection_linewise(self)
        } else {
            super::commands::editing::delete_active_selection(self)
        };
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
        if text == "\n" {
            self.insert_newline();
            return Ok(());
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

    /// Insert a line break the way Helix's Enter does.
    ///
    /// When the line has text before the cursor, trailing whitespace before
    /// the cursor is dropped and the new line copies the line's indentation.
    /// When only whitespace precedes the cursor, the line break goes at the
    /// line start so the old line, indentation included, moves down intact.
    fn insert_newline(&mut self) {
        let head = self.selection.primary().head.min(self.doc.len_chars());
        let line = self.doc.char_to_line(head);
        let line_start = self.doc.line_to_char(line);
        let before = self.doc.slice(line_start..head);

        let (from, to, text, cursor, strips_indent) = match before.last_non_whitespace_char() {
            Some(idx) => {
                let indent = leading_whitespace(&self.doc, line);
                let from = line_start + idx + 1;
                let cursor = from + 1 + indent.chars().count();
                (from, head, format!("\n{indent}"), cursor, true)
            }
            None => (line_start, line_start, "\n".to_string(), head + 1, false),
        };

        let transaction =
            Transaction::change(&self.doc, [(from, to, Some(text.into()))].into_iter());
        self.apply_transaction(transaction);
        self.selection = Selection::point(cursor);
        self.auto_indent_end = strips_indent.then_some(cursor);
    }

    /// Strip the indentation of a line break nothing was typed after, on
    /// leaving Insert mode (Helix `try_restore_indent`).
    ///
    /// The removal is folded into the session's undo step, so it records no
    /// history entry of its own.
    pub(super) fn strip_unused_auto_indent(&mut self) {
        let Some(end) = self.auto_indent_end.take() else {
            return;
        };
        let head = self.selection.primary().head;
        let line = self.doc.char_to_line(head.min(self.doc.len_chars()));
        let line_start = self.doc.line_to_char(line);
        if head != end || head != line_end_char_index(&self.doc.slice(..), line) {
            return;
        }
        let transaction = Transaction::change(&self.doc, [(line_start, head, None)].into_iter());
        transaction.apply(&mut self.doc);
        self.selection = Selection::point(line_start);
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

    /// Execute an [`InsertCommand`] at the cursor.
    ///
    /// Same single-cursor caveat as [`Self::insert_text`]. The deleted span
    /// is computed as in Helix's `delete_by_selection_insert_mode` and the
    /// cursor lands at its start.
    pub fn apply_insert_command(&mut self, command: InsertCommand) -> Result<(), UserError> {
        if command == InsertCommand::Indent {
            let unit = self.indent_unit.to_string();
            return self.insert_text(&unit);
        }
        let head = self.selection.primary().head.min(self.doc.len_chars());
        let (start, end) = self.insert_command_span(command, head);

        let transaction = Transaction::change(&self.doc, [(start, end, None)].into_iter());
        self.apply_transaction(transaction);
        self.selection = Selection::point(start);

        Ok(())
    }

    fn insert_command_span(&self, command: InsertCommand, head: usize) -> (usize, usize) {
        let text = self.doc.slice(..);
        match command {
            InsertCommand::DeleteWordBackward => {
                let start = movement::move_prev_word_start(text, Range::point(head), 1).from();
                (start.min(head), head)
            }
            InsertCommand::DeleteToLineStart => {
                let line = text.char_to_line(head);
                let first_char = text.line_to_char(line);
                let start = if head == first_char && line != 0 {
                    line_end_char_index(&text, line - 1)
                } else if let Some(pos) = text.line(line).first_non_whitespace_char()
                    && first_char + pos < head
                {
                    first_char + pos
                } else {
                    first_char
                };
                (start, head)
            }
            InsertCommand::DeleteToLineEnd => {
                let line = text.char_to_line(head);
                let line_end = line_end_char_index(&text, line);
                if head == line_end {
                    (head, text.line_to_char(line + 1).max(head))
                } else {
                    (head, line_end)
                }
            }
            InsertCommand::DeleteCharForward => (head, next_grapheme_boundary(text, head)),
            InsertCommand::DeleteWordForward => {
                let end = movement::move_next_word_end(text, Range::point(head), 1).to();
                (head, end.max(head))
            }
            InsertCommand::Indent => (head, head),
        }
    }
}
