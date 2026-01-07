//! Editing commands (delete, join, indent, dedent)

use crate::helix::simulator::{EditorMode, HelixSimulator};
use crate::security::UserError;
use helix_core::surround::find_nth_pairs_pos;
use helix_core::textobject::{TextObject, textobject_paragraph, textobject_word};
use helix_core::{Selection, Transaction};

/// Join current line with next line
pub fn join_lines<M: EditorMode>(sim: &mut HelixSimulator<M>) -> Result<(), UserError> {
    let head = sim.selection.primary().head;
    let current_line = sim.doc.char_to_line(head);

    if current_line + 1 >= sim.doc.len_lines() {
        return Ok(());
    }

    let line_end = sim.doc.line_to_char(current_line + 1) - 1;
    let transaction = Transaction::change(
        &sim.doc,
        [(line_end, line_end + 1, Some(" ".into()))].into_iter(),
    );

    sim.apply_transaction(transaction);

    Ok(())
}

/// Indent current line (add 2 spaces)
pub fn indent_line<M: EditorMode>(sim: &mut HelixSimulator<M>) -> Result<(), UserError> {
    let head = sim.selection.primary().head;
    let current_line = sim.doc.char_to_line(head);
    let line_start = sim.doc.line_to_char(current_line);

    let transaction = Transaction::change(
        &sim.doc,
        [(line_start, line_start, Some("  ".into()))].into_iter(),
    );

    sim.apply_transaction(transaction);

    let new_head = head + 2;
    sim.selection = Selection::point(new_head.min(sim.doc.len_chars()));

    Ok(())
}

/// Dedent current line (remove up to 2 spaces)
pub fn dedent_line<M: EditorMode>(sim: &mut HelixSimulator<M>) -> Result<(), UserError> {
    let head = sim.selection.primary().head;
    let current_line = sim.doc.char_to_line(head);
    let line_start = sim.doc.line_to_char(current_line);

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

    let transaction = Transaction::change(
        &sim.doc,
        [(line_start, line_start + spaces_to_remove, None)].into_iter(),
    );

    sim.apply_transaction(transaction);

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
    let start_pos = sim.selection.primary().from();

    let transaction = Transaction::change_by_selection(&sim.doc, &sim.selection, |range| {
        let start = range.from();
        let end = range.to();
        let end = if start == end {
            start.saturating_add(1).min(sim.doc.len_chars())
        } else {
            end
        };
        (start, end, None)
    });

    sim.apply_transaction(transaction);

    let new_pos = start_pos.min(sim.doc.len_chars().saturating_sub(1));
    sim.selection = Selection::point(new_pos);

    Ok(())
}

/// Switch case of selected text (Helix '~' command)
///
/// Toggles case of all characters in the selection. Cursor stays at selection start.
pub fn switch_case<M: EditorMode>(sim: &mut HelixSimulator<M>) -> Result<(), UserError> {
    // Capture cursor position (start of selection) before transaction
    let cursor_pos = sim.selection.primary().from();

    let transaction = Transaction::change_by_selection(&sim.doc, &sim.selection, |range| {
        let start = range.from();
        let end = range
            .to()
            .max(start.saturating_add(1))
            .min(sim.doc.len_chars());

        if start >= sim.doc.len_chars() {
            return (start, end, None);
        }

        // Toggle case of all characters in the range
        let text: String = sim
            .doc
            .slice(start..end)
            .chars()
            .map(|ch| {
                if ch.is_uppercase() {
                    ch.to_lowercase().next().unwrap_or(ch)
                } else if ch.is_lowercase() {
                    ch.to_uppercase().next().unwrap_or(ch)
                } else {
                    ch
                }
            })
            .collect();

        (start, end, Some(text.into()))
    });

    sim.apply_transaction(transaction);

    // Keep cursor at selection start (Helix behavior)
    let safe_pos = cursor_pos.min(sim.doc.len_chars().saturating_sub(1));
    sim.selection = Selection::point(safe_pos);

    Ok(())
}

/// Select current line (Helix 'x' command)
pub fn select_line<M: EditorMode>(sim: &mut HelixSimulator<M>) -> Result<(), UserError> {
    let head = sim.selection.primary().head;
    let line = sim.doc.char_to_line(head);
    let line_start = sim.doc.line_to_char(line);
    let line_content = sim.doc.line(line);
    let line_len = line_content.len_chars();
    let line_end = line_start + line_len;

    sim.selection = Selection::single(line_end, line_start);
    Ok(())
}

/// Extend selection to line bounds (Helix 'X' command)
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

        let slice = sim.doc.slice(start..end);
        let uppercase: String = slice.chars().flat_map(|c| c.to_uppercase()).collect();

        (start, end, Some(uppercase.into()))
    });

    sim.apply_transaction(transaction);
    Ok(())
}

