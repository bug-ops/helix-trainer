//! Editing commands (delete, join, indent, dedent)

use super::clipboard::{select_pasted, values_by_range, yank_to_register};
use crate::helix::error::SimulatorError;
use crate::helix::newline_indent::CommentContinuation;
use crate::helix::simulator::{EditorMode, HelixSimulator};
use helix_core::graphemes::next_grapheme_boundary;
use helix_core::line_ending::line_end_char_index;
use helix_core::movement::skip_while;
use helix_core::surround::find_nth_pairs_pos;
use helix_core::textobject::{TextObject, textobject_paragraph, textobject_word};
use helix_core::{Range, Selection, Tendril, Transaction};

/// Changes that join the lines of every range, as upstream Helix's
/// `join_selections_impl`: each line break plus the next line's leading blanks
/// becomes one space (none when the next line is blank).
///
/// A range within a single line joins that line with the next one. When the
/// joined line starts with the comment token of the range's first line, that
/// token is removed too; a different token becomes the current one and stays.
fn join_changes<M: EditorMode>(sim: &HelixSimulator<M>) -> Vec<(usize, usize, Option<Tendril>)> {
    let slice = sim.doc.slice(..);
    let last_line = sim.doc.len_lines() - 1;
    let blanks = |at: usize| skip_while(slice, at, |ch| matches!(ch, ' ' | '\t')).unwrap_or(at);
    let token_at = |at: usize| sim.language.comment_token_at(&slice.slice(at..).chars());
    let mut changes = Vec::new();

    for range in sim.selection.ranges() {
        let (start, mut end) = range.line_range(slice);
        if start == end {
            end = (end + 1).min(last_line);
        }
        let mut current_token = token_at(blanks(sim.doc.line_to_char(start)));

        for line in start..end {
            let from = line_end_char_index(&slice, line);
            let mut to = blanks(sim.doc.line_to_char(line + 1));
            if let Some(token) = token_at(to) {
                if current_token == Some(token) {
                    to = blanks(to + token.chars().count());
                } else {
                    current_token = Some(token);
                }
            }
            let separator = (to != line_end_char_index(&slice, line + 1)).then(|| " ".into());
            changes.push((from, to, separator));
        }
    }

    changes.sort_unstable_by_key(|&(from, _, _)| from);
    changes.dedup();
    changes
}

/// Join lines (Helix 'J' command)
///
/// Joins every selected line, or the current line with the next one for a
/// single-line range, and keeps the selection mapped over the edit. Never
/// repeated by a count prefix, as in Helix.
///
/// # Errors
///
/// Never fails: always returns `Ok(())`. The `Result` keeps the signature uniform
/// with the fallible commands that share the registry's function type.
pub fn join_lines<M: EditorMode>(sim: &mut HelixSimulator<M>) -> Result<(), SimulatorError> {
    let transaction = Transaction::change(&sim.doc, join_changes(sim).into_iter());
    sim.apply_transaction(&transaction);
    Ok(())
}

/// Sorted, deduplicated lines touched by any range of the selection
fn touched_lines<M: EditorMode>(sim: &HelixSimulator<M>) -> Vec<usize> {
    let slice = sim.doc.slice(..);
    let mut lines: Vec<usize> = sim
        .selection
        .ranges()
        .iter()
        .flat_map(|range| {
            let (start, end) = range.line_range(slice);
            start..=end
        })
        .collect();
    lines.sort_unstable();
    lines.dedup();
    lines
}

/// Apply `changes`; the selection is mapped over them, so ranges stay on their text
fn apply_changes<M: EditorMode>(
    sim: &mut HelixSimulator<M>,
    changes: Vec<(usize, usize, Option<Tendril>)>,
) {
    let transaction = Transaction::change(&sim.doc, changes.into_iter());
    sim.apply_transaction(&transaction);
}

