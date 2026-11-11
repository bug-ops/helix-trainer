//! Insert mode operations

use super::commands::{self, clipboard::yank_to_register};
use super::insert_session::{
    EntrySelection, InsertEntry, InsertEntryCommand, InsertSession, SessionChange,
};
use super::{HelixSimulator, InsertMode, NormalMode};
use crate::helix::commands::{
    CMD_INSERT_ALT_BACKSPACE, CMD_INSERT_DELETE_CHAR_FORWARD, CMD_INSERT_DELETE_KEY,
    CMD_INSERT_DELETE_TO_LINE_END, CMD_INSERT_DELETE_TO_LINE_START,
    CMD_INSERT_DELETE_WORD_BACKWARD, CMD_INSERT_DELETE_WORD_FORWARD, CMD_INSERT_TAB,
};
use crate::helix::error::SimulatorError;
use crate::helix::newline_indent::{
    CommentContinuation, NewLineStart, NewlineIndent, leading_whitespace,
};
use helix_core::{
    Range, Selection, Transaction, graphemes::next_grapheme_boundary,
    line_ending::line_end_char_index, movement,
};
use helix_stdx::rope::RopeSliceExt;

/// Where the cursor goes on a non-empty line when `I` or `A` enters Insert mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum IndentFallbackPos {
    /// The first non-whitespace character of the line (`I`).
    LineStart,
    /// The end of the line, before its line break (`A`).
    LineEnd,
}

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

/// A line opened by [`HelixSimulator::open_above_at`]
pub(super) struct OpenedLine {
    /// Where the cursor sits on the new line
    pub(super) cursor: usize,
    /// Characters inserted, line break included
    pub(super) inserted: usize,
    /// Whether the new line holds nothing but indentation
    strippable: bool,
}

impl<M: super::EditorMode> HelixSimulator<M> {
    /// Open a line above the line holding char `at`, indented by the
    /// language rule, and put the cursor on it
    pub(super) fn open_above_at(&mut self, at: usize, comments: CommentContinuation) -> OpenedLine {
        let line = self.doc.char_to_line(at.min(self.doc.len_chars()));
        let line_start = self.doc.line_to_char(line);
        let start = self.newline_rule().above(line, comments);
        let text = start.text();

        let indent_len = text.chars().count();
        let cursor = line_start + indent_len;
        let transaction = Transaction::change(
            &self.doc,
            [(line_start, line_start, Some(format!("{text}\n").into()))].into_iter(),
        )
        .with_selection(Selection::point(cursor));
        self.apply_transaction(&transaction);
        OpenedLine {
            cursor,
            inserted: indent_len + 1,
            strippable: start.comment_token.is_none(),
        }
    }

    /// Whether the cursor at `head` sits between the two halves of one of
    /// the language's auto-pairs on its own line
    fn between_auto_pair(&self, head: usize, line_start: usize) -> bool {
        head > line_start
            && head < self.doc.len_chars()
            && self
                .language
                .auto_pairs
                .contains(&(self.doc.char(head - 1), self.doc.char(head)))
    }

    fn newline_rule(&self) -> NewlineIndent<'_> {
        NewlineIndent {
            doc: &self.doc,
            language: self.language,
            unit: self.indent_unit,
        }
    }

    /// Start of the line that follows `line`, considering its text up to char
    /// index `text_end`
    fn start_after(&self, line: usize, text_end: usize) -> NewLineStart {
        self.newline_rule().below(line, text_end)
    }
}

