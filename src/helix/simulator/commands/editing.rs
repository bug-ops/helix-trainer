//! Editing commands (delete, join, indent, dedent)

use crate::helix::simulator::HelixSimulator;
use crate::security::UserError;
use helix_core::{Selection, Transaction};

/// Delete character at cursor
pub(super) fn delete_char(sim: &mut HelixSimulator) -> Result<(), UserError> {
    let transaction = Transaction::change_by_selection(&sim.doc, &sim.selection, |range| {
        let start = range.from();
        let end = start.saturating_add(1).min(sim.doc.len_chars()).max(start);
        (start, end, None)
    });

    sim.apply_transaction(transaction);
    Ok(())
}

/// Delete current line
pub(super) fn delete_line(sim: &mut HelixSimulator) -> Result<(), UserError> {
    let transaction = Transaction::change_by_selection(&sim.doc, &sim.selection, |range| {
        let line = sim.doc.char_to_line(range.head);
        let start = sim.doc.line_to_char(line);
        let end = if line + 1 < sim.doc.len_lines() {
            sim.doc.line_to_char(line + 1)
        } else {
            sim.doc.len_chars()
        };
        (start, end, None)
    });

    sim.apply_transaction(transaction);
    Ok(())
}

/// Join current line with next line
pub(super) fn join_lines(sim: &mut HelixSimulator) -> Result<(), UserError> {
    // Join current line with next line
    let head = sim.selection.primary().head;
    let current_line = sim.doc.char_to_line(head);

    // Can't join if on last line
    if current_line + 1 >= sim.doc.len_lines() {
        return Ok(());
    }

    // Find the newline character at the end of current line
    let line_end = sim.doc.line_to_char(current_line + 1) - 1;

    // Replace newline with space
    let transaction = Transaction::change(
        &sim.doc,
        [(line_end, line_end + 1, Some(" ".into()))].into_iter(),
    );

    sim.apply_transaction(transaction);

    Ok(())
}

/// Indent current line (add 2 spaces)
pub(super) fn indent_line(sim: &mut HelixSimulator) -> Result<(), UserError> {
    // Add indentation (2 spaces) at the beginning of current line
    let head = sim.selection.primary().head;
    let current_line = sim.doc.char_to_line(head);
    let line_start = sim.doc.line_to_char(current_line);

    // Insert 2 spaces at line start
    let transaction = Transaction::change(
        &sim.doc,
        [(line_start, line_start, Some("  ".into()))].into_iter(),
    );

    sim.apply_transaction(transaction);

    // Move cursor to maintain relative position
    let new_head = head + 2;
    sim.selection = Selection::point(new_head.min(sim.doc.len_chars()));

    Ok(())
}

/// Dedent current line (remove up to 2 spaces)
pub(super) fn dedent_line(sim: &mut HelixSimulator) -> Result<(), UserError> {
    // Remove indentation (up to 2 spaces) from the beginning of current line
    let head = sim.selection.primary().head;
    let current_line = sim.doc.char_to_line(head);
    let line_start = sim.doc.line_to_char(current_line);

    // Check how many spaces to remove (max 2)
    let slice = sim.doc.slice(..);
    let mut spaces_to_remove = 0;

    for i in 0..2 {
        let pos = line_start + i;
        if pos < sim.doc.len_chars() && slice.char(pos) == ' ' {
            spaces_to_remove += 1;
        } else {
            break;
        }
    }

    if spaces_to_remove == 0 {
        return Ok(());
    }

    // Remove the spaces
    let transaction = Transaction::change(
        &sim.doc,
        [(line_start, line_start + spaces_to_remove, None)].into_iter(),
    );

    sim.apply_transaction(transaction);

    // Move cursor to maintain relative position
    let new_head = head.saturating_sub(spaces_to_remove);
    sim.selection = Selection::point(new_head.min(sim.doc.len_chars()));

    Ok(())
}
