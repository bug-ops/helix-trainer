//! Undo and redo operations

use super::{EditorMode, HelixSimulator};
use crate::security::UserError;
use helix_core::{Rope, Selection};

/// A document state paired with the selection that went with it.
///
/// Stored on the undo and redo stacks so restoring a state also restores
/// where the cursor and selections were, as Helix does.
#[derive(Debug, Clone)]
pub(in crate::helix) struct HistoryEntry {
    pub(super) doc: Rope,
    pub(super) selection: Selection,
}

impl<M: EditorMode> HelixSimulator<M> {
    /// Swap the live document and selection for `entry`, returning the state
    /// that was replaced.
    fn swap_state(&mut self, entry: HistoryEntry) -> HistoryEntry {
        HistoryEntry {
            doc: std::mem::replace(&mut self.doc, entry.doc),
            selection: std::mem::replace(&mut self.selection, entry.selection),
        }
    }

    /// Undo the last operation
    ///
    /// Restores the document and the selection from before the change. The
    /// state being undone away from is moved onto the redo stack, so a
    /// subsequent `redo` can restore it.
    ///
    /// # Errors
    ///
    /// Never fails: always returns `Ok(())`. The `Result` keeps the signature uniform
    /// with the fallible commands that share the registry's function type.
    pub fn undo(&mut self) -> Result<(), UserError> {
        if let Some(prev) = self.history.pop() {
            let undone = self.swap_state(prev);
            self.redo_stack.push(undone);
        }
        Ok(())
    }

    /// Redo the most recently undone operation
    ///
    /// Restores the document and selection that `undo` last moved onto the
    /// redo stack and pushes the state being redone away from back onto the
    /// undo history, so repeated `Ctrl-r` presses walk forward through
    /// history and `undo`/`redo` round-trip.
    ///
    /// # Errors
    ///
    /// Never fails: always returns `Ok(())`. The `Result` keeps the signature uniform
    /// with the fallible commands that share the registry's function type.
    pub fn redo(&mut self) -> Result<(), UserError> {
        if let Some(redone) = self.redo_stack.pop() {
            let prev = self.swap_state(redone);
            self.history.push(prev);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::helix::simulator::HelixSimulator;
    use crate::helix::simulator::NormalMode;
    use helix_core::{Selection, Transaction};

    #[test]
    fn test_apply_transaction_skips_noop_history_entry() {
        let mut sim: HelixSimulator<NormalMode> = HelixSimulator::new("hello".to_string());

        // A transaction built from the current doc with no changes applied
        // has an empty change set.
        let noop = Transaction::new(&sim.doc);
        assert!(noop.changes().is_empty());

        sim.apply_transaction(&noop);

        assert_eq!(
            sim.history.len(),
            0,
            "a no-op transaction must not create a spurious undo entry"
        );
    }

    #[test]
    fn test_apply_transaction_noop_preserves_pending_redo() {
        let mut sim: HelixSimulator<NormalMode> = HelixSimulator::new("hello".to_string());

        let edit = Transaction::change(&sim.doc, [(0, 0, Some("x".into()))].into_iter());
        sim.apply_transaction(&edit);
        sim.undo().unwrap();
        assert_eq!(sim.redo_stack.len(), 1, "undo should leave one redo entry");

        let noop = Transaction::new(&sim.doc);
        sim.apply_transaction(&noop);

        assert_eq!(
            sim.redo_stack.len(),
            1,
            "a no-op transaction must not discard valid redo history"
        );
    }

    #[test]
    fn test_undo_restores_selection_from_before_the_change() {
        let mut sim: HelixSimulator<NormalMode> = HelixSimulator::new("abc\ndef\n".to_string());
        sim.selection = Selection::single(0, 4);

        let edit = Transaction::change(&sim.doc, [(0, 4, None)].into_iter());
        sim.apply_transaction(&edit);
        sim.selection = Selection::point(0);

        sim.undo().unwrap();

        assert_eq!(sim.doc.to_string(), "abc\ndef\n");
        assert_eq!(sim.selection, Selection::single(0, 4));
    }

    #[test]
    fn test_redo_restores_selection_from_after_the_change() {
        let mut sim: HelixSimulator<NormalMode> = HelixSimulator::new("abc\ndef\n".to_string());
        sim.selection = Selection::single(0, 4);

        let edit = Transaction::change(&sim.doc, [(0, 4, None)].into_iter());
        sim.apply_transaction(&edit);
        sim.selection = Selection::point(2);

        sim.undo().unwrap();
        sim.redo().unwrap();

        assert_eq!(sim.doc.to_string(), "def\n");
        assert_eq!(sim.selection, Selection::point(2));
    }

    #[test]
    fn test_select_line_delete_undo_delete_removes_the_line_again() {
        use crate::helix::commands::{CMD_DELETE_SELECTION, CMD_SELECT_LINE, CMD_UNDO};
        use crate::helix::simulator::AnyModeSimulator;

        let text = "fn add() {\n    sum\n}\n";
        let mut sim = AnyModeSimulator::new(text.to_string());
        for cmd in [
            CMD_SELECT_LINE,
            CMD_DELETE_SELECTION,
            CMD_UNDO,
            CMD_DELETE_SELECTION,
        ] {
            sim.execute_command(cmd).unwrap();
        }
        assert_eq!(sim.state().unwrap().content(), "    sum\n}\n");
    }
}
