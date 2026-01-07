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

/// Surround selection with character pair (Helix 'ms{char}' command)
///
/// Wraps the current selection with the specified character and its pair.
/// For brackets, the opening/closing pairs are: (), [], {}, <>.
/// For quotes, the same character is used on both sides.
pub fn surround_selection<M: EditorMode>(
    sim: &mut HelixSimulator<M>,
    surround_char: char,
) -> Result<(), UserError> {
    let (open, close) = get_surround_pair(surround_char);

    let range = sim.selection.primary();
    let start = range.from();
    let end = range.to();

    // Insert closing bracket first (at higher position) to preserve start position
    let close_str: String = close.into();
    let open_str: String = open.into();

    // Build the changes: insert open at start, close at end
    let transaction = Transaction::change(
        &sim.doc,
        [(start, start, Some(open_str.into()))].into_iter(),
    );
    sim.apply_transaction(transaction);

    // After inserting open char, positions shift by 1
    let new_end = end + 1;
    let transaction = Transaction::change(
        &sim.doc,
        [(new_end, new_end, Some(close_str.into()))].into_iter(),
    );
    sim.apply_transaction(transaction);

    // Update selection to include the surrounded text (excluding delimiters)
    sim.selection = Selection::single(start + 1, new_end);

    Ok(())
}

/// Delete surrounding pair (Helix 'md{char}' command)
///
/// Removes the innermost pair of the specified character around the cursor.
pub fn delete_surround<M: EditorMode>(
    sim: &mut HelixSimulator<M>,
    surround_char: char,
) -> Result<(), UserError> {
    let (open, close) = get_surround_pair(surround_char);
    let head = sim.selection.primary().head;

    // Find the surrounding pair around cursor
    let Some((open_pos, close_pos)) = find_surrounding_pair(sim, head, open, close) else {
        return Ok(()); // No surrounding pair found
    };

    // Delete close first (higher position) to preserve open position
    let transaction = Transaction::change(&sim.doc, [(close_pos, close_pos + 1, None)].into_iter());
    sim.apply_transaction(transaction);

    // Delete open
    let transaction = Transaction::change(&sim.doc, [(open_pos, open_pos + 1, None)].into_iter());
    sim.apply_transaction(transaction);

    // Adjust cursor position
    let new_head = if head > close_pos {
        head.saturating_sub(2)
    } else if head > open_pos {
        head.saturating_sub(1)
    } else {
        head
    };
    sim.selection = Selection::point(new_head.min(sim.doc.len_chars().saturating_sub(1)));

    Ok(())
}

/// Replace surrounding pair (Helix 'mr{from}{to}' command)
///
/// Replaces the innermost pair of `from_char` with `to_char`.
pub fn replace_surround<M: EditorMode>(
    sim: &mut HelixSimulator<M>,
    from_char: char,
    to_char: char,
) -> Result<(), UserError> {
    let (from_open, from_close) = get_surround_pair(from_char);
    let (to_open, to_close) = get_surround_pair(to_char);
    let head = sim.selection.primary().head;

    // Find the surrounding pair around cursor
    let Some((open_pos, close_pos)) = find_surrounding_pair(sim, head, from_open, from_close)
    else {
        return Ok(()); // No surrounding pair found
    };

    // Replace close first to preserve open position
    let close_str: String = to_close.into();
    let transaction = Transaction::change(
        &sim.doc,
        [(close_pos, close_pos + 1, Some(close_str.into()))].into_iter(),
    );
    sim.apply_transaction(transaction);

    // Replace open
    let open_str: String = to_open.into();
    let transaction = Transaction::change(
        &sim.doc,
        [(open_pos, open_pos + 1, Some(open_str.into()))].into_iter(),
    );
    sim.apply_transaction(transaction);

    Ok(())
}

/// Get the opening and closing characters for a surround pair
fn get_surround_pair(ch: char) -> (char, char) {
    match ch {
        '(' | ')' => ('(', ')'),
        '[' | ']' => ('[', ']'),
        '{' | '}' => ('{', '}'),
        '<' | '>' => ('<', '>'),
        // For quotes and other characters, use the same char on both sides
        _ => (ch, ch),
    }
}

