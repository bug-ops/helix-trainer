//! Clipboard operations (yank, paste)

use crate::helix::simulator::{EditorMode, HelixSimulator};
use crate::security::UserError;
use helix_core::{Selection, Transaction};

/// Yank (copy) the primary selection to the unnamed register
///
/// Copies the full `anchor..head` range of the primary selection, normalized
/// regardless of selection direction. A point selection (`anchor == head`),
/// as used for a plain cursor with no active selection, falls back to
/// yanking the single character under the cursor.
pub fn yank<M: EditorMode>(sim: &mut HelixSimulator<M>) -> Result<(), UserError> {
    yank_to_register(sim, None)
}

/// Yank (copy) the primary selection to a named register
///
/// `register: None` addresses the unnamed register, matching Helix where
/// `""y` and `y` behave identically. See [`yank`] for the selection rules.
pub fn yank_to_register<M: EditorMode>(
    sim: &mut HelixSimulator<M>,
    register: Option<char>,
) -> Result<(), UserError> {
    let range = sim.selection.primary();

    if range.anchor == range.head {
        let head = range.head;
        if head >= sim.doc.len_chars() {
            return Ok(());
        }
        let text = sim.doc.char(head).to_string();
        sim.registers.set(register, text);
    } else {
        let text = range.fragment(sim.doc.slice(..)).into_owned();
        sim.registers.set(register, text);
    }

    Ok(())
}

/// Paste the unnamed register's content after cursor
///
/// In Helix, cursor stays on the last pasted character
pub fn paste_after<M: EditorMode>(sim: &mut HelixSimulator<M>) -> Result<(), UserError> {
    paste_after_from_register(sim, None)
}

/// Paste a named register's content after cursor
///
/// `register: None` addresses the unnamed register. See [`paste_after`].
pub fn paste_after_from_register<M: EditorMode>(
    sim: &mut HelixSimulator<M>,
    register: Option<char>,
) -> Result<(), UserError> {
    if let Some(text) = sim.registers.get(register) {
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
            [(insert_pos, insert_pos, Some(text.into()))].into_iter(),
        );

        sim.apply_transaction(transaction);

        // Cursor stays on last pasted character (Helix behavior)
        let new_pos = insert_pos + text_len.saturating_sub(1);
        sim.selection = Selection::point(new_pos.min(sim.doc.len_chars().saturating_sub(1)));
    }
    Ok(())
}

/// Paste the unnamed register's content before cursor
///
/// In Helix, cursor stays on the last pasted character
pub fn paste_before<M: EditorMode>(sim: &mut HelixSimulator<M>) -> Result<(), UserError> {
    paste_before_from_register(sim, None)
}

/// Paste a named register's content before cursor
///
/// `register: None` addresses the unnamed register. See [`paste_before`].
pub fn paste_before_from_register<M: EditorMode>(
    sim: &mut HelixSimulator<M>,
    register: Option<char>,
) -> Result<(), UserError> {
    if let Some(text) = sim.registers.get(register) {
        // Insert before the start of the selection (or at the cursor for a
        // point selection, where `from()` equals `head`).
        let insert_pos = sim.selection.primary().from();
        let text_len = text.chars().count();

        let transaction = Transaction::change(
            &sim.doc,
            [(insert_pos, insert_pos, Some(text.into()))].into_iter(),
        );

        sim.apply_transaction(transaction);

        // Cursor stays on last pasted character (Helix behavior)
        let new_pos = insert_pos + text_len.saturating_sub(1);
        sim.selection = Selection::point(new_pos.min(sim.doc.len_chars().saturating_sub(1)));
    }
    Ok(())
}
