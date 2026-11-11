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
    Assoc, ChangeSet, Range, Selection, Transaction, auto_pairs,
    graphemes::{next_grapheme_boundary, prev_grapheme_boundary},
    line_ending::line_end_char_index,
    movement,
};
use helix_stdx::rope::RopeSliceExt;
use std::num::NonZeroUsize;

/// Where the cursor goes on a non-empty line when `I` or `A` enters Insert mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum IndentFallbackPos {
    /// The first non-whitespace character of the line (`I`).
    LineStart,
    /// The end of the line, before its line break (`A`).
    LineEnd,
}

/// Which side of a range `o` or `O` opens lines on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OpenSide {
    /// Below the last line of the range (`o`).
    Below,
    /// Above the first line of the range (`O`).
    Above,
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
}

/// One edit of an Insert-mode keystroke that applies at every cursor, and
/// where that cursor ends up.
pub(super) struct CursorEdit {
    from: usize,
    to: usize,
    text: String,
    /// Position in the old document the new cursor is derived from
    anchor: usize,
    /// Which side of an edit at `anchor` the cursor falls on
    assoc: Assoc,
    /// Characters past the mapped anchor
    plus: usize,
}

impl CursorEdit {
    /// Replace `from..to` with `text`, the cursor landing after the text
    /// (`to` is where the cursor was).
    fn replace(from: usize, to: usize, text: String) -> Self {
        Self {
            from,
            to,
            text,
            anchor: to,
            assoc: Assoc::After,
            plus: 0,
        }
    }

    /// Delete `from..to`, the cursor landing where the span started.
    fn delete(from: usize, to: usize) -> Self {
        Self {
            from,
            to,
            text: String::new(),
            anchor: from,
            assoc: Assoc::Before,
            plus: 0,
        }
    }

    /// No edit: the cursor just moves to `at`.
    fn move_to(at: usize) -> Self {
        Self::delete(at, at)
    }
}

impl<M: super::EditorMode> HelixSimulator<M> {
    /// Apply one edit per cursor in a single transaction and leave a point
    /// cursor per edit, in order, keeping the primary index.
    ///
    /// Overlapping edits (two cursors deleting the same word) are trimmed so
    /// the later one starts where the earlier one ended.
    pub(super) fn apply_cursor_edits(&mut self, edits: Vec<CursorEdit>) {
        let primary = self.selection.primary_index();
        let mut last = 0;
        let edits: Vec<CursorEdit> = edits
            .into_iter()
            .map(|mut edit| {
                edit.from = edit.from.max(last);
                edit.to = edit.to.max(edit.from);
                last = edit.to;
                edit
            })
            .collect();
        let transaction = Transaction::change(
            &self.doc,
            edits.iter().map(|edit| {
                (
                    edit.from,
                    edit.to,
                    (!edit.text.is_empty()).then(|| edit.text.as_str().into()),
                )
            }),
        );
        let cursors = edits
            .iter()
            .map(|edit| {
                Range::point(transaction.changes().map_pos(edit.anchor, edit.assoc) + edit.plus)
            })
            .collect();
        self.apply_transaction(&transaction.with_selection(Selection::new(cursors, primary)));
    }

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
    /// selection collapses to a point cursor per range. Everything from here to `Escape`
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
        let appended_from: Vec<usize> = self.selection.ranges().iter().map(Range::from).collect();
        let append_restore = self.append_selection();
        self.entry_changes = Some(ChangeSet::new(self.doc.slice(..)));

        if let Err(error) = self.run_entry_edit(entry) {
            self.entry_changes = None;
            self.redo_stack = saved_redo;
            return Err(Box::new((self, error)));
        }
        let entry_changes = self
            .entry_changes
            .take()
            .unwrap_or_else(|| ChangeSet::new(self.doc.slice(..)));
        commands::record_insert_entry(&mut self, &entry.uncounted().command());