/// Indent every line touched by the selection by one indent unit (Helix '>' command)
///
/// The unit comes from the document language; blank lines are left alone.
/// A count prefix repeats the command, so `2>` indents twice.
///
/// # Errors
///
/// Never fails: always returns `Ok(())`. The `Result` keeps the signature uniform
/// with the fallible commands that share the registry's function type.
pub fn indent_line<M: EditorMode>(sim: &mut HelixSimulator<M>) -> Result<(), SimulatorError> {
    let unit = sim.indent_unit.to_string();
    let changes = touched_lines(sim)
        .into_iter()
        .filter(|&line| !sim.doc.line(line).chars().all(char::is_whitespace))
        .map(|line| {
            let start = sim.doc.line_to_char(line);
            (start, start, Some(Tendril::from(unit.as_str())))
        })
        .collect();
    apply_changes(sim, changes);
    Ok(())
}

/// Remove up to one indent unit of leading whitespace from every line touched
/// by the selection (Helix '<' command)
///
/// Tabs count as one tab stop of the language, as in Helix.
///
/// # Errors
///
/// Never fails: always returns `Ok(())`. The `Result` keeps the signature uniform
/// with the fallible commands that share the registry's function type.
pub fn dedent_line<M: EditorMode>(sim: &mut HelixSimulator<M>) -> Result<(), SimulatorError> {
    let tab_width = sim.language.tab_width;
    let unit_width = sim.indent_unit.width(tab_width);
    let changes = touched_lines(sim)
        .into_iter()
        .filter_map(|line| {
            let mut width = 0;
            let mut removable = 0;
            for ch in sim.doc.line(line).chars() {
                match ch {
                    ' ' => width += 1,
                    '\t' => width = tab_width.next_stop(width),
                    _ => break,
                }
                removable += 1;
                if width >= unit_width {
                    break;
                }
            }
            let start = sim.doc.line_to_char(line);
            (removable > 0).then_some((start, start + removable, None))
        })
        .collect();
    apply_changes(sim, changes);
    Ok(())
}

/// Delete every range of the active selection (respecting multi-range
/// selections), leaving the selection mapped through the deletion.
///
/// Each original range collapses to a point at its post-deletion position;
/// range count and order are preserved (unlike rebuilding a fresh
/// `Selection::point`, which would discard every range but the primary).
/// Positions are derived via [`helix_core::Selection::map`] over the
/// transaction's changes rather than a manual pre-transaction offset, so a
/// multi-range deletion (e.g. from `C`) lands each cursor correctly even
/// when an earlier range's deletion shifts the document underneath a later
/// one.
pub(crate) fn delete_active_selection<M: EditorMode>(sim: &mut HelixSimulator<M>) {
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

    sim.apply_transaction(&transaction);
}

/// Delete selection (single 'd' - deletes current selection)
///
/// Each resulting cursor is the selection mapped through the deletion, as in
/// Helix `delete_selection`: a cursor deleted at the document end rests
/// one-past-the-end, and a later `d` there deletes nothing.
///
/// # Errors
///
/// Never fails: always returns `Ok(())`. The `Result` keeps the signature uniform with the
/// fallible commands that share the registry's function type.
pub fn delete_selection<M: EditorMode>(sim: &mut HelixSimulator<M>) -> Result<(), SimulatorError> {
    delete_selection_from_register(sim, None)
}

/// Delete selection, yanking to the given register (`"<reg>d`).
///
/// `register: None` addresses the unnamed register, matching plain `d`. See
/// [`delete_selection`] for the deletion/cursor rules.
///
/// # Errors
///
/// Returns the error of yanking to `register`; the yank currently never fails.
pub(crate) fn delete_selection_from_register<M: EditorMode>(
    sim: &mut HelixSimulator<M>,
    register: Option<char>,
) -> Result<(), SimulatorError> {
    yank_to_register(sim, register)?;
    delete_selection_impl(sim);
    Ok(())
}

/// Delete selection without yanking (Helix 'Alt-d'): deletes the full
/// active selection, leaving every register untouched.
///
/// Upstream Helix binds this alongside `change_selection_noyank` (`Alt-c`).
///
/// # Errors
///
/// Never fails: always returns `Ok(())`. The `Result` keeps the signature uniform
/// with the fallible commands that share the registry's function type.
pub fn delete_selection_noyank<M: EditorMode>(
    sim: &mut HelixSimulator<M>,
) -> Result<(), SimulatorError> {
    delete_selection_impl(sim);
    Ok(())
}

