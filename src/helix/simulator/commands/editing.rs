//! Editing commands (delete, join, indent, dedent)

use crate::helix::simulator::{EditorMode, HelixSimulator};
use crate::security::UserError;
use helix_core::{Selection, Transaction};

/// Join current line with next line
pub fn join_lines<M: EditorMode>(sim: &mut HelixSimulator<M>) -> Result<(), UserError> {
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
pub fn indent_line<M: EditorMode>(sim: &mut HelixSimulator<M>) -> Result<(), UserError> {
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
pub fn dedent_line<M: EditorMode>(sim: &mut HelixSimulator<M>) -> Result<(), UserError> {
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
pub fn delete_selection<M: EditorMode>(sim: &mut HelixSimulator<M>) -> Result<(), UserError> {
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
pub fn switch_case<M: EditorMode>(sim: &mut HelixSimulator<M>) -> Result<(), UserError> {
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
pub fn select_line<M: EditorMode>(sim: &mut HelixSimulator<M>) -> Result<(), UserError> {
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
pub fn extend_line<M: EditorMode>(sim: &mut HelixSimulator<M>) -> Result<(), UserError> {
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
pub fn select_all<M: EditorMode>(sim: &mut HelixSimulator<M>) -> Result<(), UserError> {
    sim.selection = Selection::single(0, sim.doc.len_chars());
    Ok(())
}

/// Collapse selection to cursor (Helix ';' command)
pub fn collapse_selection<M: EditorMode>(sim: &mut HelixSimulator<M>) -> Result<(), UserError> {
    let head = sim.selection.primary().head;
    sim.selection = Selection::point(head);
    Ok(())
}

/// Switch selected text to uppercase (Helix 'Alt-`' command)
pub fn switch_to_uppercase<M: EditorMode>(sim: &mut HelixSimulator<M>) -> Result<(), UserError> {
    let transaction = Transaction::change_by_selection(&sim.doc, &sim.selection, |range| {
        let start = range.from();
        let end = range.to();

        if start >= end || start >= sim.doc.len_chars() {
            return (start, end, None);
        }

        // Get the text and convert to uppercase
        let slice = sim.doc.slice(start..end);
        let uppercase: String = slice.chars().flat_map(|c| c.to_uppercase()).collect();

        (start, end, Some(uppercase.into()))
    });

    sim.apply_transaction(transaction);
    Ok(())
}

/// Replace selection with yanked text (Helix 'R' command)
///
/// Replaces the current selection with the content of the clipboard.
/// For training, we use the simulator's clipboard.
pub fn replace_with_yanked<M: EditorMode>(sim: &mut HelixSimulator<M>) -> Result<(), UserError> {
    let Some(yanked) = &sim.clipboard else {
        return Ok(()); // Nothing yanked
    };

    if yanked.is_empty() {
        return Ok(()); // Nothing to paste
    }

    let yanked = yanked.clone();
    let transaction = Transaction::change_by_selection(&sim.doc, &sim.selection, |range| {
        let start = range.from();
        let end = range.to();
        (start, end, Some(yanked.clone().into()))
    });

    sim.apply_transaction(transaction);
    Ok(())
}

/// Join lines in selection with space (Helix 'Alt-J' command)
///
/// Like J but joins all selected lines and selects the inserted space.
pub fn join_selections_space<M: EditorMode>(sim: &mut HelixSimulator<M>) -> Result<(), UserError> {
    let range = sim.selection.primary();
    let start_line = sim.doc.char_to_line(range.from());
    let end_line = sim
        .doc
        .char_to_line(range.to().saturating_sub(1).max(range.from()));

    // If only one line, nothing to join
    if start_line >= end_line {
        return Ok(());
    }

    // Join lines from end to start to avoid position shifting issues
    for line in (start_line..end_line).rev() {
        if line + 1 >= sim.doc.len_lines() {
            continue;
        }

        // Find the newline character at the end of current line
        let line_end = sim.doc.line_to_char(line + 1) - 1;

        // Replace newline with space
        let transaction = Transaction::change(
            &sim.doc,
            [(line_end, line_end + 1, Some(" ".into()))].into_iter(),
        );

        sim.apply_transaction(transaction);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::helix::simulator::NormalMode;

    #[test]
    fn test_switch_to_uppercase() {
        let mut sim: HelixSimulator<NormalMode> = HelixSimulator::new("hello".to_string());
        sim.selection = Selection::single(0, 5);

        switch_to_uppercase(&mut sim).unwrap();

        assert_eq!(sim.doc.to_string(), "HELLO");
    }

    #[test]
    fn test_switch_to_uppercase_partial() {
        let mut sim: HelixSimulator<NormalMode> = HelixSimulator::new("hello world".to_string());
        sim.selection = Selection::single(0, 5);

        switch_to_uppercase(&mut sim).unwrap();

        assert_eq!(sim.doc.to_string(), "HELLO world");
    }

    #[test]
    fn test_switch_to_uppercase_empty_selection() {
        let mut sim: HelixSimulator<NormalMode> = HelixSimulator::new("hello".to_string());
        sim.selection = Selection::point(2);

        switch_to_uppercase(&mut sim).unwrap();

        // Empty selection, no change
        assert_eq!(sim.doc.to_string(), "hello");
    }

    #[test]
    fn test_replace_with_yanked() {
        let mut sim: HelixSimulator<NormalMode> = HelixSimulator::new("hello world".to_string());
        sim.clipboard = Some("REPLACED".to_string());
        sim.selection = Selection::single(0, 5);

        replace_with_yanked(&mut sim).unwrap();

        assert_eq!(sim.doc.to_string(), "REPLACED world");
    }

    #[test]
    fn test_replace_with_yanked_no_clipboard() {
        let mut sim: HelixSimulator<NormalMode> = HelixSimulator::new("hello world".to_string());
        sim.clipboard = None;
        sim.selection = Selection::single(0, 5);

        replace_with_yanked(&mut sim).unwrap();

        // No change when clipboard is empty
        assert_eq!(sim.doc.to_string(), "hello world");
    }

    #[test]
    fn test_join_selections_space() {
        let mut sim: HelixSimulator<NormalMode> =
            HelixSimulator::new("line 1\nline 2\nline 3".to_string());
        sim.selection = Selection::single(0, 13); // Select first two lines

        join_selections_space(&mut sim).unwrap();

        assert_eq!(sim.doc.to_string(), "line 1 line 2\nline 3");
    }

    #[test]
    fn test_join_selections_space_single_line() {
        let mut sim: HelixSimulator<NormalMode> = HelixSimulator::new("single line".to_string());
        sim.selection = Selection::single(0, 11);

        join_selections_space(&mut sim).unwrap();

        // No change for single line
        assert_eq!(sim.doc.to_string(), "single line");
    }
}