        let cursors = self.collapse_to_points();
        let change = match self.history.drain(history_len..).next() {
            Some(before) => SessionChange::Changed { before },
            None => SessionChange::Pristine(EntrySelection {
                restore: match entry {
                    InsertEntryCommand::Insert => original,
                    InsertEntryCommand::Append => append_restore,
                    _ => cursors.clone(),
                },
                cursor: cursors,
            }),
        };
        let kind = match entry {
            InsertEntryCommand::Append => InsertEntry::Append {
                anchors: appended_from,
            },
            _ => InsertEntry::Other,
        };
        let session = InsertSession::new(history_len, saved_redo, change, kind, entry_changes);
        Ok(self.into_mode(session))
    }

    /// The selection Helix holds right after `a`: each range from its start
    /// through the grapheme after its end (a point cursor is one grapheme wide)
    fn append_selection(&self) -> Selection {
        let slice = self.doc.slice(..);
        let ranges = self
            .selection
            .ranges()
            .iter()
            .map(|range| {
                let end = if range.is_empty() {
                    next_grapheme_boundary(slice, range.head)
                } else {
                    range.to()
                };
                Range::new(range.from(), next_grapheme_boundary(slice, end))
            })
            .collect();
        Selection::new(ranges, self.selection.primary_index())
    }

    /// Run the edit and selection move of `entry`.
    fn run_entry_edit(&mut self, entry: InsertEntryCommand) -> Result<(), SimulatorError> {
        match entry {
            InsertEntryCommand::Insert => {
                self.selection = self
                    .selection
                    .clone()
                    .transform(|range| Range::new(range.to(), range.from()));
                Ok(())
            }
            InsertEntryCommand::Append => self.append(),
            InsertEntryCommand::InsertLineStart => self.insert_at_line_start(),
            InsertEntryCommand::AppendLineEnd => self.append_at_line_end(),
            InsertEntryCommand::OpenBelow { count } => {
                self.open_lines(OpenSide::Below, count);
                Ok(())
            }
            InsertEntryCommand::OpenAbove { count } => {
                self.open_lines(OpenSide::Above, count);
                Ok(())
            }
            InsertEntryCommand::Change { register } => {
                self.change_selection_from_register(register)
            }
            InsertEntryCommand::ChangeNoYank => self.change_selection_noyank(),
        }
    }

    /// Append: move every cursor after its current character (prepare for
    /// insert mode)
    ///
    /// # Errors
    ///
    /// Never fails: always returns `Ok(())`. The `Result` keeps the signature uniform
    /// with the fallible commands that share the registry's function type.
    pub fn append(&mut self) -> Result<(), SimulatorError> {
        let len = self.doc.len_chars();
        self.selection = self.selection.clone().transform(|range| {
            // In Helix, when head > anchor (forward selection), the visual cursor
            // is on head-1, so appending after it inserts at head; a point or
            // backward range has its visual cursor on head, so one more to the right.
            let at = if range.head > range.anchor {
                range.head
            } else {
                range.head + 1
            };
            Range::point(at.min(len))
        });
        Ok(())
    }

    /// Insert at line start (`I`): move to the first non-whitespace character
    /// of the line, or to the line start when it has none (prepare for insert
    /// mode)
    ///
    /// On a completely empty line the cursor lands on the indentation `o`
    /// would give below that line, which is inserted, as Helix's
    /// `insert_with_indent` does. The line is the one under the range's block
    /// cursor, for every range.
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

    /// Port of Helix's `insert_with_indent`: on an empty line the indent `o`
    /// would give below it is inserted, on any other line the cursor goes to
    /// `fallback`; done for every range in one transaction.
    fn insert_with_indent(&mut self, fallback: IndentFallbackPos) {
        let slice = self.doc.slice(..);
        let edits = self
            .selection
            .ranges()
            .iter()
            .map(|range| {
                let line = self.doc.char_to_line(range.cursor(slice));
                let line_start = self.doc.line_to_char(line);
                let line_end = line_end_char_index(&slice, line);
                if line_end == line_start {
                    let indent = self.start_after(line, line_start).indent;
                    CursorEdit {
                        plus: indent.chars().count(),
                        text: indent,
                        ..CursorEdit::move_to(line_start)
                    }
                } else {
                    CursorEdit::move_to(match fallback {
                        IndentFallbackPos::LineStart => slice
                            .line(line)
                            .first_non_whitespace_char()
                            .map_or(line_start, |offset| line_start + offset),
                        IndentFallbackPos::LineEnd => line_end,
                    })
                }
            })
            .collect();
        self.apply_cursor_edits(edits);
    }

    /// Open below: insert a new line below the last line of every range,
    /// indented by the language rule of
    /// [`LanguageRules`](crate::helix::LanguageRules) (prepare for insert mode)
    ///
    /// # Errors
    ///
    /// Never fails: always returns `Ok(())`. The `Result` keeps the signature uniform
    /// with the fallible commands that share the registry's function type.
    pub fn open_below(&mut self) -> Result<(), SimulatorError> {
        self.open_lines(OpenSide::Below, NonZeroUsize::MIN);
        Ok(())
    }

    /// Open above: insert a new line above the first line of every range,
    /// indented as the line `o` would open below the previous one (prepare for
    /// insert mode)
    ///
    /// # Errors
    ///
    /// Never fails: always returns `Ok(())`. The `Result` keeps the signature uniform
    /// with the fallible commands that share the registry's function type.
    pub fn open_above(&mut self) -> Result<(), SimulatorError> {
        self.open_lines(OpenSide::Above, NonZeroUsize::MIN);
        Ok(())
    }

    /// Open `count` lines on `side` of every range and put one cursor on each
    /// new line, as Helix's `open`: the primary index is kept, so it points
    /// at the first opened line of a single range.
    fn open_lines(&mut self, side: OpenSide, count: NonZeroUsize) {
        let slice = self.doc.slice(..);
        let mut inserts: Vec<(usize, String)> = Vec::new();
        let mut cursors = Vec::new();
        let mut shift = 0;
        for range in self.selection.ranges() {
            let (at, unit, cursor_in_unit) = match side {
                OpenSide::Below => {
                    let last_char = if range.anchor == range.head {
                        range.head
                    } else {
                        range.to().saturating_sub(1).max(range.from())
                    };
                    let line = self.doc.char_to_line(last_char.min(self.doc.len_chars()));
                    let line_end = line_end_char_index(&slice, line);
                    let unit = format!("\n{}", self.start_after(line, line_end).text());
                    let end = unit.chars().count();
                    (line_end, unit, end)
                }
                OpenSide::Above => {
                    let line = self
                        .doc
                        .char_to_line(range.from().min(self.doc.len_chars()));
                    let text = self
                        .newline_rule()
                        .above(line, CommentContinuation::Continue)
                        .text();
                    // Helix opens above a line by breaking the line before it
                    // (the composed edit `Escape` inspects needs that shape);
                    // only the first line has no line before it.
                    if line == 0 {
                        let indent_len = text.chars().count();
                        (0, format!("{text}\n"), indent_len)
                    } else {
                        let unit = format!("\n{text}");
                        let end = unit.chars().count();
                        (line_end_char_index(&slice, line - 1), unit, end)
                    }
                }
            };
            let unit_len = unit.chars().count();
            for line in 0..count.get() {
                cursors.push(Range::point(at + shift + line * unit_len + cursor_in_unit));
            }
            shift += unit_len * count.get();
            inserts.push((at, unit.repeat(count.get())));
        }
        let transaction = Transaction::change(
            &self.doc,
            inserts
                .into_iter()
                .map(|(at, text)| (at, at, Some(text.into()))),
        )
        .with_selection(Selection::new(
            cursors.into(),
            self.selection.primary_index(),
        ));
        self.apply_transaction(&transaction);
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
    /// mapped selection for `begin_insert` to collapse to one point cursor per
    /// range, as for every other insert-entry command (`a`, `i`, `o`, `O`, ...).
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
    /// The heads of every cursor, in document order
    fn heads(&self) -> Vec<usize> {
        let len = self.doc.len_chars();
        self.selection
            .ranges()
            .iter()
            .map(|range| range.head.min(len))
            .collect()
    }

    /// Insert text at every cursor, in one transaction
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
        let mut chars = text.chars();
        if let (Some(ch), None) = (chars.next(), chars.next())
            && self.insert_with_auto_pairs(ch)
        {
            return Ok(());
        }
        let edits = self
            .heads()
            .into_iter()
            .map(|head| CursorEdit::replace(head, head, text.to_string()))
            .collect();
        self.apply_cursor_edits(edits);
        Ok(())
    }

    /// Type `ch` through Helix's auto-pair hook, one character at a time, and
    /// return whether the hook handled it. Each range ends up as a point
    /// cursor at its head, so several ranges pair independently.
    fn insert_with_auto_pairs(&mut self, ch: char) -> bool {
        // Helix hands the hook its Insert-mode ranges: one grapheme wide and
        // backward, the shape `i` leaves, which the hook's cursor math expects.
        let slice = self.doc.slice(..);
        let hook_selection = self
            .selection
            .clone()
            .transform(|range| Range::new(next_grapheme_boundary(slice, range.head), range.head));
        let Some(transaction) = auto_pairs::hook(&self.doc, &hook_selection, ch, &self.auto_pairs)
        else {
            return false;
        };
        let cursors = transaction
            .selection()
            .unwrap_or(&hook_selection)
            .clone()
            .transform(|range| Range::point(range.head));
        self.apply_transaction(&transaction.with_selection(cursors));
        true
    }

    /// Insert a line break at every cursor the way Helix's Enter does.
    fn insert_newline(&mut self) {
        let edits = self
            .heads()
            .into_iter()
            .map(|head| self.newline_edit(head))
            .collect();
        self.apply_cursor_edits(edits);
    }

    /// The edit of Enter at `head`.
    ///
    /// When the line has text before the cursor, trailing whitespace before
    /// the cursor is dropped and the new line gets the indentation of the
    /// language rule (see `newline_indent`), continuing a comment when the
    /// line is one. Between an auto-pair (`{}`) the closer moves to its own
    /// line and the cursor lands on an indented line between them. When only
    /// whitespace precedes the cursor, the line break goes at the line start
    /// so the old line, indentation included, moves down intact.
    fn newline_edit(&self, head: usize) -> CursorEdit {
        let line = self.doc.char_to_line(head);
        let line_start = self.doc.line_to_char(line);
        let before = self.doc.slice(line_start..head);

        let start = self.start_after(line, head);
        if start.comment_token.is_none() && self.between_auto_pair(head, line_start) {
            let base = leading_whitespace(&self.doc, line);
            let inner = format!("{base}{}", self.indent_unit);
            return CursorEdit {
                from: head,
                to: head,
                anchor: head,
                assoc: Assoc::Before,
                plus: 1 + inner.chars().count(),
                text: format!("\n{inner}\n{base}"),
            };
        }
        match before.last_non_whitespace_char() {
            Some(idx) => {
                CursorEdit::replace(line_start + idx + 1, head, format!("\n{}", start.text()))
            }
            None => CursorEdit {
                anchor: head,
                assoc: Assoc::After,
                ..CursorEdit::replace(line_start, line_start, "\n".to_string())
            },
        }
    }

    /// Strip the indentation of a line break nothing was typed after, on
    /// leaving Insert mode (Helix `try_restore_indent`).
    ///
    /// The removal is part of the session, so it records no history entry of
    /// its own.
    pub(super) fn strip_unused_auto_indent(&mut self) {
        let head = self.selection.primary().head.min(self.doc.len_chars());
        let line = self.doc.char_to_line(head);
        let line_start = self.doc.line_to_char(line);
        let line_end = line_end_char_index(&self.doc.slice(..), line);
        if !self.session.inserted_a_new_blank_line(head, line_end) {
            return;
        }
        let transaction = Transaction::change(&self.doc, [(line_start, head, None)].into_iter())
            .with_selection(Selection::point(line_start));
        self.apply_transaction(&transaction);
    }

    /// Delete the character before every cursor (backspace), both halves of
    /// an auto-pair when the cursor sits between them
    ///
    /// # Errors
    ///
    /// Never fails: always returns `Ok(())`. The `Result` keeps the signature uniform
    /// with the fallible commands that share the registry's function type.
    pub fn backspace(&mut self) -> Result<(), SimulatorError> {
        let slice = self.doc.slice(..);
        let edits = self
            .heads()
            .into_iter()
            .map(|head| {
                let start = prev_grapheme_boundary(slice, head);
                if self.between_auto_pair(head, 0) {
                    CursorEdit::delete(start, next_grapheme_boundary(slice, head))
                } else {
                    CursorEdit::delete(start, head)
                }
            })
            .collect();
        self.apply_cursor_edits(edits);
        Ok(())
    }

    /// Execute an [`InsertCommand`] at every cursor.
    ///
    /// The deleted span is computed as in Helix's
    /// `delete_by_selection_insert_mode` and the cursor lands at its start.
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
        let edits = self
            .heads()
            .into_iter()
            .map(|head| {
                let (start, end) = self.insert_command_span(command, head);
                CursorEdit::delete(start, end)
            })
            .collect();
        self.apply_cursor_edits(edits);
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
                match head.cmp(&line_end) {
                    std::cmp::Ordering::Less => (head, line_end),
                    std::cmp::Ordering::Equal => (head, text.line_to_char(line + 1).max(head)),
                    // Inside a CRLF pair typed one character at a time: nothing to delete
                    std::cmp::Ordering::Greater => (head, head),
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