/// Find the innermost surrounding pair around a position
fn find_surrounding_pair<M: EditorMode>(
    sim: &HelixSimulator<M>,
    pos: usize,
    open: char,
    close: char,
) -> Option<(usize, usize)> {
    let slice = sim.doc.slice(..);
    let len = sim.doc.len_chars();

    // For same-char pairs (quotes), find nearest on each side
    if open == close {
        // Search backwards for opening quote
        let mut open_pos = None;
        for i in (0..pos).rev() {
            if slice.char(i) == open {
                open_pos = Some(i);
                break;
            }
        }

        // Search forwards for closing quote
        let mut close_pos = None;
        for i in pos..len {
            if slice.char(i) == close && Some(i) != open_pos {
                close_pos = Some(i);
                break;
            }
        }

        open_pos.and_then(|o| close_pos.map(|c| (o, c)))
    } else {
        // For bracket pairs, need to track nesting
        let mut open_pos = None;
        let mut depth = 0;

        // Search backwards for matching open bracket
        for i in (0..=pos).rev() {
            let ch = slice.char(i);
            if ch == close {
                depth += 1;
            } else if ch == open {
                if depth == 0 {
                    open_pos = Some(i);
                    break;
                }
                depth -= 1;
            }
        }

        let open_idx = open_pos?;

        // Search forwards for matching close bracket
        let mut close_pos = None;
        depth = 0;

        for i in open_idx + 1..len {
            let ch = slice.char(i);
            if ch == open {
                depth += 1;
            } else if ch == close {
                if depth == 0 {
                    close_pos = Some(i);
                    break;
                }
                depth -= 1;
            }
        }

        close_pos.map(|c| (open_idx, c))
    }
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

    // Surround command tests
    #[test]
    fn test_surround_selection_parens() {
        let mut sim: HelixSimulator<NormalMode> = HelixSimulator::new("hello world".to_string());
        sim.selection = Selection::single(0, 5); // Select "hello"

        surround_selection(&mut sim, '(').unwrap();

        assert_eq!(sim.doc.to_string(), "(hello) world");
    }

    #[test]
    fn test_surround_selection_brackets() {
        let mut sim: HelixSimulator<NormalMode> = HelixSimulator::new("hello world".to_string());
        sim.selection = Selection::single(0, 5);

        surround_selection(&mut sim, '[').unwrap();

        assert_eq!(sim.doc.to_string(), "[hello] world");
    }

    #[test]
    fn test_surround_selection_quotes() {
        let mut sim: HelixSimulator<NormalMode> = HelixSimulator::new("hello world".to_string());
        sim.selection = Selection::single(0, 5);

        surround_selection(&mut sim, '"').unwrap();

        assert_eq!(sim.doc.to_string(), "\"hello\" world");
    }

    #[test]
    fn test_delete_surround_parens() {
        let mut sim: HelixSimulator<NormalMode> = HelixSimulator::new("(hello) world".to_string());
        sim.selection = Selection::point(3); // Cursor inside parens

        delete_surround(&mut sim, '(').unwrap();

        assert_eq!(sim.doc.to_string(), "hello world");
    }

    #[test]
    fn test_delete_surround_brackets() {
        let mut sim: HelixSimulator<NormalMode> = HelixSimulator::new("[hello] world".to_string());
        sim.selection = Selection::point(3);

        delete_surround(&mut sim, '[').unwrap();

        assert_eq!(sim.doc.to_string(), "hello world");
    }

    #[test]
    fn test_delete_surround_quotes() {
        let mut sim: HelixSimulator<NormalMode> =
            HelixSimulator::new("\"hello\" world".to_string());
        sim.selection = Selection::point(3);

        delete_surround(&mut sim, '"').unwrap();

        assert_eq!(sim.doc.to_string(), "hello world");
    }

    #[test]
    fn test_delete_surround_nested() {
        let mut sim: HelixSimulator<NormalMode> = HelixSimulator::new("((inner))".to_string());
        sim.selection = Selection::point(3); // Cursor on 'n'

        delete_surround(&mut sim, '(').unwrap();

        assert_eq!(sim.doc.to_string(), "(inner)");
    }

    #[test]
    fn test_replace_surround_parens_to_brackets() {
        let mut sim: HelixSimulator<NormalMode> = HelixSimulator::new("(hello) world".to_string());
        sim.selection = Selection::point(3);

        replace_surround(&mut sim, '(', '[').unwrap();

        assert_eq!(sim.doc.to_string(), "[hello] world");
    }

    #[test]
    fn test_replace_surround_quotes_to_single() {
        let mut sim: HelixSimulator<NormalMode> =
            HelixSimulator::new("\"hello\" world".to_string());
        sim.selection = Selection::point(3);

        replace_surround(&mut sim, '"', '\'').unwrap();

        assert_eq!(sim.doc.to_string(), "'hello' world");
    }

    #[test]
    fn test_surround_pair_mapping() {
        assert_eq!(get_surround_pair('('), ('(', ')'));
        assert_eq!(get_surround_pair(')'), ('(', ')'));
        assert_eq!(get_surround_pair('['), ('[', ']'));
        assert_eq!(get_surround_pair(']'), ('[', ']'));
        assert_eq!(get_surround_pair('{'), ('{', '}'));
        assert_eq!(get_surround_pair('}'), ('{', '}'));
        assert_eq!(get_surround_pair('<'), ('<', '>'));
        assert_eq!(get_surround_pair('>'), ('<', '>'));
        assert_eq!(get_surround_pair('"'), ('"', '"'));
        assert_eq!(get_surround_pair('\''), ('\'', '\''));
    }

    #[test]
    fn test_delete_surround_no_pair_found() {
        let mut sim: HelixSimulator<NormalMode> = HelixSimulator::new("hello world".to_string());
        sim.selection = Selection::point(3);

        // Should not error, just do nothing
        delete_surround(&mut sim, '(').unwrap();

        assert_eq!(sim.doc.to_string(), "hello world");
    }
}
