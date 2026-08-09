//! Undo and redo operations

use super::{EditorMode, HelixSimulator};
use crate::security::UserError;
use helix_core::Selection;

impl<M: EditorMode> HelixSimulator<M> {
    /// Undo the last operation
    ///
    /// The document state being undone away from is moved onto the redo
    /// stack, so a subsequent `redo` can restore it.
    pub fn undo(&mut self) -> Result<(), UserError> {
        if let Some(prev_doc) = self.history.pop() {
            let undone_doc = std::mem::replace(&mut self.doc, prev_doc);
            self.redo_stack.push(undone_doc);

            // Clamp cursor to valid position
            let head = self.selection.primary().head.min(self.doc.len_chars());
            self.selection = Selection::point(head);
        }
        Ok(())
    }

    /// Redo the most recently undone operation
    ///
    /// Restores the document state that `undo` last moved onto the redo
    /// stack and pushes the state being redone away from back onto the undo
    /// history, so repeated `Ctrl-r` presses walk forward through history
    /// and `undo`/`redo` round-trip.
    pub fn redo(&mut self) -> Result<(), UserError> {
        if let Some(redone_doc) = self.redo_stack.pop() {
            let prev_doc = std::mem::replace(&mut self.doc, redone_doc);
            self.history.push(prev_doc);

            // Clamp cursor to valid position
            let head = self.selection.primary().head.min(self.doc.len_chars());
            self.selection = Selection::point(head);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::helix::simulator::HelixSimulator;
    use crate::helix::simulator::NormalMode;
    use helix_core::Transaction;

    #[test]
    fn test_apply_transaction_skips_noop_history_entry() {
        let mut sim: HelixSimulator<NormalMode> = HelixSimulator::new("hello".to_string());

        // A transaction built from the current doc with no changes applied
        // has an empty change set.
        let noop = Transaction::new(&sim.doc);
        assert!(noop.changes().is_empty());

        sim.apply_transaction(noop);

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
        sim.apply_transaction(edit);
        sim.undo().unwrap();
        assert_eq!(sim.redo_stack.len(), 1, "undo should leave one redo entry");

        let noop = Transaction::new(&sim.doc);
        sim.apply_transaction(noop);

        assert_eq!(
            sim.redo_stack.len(),
            1,
            "a no-op transaction must not discard valid redo history"
        );
    }
}
