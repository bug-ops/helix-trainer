//! Undo and redo operations

use super::HelixSimulator;
use crate::security::UserError;
use helix_core::Selection;

impl HelixSimulator {
    /// Undo the last operation
    pub(super) fn undo(&mut self) -> Result<(), UserError> {
        if let Some((_transaction, prev_doc)) = self.history.pop() {
            // Restore the previous document state
            self.doc = prev_doc;

            // Clamp cursor to valid position
            let head = self.selection.primary().head.min(self.doc.len_chars());
            self.selection = Selection::point(head);
        }
        Ok(())
    }

    /// Redo the last undone operation
    ///
    /// Currently a placeholder - full redo would require a separate redo stack
    pub(super) fn redo(&mut self) -> Result<(), UserError> {
        // Full redo would require keeping a separate redo stack
        // For now, this is a placeholder
        Ok(())
    }
}