fn delete_selection_impl<M: EditorMode>(sim: &mut HelixSimulator<M>) {
    delete_active_selection(sim);
}

/// True when every range of the active selection spans one or more whole
/// lines, from line start through the following line's start (so each
/// range includes its trailing newline).
///
/// Mirrors upstream Helix's `selection_is_linewise` - the condition under
/// which `change_selection` opens a blank line (`Open::Above`) instead of
/// leaving a plain mid-line insertion point. `x` (`select_line`) produces
/// exactly this shape of selection.
pub(crate) fn is_selection_linewise<M: EditorMode>(sim: &HelixSimulator<M>) -> bool {
    let slice = sim.doc.slice(..);
    sim.selection.ranges().iter().all(|range| {
        if range.slice(slice).len_lines() < 2 {
            return false;
        }
        let (start_line, end_line) = range.line_range(slice);
        let start = sim.doc.line_to_char(start_line);
        let end = sim
            .doc
            .line_to_char((end_line + 1).min(sim.doc.len_lines()));
        start == range.from() && end == range.to()
    })
}

/// Replace every range of a linewise selection with a new line and leave
/// each cursor on it, indented as `O` would indent a line opened there
///
/// Mirrors upstream Helix's `change_selection` falling back to
/// `open(Open::Above)` for a whole-line selection (Helix `xc`): the lines are
/// deleted, then a line is opened above what followed them, with the indent
/// of the language rule but without continuing a comment. The cursor
/// positions are computed here rather than via [`helix_core::Selection::map`]:
/// adjacent deleted ranges would map to the same point and merge.
pub(crate) fn change_selection_linewise<M: EditorMode>(sim: &mut HelixSimulator<M>) {
    let primary = sim.selection.primary_index();
    let mut removed = 0;
    let starts: Vec<usize> = sim
        .selection
        .ranges()
        .iter()
        .map(|range| {
            let start = range.from() - removed;
            removed += range.to() - range.from();
            start
        })
        .collect();

    let deletion = Transaction::change_by_selection(&sim.doc, &sim.selection, |range| {
        (range.from(), range.to(), None)
    })
    .with_selection(Selection::point(starts[primary]));
    sim.apply_transaction(&deletion);

    let mut shift = 0;
    let cursors: Vec<Range> = starts
        .into_iter()
        .map(|start| {
            let opened = sim.open_above_at(start + shift, CommentContinuation::Skip);
            shift += opened.inserted;
            Range::point(opened.cursor)
        })
        .collect();
    sim.selection = Selection::new(cursors.into(), primary);
}

/// Apply `transaction` and keep the selection: non-empty ranges are mapped
/// over the edit, empty ranges stay where they were (mapping would push a
/// cursor on a replaced character past it).
fn apply_keeping_selection<M: EditorMode>(sim: &mut HelixSimulator<M>, transaction: Transaction) {
    let mapped = sim.selection.clone().map(transaction.changes());
    let kept = if mapped.len() == sim.selection.len() {
        let ranges: Vec<Range> = sim
            .selection
            .ranges()
            .iter()
            .zip(mapped.ranges())
            .map(|(old, new)| if old.is_empty() { *old } else { *new })
            .collect();
        Selection::new(ranges.into(), mapped.primary_index())
    } else {
        mapped
    };
    sim.apply_transaction(&transaction.with_selection(kept));
}

