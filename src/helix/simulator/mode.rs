//! Editor mode typestate markers
//!
//! This module provides zero-cost type-level markers for editor modes,
//! enabling compile-time enforcement of mode-specific operations.
//!
//! # Type Safety
//!
//! The typestate pattern ensures that mode-specific commands are only
//! available in the correct mode:
//!
//! ```
//! use helix_trainer::helix::{HelixSimulator, InsertMode, NormalMode};
//!
//! let sim: HelixSimulator<NormalMode> = HelixSimulator::new("hello".to_string());
//! let mut sim: HelixSimulator<InsertMode> = sim.enter_insert_mode();
//! sim.insert_text("x")?;
//! let _sim: HelixSimulator<NormalMode> = sim.exit_insert_mode();
//! # Ok::<(), helix_trainer::security::UserError>(())
//! ```
//!
//! Text insertion is not available in Normal mode:
//!
//! ```compile_fail
//! use helix_trainer::helix::{HelixSimulator, NormalMode};
//!
//! let mut sim: HelixSimulator<NormalMode> = HelixSimulator::new("hello".to_string());
//! sim.insert_text("x");
//! ```

use super::insert_session::InsertSession;
use super::undo::HistoryEntry;
use helix_core::ChangeSet;

/// Normal mode marker (zero-sized type)
///
/// Represents the editor in Normal mode where commands are executed.
/// This is a zero-sized type (ZST) that exists only at compile time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NormalMode;

/// Insert mode marker (zero-sized type)
///
/// Represents the editor in Insert mode where text is being inserted.
/// This is a zero-sized type (ZST) that exists only at compile time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InsertMode;

/// Runtime classification of the editor mode for display and dispatch
///
/// Unlike the typestate markers, this is a value that UI code can match
/// exhaustively, so adding a mode forces every consumer to handle it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModeKind {
    /// Normal mode: commands are executed
    Normal,
    /// Insert mode: typed characters are inserted
    Insert,
    /// Select (extend) mode: motions extend the selection
    Select,
}

/// Private module for sealing the `EditorMode` trait
///
/// This prevents external crates from implementing `EditorMode`,
/// allowing us to add methods in the future without breaking changes.
mod private {
    use crate::helix::simulator::undo::HistoryEntry;
    use helix_core::ChangeSet;

    pub trait Sealed {}

    /// What a mode remembers between commands, notified of every document change
    pub trait Session {
        /// Record that `changes` are about to be applied.
        ///
        /// `before` builds the state from before the change; it is only
        /// called when the session needs it, so typing does not clone per key.
        fn record_change(
            &mut self,
            history: &mut Vec<HistoryEntry>,
            redo_stack: &mut Vec<HistoryEntry>,
            changes: &ChangeSet,
            before: impl FnOnce() -> HistoryEntry,
        );
    }
}

pub(super) use private::Session;

/// Sealed trait for editor modes
///
/// This trait can only be implemented by types in this module,
/// ensuring all possible modes are known at compile time.
pub trait EditorMode: private::Sealed {
    /// State the mode carries between commands: `()` in Normal mode, the
    /// open [`InsertSession`] in Insert mode
    type Session: private::Session;

    /// Get the display name of this mode
    fn name() -> &'static str;
}

// Implement sealed trait for our mode markers
impl private::Sealed for NormalMode {}
impl private::Sealed for InsertMode {}

impl private::Session for () {
    /// Normal mode: every change is its own undo step and discards redo history.
    fn record_change(
        &mut self,
        history: &mut Vec<HistoryEntry>,
        redo_stack: &mut Vec<HistoryEntry>,
        _changes: &ChangeSet,
        before: impl FnOnce() -> HistoryEntry,
    ) {
        history.push(before());
        redo_stack.clear();
    }
}

impl private::Session for InsertSession {
    /// Insert mode: the session owns the history entry; nothing is pushed per change.
    fn record_change(
        &mut self,
        _history: &mut Vec<HistoryEntry>,
        _redo_stack: &mut Vec<HistoryEntry>,
        changes: &ChangeSet,
        before: impl FnOnce() -> HistoryEntry,
    ) {
        self.on_change(changes, before);
    }
}

impl EditorMode for NormalMode {
    type Session = ();

    fn name() -> &'static str {
        "NORMAL"
    }
}

impl EditorMode for InsertMode {
    type Session = InsertSession;

    fn name() -> &'static str {
        "INSERT"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::marker::PhantomData;

    #[test]
    fn test_mode_names() {
        assert_eq!(NormalMode::name(), "NORMAL");
        assert_eq!(InsertMode::name(), "INSERT");
    }

    #[test]
    fn test_any_mode_simulator_reports_mode_kind() {
        let mut sim = crate::helix::AnyModeSimulator::new("abc".to_string()).with_language("rs");
        assert_eq!(sim.mode_kind(), ModeKind::Normal);
        sim.execute_command("i").unwrap();
        assert_eq!(sim.mode_kind(), ModeKind::Insert);
    }

    #[test]
    fn test_mode_markers_are_zero_sized() {
        // PhantomData is ZST, so our modes should be too
        assert_eq!(std::mem::size_of::<NormalMode>(), 0);
        assert_eq!(std::mem::size_of::<InsertMode>(), 0);
        assert_eq!(std::mem::size_of::<PhantomData<NormalMode>>(), 0);
        assert_eq!(std::mem::size_of::<PhantomData<InsertMode>>(), 0);
    }
}
