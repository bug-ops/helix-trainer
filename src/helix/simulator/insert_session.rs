//! The Insert session: everything between a command that enters Insert mode
//! and the `Escape` that leaves it is one undo step.
//!
//! Helix commits its history after a key only when the mode is not Insert, so
//! `i a b Escape u` restores the original text in one go. The session is owned
//! by the `HelixSimulator<InsertMode>` typestate and can only be opened through
//! [`HelixSimulator::begin_insert`](super::HelixSimulator::begin_insert) with an
//! [`InsertEntryCommand`], so an unclosed boundary cannot exist.

use super::undo::HistoryEntry;
use crate::helix::commands::{
    CMD_APPEND, CMD_APPEND_LINE_END, CMD_CHANGE, CMD_CHANGE_SELECTION_NOYANK, CMD_INSERT,
    CMD_INSERT_LINE_START, CMD_OPEN_ABOVE, CMD_OPEN_BELOW, CMD_SELECT_REGISTER,
};
use helix_core::{Assoc, ChangeSet, Rope, Selection};

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
/// assert_eq!(InsertEntryCommand::parse("o"), Some(InsertEntryCommand::OpenBelow));
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
    /// `o`: open a line below.
    OpenBelow,
    /// `O`: open a line above.
    OpenAbove,
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
    /// Parse a canonical command string, returning `None` for any command
    /// that does not enter Insert mode.
    pub fn parse(cmd: &str) -> Option<Self> {
        match cmd {
            CMD_INSERT => Some(Self::Insert),
            CMD_APPEND => Some(Self::Append),
            CMD_INSERT_LINE_START => Some(Self::InsertLineStart),
            CMD_APPEND_LINE_END => Some(Self::AppendLineEnd),
            CMD_OPEN_BELOW => Some(Self::OpenBelow),
            CMD_OPEN_ABOVE => Some(Self::OpenAbove),
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
            Self::OpenBelow => CMD_OPEN_BELOW.to_string(),
            Self::OpenAbove => CMD_OPEN_ABOVE.to_string(),
            Self::Change { register: None } => CMD_CHANGE.to_string(),
            Self::Change {
                register: Some(register),
            } => format!("{CMD_SELECT_REGISTER}{register}{CMD_CHANGE}"),
            Self::ChangeNoYank => CMD_CHANGE_SELECTION_NOYANK.to_string(),
        }
    }
}

/// Selections to restore when the session's first change happens before the
/// cursor moved: Helix stores the selection at the first change as the
/// revision's old state, and the simulator collapses the selection to one
/// cursor on entry.
#[derive(Debug, Clone)]
pub(super) struct EntrySelection {
    /// The collapsed cursor Insert mode starts with.
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum InsertEntry {
    /// `a`: Escape selects from the original selection start through the
    /// inserted text; `anchor` follows the document changes.
    Append { anchor: usize },
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
    /// End of an auto-indented line break that nothing was typed after.
    pub(super) auto_indent: Option<usize>,
}

impl InsertSession {
    pub(super) fn new(
        history_len: usize,
        saved_redo: Vec<HistoryEntry>,
        change: SessionChange,
        entry: InsertEntry,
        auto_indent: Option<usize>,
    ) -> Self {
        Self {
            history_len,
            saved_redo,
            change,
            entry,
            auto_indent,
        }
    }

    /// Note a document change about to be applied: capture the pre-session
    /// state on the first one, follow the append anchor and drop the pending
    /// auto-indent (anything typed after it makes it permanent).
    pub(super) fn on_change(&mut self, changes: &ChangeSet, before: impl FnOnce() -> HistoryEntry) {
        if let SessionChange::Pristine(entry) = &self.change {
            let mut before = before();
            if before.selection == entry.cursor {
                before.selection = entry.restore.clone();
            }
            self.change = SessionChange::Changed { before };
        }
        if let InsertEntry::Append { anchor } = &mut self.entry {
            *anchor = changes.map_pos(*anchor, Assoc::AfterSticky);
        }
        self.auto_indent = None;
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
        self.entry
    }
}