/// Switch case of selected text (Helix '~' command)
///
/// Toggles case of all characters in the selection and keeps the selection,
/// so the next command still applies to the same span.
///
/// # Errors
///
/// Never fails: always returns `Ok(())`. The `Result` keeps the signature uniform
/// with the fallible commands that share the registry's function type.
pub fn switch_case<M: EditorMode>(sim: &mut HelixSimulator<M>) -> Result<(), SimulatorError> {
    let transaction = Transaction::change_by_selection(&sim.doc, &sim.selection, |range| {
        let start = range.from();
        let end = range
            .to()
            .max(start.saturating_add(1))
            .min(sim.doc.len_chars());

        if start >= sim.doc.len_chars() {
            return (start, end, None);
        }

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

    apply_keeping_selection(sim, transaction);
    Ok(())
}

/// Select the line below, or extend a whole-line selection by one more line
/// (Helix 'x' command), for every range.
///
/// Produces forward ranges ending on the next line start, so vertical motion
/// starts from the selected line's newline and repeating `x` (also via a
/// count) grows the selection one line at a time.
///
/// # Errors
///
/// Never fails: always returns `Ok(())`. The `Result` keeps the signature uniform
/// with the fallible commands that share the registry's function type.
pub fn select_line<M: EditorMode>(sim: &mut HelixSimulator<M>) -> Result<(), SimulatorError> {
    let doc = &sim.doc;
    let slice = doc.slice(..);
    sim.selection = sim.selection.clone().transform(|range| {
        let (start_line, end_line) = range.line_range(slice);
        let start = doc.line_to_char(start_line);
        let end = doc.line_to_char((end_line + 1).min(doc.len_lines()));
        let last_selected_line = if range.from() == start && range.to() == end {
            end_line + 1
        } else {
            end_line
        };
        let head = doc.line_to_char((last_selected_line + 1).min(doc.len_lines()));
        Range::new(start, head)
    });
    Ok(())
}

/// Extend every range to whole-line bounds, keeping its direction
/// (Helix 'X' command, `extend_to_line_bounds`).
///
/// # Errors
///
/// Never fails: always returns `Ok(())`. The `Result` keeps the signature uniform
/// with the fallible commands that share the registry's function type.
pub fn extend_line<M: EditorMode>(sim: &mut HelixSimulator<M>) -> Result<(), SimulatorError> {
    let doc = &sim.doc;
    let slice = doc.slice(..);
    sim.selection = sim.selection.clone().transform(|range| {
        let (start_line, end_line) = range.line_range(slice);
        let start = doc.line_to_char(start_line);
        let end = doc.line_to_char((end_line + 1).min(doc.len_lines()));
        Range::new(start, end).with_direction(range.direction())
    });
    Ok(())
}

/// Select entire document (Helix '%' command)
///
/// # Errors
///
/// Never fails: always returns `Ok(())`. The `Result` keeps the signature uniform
/// with the fallible commands that share the registry's function type.
pub fn select_all<M: EditorMode>(sim: &mut HelixSimulator<M>) -> Result<(), SimulatorError> {
    sim.selection = Selection::single(0, sim.doc.len_chars());
    Ok(())
}

/// Collapse every range to its block cursor (Helix ';' command)
///
/// A forward range's cursor is its last selected character (`head - 1`), as in Helix.
///
/// # Errors
///
/// Never fails: always returns `Ok(())`. The `Result` keeps the signature uniform
/// with the fallible commands that share the registry's function type.
pub fn collapse_selection<M: EditorMode>(
    sim: &mut HelixSimulator<M>,
) -> Result<(), SimulatorError> {
    sim.selection = sim.selection.clone().cursors(sim.doc.slice(..));
    Ok(())
}

/// Switch selected text to uppercase (Helix Alt-backtick command), keeping the selection
///
/// # Errors
///
/// Never fails: always returns `Ok(())`. The `Result` keeps the signature uniform
/// with the fallible commands that share the registry's function type.
pub fn switch_to_uppercase<M: EditorMode>(
    sim: &mut HelixSimulator<M>,
) -> Result<(), SimulatorError> {
    map_selected_text(sim, str::to_uppercase);
    Ok(())
}

/// Switch selected text to lowercase (Helix backtick command), keeping the selection
///
/// # Errors
///
/// Never fails: always returns `Ok(())`. The `Result` keeps the signature uniform
/// with the fallible commands that share the registry's function type.
pub fn switch_to_lowercase<M: EditorMode>(
    sim: &mut HelixSimulator<M>,
) -> Result<(), SimulatorError> {
    map_selected_text(sim, str::to_lowercase);
    Ok(())
}

fn map_selected_text<M: EditorMode>(sim: &mut HelixSimulator<M>, map: fn(&str) -> String) {
    let transaction = Transaction::change_by_selection(&sim.doc, &sim.selection, |range| {
        let start = range.from();
        let end = range.to();

        if start >= end || start >= sim.doc.len_chars() {
            return (start, end, None);
        }

        let mapped = map(&sim.doc.slice(start..end).to_string());
        (start, end, Some(mapped.into()))
    });

    apply_keeping_selection(sim, transaction);
}

/// Replace selection with the unnamed register's content (Helix 'R' command)
///
/// A point cursor replaces the one grapheme under it; afterwards each range
/// covers the text that replaced it.
///
/// # Errors
///
/// Never fails: always returns `Ok(())`. The `Result` keeps the signature uniform with the
/// fallible commands that share the registry's function type.
pub(crate) fn replace_with_yanked<M: EditorMode>(
    sim: &mut HelixSimulator<M>,
) -> Result<(), SimulatorError> {
    replace_with_yanked_from_register(sim, None)
}

/// Replace selection with a named register's content (Helix 'R' command)
///
/// `register: None` addresses the unnamed register. See `replace_with_yanked`.
///
/// # Errors
///
/// Never fails: always returns `Ok(())`. The `Result` keeps the signature uniform
/// with the fallible commands that share the registry's function type.
pub fn replace_with_yanked_from_register<M: EditorMode>(
    sim: &mut HelixSimulator<M>,
    register: Option<char>,
) -> Result<(), SimulatorError> {
    let Some(values) = sim.registers.get_values(register).map(<[String]>::to_vec) else {
        return Ok(());
    };
    let Some(last_value) = values.last() else {
        return Ok(());
    };
    let mut value_iter = values_by_range(&values, last_value);

    let slice = sim.doc.slice(..);
    let mut offset: isize = 0;
    let mut last_to = 0;
    let mut replaced: Vec<Range> = Vec::with_capacity(sim.selection.len());
    let transaction = Transaction::change_by_selection(&sim.doc, &sim.selection, |range| {
        let (from, to) = if range.anchor == range.head {
            (range.head, next_grapheme_boundary(slice, range.head))
        } else {
            (range.from(), range.to())
        };
        // Point cursors inside one grapheme share its end; only the first replaces it.
        if from < last_to {
            replaced.push(Range::point(
                (last_to.cast_signed() + offset).cast_unsigned(),
            ));
            return (last_to, last_to, None);
        }
        last_to = to;
        let value = value_iter.next().map_or("", String::as_str);
        let value_len = value.chars().count();
        replaced.push(select_pasted(
            from.saturating_add_signed(offset),
            value_len,
            range,
        ));
        offset += value_len.cast_signed() - (to - from).cast_signed();
        (from, to, Some(value.into()))
    })
    .with_selection(Selection::new(
        replaced.into(),
        sim.selection.primary_index(),
    ));

    sim.apply_transaction(&transaction);
    Ok(())
}

/// Surround every range with a character pair (Helix 'ms{char}' command)
///
/// One transaction (one undo step); each range then spans its surround
/// characters, keeping its direction.
///
/// # Errors
///
/// Never fails: always returns `Ok(())`. The `Result` keeps the signature uniform
/// with the fallible commands that share the registry's function type.
pub fn surround_selection<M: EditorMode>(
    sim: &mut HelixSimulator<M>,
    surround_char: char,
) -> Result<(), SimulatorError> {
    let (open, close) = get_surround_pair(surround_char);

    let mut changes = Vec::with_capacity(sim.selection.len() * 2);
    let mut ranges = Vec::with_capacity(sim.selection.len());
    let mut offset = 0;
    for range in sim.selection.ranges() {
        changes.push((
            range.from(),
            range.from(),
            Some(Tendril::from(open.to_string())),
        ));
        changes.push((
            range.to(),
            range.to(),
            Some(Tendril::from(close.to_string())),
        ));
        ranges.push(
            Range::new(offset + range.from(), offset + range.to() + 2)
                .with_direction(range.direction()),
        );
        offset += 2;
    }

    let selection = Selection::new(ranges.into(), sim.selection.primary_index());
    let transaction = Transaction::change(&sim.doc, changes.into_iter()).with_selection(selection);
    sim.apply_transaction(&transaction);

    Ok(())
}

/// Delete surrounding pair (Helix 'md{char}' command)
///
/// One transaction (one undo step); the selection is mapped over the edit.
///
/// # Errors
///
/// Never fails: always returns `Ok(())`. The `Result` keeps the signature uniform
/// with the fallible commands that share the registry's function type.
pub fn delete_surround<M: EditorMode>(
    sim: &mut HelixSimulator<M>,
    surround_char: char,
) -> Result<(), SimulatorError> {
    let (open, _) = get_surround_pair(surround_char);

    let Some((open_pos, close_pos)) = find_ordered_surrounding_pair(sim, open) else {
        return Ok(());
    };

    let transaction = Transaction::change(
        &sim.doc,
        [
            (open_pos, open_pos + 1, None),
            (close_pos, close_pos + 1, None),
        ]
        .into_iter(),
    );
    sim.apply_transaction(&transaction);

    Ok(())
}

/// Replace surrounding pair (Helix 'mr{from}{to}' command)
///
/// One transaction (one undo step); the selection is unchanged because the
/// replacement characters have the same width.
///
/// # Errors
///
/// Never fails: always returns `Ok(())`. The `Result` keeps the signature uniform
/// with the fallible commands that share the registry's function type.
pub fn replace_surround<M: EditorMode>(
    sim: &mut HelixSimulator<M>,
    from_char: char,
    to_char: char,
) -> Result<(), SimulatorError> {
    let (from_open, _) = get_surround_pair(from_char);
    let (to_open, to_close) = get_surround_pair(to_char);

    let Some((open_pos, close_pos)) = find_ordered_surrounding_pair(sim, from_open) else {
        return Ok(());
    };

    let transaction = Transaction::change(
        &sim.doc,
        [
            (
                open_pos,
                open_pos + 1,
                Some(Tendril::from(to_open.to_string())),
            ),
            (
                close_pos,
                close_pos + 1,
                Some(Tendril::from(to_close.to_string())),
            ),
        ]
        .into_iter(),
    );
    sim.apply_transaction(&transaction);

    Ok(())
}

use crate::helix::get_bracket_pair as get_surround_pair;

/// Positions of the pair around the primary range, opener first.
///
/// helix-core returns them in the range's direction, so a backward range
/// yields `(close, open)`, which would make overlapping, unsorted changes.
fn find_ordered_surrounding_pair<M: EditorMode>(
    sim: &HelixSimulator<M>,
    open: char,
) -> Option<(usize, usize)> {
    let range = sim.selection.primary();
    let (first, second) = find_nth_pairs_pos(sim.doc.slice(..), open, range, 1).ok()?;
    Some((first.min(second), first.max(second)))
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
///
/// # Errors
///
/// Never fails: always returns `Ok(())`. The `Result` keeps the signature uniform
/// with the fallible commands that share the registry's function type.
pub fn select_around_textobject<M: EditorMode>(
    sim: &mut HelixSimulator<M>,
    obj: char,
) -> Result<(), SimulatorError> {
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
        _ => {}
    }
    Ok(())
}

/// Select inside text object (Helix 'mi{obj}' command)
///
/// # Errors
///
/// Never fails: always returns `Ok(())`. The `Result` keeps the signature uniform
/// with the fallible commands that share the registry's function type.
pub fn select_inside_textobject<M: EditorMode>(
    sim: &mut HelixSimulator<M>,
    obj: char,
) -> Result<(), SimulatorError> {
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
        _ => {}
    }
    Ok(())
}

fn select_around_word<M: EditorMode>(sim: &mut HelixSimulator<M>, big_word: bool) {
    if sim.doc.len_chars() == 0 {
        return;
    }

    let slice = sim.doc.slice(..);
    let range = sim.selection.primary();

    let new_range = textobject_word(slice, range, TextObject::Around, 1, big_word);

    sim.selection = Selection::single(new_range.anchor, new_range.head);
}

fn select_inside_word<M: EditorMode>(sim: &mut HelixSimulator<M>, big_word: bool) {
    if sim.doc.len_chars() == 0 {
        return;
    }

    let slice = sim.doc.slice(..);
    let range = sim.selection.primary();

    let new_range = textobject_word(slice, range, TextObject::Inside, 1, big_word);

    sim.selection = Selection::single(new_range.anchor, new_range.head);
}

fn select_around_bracket<M: EditorMode>(sim: &mut HelixSimulator<M>, open: char, close: char) {
    let head = sim.selection.primary().head;

    let Some((open_pos, close_pos)) = find_surrounding_pair(sim, head, open, close) else {
        return;
    };

    sim.selection = Selection::single(open_pos, close_pos + 1);
}

fn select_inside_bracket<M: EditorMode>(sim: &mut HelixSimulator<M>, open: char, close: char) {
    let head = sim.selection.primary().head;

    let Some((open_pos, close_pos)) = find_surrounding_pair(sim, head, open, close) else {
        return;
    };

    if open_pos + 1 < close_pos {
        sim.selection = Selection::single(open_pos + 1, close_pos);
    } else {
        sim.selection = Selection::point(open_pos + 1);
    }
}

fn select_around_quote<M: EditorMode>(sim: &mut HelixSimulator<M>, quote: char) {
    let head = sim.selection.primary().head;

    let Some((open_pos, close_pos)) = find_surrounding_pair(sim, head, quote, quote) else {
        return;
    };

    sim.selection = Selection::single(open_pos, close_pos + 1);
}

fn select_inside_quote<M: EditorMode>(sim: &mut HelixSimulator<M>, quote: char) {
    let head = sim.selection.primary().head;

    let Some((open_pos, close_pos)) = find_surrounding_pair(sim, head, quote, quote) else {
        return;
    };

    if open_pos + 1 < close_pos {
        sim.selection = Selection::single(open_pos + 1, close_pos);
    } else {
        sim.selection = Selection::point(open_pos + 1);
    }
}

fn select_around_paragraph<M: EditorMode>(sim: &mut HelixSimulator<M>) {
    if sim.doc.len_chars() == 0 {
        return;
    }

    let slice = sim.doc.slice(..);
    let range = sim.selection.primary();

    let new_range = textobject_paragraph(slice, range, TextObject::Around, 1);

    sim.selection = Selection::single(new_range.anchor, new_range.head);
}

fn select_inside_paragraph<M: EditorMode>(sim: &mut HelixSimulator<M>) {
    if sim.doc.len_chars() == 0 {
        return;
    }

    let slice = sim.doc.slice(..);
    let range = sim.selection.primary();

    let new_range = textobject_paragraph(slice, range, TextObject::Inside, 1);

    sim.selection = Selection::single(new_range.anchor, new_range.head);
}

/// Join lines like [`join_lines`] and select every inserted space
/// (Helix 'Alt-J' command).
///
/// # Errors
///
/// Never fails: always returns `Ok(())`. The `Result` keeps the signature uniform
/// with the fallible commands that share the registry's function type.
pub fn join_selections_space<M: EditorMode>(
    sim: &mut HelixSimulator<M>,
) -> Result<(), SimulatorError> {
    let changes = join_changes(sim);
    let mut offset = 0;
    let mut spaces = Vec::new();
    for (from, to, separator) in &changes {
        if separator.is_some() {
            let pos = from - offset;
            spaces.push(Range::new(pos, pos + 1));
            offset += to - from - 1;
        } else {
            offset += to - from;
        }
    }

    let mut transaction = Transaction::change(&sim.doc, changes.into_iter());
    if !spaces.is_empty() {
        transaction = transaction.with_selection(Selection::new(spaces.into(), 0));
    }
    sim.apply_transaction(&transaction);
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
    fn test_switch_to_lowercase() {
        let mut sim: HelixSimulator<NormalMode> = HelixSimulator::new("HeLLO".to_string());
        sim.selection = Selection::single(0, 5);

        switch_to_lowercase(&mut sim).unwrap();

        assert_eq!(sim.doc.to_string(), "hello");
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
        sim.registers.set(None, "REPLACED".to_string());
        sim.selection = Selection::single(0, 5);

        replace_with_yanked(&mut sim).unwrap();

        assert_eq!(sim.doc.to_string(), "REPLACED world");
    }

    #[test]
    fn test_replace_with_yanked_no_clipboard() {
        let mut sim: HelixSimulator<NormalMode> = HelixSimulator::new("hello world".to_string());
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
        sim.selection = Selection::single(0, 20); // All 3 lines

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

    // ========================================================================
    // Linewise change_selection tests (Helix `xc` -> `Open::Above`)
    // ========================================================================

    #[test]
    fn test_is_selection_linewise_true_for_whole_line_selection() {
        let mut sim: HelixSimulator<NormalMode> =
            HelixSimulator::new("one\ntwo\nthree".to_string());
        // Selects "one\n" (line_start=0 through start of next line=4),
        // matching what `x` (select_line) produces.
        sim.selection = Selection::single(4, 0);

        assert!(is_selection_linewise(&sim));
    }

    #[test]
    fn test_is_selection_linewise_false_for_mid_line_selection() {
        let mut sim: HelixSimulator<NormalMode> = HelixSimulator::new("one\ntwo".to_string());
        sim.selection = Selection::single(0, 2); // "on", not a whole line

        assert!(!is_selection_linewise(&sim));
    }

    #[test]
    fn test_is_selection_linewise_false_for_last_line_without_trailing_newline() {
        let mut sim: HelixSimulator<NormalMode> = HelixSimulator::new("one\ntwo".to_string());
        // Selects "two" (no trailing newline in the document to include)
        sim.selection = Selection::single(7, 4);

        assert!(!is_selection_linewise(&sim));
    }

    #[test]
    fn test_change_selection_linewise_replaces_line_with_blank_line() {
        let mut sim: HelixSimulator<NormalMode> =
            HelixSimulator::new("one\ntwo\nthree".to_string());
        sim.selection = Selection::single(4, 0); // whole first line, as `x` produces

        change_selection_linewise(&mut sim);
        let mapped = sim.selection.clone();

        assert_eq!(sim.doc.to_string(), "\ntwo\nthree");
        assert_eq!(mapped.ranges().len(), 1);
        assert_eq!(
            mapped.primary().head,
            0,
            "cursor must sit on the blank line"
        );
    }

    #[test]
    fn test_change_selection_linewise_multi_range_maps_each_cursor() {
        // Exercises `change_selection_linewise`'s internal transaction/offset
        // math directly, not real multi-cursor UX: a live `xc` never reaches
        // Insert mode with more than one cursor, since `enter_insert_mode`
        // collapses to the primary range immediately after this returns (see
        // its doc comment). This only proves the mapped positions for every
        // range are correct if/when that limitation is ever lifted.
        let mut sim: HelixSimulator<NormalMode> =
            HelixSimulator::new("one\ntwo\nthree".to_string());
        // Two whole-line ranges: "one\n" and "two\n"
        let ranges = vec![Range::new(0, 4), Range::new(4, 8)];
        sim.selection = Selection::new(ranges.into(), 0);

        assert!(is_selection_linewise(&sim));
        change_selection_linewise(&mut sim);
        let mapped = sim.selection.clone();

        assert_eq!(sim.doc.to_string(), "\n\nthree");
        assert_eq!(mapped.ranges().len(), 2);
        assert_eq!(mapped.ranges()[0].head, 0, "first cursor on its blank line");
        assert_eq!(
            mapped.ranges()[1].head,
            1,
            "second cursor mapped past the first range's replacement"
        );
    }
}