/// Replace selection with yanked text (Helix 'R' command)
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
pub fn surround_selection<M: EditorMode>(
    sim: &mut HelixSimulator<M>,
    surround_char: char,
) -> Result<(), UserError> {
    let (open, close) = get_surround_pair(surround_char);

    let range = sim.selection.primary();
    let start = range.from();
    let end = range.to();

    let close_str: String = close.into();
    let open_str: String = open.into();

    let transaction = Transaction::change(
        &sim.doc,
        [(start, start, Some(open_str.into()))].into_iter(),
    );
    sim.apply_transaction(transaction);

    let new_end = end + 1;
    let transaction = Transaction::change(
        &sim.doc,
        [(new_end, new_end, Some(close_str.into()))].into_iter(),
    );
    sim.apply_transaction(transaction);

    sim.selection = Selection::single(start + 1, new_end);

    Ok(())
}

/// Delete surrounding pair (Helix 'md{char}' command)
pub fn delete_surround<M: EditorMode>(
    sim: &mut HelixSimulator<M>,
    surround_char: char,
) -> Result<(), UserError> {
    let (open, close) = get_surround_pair(surround_char);
    let head = sim.selection.primary().head;

    let Some((open_pos, close_pos)) = find_surrounding_pair(sim, head, open, close) else {
        return Ok(());
    };

    let transaction = Transaction::change(&sim.doc, [(close_pos, close_pos + 1, None)].into_iter());
    sim.apply_transaction(transaction);

    let transaction = Transaction::change(&sim.doc, [(open_pos, open_pos + 1, None)].into_iter());
    sim.apply_transaction(transaction);

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
pub fn replace_surround<M: EditorMode>(
    sim: &mut HelixSimulator<M>,
    from_char: char,
    to_char: char,
) -> Result<(), UserError> {
    let (from_open, from_close) = get_surround_pair(from_char);
    let (to_open, to_close) = get_surround_pair(to_char);
    let head = sim.selection.primary().head;

    let Some((open_pos, close_pos)) = find_surrounding_pair(sim, head, from_open, from_close)
    else {
        return Ok(());
    };

    let close_str: String = to_close.into();
    let transaction = Transaction::change(
        &sim.doc,
        [(close_pos, close_pos + 1, Some(close_str.into()))].into_iter(),
    );
    sim.apply_transaction(transaction);

    let open_str: String = to_open.into();
    let transaction = Transaction::change(
        &sim.doc,
        [(open_pos, open_pos + 1, Some(open_str.into()))].into_iter(),
    );
    sim.apply_transaction(transaction);

    Ok(())
}

fn get_surround_pair(ch: char) -> (char, char) {
    match ch {
        '(' | ')' => ('(', ')'),
        '[' | ']' => ('[', ']'),
        '{' | '}' => ('{', '}'),
        '<' | '>' => ('<', '>'),
        _ => (ch, ch),
    }
}

fn find_surrounding_pair<M: EditorMode>(
    sim: &HelixSimulator<M>,
    _pos: usize,
    open: char,
    _close: char,
) -> Option<(usize, usize)> {
    let slice = sim.doc.slice(..);
    let range = sim.selection.primary();
    find_nth_pairs_pos(slice, open, range, 1).ok()
}

/// Select around text object (Helix 'ma{obj}' command)
pub fn select_around_textobject<M: EditorMode>(
    sim: &mut HelixSimulator<M>,
    obj: char,
) -> Result<(), UserError> {
    match obj {
        'w' => select_around_word(sim, false),
        'W' => select_around_word(sim, true),
        '(' | ')' => select_around_bracket(sim, '(', ')'),
        '[' | ']' => select_around_bracket(sim, '[', ']'),
        '{' | '}' => select_around_bracket(sim, '{', '}'),
        '<' | '>' => select_around_bracket(sim, '<', '>'),
        '"' => select_around_quote(sim, '"'),
        '\'' => select_around_quote(sim, '\''),
        '`' => select_around_quote(sim, '`'),
        'p' => select_around_paragraph(sim),
        _ => Ok(()),
    }
}

/// Select inside text object (Helix 'mi{obj}' command)
pub fn select_inside_textobject<M: EditorMode>(
    sim: &mut HelixSimulator<M>,
    obj: char,
) -> Result<(), UserError> {
    match obj {
        'w' => select_inside_word(sim, false),
        'W' => select_inside_word(sim, true),
        '(' | ')' => select_inside_bracket(sim, '(', ')'),
        '[' | ']' => select_inside_bracket(sim, '[', ']'),
        '{' | '}' => select_inside_bracket(sim, '{', '}'),
        '<' | '>' => select_inside_bracket(sim, '<', '>'),
        '"' => select_inside_quote(sim, '"'),
        '\'' => select_inside_quote(sim, '\''),
        '`' => select_inside_quote(sim, '`'),
        'p' => select_inside_paragraph(sim),
        _ => Ok(()),
    }
}

fn select_around_word<M: EditorMode>(
    sim: &mut HelixSimulator<M>,
    big_word: bool,
) -> Result<(), UserError> {
    if sim.doc.len_chars() == 0 {
        return Ok(());
    }

    let slice = sim.doc.slice(..);
    let range = sim.selection.primary();

    let new_range = textobject_word(slice, range, TextObject::Around, 1, big_word);

    sim.selection = Selection::single(new_range.anchor, new_range.head);
    Ok(())
}

fn select_inside_word<M: EditorMode>(
    sim: &mut HelixSimulator<M>,
    big_word: bool,
) -> Result<(), UserError> {
    if sim.doc.len_chars() == 0 {
        return Ok(());
    }

    let slice = sim.doc.slice(..);
    let range = sim.selection.primary();

    let new_range = textobject_word(slice, range, TextObject::Inside, 1, big_word);

    sim.selection = Selection::single(new_range.anchor, new_range.head);
    Ok(())
}

fn select_around_bracket<M: EditorMode>(
    sim: &mut HelixSimulator<M>,
    open: char,
    close: char,
) -> Result<(), UserError> {
    let head = sim.selection.primary().head;

    let Some((open_pos, close_pos)) = find_surrounding_pair(sim, head, open, close) else {
        return Ok(());
    };

    sim.selection = Selection::single(open_pos, close_pos + 1);
    Ok(())
}

fn select_inside_bracket<M: EditorMode>(
    sim: &mut HelixSimulator<M>,
    open: char,
    close: char,
) -> Result<(), UserError> {
    let head = sim.selection.primary().head;

    let Some((open_pos, close_pos)) = find_surrounding_pair(sim, head, open, close) else {
        return Ok(());
    };

    if open_pos + 1 < close_pos {
        sim.selection = Selection::single(open_pos + 1, close_pos);
    } else {
        sim.selection = Selection::point(open_pos + 1);
    }
    Ok(())
}

fn select_around_quote<M: EditorMode>(
    sim: &mut HelixSimulator<M>,
    quote: char,
) -> Result<(), UserError> {
    let head = sim.selection.primary().head;

    let Some((open_pos, close_pos)) = find_surrounding_pair(sim, head, quote, quote) else {
        return Ok(());
    };

    sim.selection = Selection::single(open_pos, close_pos + 1);
    Ok(())
}

fn select_inside_quote<M: EditorMode>(
    sim: &mut HelixSimulator<M>,
    quote: char,
) -> Result<(), UserError> {
    let head = sim.selection.primary().head;

    let Some((open_pos, close_pos)) = find_surrounding_pair(sim, head, quote, quote) else {
        return Ok(());
    };

    if open_pos + 1 < close_pos {
        sim.selection = Selection::single(open_pos + 1, close_pos);
    } else {
        sim.selection = Selection::point(open_pos + 1);
    }
    Ok(())
}

fn select_around_paragraph<M: EditorMode>(sim: &mut HelixSimulator<M>) -> Result<(), UserError> {
    if sim.doc.len_chars() == 0 {
        return Ok(());
    }

    let slice = sim.doc.slice(..);
    let range = sim.selection.primary();

    let new_range = textobject_paragraph(slice, range, TextObject::Around, 1);

    sim.selection = Selection::single(new_range.anchor, new_range.head);
    Ok(())
}

fn select_inside_paragraph<M: EditorMode>(sim: &mut HelixSimulator<M>) -> Result<(), UserError> {
    if sim.doc.len_chars() == 0 {
        return Ok(());
    }

    let slice = sim.doc.slice(..);
    let range = sim.selection.primary();

    let new_range = textobject_paragraph(slice, range, TextObject::Inside, 1);

    sim.selection = Selection::single(new_range.anchor, new_range.head);
    Ok(())
}

/// Join lines in selection with space (Helix 'Alt-J' command)
///
/// Like J but joins all selected lines and selects the inserted spaces.
pub fn join_selections_space<M: EditorMode>(sim: &mut HelixSimulator<M>) -> Result<(), UserError> {
    let range = sim.selection.primary();
    let start_line = sim.doc.char_to_line(range.from());
    let end_line = sim
        .doc
        .char_to_line(range.to().saturating_sub(1).max(range.from()));

    if start_line >= end_line {
        return Ok(());
    }

    let mut first_space_pos = None;

    for line in (start_line..end_line).rev() {
        if line + 1 >= sim.doc.len_lines() {
            continue;
        }

        let line_end = sim.doc.line_to_char(line + 1) - 1;
        let transaction = Transaction::change(
            &sim.doc,
            [(line_end, line_end + 1, Some(" ".into()))].into_iter(),
        );

        sim.apply_transaction(transaction);

        first_space_pos = Some(line_end);
    }

    if let Some(first_pos) = first_space_pos {
        sim.selection = Selection::single(first_pos, first_pos + 1);
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

        // Key feature of Alt-J: selection covers the inserted space
        let sel = sim.selection.primary();
        assert_eq!(sel.from(), 6, "selection should start at inserted space");
        assert_eq!(sel.to(), 7, "selection should end after inserted space");
    }

    #[test]
    fn test_join_selections_space_single_line() {
        let mut sim: HelixSimulator<NormalMode> = HelixSimulator::new("single line".to_string());
        sim.selection = Selection::single(0, 11);

        join_selections_space(&mut sim).unwrap();

        // No change for single line
        assert_eq!(sim.doc.to_string(), "single line");
    }

    #[test]
    fn test_join_selections_space_three_lines() {
        // Joining 3 lines - verifies first space is selected (not last)
        let mut sim: HelixSimulator<NormalMode> =
            HelixSimulator::new("line 1\nline 2\nline 3".to_string());
        sim.selection = Selection::single(0, 21); // All 3 lines

        join_selections_space(&mut sim).unwrap();

        assert_eq!(sim.doc.to_string(), "line 1 line 2 line 3");
        // Should select the FIRST inserted space (between line 1 and line 2)
        let sel = sim.selection.primary();
        assert_eq!(
            sel.from(),
            6,
            "selection should start at first inserted space"
        );
        assert_eq!(sel.to(), 7, "selection should cover one space");
    }

    #[test]
    fn test_join_selections_space_minimal() {
        // Minimal test case: "a\nb" -> "a b" with space at position 1
        let mut sim: HelixSimulator<NormalMode> = HelixSimulator::new("a\nb".to_string());
        sim.selection = Selection::single(0, 3);

        join_selections_space(&mut sim).unwrap();

        assert_eq!(sim.doc.to_string(), "a b");
        let sel = sim.selection.primary();
        assert_eq!(sel.from(), 1);
        assert_eq!(sel.to(), 2);
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

    // Text object tests
    #[test]
    fn test_select_around_word() {
        let mut sim: HelixSimulator<NormalMode> =
            HelixSimulator::new("hello world test".to_string());
        sim.selection = Selection::point(7); // Cursor on 'o' in "world"

        select_around_textobject(&mut sim, 'w').unwrap();

        let range = sim.selection.primary();
        // Should select "world " (including trailing space)
        assert_eq!(range.from(), 6);
        assert_eq!(range.to(), 12);
    }

    #[test]
    fn test_select_inside_word() {
        let mut sim: HelixSimulator<NormalMode> =
            HelixSimulator::new("hello world test".to_string());
        sim.selection = Selection::point(7); // Cursor on 'o' in "world"

        select_inside_textobject(&mut sim, 'w').unwrap();

        let range = sim.selection.primary();
        // Should select "world" (excluding trailing space)
        assert_eq!(range.from(), 6);
        assert_eq!(range.to(), 11);
    }

    #[test]
    fn test_select_around_big_word() {
        let mut sim: HelixSimulator<NormalMode> =
            HelixSimulator::new("hello-world test".to_string());
        sim.selection = Selection::point(7); // Cursor on 'w' in "hello-world"

        select_around_textobject(&mut sim, 'W').unwrap();

        let range = sim.selection.primary();
        // WORD includes hyphen
        assert_eq!(range.from(), 0);
        assert_eq!(range.to(), 12); // "hello-world "
    }

    #[test]
    fn test_select_inside_big_word() {
        let mut sim: HelixSimulator<NormalMode> =
            HelixSimulator::new("hello-world test".to_string());
        sim.selection = Selection::point(7);

        select_inside_textobject(&mut sim, 'W').unwrap();

        let range = sim.selection.primary();
        // WORD includes hyphen
        assert_eq!(range.from(), 0);
        assert_eq!(range.to(), 11); // "hello-world"
    }

    #[test]
    fn test_select_around_parens() {
        let mut sim: HelixSimulator<NormalMode> = HelixSimulator::new("fn(arg1, arg2)".to_string());
        sim.selection = Selection::point(5); // Cursor on 'g' in "arg1"

        select_around_textobject(&mut sim, '(').unwrap();

        let range = sim.selection.primary();
        // Should select "(arg1, arg2)" including parens
        assert_eq!(range.from(), 2);
        assert_eq!(range.to(), 14);
    }

    #[test]
    fn test_select_inside_parens() {
        let mut sim: HelixSimulator<NormalMode> = HelixSimulator::new("fn(arg1, arg2)".to_string());
        sim.selection = Selection::point(5);

        select_inside_textobject(&mut sim, '(').unwrap();

        let range = sim.selection.primary();
        // Should select "arg1, arg2" excluding parens
        assert_eq!(range.from(), 3);
        assert_eq!(range.to(), 13);
    }

    #[test]
    fn test_select_around_brackets() {
        let mut sim: HelixSimulator<NormalMode> = HelixSimulator::new("[1, 2, 3]".to_string());
        sim.selection = Selection::point(4); // Cursor on '2'

        select_around_textobject(&mut sim, '[').unwrap();

        let range = sim.selection.primary();
        assert_eq!(range.from(), 0);
        assert_eq!(range.to(), 9);
    }

    #[test]
    fn test_select_inside_brackets() {
        let mut sim: HelixSimulator<NormalMode> = HelixSimulator::new("[1, 2, 3]".to_string());
        sim.selection = Selection::point(4);

        select_inside_textobject(&mut sim, '[').unwrap();

        let range = sim.selection.primary();
        assert_eq!(range.from(), 1);
        assert_eq!(range.to(), 8);
    }

    #[test]
    fn test_select_around_braces() {
        let mut sim: HelixSimulator<NormalMode> = HelixSimulator::new("{ x: 1, y: 2 }".to_string());
        sim.selection = Selection::point(5); // Cursor on '1'

        select_around_textobject(&mut sim, '{').unwrap();

        let range = sim.selection.primary();
        assert_eq!(range.from(), 0);
        assert_eq!(range.to(), 14);
    }

    #[test]
    fn test_select_inside_braces() {
        let mut sim: HelixSimulator<NormalMode> = HelixSimulator::new("{ x: 1, y: 2 }".to_string());
        sim.selection = Selection::point(5);

        select_inside_textobject(&mut sim, '{').unwrap();

        let range = sim.selection.primary();
        assert_eq!(range.from(), 1);
        assert_eq!(range.to(), 13);
    }

    #[test]
    fn test_select_around_angle_brackets() {
        let mut sim: HelixSimulator<NormalMode> = HelixSimulator::new("Vec<T>".to_string());
        sim.selection = Selection::point(4); // Cursor on 'T'

        select_around_textobject(&mut sim, '<').unwrap();

        let range = sim.selection.primary();
        assert_eq!(range.from(), 3);
        assert_eq!(range.to(), 6);
    }

    #[test]
    fn test_select_inside_angle_brackets() {
        let mut sim: HelixSimulator<NormalMode> = HelixSimulator::new("Vec<T>".to_string());
        sim.selection = Selection::point(4);

        select_inside_textobject(&mut sim, '<').unwrap();

        let range = sim.selection.primary();
        assert_eq!(range.from(), 4);
        assert_eq!(range.to(), 5);
    }

    #[test]
    fn test_select_around_double_quotes() {
        let mut sim: HelixSimulator<NormalMode> =
            HelixSimulator::new("let s = \"hello\";".to_string());
        sim.selection = Selection::point(10); // Cursor on 'e' in "hello"

        select_around_textobject(&mut sim, '"').unwrap();

        let range = sim.selection.primary();
        // Should select "\"hello\"" including quotes
        assert_eq!(range.from(), 8);
        assert_eq!(range.to(), 15);
    }

    #[test]
    fn test_select_inside_double_quotes() {
        let mut sim: HelixSimulator<NormalMode> =
            HelixSimulator::new("let s = \"hello\";".to_string());
        sim.selection = Selection::point(10);

        select_inside_textobject(&mut sim, '"').unwrap();

        let range = sim.selection.primary();
        // Should select "hello" excluding quotes
        assert_eq!(range.from(), 9);
        assert_eq!(range.to(), 14);
    }

    #[test]
    fn test_select_around_single_quotes() {
        let mut sim: HelixSimulator<NormalMode> = HelixSimulator::new("let c = 'x';".to_string());
        sim.selection = Selection::point(9); // Cursor on 'x'

        select_around_textobject(&mut sim, '\'').unwrap();

        let range = sim.selection.primary();
        assert_eq!(range.from(), 8);
        assert_eq!(range.to(), 11);
    }

    #[test]
    fn test_select_inside_single_quotes() {
        let mut sim: HelixSimulator<NormalMode> = HelixSimulator::new("let c = 'x';".to_string());
        sim.selection = Selection::point(9);

        select_inside_textobject(&mut sim, '\'').unwrap();

        let range = sim.selection.primary();
        assert_eq!(range.from(), 9);
        assert_eq!(range.to(), 10);
    }

    #[test]
    fn test_select_around_backticks() {
        let mut sim: HelixSimulator<NormalMode> = HelixSimulator::new("run `ls -la`".to_string());
        sim.selection = Selection::point(6); // Cursor on 's'

        select_around_textobject(&mut sim, '`').unwrap();

        let range = sim.selection.primary();
        assert_eq!(range.from(), 4);
        assert_eq!(range.to(), 12);
    }

    #[test]
    fn test_select_inside_backticks() {
        let mut sim: HelixSimulator<NormalMode> = HelixSimulator::new("run `ls -la`".to_string());
        sim.selection = Selection::point(6);

        select_inside_textobject(&mut sim, '`').unwrap();

        let range = sim.selection.primary();
        assert_eq!(range.from(), 5);
        assert_eq!(range.to(), 11);
    }

    #[test]
    fn test_select_around_paragraph() {
        let mut sim: HelixSimulator<NormalMode> =
            HelixSimulator::new("line 1\nline 2\n\nline 3".to_string());
        sim.selection = Selection::point(8); // Cursor on line 2

        select_around_textobject(&mut sim, 'p').unwrap();

        let range = sim.selection.primary();
        // Should select first paragraph including the blank line separator
        // "line 1\nline 2\n\n" = positions 0-14 (Selection end is exclusive)
        assert_eq!(range.from(), 0);
        assert_eq!(range.to(), 15);
    }

    #[test]
    fn test_select_inside_paragraph() {
        let mut sim: HelixSimulator<NormalMode> =
            HelixSimulator::new("line 1\nline 2\n\nline 3".to_string());
        sim.selection = Selection::point(8); // Cursor on line 2

        select_inside_textobject(&mut sim, 'p').unwrap();

        let range = sim.selection.primary();
        // Should select first paragraph content including line newlines
        // "line 1\nline 2\n" = positions 0-13 (Selection end is exclusive)
        assert_eq!(range.from(), 0);
        assert_eq!(range.to(), 14);
    }

    #[test]
    fn test_select_inside_word_on_whitespace() {
        let mut sim: HelixSimulator<NormalMode> = HelixSimulator::new("hello   world".to_string());
        sim.selection = Selection::point(6); // Cursor on whitespace

        select_inside_textobject(&mut sim, 'w').unwrap();

        // When on whitespace, inside word does nothing
        let range = sim.selection.primary();
        assert_eq!(range.from(), 6);
        assert_eq!(range.to(), 6);
    }

    #[test]
    fn test_select_around_word_on_whitespace() {
        let mut sim: HelixSimulator<NormalMode> = HelixSimulator::new("hello   world".to_string());
        sim.selection = Selection::point(6); // Cursor on whitespace (second space)

        select_around_textobject(&mut sim, 'w').unwrap();

        let range = sim.selection.primary();
        // helix-core behavior: around on pure whitespace returns same position (no change)
        // This is consistent with Helix editor behavior where maw on whitespace does nothing
        assert_eq!(range.from(), 6);
        assert_eq!(range.to(), 6);
    }

    #[test]
    fn test_select_textobject_unknown() {
        let mut sim: HelixSimulator<NormalMode> = HelixSimulator::new("hello".to_string());
        sim.selection = Selection::point(2);

        // Unknown text object should do nothing
        select_around_textobject(&mut sim, 'z').unwrap();
        assert_eq!(sim.selection.primary().head, 2);

        select_inside_textobject(&mut sim, 'z').unwrap();
        assert_eq!(sim.selection.primary().head, 2);
    }

    #[test]
    fn test_select_inside_empty_brackets() {
        let mut sim: HelixSimulator<NormalMode> = HelixSimulator::new("foo()".to_string());
        sim.selection = Selection::point(4); // Cursor after '('

        select_inside_textobject(&mut sim, '(').unwrap();

        // Empty inside - should be a point selection
        let range = sim.selection.primary();
        assert_eq!(range.from(), 4);
        assert_eq!(range.to(), 4);
    }

    #[test]
    fn test_select_around_nested_parens() {
        let mut sim: HelixSimulator<NormalMode> = HelixSimulator::new("fn(a, (b, c))".to_string());
        sim.selection = Selection::point(8); // Cursor on 'b'

        select_around_textobject(&mut sim, '(').unwrap();

        let range = sim.selection.primary();
        // Should select innermost "(b, c)"
        assert_eq!(range.from(), 6);
        assert_eq!(range.to(), 12);
    }

    // Critical: Empty document tests
    #[test]
    fn test_select_textobject_empty_document() {
        let mut sim: HelixSimulator<NormalMode> = HelixSimulator::new(String::new());
        sim.selection = Selection::point(0);

        // Should not panic on any text object
        select_around_textobject(&mut sim, 'w').unwrap();
        select_inside_textobject(&mut sim, 'w').unwrap();
        select_around_textobject(&mut sim, '(').unwrap();
        select_inside_textobject(&mut sim, '"').unwrap();
        select_around_textobject(&mut sim, 'p').unwrap();
    }

    // Critical: Boundary position tests
    #[test]
    fn test_select_around_word_at_document_start() {
        let mut sim: HelixSimulator<NormalMode> = HelixSimulator::new("hello world".to_string());
        sim.selection = Selection::point(0);

        select_around_textobject(&mut sim, 'w').unwrap();

        let range = sim.selection.primary();
        assert_eq!(range.from(), 0);
        assert_eq!(range.to(), 6); // "hello "
    }

    #[test]
    fn test_select_around_word_at_document_end() {
        let mut sim: HelixSimulator<NormalMode> = HelixSimulator::new("hello world".to_string());
        sim.selection = Selection::point(10); // On 'd' in "world"

        select_around_textobject(&mut sim, 'w').unwrap();

        let range = sim.selection.primary();
        // helix-core behavior: "around" word at document end includes leading space
        // " world" = positions 5-11
        assert_eq!(range.from(), 5);
        assert_eq!(range.to(), 11);
    }

    // High: Closing bracket variants
    #[test]
    fn test_select_around_closing_paren() {
        let mut sim: HelixSimulator<NormalMode> = HelixSimulator::new("fn(arg)".to_string());
        sim.selection = Selection::point(4);

        // Use closing paren ')' instead of opening '('
        select_around_textobject(&mut sim, ')').unwrap();

        let range = sim.selection.primary();
        assert_eq!(range.from(), 2);
        assert_eq!(range.to(), 7);
    }

    #[test]
    fn test_select_inside_closing_bracket() {
        let mut sim: HelixSimulator<NormalMode> = HelixSimulator::new("[1, 2]".to_string());
        sim.selection = Selection::point(2);

        // Use closing bracket ']'
        select_inside_textobject(&mut sim, ']').unwrap();

        let range = sim.selection.primary();
        assert_eq!(range.from(), 1);
        assert_eq!(range.to(), 5);
    }

    #[test]
    fn test_select_around_closing_brace() {
        let mut sim: HelixSimulator<NormalMode> = HelixSimulator::new("{x}".to_string());
        sim.selection = Selection::point(1);

        select_around_textobject(&mut sim, '}').unwrap();

        let range = sim.selection.primary();
        assert_eq!(range.from(), 0);
        assert_eq!(range.to(), 3);
    }

    #[test]
    fn test_select_inside_closing_angle() {
        let mut sim: HelixSimulator<NormalMode> = HelixSimulator::new("<T>".to_string());
        sim.selection = Selection::point(1);

        select_inside_textobject(&mut sim, '>').unwrap();

        let range = sim.selection.primary();
        assert_eq!(range.from(), 1);
        assert_eq!(range.to(), 2);
    }

    // High: Unmatched brackets
    #[test]
    fn test_select_inside_unmatched_bracket() {
        let mut sim: HelixSimulator<NormalMode> = HelixSimulator::new("(hello world".to_string());
        sim.selection = Selection::point(3);

        // No matching closing bracket - should do nothing
        select_inside_textobject(&mut sim, '(').unwrap();

        // Selection should remain unchanged
        let range = sim.selection.primary();
        assert_eq!(range.head, 3);
    }

    #[test]
    fn test_select_around_unmatched_quote() {
        let mut sim: HelixSimulator<NormalMode> =
            HelixSimulator::new("let s = \"hello".to_string());
        sim.selection = Selection::point(10);

        // No matching closing quote - should do nothing
        select_around_textobject(&mut sim, '"').unwrap();

        let range = sim.selection.primary();
        assert_eq!(range.head, 10);
    }

    // High: Deeply nested brackets
    #[test]
    fn test_select_around_deeply_nested() {
        let mut sim: HelixSimulator<NormalMode> = HelixSimulator::new("(((deep)))".to_string());
        sim.selection = Selection::point(4); // On 'e'

        select_around_textobject(&mut sim, '(').unwrap();

        let range = sim.selection.primary();
        // Should select innermost "(deep)"
        assert_eq!(range.from(), 2);
        assert_eq!(range.to(), 8);
    }

    #[test]
    fn test_select_inside_triple_nested() {
        let mut sim: HelixSimulator<NormalMode> = HelixSimulator::new("[[[inner]]]".to_string());
        sim.selection = Selection::point(4); // On 'n'

        select_inside_textobject(&mut sim, '[').unwrap();

        let range = sim.selection.primary();
        // Should select "inner" from innermost brackets
        assert_eq!(range.from(), 3);
        assert_eq!(range.to(), 8);
    }

    // High: Paragraph on blank line
    #[test]
    fn test_select_paragraph_cursor_on_blank_line() {
        let mut sim: HelixSimulator<NormalMode> = HelixSimulator::new("para1\n\npara2".to_string());
        sim.selection = Selection::point(6); // On blank line (the \n)

        select_around_textobject(&mut sim, 'p').unwrap();

        // Should select something (the blank line itself or adjacent paragraph)
        let range = sim.selection.primary();
        // Exact behavior may vary; ensure no panic
        assert!(range.to() > range.from());
    }

    #[test]
    fn test_select_paragraph_single_line_document() {
        let mut sim: HelixSimulator<NormalMode> = HelixSimulator::new("single line".to_string());
        sim.selection = Selection::point(5);

        select_around_textobject(&mut sim, 'p').unwrap();

        let range = sim.selection.primary();
        // Should select entire document
        assert_eq!(range.from(), 0);
        assert_eq!(range.to(), 11);
    }

    // Medium: Empty quotes
    #[test]
    fn test_select_inside_empty_double_quotes() {
        let mut sim: HelixSimulator<NormalMode> = HelixSimulator::new("x = \"\"".to_string());
        sim.selection = Selection::point(5); // Between the quotes

        select_inside_textobject(&mut sim, '"').unwrap();

        // Empty inside - should be point selection
        let range = sim.selection.primary();
        assert_eq!(range.from(), range.to());
    }

    #[test]
    fn test_select_inside_empty_single_quotes() {
        let mut sim: HelixSimulator<NormalMode> = HelixSimulator::new("c = ''".to_string());
        sim.selection = Selection::point(5);

        select_inside_textobject(&mut sim, '\'').unwrap();

        let range = sim.selection.primary();
        assert_eq!(range.from(), range.to());
    }

    // Medium: Single character word
    #[test]
    fn test_select_around_single_char_word() {
        let mut sim: HelixSimulator<NormalMode> = HelixSimulator::new("a b c".to_string());
        sim.selection = Selection::point(2); // On 'b'

        select_around_textobject(&mut sim, 'w').unwrap();

        let range = sim.selection.primary();
        // "b " including trailing space
        assert_eq!(range.from(), 2);
        assert_eq!(range.to(), 4);
    }

    #[test]
    fn test_select_inside_single_char_word() {
        let mut sim: HelixSimulator<NormalMode> = HelixSimulator::new("a b c".to_string());
        sim.selection = Selection::point(2);

        select_inside_textobject(&mut sim, 'w').unwrap();

        let range = sim.selection.primary();
        // Just "b"
        assert_eq!(range.from(), 2);
        assert_eq!(range.to(), 3);
    }

    // Medium: WORD with special characters
    #[test]
    fn test_select_around_big_word_with_symbols() {
        let mut sim: HelixSimulator<NormalMode> =
            HelixSimulator::new("foo@bar.com test".to_string());
        sim.selection = Selection::point(5); // On 'a' in bar

        select_around_textobject(&mut sim, 'W').unwrap();

        let range = sim.selection.primary();
        // "foo@bar.com " is one WORD
        assert_eq!(range.from(), 0);
        assert_eq!(range.to(), 12);
    }

    // Medium: Multiline brackets
    #[test]
    fn test_select_around_multiline_brackets() {
        let mut sim: HelixSimulator<NormalMode> = HelixSimulator::new("{\n  x: 1\n}".to_string());
        sim.selection = Selection::point(5); // On 'x'

        select_around_textobject(&mut sim, '{').unwrap();

        let range = sim.selection.primary();
        assert_eq!(range.from(), 0);
        assert_eq!(range.to(), 10);
    }

    #[test]
    fn test_select_inside_multiline_parens() {
        let mut sim: HelixSimulator<NormalMode> = HelixSimulator::new("(\n  arg\n)".to_string());
        sim.selection = Selection::point(4); // On 'a'

        select_inside_textobject(&mut sim, '(').unwrap();

        let range = sim.selection.primary();
        assert_eq!(range.from(), 1);
        assert_eq!(range.to(), 8);
    }
}
