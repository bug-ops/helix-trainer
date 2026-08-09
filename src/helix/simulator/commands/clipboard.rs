//! Clipboard operations (yank, paste)

use crate::helix::simulator::{EditorMode, HelixSimulator};
use crate::security::UserError;
use helix_core::{Selection, Transaction};

/// Yank (copy) the active selection to the unnamed register
///
/// Copies the full `anchor..head` range of every range in the selection,
/// each normalized regardless of its direction and concatenated in range
/// order. A point range (`anchor == head`), as used for a plain cursor with
/// no active selection, falls back to yanking the single character under
/// that cursor. If every range is empty (e.g. all cursors sit past the end
/// of the document), the register is left untouched.
pub fn yank<M: EditorMode>(sim: &mut HelixSimulator<M>) -> Result<(), UserError> {
    yank_to_register(sim, None)
}

/// Yank (copy) the active selection to a named register
///
/// `register: None` addresses the unnamed register, matching Helix where
/// `""y` and `y` behave identically. See [`yank`] for the selection rules.
pub fn yank_to_register<M: EditorMode>(
    sim: &mut HelixSimulator<M>,
    register: Option<char>,
) -> Result<(), UserError> {
    let slice = sim.doc.slice(..);
    let len_chars = sim.doc.len_chars();

    let fragments: Vec<String> = sim
        .selection
        .ranges()
        .iter()
        .map(|range| {
            if range.anchor == range.head {
                if range.head >= len_chars {
                    String::new()
                } else {
                    sim.doc.char(range.head).to_string()
                }
            } else {
                range.fragment(slice).into_owned()
            }
        })
        .collect();

    if fragments.iter().all(String::is_empty) {
        return Ok(());
    }

    sim.registers.set(register, fragments.concat());

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