// Operations that prepare for entering insert mode (Normal mode methods)
impl HelixSimulator<NormalMode> {
    /// Run an Insert-entering command and open its Insert session.
    ///
    /// The command's edit and selection move happen in Normal mode, then the
    /// selection collapses to one cursor. Everything from here to `Escape`
    /// (including that edit) becomes one undo step, and the redo history is
    /// set aside until the session closes.
    ///
    /// # Errors
    ///
    /// Returns the unchanged Normal-mode simulator together with the
    /// command's error (boxed: the simulator is large), so the caller keeps
    /// its state.
    ///
    /// # Examples
    ///
    /// ```
    /// use helix_trainer::helix::HelixSimulator;
    /// use helix_trainer::helix::simulator::InsertEntryCommand;
    ///
    /// let sim = HelixSimulator::new("fn".to_string());
    /// let mut sim = sim.begin_insert(InsertEntryCommand::Append).map_err(|failed| failed.1)?;
    /// sim.insert_text("x")?;
    /// let sim = sim.exit_insert_mode();
    /// assert_eq!(sim.to_snapshot().content, "fxn");
    /// # Ok::<(), helix_trainer::helix::error::SimulatorError>(())
    /// ```
    pub fn begin_insert(
        mut self,
        entry: InsertEntryCommand,
    ) -> Result<HelixSimulator<InsertMode>, Box<(Self, SimulatorError)>> {
        let saved_redo = std::mem::take(&mut self.redo_stack);
        let history_len = self.history.len();
        let original = self.selection.clone();
        let appended_from = self.selection.primary().from();
        let append_restore = self.append_selection();

        let auto_indent = match self.run_entry_edit(entry) {
            Ok(auto_indent) => auto_indent,
            Err(error) => {
                self.redo_stack = saved_redo;
                return Err(Box::new((self, error)));
            }
        };
        commands::record_insert_entry(&mut self, &entry.command());

        let cursor = self.collapse_to_cursor();
        let change = match self.history.drain(history_len..).next() {
            Some(before) => SessionChange::Changed { before },
            None => SessionChange::Pristine(EntrySelection {
                restore: match entry {
                    InsertEntryCommand::Insert => original,
                    InsertEntryCommand::Append => append_restore,
                    _ => cursor.clone(),
                },
                cursor,
            }),
        };
        let kind = match entry {
            InsertEntryCommand::Append => InsertEntry::Append {
                anchor: appended_from,
            },
            _ => InsertEntry::Other,
        };
        let session = InsertSession::new(history_len, saved_redo, change, kind, auto_indent);
        Ok(self.into_mode(session))
    }

    /// The selection Helix holds right after `a`: the primary range from its
    /// start through the grapheme after its end (a point cursor is one grapheme wide)
    fn append_selection(&self) -> Selection {
        let slice = self.doc.slice(..);
        let range = self.selection.primary();
        let end = if range.is_empty() {
            next_grapheme_boundary(slice, range.head)
        } else {
            range.to()
        };
        Selection::from(Range::new(range.from(), next_grapheme_boundary(slice, end)))
    }

    /// Run the edit and selection move of `entry`; returns where an unused
    /// auto-indent would be stripped.
    fn run_entry_edit(
        &mut self,
        entry: InsertEntryCommand,
    ) -> Result<Option<usize>, SimulatorError> {
        match entry {
            InsertEntryCommand::Insert => {
                self.selection = self
                    .selection
                    .clone()
                    .transform(|range| Range::new(range.to(), range.from()));
                Ok(None)
            }
            InsertEntryCommand::Append => self.append().map(|()| None),
            InsertEntryCommand::InsertLineStart => self.insert_at_line_start().map(|()| None),
            InsertEntryCommand::AppendLineEnd => self.append_at_line_end().map(|()| None),
            InsertEntryCommand::OpenBelow => Ok(self.open_line_below()),
            InsertEntryCommand::OpenAbove => Ok(self.open_line_above()),
            InsertEntryCommand::Change { register } => {
                self.change_selection_from_register(register).map(|()| None)
            }
            InsertEntryCommand::ChangeNoYank => self.change_selection_noyank().map(|()| None),
        }
    }

