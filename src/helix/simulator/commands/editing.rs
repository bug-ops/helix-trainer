//! Editing commands (delete, join, indent, dedent)

use crate::helix::simulator::{EditorMode, HelixSimulator};
use crate::security::UserError;
use helix_core::{Selection, Transaction};

/// Join current line with next line
pub(super) fn join_lines<M: EditorMode>(sim: &mut HelixSimulator<M>) -> Result<(), UserError> {
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
pub(super) fn indent_line<M: EditorMode>(sim: &mut HelixSimulator<M>) -> Result<(), UserError> {
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
pub(super) fn dedent_line<M: EditorMode>(sim: &mut HelixSimulator<M>) -> Result<(), UserError> {
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
    // If cursor is within the removed spaces, keep it at line start
    // Otherwise, shift it left by the number of removed spaces
    let new_head = if head <= line_start + spaces_to_remove {
        line_start
    } else {
        head - spaces_to_remove
    };
    sim.selection = Selection::point(new_head);

    Ok(())
}

/// Delete selection (single 'd' - deletes current selection)
pub(super) fn delete_selection<M: EditorMode>(
    sim: &mut HelixSimulator<M>,
) -> Result<(), UserError> {
    // Get the start position before deletion
    let start_pos = sim.selection.primary().from();

    let transaction = Transaction::change_by_selection(&sim.doc, &sim.selection, |range| {
        let start = range.from();
        let end = range.to();
        // Ensure we delete at least one character
        let end = if start == end {
            start.saturating_add(1).min(sim.doc.len_chars())
        } else {
            end
        };
        (start, end, None)
    });

    sim.apply_transaction(transaction);

    // Reset selection to point at start position (clamped to doc bounds)
    let new_pos = start_pos.min(sim.doc.len_chars().saturating_sub(1));
    sim.selection = Selection::point(new_pos);

    Ok(())
}

/// Switch case of character under cursor
pub(super) fn switch_case<M: EditorMode>(sim: &mut HelixSimulator<M>) -> Result<(), UserError> {
    let transaction = Transaction::change_by_selection(&sim.doc, &sim.selection, |range| {
        let start = range.from();
        let end = start.saturating_add(1).min(sim.doc.len_chars());

        if start >= sim.doc.len_chars() {
            return (start, end, None);
        }

        let ch = sim.doc.char(start);
        let new_ch = if ch.is_uppercase() {
            ch.to_lowercase().next().unwrap_or(ch)
        } else if ch.is_lowercase() {
            ch.to_uppercase().next().unwrap_or(ch)
        } else {
            ch
        };

        (start, end, Some(new_ch.to_string().into()))
    });

    sim.apply_transaction(transaction);

    // Move cursor right after switch
    let head = sim.selection.primary().head;
    sim.selection = Selection::point(head.saturating_add(1).min(sim.doc.len_chars()));

    Ok(())
}

/// Select current line (Helix 'x' command)
/// In Helix, 'x' selects the line including the trailing newline.
/// Selection goes from line start to end of line (including \n).
pub(super) fn select_line<M: EditorMode>(sim: &mut HelixSimulator<M>) -> Result<(), UserError> {
    let head = sim.selection.primary().head;
    let line = sim.doc.char_to_line(head);
    let line_start = sim.doc.line_to_char(line);

    // Get line content to find the actual end position
    let line_content = sim.doc.line(line);
    let line_len = line_content.len_chars();

    // line_end is line_start + line length (includes \n if present)
    let line_end = line_start + line_len;

    // Selection::single(anchor, head) - head is where cursor appears
    // In Helix 'x', cursor stays at line start, anchor is at line end
    sim.selection = Selection::single(line_end, line_start);
    Ok(())
}

/// Extend selection to line bounds (Helix 'X' command)
/// In Helix, 'X' extends selection to full lines with cursor at selection start.
pub(super) fn extend_line<M: EditorMode>(sim: &mut HelixSimulator<M>) -> Result<(), UserError> {
    let range = sim.selection.primary();
    let start_line = sim.doc.char_to_line(range.from());
    let end_line = sim
        .doc
        .char_to_line(range.to().saturating_sub(1).max(range.from()));

    let line_start = sim.doc.line_to_char(start_line);
    let line_end = if end_line + 1 < sim.doc.len_lines() {
        sim.doc.line_to_char(end_line + 1)
    } else {
        sim.doc.len_chars()
    };

    // Selection::single(anchor, head) - head is where cursor appears
    // In Helix 'X', cursor stays at line start, anchor is at line end
    sim.selection = Selection::single(line_end, line_start);
    Ok(())
}

/// Select entire document (Helix '%' command)
pub(super) fn select_all<M: EditorMode>(sim: &mut HelixSimulator<M>) -> Result<(), UserError> {
    sim.selection = Selection::single(0, sim.doc.len_chars());
    Ok(())
}

/// Collapse selection to cursor (Helix ';' command)
pub(super) fn collapse_selection<M: EditorMode>(
    sim: &mut HelixSimulator<M>,
) -> Result<(), UserError> {
    let head = sim.selection.primary().head;
    sim.selection = Selection::point(head);
    Ok(())
}
