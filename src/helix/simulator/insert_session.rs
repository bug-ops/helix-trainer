//! The Insert session: everything between a command that enters Insert mode
//! and the `Escape` that leaves it is one undo step.
//!
//! Helix commits its history after a key only when the mode is not Insert, so
//! `i a b Escape u` restores the original text in one go. The session is owned
//! by the `HelixSimulator<InsertMode>` typestate and can only be opened through
//! [`HelixSimulator::begin_insert`](super::HelixSimulator::begin_insert) with an
//! [`InsertEntryCommand`], so an unclosed boundary cannot exist.

use super::undo::HistoryEntry;
use crate::grammar::commands::{
    CMD_APPEND, CMD_APPEND_LINE_END, CMD_CHANGE, CMD_CHANGE_SELECTION_NOYANK, CMD_INSERT,
    CMD_INSERT_LINE_START, CMD_OPEN_ABOVE, CMD_OPEN_BELOW, CMD_SELECT_REGISTER,
};
use helix_core::{Assoc, ChangeSet, Operation, Rope, Selection};
use std::num::NonZeroUsize;

/// A command that enters Insert mode.
///
/// This is the closed set the dispatcher recognises; the registry marks the
/// same keys with `ModeTransition::ToInsert` (a test keeps the two in sync).
///
/// # Examples
///
/// ```
/// use helix_trainer::helix::simulator::InsertEntryCommand;
///
/// assert_eq!(
///     InsertEntryCommand::parse("o"),
///     InsertEntryCommand::parse("1o"),
/// );
/// assert_eq!(
///     InsertEntryCommand::parse("2O").map(InsertEntryCommand::command),
///     Some("2O".to_string())
/// );
/// assert_eq!(
///     InsertEntryCommand::parse("\"ac"),
///     Some(InsertEntryCommand::Change { register: Some('a') })
/// );
/// assert_eq!(InsertEntryCommand::parse("d"), None);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InsertEntryCommand {
    /// `i`: insert at the selection.
    Insert,
    /// `a`: append after the selection.
    Append,
    /// `I`: insert at the first non-blank character of the line.
    InsertLineStart,
    /// `A`: append at the end of the line.
    AppendLineEnd,
    /// `o`: open `count` lines below each range, one cursor on each.
    OpenBelow {
        /// How many lines to open per range.
        count: NonZeroUsize,
    },
    /// `O`: open `count` lines above each range, one cursor on each.
    OpenAbove {
        /// How many lines to open per range.
        count: NonZeroUsize,
    },
    /// `c` or `"<reg>c`: delete the selection into `register` (the unnamed
    /// register for `None`) and insert.
    Change {
        /// Register the deleted text goes to.
        register: Option<char>,
    },
    /// `Alt-c`: delete the selection without touching any register and insert.
    ChangeNoYank,
}

impl InsertEntryCommand {
    /// The count of `o` and `O` without a prefix.
    const ONCE: NonZeroUsize = NonZeroUsize::MIN;

    /// Parse a canonical command string (`o`, `3O`, `"ac`), returning `None`
    /// for any command that does not enter Insert mode.
    pub fn parse(cmd: &str) -> Option<Self> {
        if let Some((count, base)) = crate::helix::registry::parse_counted_command(cmd) {
            return match base {
                CMD_OPEN_BELOW => Some(Self::OpenBelow { count }),
                CMD_OPEN_ABOVE => Some(Self::OpenAbove { count }),
                _ => None,
            };
        }
        match cmd {
            CMD_INSERT => Some(Self::Insert),
            CMD_APPEND => Some(Self::Append),
            CMD_INSERT_LINE_START => Some(Self::InsertLineStart),
            CMD_APPEND_LINE_END => Some(Self::AppendLineEnd),
            CMD_OPEN_BELOW => Some(Self::OpenBelow { count: Self::ONCE }),
            CMD_OPEN_ABOVE => Some(Self::OpenAbove { count: Self::ONCE }),
            CMD_CHANGE => Some(Self::Change { register: None }),
            CMD_CHANGE_SELECTION_NOYANK => Some(Self::ChangeNoYank),
            _ => {
                let mut chars = cmd.chars();
                match (chars.next(), chars.next(), chars.next(), chars.next()) {
                    (Some('"'), Some(register), Some('c'), None) => Some(Self::Change {
                        register: Some(register),
                    }),
                    _ => None,
                }
            }
        }
    }

    /// The canonical command string, accepted back by [`Self::parse`].
    pub fn command(self) -> String {
        match self {
            Self::Insert => CMD_INSERT.to_string(),
            Self::Append => CMD_APPEND.to_string(),
            Self::InsertLineStart => CMD_INSERT_LINE_START.to_string(),
            Self::AppendLineEnd => CMD_APPEND_LINE_END.to_string(),
            Self::OpenBelow { count } => counted(count, CMD_OPEN_BELOW),
            Self::OpenAbove { count } => counted(count, CMD_OPEN_ABOVE),
            Self::Change { register: None } => CMD_CHANGE.to_string(),
            Self::Change {
                register: Some(register),
            } => format!("{CMD_SELECT_REGISTER}{register}{CMD_CHANGE}"),
            Self::ChangeNoYank => CMD_CHANGE_SELECTION_NOYANK.to_string(),
        }
    }

