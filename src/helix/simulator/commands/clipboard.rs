//! Clipboard operations (yank, paste)

use crate::helix::simulator::{EditorMode, HelixSimulator};
use crate::security::UserError;
use helix_core::{Selection, Transaction};

/// Yank (copy) the primary selection to clipboard
///
/// Copies the full `anchor..head` range of the primary selection, normalized
/// regardless of selection direction. A point selection (`anchor == head`),
/// as used for a plain cursor with no active selection, falls back to
/// yanking the single character under the cursor.
pub fn yank<M: EditorMode>(sim: &mut HelixSimulator<M>) -> Result<(), UserError> {
    let range = sim.selection.primary();

    if range.anchor == range.head {
        let head = range.head;
        if head >= sim.doc.len_chars() {
            return Ok(());
        }
        sim.clipboard = Some(sim.doc.char(head).to_string());
    } else {
        let text = range.fragment(sim.doc.slice(..)).into_owned();
        sim.clipboard = Some(text);
    }

    Ok(())
}

/// Paste clipboard content after cursor
///
/// In Helix, cursor stays on the last pasted character
pub fn paste_after<M: EditorMode>(sim: &mut HelixSimulator<M>) -> Result<(), UserError> {
    if let Some(text) = &sim.clipboard {
        let range = sim.selection.primary();
        // A point selection (plain cursor) sits ON its char, so "after" is
        // one past it. A range selection's `to()` is already one past its
        // last char (half-open, like `Range::fragment`), so insert there.
        let insert_pos = if range.anchor == range.head {
            (range.head + 1).min(sim.doc.len_chars())
        } else {
            range.to().min(sim.doc.len_chars())
        };
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
        // Insert before the start of the selection (or at the cursor for a
        // point selection, where `from()` equals `head`).
        let insert_pos = sim.selection.primary().from();
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
