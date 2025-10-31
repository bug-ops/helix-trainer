//! Clipboard operations (yank, paste)

use crate::helix::simulator::HelixSimulator;
use crate::security::UserError;
use helix_core::{Selection, Transaction};

/// Yank (copy) current character to clipboard
pub(super) fn yank(sim: &mut HelixSimulator) -> Result<(), UserError> {
    // Copy current character to clipboard
    let head = sim.selection.primary().head;

    if head >= sim.doc.len_chars() {
        return Ok(());
    }

    let current_char = sim.doc.char(head);
    sim.clipboard = Some(current_char.to_string());
    Ok(())
}

/// Paste clipboard content after cursor
pub(super) fn paste_after(sim: &mut HelixSimulator) -> Result<(), UserError> {
    // Paste clipboard content after cursor
    if let Some(text) = &sim.clipboard {
        let head = sim.selection.primary().head;
        let insert_pos = (head + 1).min(sim.doc.len_chars());
        let text_len = text.chars().count();

        let transaction = Transaction::change(
            &sim.doc,
            [(insert_pos, insert_pos, Some(text.as_str().into()))].into_iter(),
        );

        sim.apply_transaction(transaction);

        // Move cursor to the end of pasted text
        let new_pos = insert_pos + text_len;
        sim.selection = Selection::point(new_pos.min(sim.doc.len_chars()));
    }
    Ok(())
}

/// Paste clipboard content before cursor
pub(super) fn paste_before(sim: &mut HelixSimulator) -> Result<(), UserError> {
    // Paste clipboard content before cursor
    if let Some(text) = &sim.clipboard {
        let head = sim.selection.primary().head;
        let text_len = text.chars().count();

        let transaction = Transaction::change(
            &sim.doc,
            [(head, head, Some(text.as_str().into()))].into_iter(),
        );

        sim.apply_transaction(transaction);

        // Move cursor to the end of pasted text
        let new_pos = head + text_len;
        sim.selection = Selection::point(new_pos.min(sim.doc.len_chars()));
    }
    Ok(())
}