    /// The command without its count, which is what `.` replays: Helix does
    /// not record the count of the Insert session it repeats.
    #[must_use]
    pub fn uncounted(self) -> Self {
        match self {
            Self::OpenBelow { .. } => Self::OpenBelow { count: Self::ONCE },
            Self::OpenAbove { .. } => Self::OpenAbove { count: Self::ONCE },
            other => other,
        }
    }
}

fn counted(count: NonZeroUsize, base: &str) -> String {
    if count == InsertEntryCommand::ONCE {
        base.to_string()
    } else {
        format!("{count}{base}")
    }
}

/// Whether `text` is one line break (a CRLF is one grapheme) followed only by
/// whitespace that is not a line break.
fn line_break_then_blanks(text: &str) -> bool {
    let rest = text
        .strip_prefix("\r\n")
        .or_else(|| text.strip_prefix(['\n', '\r']));
    rest.is_some_and(|rest| {
        rest.chars()
            .all(|c| c.is_whitespace() && c != '\n' && c != '\r')
    })
}

/// Selections to restore when the session's first change happens before the
/// cursor moved: Helix stores the selection at the first change as the
/// revision's old state, and the simulator collapses every range to a point
/// cursor on entry.
#[derive(Debug, Clone)]
pub(super) struct EntrySelection {
    /// The point cursors Insert mode starts with.
    pub(super) cursor: Selection,
    /// What Helix holds at that point, restored by `u`.
    pub(super) restore: Selection,
}

/// Whether the session has changed the document yet.
#[derive(Debug, Clone)]
pub(super) enum SessionChange {
    /// Nothing changed: the selection to restore if the first change comes
    /// while the cursor is still where Insert mode started.
    Pristine(EntrySelection),
    /// The state from before the session's first change, which becomes the
    /// session's single undo entry.
    Changed { before: HistoryEntry },
}

/// How the session was entered, for the selection `Escape` leaves behind.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum InsertEntry {
    /// `a`: Escape selects, per range, from the original selection start
    /// through the inserted text; each anchor follows the document changes.
    Append { anchors: Vec<usize> },
    /// Every other entry.
    Other,
}

/// The open Insert session of a [`HelixSimulator<InsertMode>`](super::HelixSimulator).
///
/// Opaque: it exists so the typestate can carry the session; it is created by
/// `begin_insert` or `enter_insert_mode` and closed by `exit_insert_mode`.
#[derive(Debug)]
pub struct InsertSession {
    history_len: usize,
    saved_redo: Vec<HistoryEntry>,
    change: SessionChange,
    entry: InsertEntry,
    /// Every change of the session, the entering command's edit included,
    /// composed into one: what Helix inspects to strip an unused indent.
    /// Boxed to keep the Insert-mode simulator from outgrowing the Normal one.
    changes: Box<ChangeSet>,
}

impl InsertSession {
    pub(super) fn new(
        history_len: usize,
        saved_redo: Vec<HistoryEntry>,
        change: SessionChange,
        entry: InsertEntry,
        changes: ChangeSet,
    ) -> Self {
        Self {
            history_len,
            saved_redo,
            change,
            entry,
            changes: Box::new(changes),
        }
    }

    /// Note a document change about to be applied: capture the pre-session
    /// state on the first one, follow the append anchors and fold the change
    /// into the session's composed change set.
    pub(super) fn on_change(&mut self, changes: &ChangeSet, before: impl FnOnce() -> HistoryEntry) {
        if let SessionChange::Pristine(entry) = &self.change {
            let mut before = before();
            if before.selection == entry.cursor {
                before.selection = entry.restore.clone();
            }
            self.change = SessionChange::Changed { before };
        }
        if let InsertEntry::Append { anchors } = &mut self.entry {
            changes.update_positions(
                anchors
                    .iter_mut()
                    .map(|anchor| (anchor, Assoc::AfterSticky)),
            );
        }
        let composed = std::mem::replace(&mut *self.changes, ChangeSet::with_capacity(0));
        *self.changes = composed.compose(changes.clone());
    }

    /// Whether the session as a whole is one new line break followed only by
    /// whitespace, with the cursor `cursor` at its end on a line ending at
    /// `line_end`: Helix's `inserted_a_new_blank_line`, whose result decides
    /// if `Escape` strips the auto-indent.
    pub(super) fn inserted_a_new_blank_line(&self, cursor: usize, line_end: usize) -> bool {
        let [
            Operation::Retain(moved),
            Operation::Insert(inserted),
            Operation::Retain(_),
        ] = self.changes.changes()
        else {
            return false;
        };
        // Helix adds the byte length of the inserted text to a char count, so
        // the sum is only exact for ASCII whitespace; mirrored as is.
        moved + inserted.len() == cursor && cursor == line_end && line_break_then_blanks(inserted)
    }

    /// Close the session: record at most one undo entry (none when the
    /// document ended up unchanged, which also keeps the redo history) and
    /// return how the session was entered.
    pub(super) fn close(
        &mut self,
        doc: &Rope,
        history: &mut Vec<HistoryEntry>,
        redo_stack: &mut Vec<HistoryEntry>,
    ) -> InsertEntry {
        debug_assert_eq!(history.len(), self.history_len);
        let saved_redo = std::mem::take(&mut self.saved_redo);
        match &self.change {
            SessionChange::Changed { before } if before.doc != *doc => {
                history.push(before.clone());
                redo_stack.clear();
            }
            _ => *redo_stack = saved_redo,
        }
        self.entry.clone()
    }
}
