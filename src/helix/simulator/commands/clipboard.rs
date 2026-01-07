//! Clipboard operations (yank, paste)

use crate::helix::simulator::{EditorMode, HelixSimulator};
use crate::security::UserError;
use helix_core::{Selection, Transaction};

/// Yank (copy) current character to clipboard
pub fn yank<M: EditorMode>(sim: &mut HelixSimulator<M>) -> Result<(), UserError> {
    let head = sim.selection.primary().head;

    if head >= sim.doc.len_chars() {
        return Ok(());
    }

    let current_char = sim.doc.char(head);
    sim.clipboard = Some(current_char.to_string());
    Ok(())
}

/// Paste clipboard content after cursor
///
/// In Helix, cursor stays on the last pasted character
pub fn paste_after<M: EditorMode>(sim: &mut HelixSimulator<M>) -> Result<(), UserError> {
    if let Some(text) = &sim.clipboard {
        let head = sim.selection.primary().head;
        let insert_pos = (head + 1).min(sim.doc.len_chars());
        let text_len = text.chars().count();

        let transaction = Transaction::change(
            &sim.doc,
            [(insert_pos, insert_pos, Some(text.as_str().into()))].into_iter(),
        );

        sim.apply_transaction(transaction);

        // Cursor stays on last pasted character (Helix behavior)
        let new_pos = insert_pos + text_len.saturating_sub(1);
        sim.selection = Selection::point(new_pos.min(sim.doc.len_chars().saturating_sub(1)));
    }
    Ok(())
}

/// Paste clipboard content before cursor
///
/// In Helix, cursor stays on the last pasted character
pub fn paste_before<M: EditorMode>(sim: &mut HelixSimulator<M>) -> Result<(), UserError> {
    if let Some(text) = &sim.clipboard {
        let head = sim.selection.primary().head;
        let text_len = text.chars().count();

        let transaction = Transaction::change(
            &sim.doc,
            [(head, head, Some(text.as_str().into()))].into_iter(),
        );

        sim.apply_transaction(transaction);

        // Cursor stays on last pasted character (Helix behavior)
        let new_pos = head + text_len.saturating_sub(1);
        sim.selection = Selection::point(new_pos.min(sim.doc.len_chars().saturating_sub(1)));
    }
    Ok(())
}