    /// Append: move cursor after current character (prepare for insert mode)
    ///
    /// # Errors
    ///
    /// Never fails: always returns `Ok(())`. The `Result` keeps the signature uniform
    /// with the fallible commands that share the registry's function type.
    pub fn append(&mut self) -> Result<(), SimulatorError> {
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

    /// Insert at line start (`I`): move to the first non-whitespace character
    /// of the line, or to the line start when it has none (prepare for insert
    /// mode)
    ///
    /// On a completely empty line the cursor lands on the indentation `o`
    /// would give below that line, which is inserted, as Helix's
    /// `insert_with_indent` does. The line is the one under the primary
    /// range's block cursor.
    ///
    /// # Errors
    ///
    /// Never fails: always returns `Ok(())`. The `Result` keeps the signature uniform
    /// with the fallible commands that share the registry's function type.
    pub fn insert_at_line_start(&mut self) -> Result<(), SimulatorError> {
        self.insert_with_indent(IndentFallbackPos::LineStart);
        Ok(())
    }

    /// Append at line end (`A`): move to the end of the line (prepare for
    /// insert mode)
    ///
    /// Shares [`Self::insert_at_line_start`]'s rule for an empty line: it is
    /// auto-indented, and the line is the one under the block cursor.
    ///
    /// # Errors
    ///
    /// Never fails: always returns `Ok(())`. The `Result` keeps the signature uniform
    /// with the fallible commands that share the registry's function type.
    pub fn append_at_line_end(&mut self) -> Result<(), SimulatorError> {
        self.insert_with_indent(IndentFallbackPos::LineEnd);
        Ok(())
    }

    /// Port of Helix's `insert_with_indent` for the primary range: an empty
    /// line gets the indent `o` would give below it, any other line puts the
    /// cursor at `fallback`.
    fn insert_with_indent(&mut self, fallback: IndentFallbackPos) {
        let slice = self.doc.slice(..);
        let line = self
            .doc
            .char_to_line(self.selection.primary().cursor(slice));
        let line_start = self.doc.line_to_char(line);
        let line_end = line_end_char_index(&slice, line);

        if line_end == line_start {
            let indent = self.start_after(line, line_start).indent;
            let cursor = line_start + indent.chars().count();
            let transaction = Transaction::change(
                &self.doc,
                [(line_start, line_start, Some(indent.into()))].into_iter(),
            )
            .with_selection(Selection::point(cursor));
            self.apply_transaction(&transaction);
        } else {
            let position = match fallback {
                IndentFallbackPos::LineStart => slice
                    .line(line)
                    .first_non_whitespace_char()
                    .map_or(line_start, |offset| line_start + offset),
                IndentFallbackPos::LineEnd => line_end,
            };
            self.selection = Selection::point(position);
        }
    }

    /// Open below: insert new line below the selection's last line, indented
    /// by the language rule of [`LanguageRules`](crate::helix::LanguageRules)
    /// (prepare for insert mode)
    ///
    /// # Errors
    ///
    /// Never fails: always returns `Ok(())`. The `Result` keeps the signature uniform
    /// with the fallible commands that share the registry's function type.
    pub fn open_below(&mut self) -> Result<(), SimulatorError> {
        self.open_line_below();
        Ok(())
    }

    /// Open the line below and return where leaving Insert mode with nothing
    /// typed strips its indentation, if it is plain indentation.
    fn open_line_below(&mut self) -> Option<usize> {
        let range = self.selection.primary();
        let last_char = if range.anchor == range.head {
            range.head
        } else {
            range.to().saturating_sub(1).max(range.from())
        };
        let current_line = self.doc.char_to_line(last_char.min(self.doc.len_chars()));
        let line_end = line_end_char_index(&self.doc.slice(..), current_line);
        let start = self.start_after(current_line, line_end);
        let text = start.text();

        let cursor = line_end + 1 + text.chars().count();
        let transaction = Transaction::change(
            &self.doc,
            [(line_end, line_end, Some(format!("\n{text}").into()))].into_iter(),
        )
        .with_selection(Selection::point(cursor));
        self.apply_transaction(&transaction);
        start.comment_token.is_none().then_some(cursor)
    }

    /// Open above: insert new line above the selection's first line, indented
    /// as the line `o` would open below the previous one (prepare for insert
    /// mode)
    ///
    /// # Errors
    ///
    /// Never fails: always returns `Ok(())`. The `Result` keeps the signature uniform
    /// with the fallible commands that share the registry's function type.
    pub fn open_above(&mut self) -> Result<(), SimulatorError> {
        self.open_line_above();
        Ok(())
    }

    /// Open the line above; returns the strip position like [`Self::open_line_below`].
    fn open_line_above(&mut self) -> Option<usize> {
        let from = self.selection.primary().from();
        let opened = self.open_above_at(from, CommentContinuation::Continue);
        opened.strippable.then_some(opened.cursor)
    }

    /// Replace every character of every selection range with `ch` (Helix `r`)
    ///
    /// A point range is the character under the cursor; a cursor at the end
    /// of the document changes nothing. Line endings inside a range are
    /// replaced too, as in Helix. The selection is kept.
    ///
    /// # Errors
    ///
    /// Never fails: always returns `Ok(())`. The `Result` keeps the signature uniform
    /// with the fallible commands that share the registry's function type.
    pub fn replace_char(&mut self, ch: char) -> Result<(), SimulatorError> {
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

        let before = self.selection.clone();
        self.apply_transaction(&transaction);
        if self.doc.len_chars() == len_chars {
            self.selection = before;
        }
        Ok(())
    }

    /// Change selection: delete the full selection and write it to the
    /// default register (prepare for insert mode; Helix 'c').
    ///
    /// # Errors
    ///
    /// Never fails: always returns `Ok(())`. The `Result` keeps the signature uniform with the
    /// fallible commands that share the registry's function type.
    pub fn change_selection(&mut self) -> Result<(), SimulatorError> {
        self.change_selection_from_register(None)
    }

    /// Change selection, yanking to the given register (`"<reg>c`).
    ///
    /// `register: None` addresses the unnamed register, matching plain
    /// `c`. See [`Self::change_selection`].
    ///
    /// # Errors
    ///
    /// Returns the error of yanking to `register`; the yank currently never fails.
    pub fn change_selection_from_register(
        &mut self,
        register: Option<char>,
    ) -> Result<(), SimulatorError> {
        yank_to_register(self, register)?;
        self.change_selection_impl();
        Ok(())
    }

    /// Change selection without yanking: delete the full selection, leaving
    /// registers untouched (prepare for insert mode; Helix 'Alt-c').
    ///
    /// # Errors
    ///
    /// Never fails: always returns `Ok(())`. The `Result` keeps the signature uniform
    /// with the fallible commands that share the registry's function type.
    pub fn change_selection_noyank(&mut self) -> Result<(), SimulatorError> {
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
        if super::commands::editing::is_selection_linewise(self) {
            super::commands::editing::change_selection_linewise(self);
        } else {
            super::commands::editing::delete_active_selection(self);
        }
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
    ///
    /// # Errors
    ///
    /// Never fails: always returns `Ok(())`. The `Result` keeps the signature uniform
    /// with the fallible commands that share the registry's function type.
    pub fn insert_text(&mut self, text: &str) -> Result<(), SimulatorError> {
        if text == "\n" {
            self.insert_newline();
            return Ok(());
        }
        let head = self.selection.primary().head;
        let text_len = text.chars().count();

        let transaction =
            Transaction::change(&self.doc, [(head, head, Some(text.into()))].into_iter())
                .with_selection(Selection::point(head + text_len));
        self.apply_transaction(&transaction);

        Ok(())
    }

    /// Insert a line break the way Helix's Enter does.
    ///
    /// When the line has text before the cursor, trailing whitespace before
    /// the cursor is dropped and the new line gets the indentation of the
    /// language rule (see `newline_indent`), continuing a comment when the
    /// line is one. Between an auto-pair (`{}`) the closer moves to its own
    /// line and the cursor lands on an indented line between them. When only
    /// whitespace precedes the cursor, the line break goes at the line start
    /// so the old line, indentation included, moves down intact.
    fn insert_newline(&mut self) {
        let head = self.selection.primary().head.min(self.doc.len_chars());
        let line = self.doc.char_to_line(head);
        let line_start = self.doc.line_to_char(line);
        let before = self.doc.slice(line_start..head);

        let start = self.start_after(line, head);
        let splits_pair = start.comment_token.is_none() && self.between_auto_pair(head, line_start);
        let (from, to, text, cursor, strips_indent) = if splits_pair {
            let base = leading_whitespace(&self.doc, line);
            let inner = format!("{base}{}", self.indent_unit);
            let cursor = head + 1 + inner.chars().count();
            (head, head, format!("\n{inner}\n{base}"), cursor, false)
        } else {
            before.last_non_whitespace_char().map_or_else(
                || (line_start, line_start, "\n".to_string(), head + 1, false),
                |idx| {
                    let text = start.text();
                    let from = line_start + idx + 1;
                    let cursor = from + 1 + text.chars().count();
                    (
                        from,
                        head,
                        format!("\n{text}"),
                        cursor,
                        start.comment_token.is_none(),
                    )
                },
            )
        };

        let transaction =
            Transaction::change(&self.doc, [(from, to, Some(text.into()))].into_iter())
                .with_selection(Selection::point(cursor));
        self.apply_transaction(&transaction);
        self.session.auto_indent = strips_indent.then_some(cursor);
    }

    /// Strip the indentation of a line break nothing was typed after, on
    /// leaving Insert mode (Helix `try_restore_indent`).
    ///
    /// The removal is part of the session, so it records no history entry of
    /// its own.
    pub(super) fn strip_unused_auto_indent(&mut self) {
        let Some(end) = self.session.auto_indent.take() else {
            return;
        };
        let head = self.selection.primary().head;
        let line = self.doc.char_to_line(head.min(self.doc.len_chars()));
        let line_start = self.doc.line_to_char(line);
        if head != end || head != line_end_char_index(&self.doc.slice(..), line) {
            return;
        }
        let transaction = Transaction::change(&self.doc, [(line_start, head, None)].into_iter())
            .with_selection(Selection::point(line_start));
        self.apply_transaction(&transaction);
    }

    /// Delete character before cursor (backspace)
    ///
    /// Same single-cursor caveat as [`Self::insert_text`].
    ///
    /// # Errors
    ///
    /// Never fails: always returns `Ok(())`. The `Result` keeps the signature uniform
    /// with the fallible commands that share the registry's function type.
    pub fn backspace(&mut self) -> Result<(), SimulatorError> {
        let head = self.selection.primary().head;

        // Can't backspace at position 0
        if head == 0 {
            return Ok(());
        }

        // Delete character before cursor
        let delete_start = head - 1;
        let transaction = Transaction::change(&self.doc, [(delete_start, head, None)].into_iter())
            .with_selection(Selection::point(delete_start));
        self.apply_transaction(&transaction);

        Ok(())
    }

    /// Execute an [`InsertCommand`] at the cursor.
    ///
    /// Same single-cursor caveat as [`Self::insert_text`]. The deleted span
    /// is computed as in Helix's `delete_by_selection_insert_mode` and the
    /// cursor lands at its start.
    ///
    /// # Errors
    ///
    /// Never fails: always returns `Ok(())`. The `Result` keeps the signature uniform
    /// with the fallible commands that share the registry's function type.
    pub fn apply_insert_command(&mut self, command: InsertCommand) -> Result<(), SimulatorError> {
        if command == InsertCommand::Indent {
            let unit = self.indent_unit.to_string();
            return self.insert_text(&unit);
        }
        let head = self.selection.primary().head.min(self.doc.len_chars());
        let (start, end) = self.insert_command_span(command, head);

        let transaction = Transaction::change(&self.doc, [(start, end, None)].into_iter())
            .with_selection(Selection::point(start));
        self.apply_transaction(&transaction);

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
