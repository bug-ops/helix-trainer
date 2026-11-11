//! Clipboard operations (yank, paste)

use crate::helix::simulator::{EditorMode, HelixSimulator};
use crate::security::UserError;
use helix_core::{Range, Selection, Transaction, line_ending::get_line_ending_of_str};
use std::iter::{Chain, Repeat};
use std::slice::Iter;

/// Yank (copy) the active selection to the unnamed register
///
/// Copies the full `anchor..head` range of every range in the selection,
/// each normalized regardless of its direction and stored as its own
/// register value in range order. A point range (`anchor == head`), as used
/// for a plain cursor with no active selection, falls back to yanking the
/// single character under that cursor. If every range is empty (e.g. all
/// cursors sit past the end of the document), the register is left
/// untouched.
///
/// # Errors
///
/// Never fails: always returns `Ok(())`. The `Result` keeps the signature uniform with the
/// fallible commands that share the registry's function type.
pub fn yank<M: EditorMode>(sim: &mut HelixSimulator<M>) -> Result<(), UserError> {
    yank_to_register(sim, None)
}

/// Yank (copy) the active selection to a named register
///
/// `register: None` addresses the unnamed register, matching Helix where
/// `""y` and `y` behave identically. See [`yank`] for the selection rules.
///
/// # Errors
///
/// Never fails: always returns `Ok(())`. The `Result` keeps the signature uniform
/// with the fallible commands that share the registry's function type.
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

    sim.registers.set_values(register, fragments);

    Ok(())
}

/// Where a paste lands relative to each selection range.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PastePlacement {
    After,
    Before,
}

/// Paste the unnamed register's content after the selection
///
/// See [`paste_after_from_register`].
///
/// # Errors
///
/// Never fails: always returns `Ok(())`. The `Result` keeps the signature uniform with the
/// fallible commands that share the registry's function type.
pub fn paste_after<M: EditorMode>(sim: &mut HelixSimulator<M>) -> Result<(), UserError> {
    paste_after_from_register(sim, None)
}

/// Paste a named register's content after the selection
///
/// `register: None` addresses the unnamed register. Value `i` is pasted at
/// range `i` (the last value is reused when there are more ranges than
/// values). If any value ends with a line ending the paste is linewise: it
/// goes at the start of the line after each range's last line, and a
/// separator is added when that line is an unterminated final line.
/// Otherwise the value goes right after the range (one past a point
/// cursor). Every range then selects its pasted text, so a following
/// `d`/`c`/`y` acts on exactly that text.
///
/// # Errors
///
/// Never fails: always returns `Ok(())`. The `Result` keeps the signature uniform
/// with the fallible commands that share the registry's function type.
pub fn paste_after_from_register<M: EditorMode>(
    sim: &mut HelixSimulator<M>,
    register: Option<char>,
) -> Result<(), UserError> {
    paste_from_register(sim, register, PastePlacement::After);
    Ok(())
}

/// Paste the unnamed register's content before the selection
///
/// See [`paste_before_from_register`].
///
/// # Errors
///
/// Never fails: always returns `Ok(())`. The `Result` keeps the signature uniform with the
/// fallible commands that share the registry's function type.
pub fn paste_before<M: EditorMode>(sim: &mut HelixSimulator<M>) -> Result<(), UserError> {
    paste_before_from_register(sim, None)
}

/// Paste a named register's content before the selection
///
/// `register: None` addresses the unnamed register. Same value distribution
/// and resulting selection as [`paste_after_from_register`]; the paste goes
/// at the start of each range, or at the start of its first line when
/// linewise.
///
/// # Errors
///
/// Never fails: always returns `Ok(())`. The `Result` keeps the signature uniform
/// with the fallible commands that share the registry's function type.
pub fn paste_before_from_register<M: EditorMode>(
    sim: &mut HelixSimulator<M>,
    register: Option<char>,
) -> Result<(), UserError> {
    paste_from_register(sim, register, PastePlacement::Before);
    Ok(())
}

/// Yields value `i` for range `i`, repeating the last value for extra ranges.
pub(super) fn values_by_range<'a>(
    values: &'a [String],
    last_value: &'a String,
) -> Chain<Iter<'a, String>, Repeat<&'a String>> {
    values.iter().chain(std::iter::repeat(last_value))
}

fn paste_from_register<M: EditorMode>(
    sim: &mut HelixSimulator<M>,
    register: Option<char>,
    placement: PastePlacement,
) {
    let Some(values) = sim.registers.get_values(register) else {
        return;
    };
    let Some(last_value) = values.last() else {
        return;
    };
    let linewise = values.iter().any(|v| get_line_ending_of_str(v).is_some());
    let mut value_iter = values_by_range(values, last_value);

    let doc = &sim.doc;
    let len_chars = doc.len_chars();
    let mut offset = 0;
    let mut pasted: Vec<Range> = Vec::with_capacity(sim.selection.len());

    let transaction = Transaction::change_by_selection(doc, &sim.selection, |range| {
        let (from, to) = if range.anchor == range.head {
            (range.head, (range.head + 1).min(len_chars))
        } else {
            (range.from(), range.to())
        };

        let pos = match (placement, linewise) {
            (PastePlacement::Before, true) => doc.line_to_char(doc.char_to_line(from)),
            (PastePlacement::After, true) => {
                let last_line = doc.char_to_line(to.saturating_sub(1).max(from).min(len_chars));
                doc.line_to_char((last_line + 1).min(doc.len_lines()))
            }
            (PastePlacement::Before, false) => from,
            (PastePlacement::After, false) => to,
        };

        let value = value_iter.next().map_or("", String::as_str);
        let needs_separator = linewise && pos == len_chars && pos > 0 && doc.char(pos - 1) != '\n';
        let separator_len = usize::from(needs_separator);
        let value_len = value.chars().count();

        pasted.push(select_pasted(
            offset + pos + separator_len,
            value_len,
            range,
        ));
        offset += separator_len + value_len;

        let text = if needs_separator {
            format!("\n{value}")
        } else {
            value.to_owned()
        };
        (pos, pos, Some(text.into()))
    });

    let primary = sim.selection.primary_index();
    sim.apply_transaction(&transaction);
    sim.selection = Selection::new(pasted.into(), primary);
}

/// The selection a pasted value of `len` chars leaves in a range, keeping
/// the range's direction. A single char is a plain cursor, matching how the
/// simulator represents Helix's one-wide cursor range.
fn select_pasted(start: usize, len: usize, original: &Range) -> Range {
    if len <= 1 {
        Range::point(start)
    } else if original.head < original.anchor {
        Range::new(start + len, start)
    } else {
        Range::new(start, start + len)
    }
}
