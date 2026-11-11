//! Movement commands

use crate::helix::simulator::{EditorMode, FindDirection, FindType, HelixSimulator};
use crate::security::UserError;
use helix_core::{
    Range, RopeSlice, Selection,
    doc_formatter::TextFormat,
    graphemes::prev_grapheme_boundary,
    line_ending::line_end_char_index,
    match_brackets::find_matching_bracket_plaintext,
    movement::{self, Movement},
    search::{find_nth_next, find_nth_prev},
    text_annotations::TextAnnotations,
};
use std::num::NonZeroUsize;

/// Move left by count characters
///
/// # Errors
///
/// Never fails: always returns `Ok(())`. The `Result` keeps the signature uniform
/// with the fallible commands that share the registry's function type.
pub fn move_left<M: EditorMode>(
    sim: &mut HelixSimulator<M>,
    count: usize,
) -> Result<(), UserError> {
    use helix_core::movement::Direction;

    let slice = sim.doc.slice(..);
    let text_fmt = TextFormat::default();
    let mut annotations = TextAnnotations::default();

    let new_selection = sim.selection.clone().transform(|range| {
        movement::move_horizontally(
            slice,
            range,
            Direction::Backward,
            count,
            Movement::Move,
            &text_fmt,
            &mut annotations,
        )
    });

    sim.selection = new_selection;
    Ok(())
}

/// Move right by count characters
///
/// # Errors
///
/// Never fails: always returns `Ok(())`. The `Result` keeps the signature uniform
/// with the fallible commands that share the registry's function type.
pub fn move_right<M: EditorMode>(
    sim: &mut HelixSimulator<M>,
    count: usize,
) -> Result<(), UserError> {
    use helix_core::movement::Direction;

    let slice = sim.doc.slice(..);
    let text_fmt = TextFormat::default();
    let mut annotations = TextAnnotations::default();

    let new_selection = sim.selection.clone().transform(|range| {
        movement::move_horizontally(
            slice,
            range,
            Direction::Forward,
            count,
            Movement::Move,
            &text_fmt,
            &mut annotations,
        )
    });

    sim.selection = new_selection;
    Ok(())
}

/// Move down by count lines
///
/// # Errors
///
/// Never fails: always returns `Ok(())`. The `Result` keeps the signature uniform
/// with the fallible commands that share the registry's function type.
pub fn move_down<M: EditorMode>(
    sim: &mut HelixSimulator<M>,
    count: usize,
) -> Result<(), UserError> {
    use helix_core::movement::Direction;

    let slice = sim.doc.slice(..);
    let text_fmt = TextFormat::default();
    let mut annotations = TextAnnotations::default();

    let new_selection = sim.selection.clone().transform(|range| {
        movement::move_vertically(
            slice,
            range,
            Direction::Forward,
            count,
            Movement::Move,
            &text_fmt,
            &mut annotations,
        )
    });

    sim.selection = new_selection;
    Ok(())
}

/// Move up by count lines
///
/// # Errors
///
/// Never fails: always returns `Ok(())`. The `Result` keeps the signature uniform
/// with the fallible commands that share the registry's function type.
pub fn move_up<M: EditorMode>(sim: &mut HelixSimulator<M>, count: usize) -> Result<(), UserError> {
    use helix_core::movement::Direction;

    let slice = sim.doc.slice(..);
    let text_fmt = TextFormat::default();
    let mut annotations = TextAnnotations::default();

    let new_selection = sim.selection.clone().transform(|range| {
        movement::move_vertically(
            slice,
            range,
            Direction::Backward,
            count,
            Movement::Move,
            &text_fmt,
            &mut annotations,
        )
    });

    sim.selection = new_selection;
    Ok(())
}

/// Move to start of next word
///
/// # Errors
///
/// Never fails: always returns `Ok(())`. The `Result` keeps the signature uniform
/// with the fallible commands that share the registry's function type.
pub fn move_next_word_start(sim: &mut HelixSimulator, count: usize) -> Result<(), UserError> {
    let slice = sim.doc.slice(..);
    let new_selection = sim
        .selection
        .clone()
        .transform(|range| movement::move_next_word_start(slice, range, count));

    sim.selection = new_selection;
    Ok(())
}

/// Move to start of previous word
///
/// # Errors
///
/// Never fails: always returns `Ok(())`. The `Result` keeps the signature uniform
/// with the fallible commands that share the registry's function type.
pub fn move_prev_word_start(sim: &mut HelixSimulator, count: usize) -> Result<(), UserError> {
    let slice = sim.doc.slice(..);
    let new_selection = sim
        .selection
        .clone()
        .transform(|range| movement::move_prev_word_start(slice, range, count));

    sim.selection = new_selection;
    Ok(())
}

/// Move to end of next word
///
/// # Errors
///
/// Never fails: always returns `Ok(())`. The `Result` keeps the signature uniform
/// with the fallible commands that share the registry's function type.
pub fn move_next_word_end<M: EditorMode>(
    sim: &mut HelixSimulator<M>,
    count: usize,
) -> Result<(), UserError> {
    let slice = sim.doc.slice(..);
    let new_selection = sim
        .selection
        .clone()
        .transform(|range| movement::move_next_word_end(slice, range, count));

    sim.selection = new_selection;
    Ok(())
}

/// Move to start of next WORD (whitespace-delimited)
///
/// # Errors
///
/// Never fails: always returns `Ok(())`. The `Result` keeps the signature uniform
/// with the fallible commands that share the registry's function type.
pub fn move_next_long_word_start<M: EditorMode>(
    sim: &mut HelixSimulator<M>,
    count: usize,
) -> Result<(), UserError> {
    let slice = sim.doc.slice(..);
    let new_selection = sim
        .selection
        .clone()
        .transform(|range| movement::move_next_long_word_start(slice, range, count));

    sim.selection = new_selection;
    Ok(())
}

/// Move to start of previous WORD (whitespace-delimited)
///
/// # Errors
///
/// Never fails: always returns `Ok(())`. The `Result` keeps the signature uniform
/// with the fallible commands that share the registry's function type.
pub fn move_prev_long_word_start<M: EditorMode>(
    sim: &mut HelixSimulator<M>,
    count: usize,
) -> Result<(), UserError> {
    let slice = sim.doc.slice(..);
    let new_selection = sim
        .selection
        .clone()
        .transform(|range| movement::move_prev_long_word_start(slice, range, count));

    sim.selection = new_selection;
    Ok(())
}

/// Move to end of next WORD (whitespace-delimited)
///
/// # Errors
///
/// Never fails: always returns `Ok(())`. The `Result` keeps the signature uniform
/// with the fallible commands that share the registry's function type.
pub fn move_next_long_word_end<M: EditorMode>(
    sim: &mut HelixSimulator<M>,
    count: usize,
) -> Result<(), UserError> {
    let slice = sim.doc.slice(..);
    let new_selection = sim
        .selection
        .clone()
        .transform(|range| movement::move_next_long_word_end(slice, range, count));

    sim.selection = new_selection;
    Ok(())
}

/// Move to start of current line
///
/// # Errors
///
/// Never fails: always returns `Ok(())`. The `Result` keeps the signature uniform
/// with the fallible commands that share the registry's function type.
pub fn move_line_start<M: EditorMode>(sim: &mut HelixSimulator<M>) -> Result<(), UserError> {
    let head = sim.selection.primary().head;
    let line = sim.doc.char_to_line(head);
    let line_start = sim.doc.line_to_char(line);

    sim.selection = Selection::point(line_start);
    Ok(())
}

/// Move to the last character of the current line (Helix `gl`)
///
/// Lands on the last character, not on the line ending, and never past the
/// end of a final line without a trailing newline. An empty line stays put.
///
/// # Errors
///
/// Never fails: always returns `Ok(())`. The `Result` keeps the signature uniform
/// with the fallible commands that share the registry's function type.
pub fn move_line_end<M: EditorMode>(sim: &mut HelixSimulator<M>) -> Result<(), UserError> {
    let text = sim.doc.slice(..);
    sim.selection = sim.selection.clone().transform(|range| {
        let line = range.cursor_line(text);
        let line_start = text.line_to_char(line);
        let pos = prev_grapheme_boundary(text, line_end_char_index(&text, line)).max(line_start);
        range.put_cursor(text, pos, false)
    });
    Ok(())
}

/// Move to start of document
///
/// # Errors
///
/// Never fails: always returns `Ok(())`. The `Result` keeps the signature uniform
/// with the fallible commands that share the registry's function type.
pub fn move_document_start<M: EditorMode>(sim: &mut HelixSimulator<M>) -> Result<(), UserError> {
    sim.selection = Selection::point(0);
    Ok(())
}

/// Move to end of document
///
/// # Errors
///
/// Never fails: always returns `Ok(())`. The `Result` keeps the signature uniform
/// with the fallible commands that share the registry's function type.
pub fn move_document_end<M: EditorMode>(sim: &mut HelixSimulator<M>) -> Result<(), UserError> {
    let end = sim.doc.len_chars();
    sim.selection = Selection::point(end);
    Ok(())
}

/// Port of helix-term 25.07.1 `find_next_char_impl` / `find_prev_char_impl`
/// (`find_char_impl`'s `search_fn`): the head position for the new range.
fn find_target(
    text: RopeSlice,
    ch: char,
    search_start: usize,
    count: usize,
    find_type: FindType,
    direction: FindDirection,
) -> Option<usize> {
    match (direction, find_type) {
        (FindDirection::Forward, FindType::Find) => {
            let start = (search_start + 1).min(text.len_chars());
            find_nth_next(text, ch, start, count)
        }
        (FindDirection::Forward, FindType::Till) => {
            let start = (search_start + 1).min(text.len_chars());
            let n = if text.get_char(start) == Some(ch) {
                count + 1
            } else {
                count
            };
            find_nth_next(text, ch, start, n).map(|found| found.saturating_sub(1))
        }
        (FindDirection::Backward, FindType::Find) => find_nth_prev(text, ch, search_start, count),
        (FindDirection::Backward, FindType::Till) => {
            let n = if text.get_char(search_start.saturating_sub(1)) == Some(ch) {
                count + 1
            } else {
                count
            };
            find_nth_prev(text, ch, search_start, n).map(|found| (found + 1).min(text.len_chars()))
        }
    }
}

/// Shared body of `f`/`F`/`t`/`T` and `Alt-.`.
///
/// Each range extends from its cursor to the found position, as in
/// helix-term 25.07.1 (`f a d` deletes through the `a`). The search spans the
/// whole buffer, not just the cursor line; a range with no match is unchanged.
fn find_char<M: EditorMode>(
    sim: &mut HelixSimulator<M>,
    ch: char,
    find_type: FindType,
    direction: FindDirection,
    count: usize,
) {
    sim.find_state.set(ch, find_type, direction);

    let text = sim.doc.slice(..);
    sim.selection = sim.selection.clone().transform(|range| {
        let search_start = if range.anchor < range.head {
            range.head - 1
        } else {
            range.head
        };
        find_target(text, ch, search_start, count, find_type, direction).map_or(range, |head| {
            Range::point(range.cursor(text)).put_cursor(text, head, true)
        })
    });
}

/// Find next occurrence of character (Helix 'f' command)
///
/// # Errors
///
/// Never fails: always returns `Ok(())`. The `Result` keeps the signature uniform
/// with the fallible commands that share the registry's function type.
pub fn find_next_char<M: EditorMode>(
    sim: &mut HelixSimulator<M>,
    ch: char,
    count: usize,
) -> Result<(), UserError> {
    find_char(sim, ch, FindType::Find, FindDirection::Forward, count);
    Ok(())
}

/// Find previous occurrence of character (Helix 'F' command)
///
/// # Errors
///
/// Never fails: always returns `Ok(())`. The `Result` keeps the signature uniform
/// with the fallible commands that share the registry's function type.
pub fn find_prev_char<M: EditorMode>(
    sim: &mut HelixSimulator<M>,
    ch: char,
    count: usize,
) -> Result<(), UserError> {
    find_char(sim, ch, FindType::Find, FindDirection::Backward, count);
    Ok(())
}

/// Move till (before) next occurrence of character (Helix 't' command)
///
/// # Errors
///
/// Never fails: always returns `Ok(())`. The `Result` keeps the signature uniform
/// with the fallible commands that share the registry's function type.
pub fn till_next_char<M: EditorMode>(
    sim: &mut HelixSimulator<M>,
    ch: char,
    count: usize,
) -> Result<(), UserError> {
    find_char(sim, ch, FindType::Till, FindDirection::Forward, count);
    Ok(())
}

/// Move till (after) previous occurrence of character (Helix 'T' command)
///
/// # Errors
///
/// Never fails: always returns `Ok(())`. The `Result` keeps the signature uniform
/// with the fallible commands that share the registry's function type.
pub fn till_prev_char<M: EditorMode>(
    sim: &mut HelixSimulator<M>,
    ch: char,
    count: usize,
) -> Result<(), UserError> {
    find_char(sim, ch, FindType::Till, FindDirection::Backward, count);
    Ok(())
}

/// Go to line `count` (1-based), clamped to the last real line (Helix `{n}G`, `{n}gg`)
///
/// A trailing empty line produced by a final newline is not a target, matching
/// helix-term 25.07.1 `goto_line_without_jumplist`. Bare `G`/`gg` without a
/// count never reach this function.
pub fn goto_line<M: EditorMode>(sim: &mut HelixSimulator<M>, count: NonZeroUsize) {
    let text = sim.doc.slice(..);
    let len_lines = text.len_lines();
    let max_line = if text.line(len_lines - 1).len_chars() == 0 {
        len_lines.saturating_sub(2)
    } else {
        len_lines - 1
    };
    let pos = text.line_to_char((count.get() - 1).min(max_line));
    sim.selection = sim
        .selection
        .clone()
        .transform(|range| range.put_cursor(text, pos, false));
}

/// Move to first non-whitespace character on line (Helix 'gs' command)
///
/// # Errors
///
/// Never fails: always returns `Ok(())`. The `Result` keeps the signature uniform
/// with the fallible commands that share the registry's function type.
pub fn goto_first_nonwhitespace<M: EditorMode>(
    sim: &mut HelixSimulator<M>,
) -> Result<(), UserError> {
    let head = sim.selection.primary().head;
    let line = sim.doc.char_to_line(head);
    let line_start = sim.doc.line_to_char(line);
    let line_end = if line + 1 < sim.doc.len_lines() {
        sim.doc.line_to_char(line + 1)
    } else {
        sim.doc.len_chars()
    };

    let slice = sim.doc.slice(..);
    let mut pos = line_start;

    while pos < line_end {
        if let Some(c) = slice.get_char(pos)
            && !c.is_whitespace()
        {
            sim.selection = Selection::point(pos);
            return Ok(());
        }
        pos += 1;
    }

    // All whitespace - go to line start
    sim.selection = Selection::point(line_start);
    Ok(())
}

/// Go to last line of document (Helix 'ge' command)
///
/// # Errors
///
/// Never fails: always returns `Ok(())`. The `Result` keeps the signature uniform
/// with the fallible commands that share the registry's function type.
pub fn goto_last_line<M: EditorMode>(sim: &mut HelixSimulator<M>) -> Result<(), UserError> {
    let total_lines = sim.doc.len_lines();
    let doc_len = sim.doc.len_chars();

    if doc_len == 0 {
        sim.selection = Selection::point(0);
        return Ok(());
    }

    let mut last_line = total_lines.saturating_sub(1);

    let last_line_start = sim.doc.line_to_char(last_line);
    let last_line_len = if last_line + 1 < total_lines {
        sim.doc.line_to_char(last_line + 1) - last_line_start
    } else {
        doc_len - last_line_start
    };

    if last_line_len == 0 && last_line > 0 {
        last_line = last_line.saturating_sub(1);
    }

    let line_start = sim.doc.line_to_char(last_line);
    sim.selection = Selection::point(line_start);
    Ok(())
}

/// Match brackets - find matching bracket pair (Helix 'mm' command)
///
/// # Errors
///
/// Never fails: always returns `Ok(())`. The `Result` keeps the signature uniform
/// with the fallible commands that share the registry's function type.
pub fn match_brackets<M: EditorMode>(sim: &mut HelixSimulator<M>) -> Result<(), UserError> {
    let head = sim.selection.primary().head;
    let slice = sim.doc.slice(..);

    if let Some(matching_pos) = find_matching_bracket_plaintext(slice, head) {
        sim.selection = Selection::point(matching_pos);
    }

    Ok(())
}

/// Flip selection direction (swap anchor and head) - Helix 'Alt-;' command
///
/// # Errors
///
/// Never fails: always returns `Ok(())`. The `Result` keeps the signature uniform
/// with the fallible commands that share the registry's function type.
pub fn flip_selections<M: EditorMode>(sim: &mut HelixSimulator<M>) -> Result<(), UserError> {
    let range = sim.selection.primary();
    sim.selection = Selection::single(range.head, range.anchor);
    Ok(())
}

/// Move every range with `motion`, extending in Select mode as `hx` does
fn move_paragraph<M: EditorMode>(
    sim: &mut HelixSimulator<M>,
    count: usize,
    motion: fn(RopeSlice, Range, usize, Movement) -> Range,
) {
    let slice = sim.doc.slice(..);
    sim.selection = if sim.select_mode {
        sim.selection
            .clone()
            .transform(|range| motion(slice, range, count, Movement::Extend))
    } else {
        let range = motion(slice, sim.selection.primary(), count, Movement::Move);
        Selection::single(range.anchor, range.head)
    };
}

/// Move to previous paragraph (Helix '[p' command)
///
/// # Errors
///
/// Never fails: always returns `Ok(())`. The `Result` keeps the signature uniform
/// with the fallible commands that share the registry's function type.
pub fn goto_prev_paragraph<M: EditorMode>(
    sim: &mut HelixSimulator<M>,
    count: usize,
) -> Result<(), UserError> {
    move_paragraph(sim, count, movement::move_prev_paragraph);
    Ok(())
}

/// Move to next paragraph (Helix ']p' command)
///
/// # Errors
///
/// Never fails: always returns `Ok(())`. The `Result` keeps the signature uniform
/// with the fallible commands that share the registry's function type.
pub fn goto_next_paragraph<M: EditorMode>(
    sim: &mut HelixSimulator<M>,
    count: usize,
) -> Result<(), UserError> {
    move_paragraph(sim, count, movement::move_next_paragraph);
    Ok(())
}

/// Repeat last f/F/t/T motion in the same direction (Helix 'Alt-.' command)
///
/// # Errors
///
/// Never fails: always returns `Ok(())`. The `Result` keeps the signature uniform
/// with the fallible commands that share the registry's function type.
pub fn repeat_last_motion<M: EditorMode>(sim: &mut HelixSimulator<M>) -> Result<(), UserError> {
    if let Some((ch, find_type, direction)) = sim.find_state.get() {
        find_char(sim, ch, find_type, direction, 1);
    }
    Ok(())
}

const PAGE_SIZE: usize = 20;

/// Page up movement (Ctrl-b command)
///
/// # Errors
///
/// Never fails: always returns `Ok(())`. The `Result` keeps the signature uniform
/// with the fallible commands that share the registry's function type.
pub fn page_up<M: EditorMode>(sim: &mut HelixSimulator<M>, count: usize) -> Result<(), UserError> {
    let head = sim.selection.primary().head;
    let current_line = sim.doc.char_to_line(head);
    let line_start = sim.doc.line_to_char(current_line);
    let col = head - line_start;

    let lines_to_move = PAGE_SIZE.saturating_mul(count.max(1));
    let target_line = current_line.saturating_sub(lines_to_move);

    let target_line_start = sim.doc.line_to_char(target_line);
    let target_line_len = if target_line + 1 < sim.doc.len_lines() {
        sim.doc.line_to_char(target_line + 1) - target_line_start - 1
    } else {
        sim.doc.len_chars() - target_line_start
    };

    let new_col = col.min(target_line_len);
    sim.selection = Selection::point(target_line_start + new_col);

    Ok(())
}

/// Page down movement (Ctrl-f command)
///
/// # Errors
///
/// Never fails: always returns `Ok(())`. The `Result` keeps the signature uniform
/// with the fallible commands that share the registry's function type.
pub fn page_down<M: EditorMode>(
    sim: &mut HelixSimulator<M>,
    count: usize,
) -> Result<(), UserError> {
    let head = sim.selection.primary().head;
    let current_line = sim.doc.char_to_line(head);
    let line_start = sim.doc.line_to_char(current_line);
    let col = head - line_start;

    let total_lines = sim.doc.len_lines();
    let lines_to_move = PAGE_SIZE.saturating_mul(count.max(1));
    let target_line = (current_line + lines_to_move).min(total_lines.saturating_sub(1));

    let target_line_start = sim.doc.line_to_char(target_line);
    let target_line_len = if target_line + 1 < sim.doc.len_lines() {
        sim.doc.line_to_char(target_line + 1) - target_line_start - 1
    } else {
        sim.doc.len_chars() - target_line_start
    };

    let new_col = col.min(target_line_len);
    sim.selection = Selection::point(target_line_start + new_col);

    Ok(())
}

/// Half page up movement (Ctrl-u command)
///
/// # Errors
///
/// Never fails: always returns `Ok(())`. The `Result` keeps the signature uniform
/// with the fallible commands that share the registry's function type.
pub fn half_page_up<M: EditorMode>(
    sim: &mut HelixSimulator<M>,
    count: usize,
) -> Result<(), UserError> {
    let head = sim.selection.primary().head;
    let current_line = sim.doc.char_to_line(head);
    let line_start = sim.doc.line_to_char(current_line);
    let col = head - line_start;

    let lines_to_move = (PAGE_SIZE / 2).saturating_mul(count.max(1));
    let target_line = current_line.saturating_sub(lines_to_move);

    let target_line_start = sim.doc.line_to_char(target_line);
    let target_line_len = if target_line + 1 < sim.doc.len_lines() {
        sim.doc.line_to_char(target_line + 1) - target_line_start - 1
    } else {
        sim.doc.len_chars() - target_line_start
    };

    let new_col = col.min(target_line_len);
    sim.selection = Selection::point(target_line_start + new_col);

    Ok(())
}

/// Half page down movement (Ctrl-d command)
///
/// # Errors
///
/// Never fails: always returns `Ok(())`. The `Result` keeps the signature uniform
/// with the fallible commands that share the registry's function type.
pub fn half_page_down<M: EditorMode>(
    sim: &mut HelixSimulator<M>,
    count: usize,
) -> Result<(), UserError> {
    let head = sim.selection.primary().head;
    let current_line = sim.doc.char_to_line(head);
    let line_start = sim.doc.line_to_char(current_line);
    let col = head - line_start;

    let total_lines = sim.doc.len_lines();
    let lines_to_move = (PAGE_SIZE / 2).saturating_mul(count.max(1));
    let target_line = (current_line + lines_to_move).min(total_lines.saturating_sub(1));

    let target_line_start = sim.doc.line_to_char(target_line);
    let target_line_len = if target_line + 1 < sim.doc.len_lines() {
        sim.doc.line_to_char(target_line + 1) - target_line_start - 1
    } else {
        sim.doc.len_chars() - target_line_start
    };

    let new_col = col.min(target_line_len);
    sim.selection = Selection::point(target_line_start + new_col);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::helix::simulator::HelixSimulator;

    fn cursor(sim: &HelixSimulator) -> usize {
        sim.selection.primary().cursor(sim.doc.slice(..))
    }

    fn bounds(sim: &HelixSimulator) -> (usize, usize) {
        let r = sim.selection.primary();
        (r.anchor, r.head)
    }

    #[test]
    fn test_find_records_state() {
        let mut sim = HelixSimulator::new("hello world".to_string());
        find_next_char(&mut sim, 'o', 1).unwrap();

        // Check that find state was recorded
        let (ch, ft, dir) = sim.find_state.get().unwrap();
        assert_eq!(ch, 'o');
        assert_eq!(ft, FindType::Find);
        assert_eq!(dir, FindDirection::Forward);
    }

    #[test]
    fn test_find_backward_records_state() {
        let mut sim = HelixSimulator::new("hello world".to_string());
        // Move to end
        sim.selection = Selection::point(10);
        find_prev_char(&mut sim, 'l', 1).unwrap();

        let (ch, ft, dir) = sim.find_state.get().unwrap();
        assert_eq!(ch, 'l');
        assert_eq!(ft, FindType::Find);
        assert_eq!(dir, FindDirection::Backward);
    }

    #[test]
    fn test_till_records_state() {
        let mut sim = HelixSimulator::new("hello world".to_string());
        till_next_char(&mut sim, 'o', 1).unwrap();

        let (ch, ft, dir) = sim.find_state.get().unwrap();
        assert_eq!(ch, 'o');
        assert_eq!(ft, FindType::Till);
        assert_eq!(dir, FindDirection::Forward);
    }

    #[test]
    fn test_repeat_last_motion_find_forward() {
        let mut sim = HelixSimulator::new("abcabc".to_string());
        // First find 'b'
        find_next_char(&mut sim, 'b', 1).unwrap();
        assert_eq!(cursor(&sim), 1);

        // Repeat should find next 'b'
        repeat_last_motion(&mut sim).unwrap();
        assert_eq!(cursor(&sim), 4);
    }

    #[test]
    fn test_repeat_without_prior_motion() {
        let mut sim = HelixSimulator::new("hello".to_string());
        // No prior motion, should be no-op
        repeat_last_motion(&mut sim).unwrap();
        assert_eq!(sim.selection.primary().head, 0);
    }

    #[test]
    fn test_paragraph_movement_next() {
        let mut sim = HelixSimulator::new("line1\n\nline2\n\nline3".to_string());
        // Start at beginning
        assert_eq!(sim.selection.primary().head, 0);

        // Move to next paragraph (blank line)
        goto_next_paragraph(&mut sim, 1).unwrap();
        // Should have moved past the first line
        assert!(sim.selection.primary().head > 0);
    }

    #[test]
    fn test_paragraph_movement_prev() {
        let mut sim = HelixSimulator::new("line1\n\nline2\n\nline3".to_string());
        // Start at end
        sim.selection = Selection::point(17); // at "line3"

        // Move to previous paragraph
        goto_prev_paragraph(&mut sim, 1).unwrap();
        // Should be at an earlier position
        assert!(sim.selection.primary().head < 17);
    }

    #[test]
    fn test_till_backward_records_state() {
        let mut sim = HelixSimulator::new("hello world".to_string());
        sim.selection = Selection::point(10);
        till_prev_char(&mut sim, 'l', 1).unwrap();

        let (ch, ft, dir) = sim.find_state.get().unwrap();
        assert_eq!(ch, 'l');
        assert_eq!(ft, FindType::Till);
        assert_eq!(dir, FindDirection::Backward);
    }

    #[test]
    fn test_repeat_last_motion_till_forward() {
        let mut sim = HelixSimulator::new("axbxcxd".to_string());
        // First till 'x' from position 0
        till_next_char(&mut sim, 'x', 1).unwrap();
        // Should be at position 0 (position before 'x' at 1, but cursor stays if adjacent)

        // Move to position 2 manually for repeat test
        sim.selection = Selection::point(2);
        repeat_last_motion(&mut sim).unwrap();
        // Should find next 'x' at position 3, stop at 2 (till stops before)
    }

    #[test]
    fn test_paragraph_at_document_start() {
        let mut sim = HelixSimulator::new("line1\n\nline2".to_string());
        // At start, previous paragraph should stay at 0
        goto_prev_paragraph(&mut sim, 1).unwrap();
        assert_eq!(sim.selection.primary().head, 0);
    }

    #[test]
    fn test_paragraph_at_document_end() {
        let mut sim = HelixSimulator::new("line1\n\nline2".to_string());
        let doc_len = sim.doc.len_chars();
        sim.selection = Selection::point(doc_len.saturating_sub(1));

        // At end, next paragraph should handle gracefully
        goto_next_paragraph(&mut sim, 1).unwrap();
        // Should be at or past current position (end of doc)
        assert!(sim.selection.primary().head <= doc_len);
    }

    #[test]
    fn test_page_down() {
        // Create content with many lines
        let content = (0..50)
            .map(|i| format!("line {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let mut sim = HelixSimulator::new(content);

        // Start at beginning
        assert_eq!(sim.selection.primary().head, 0);

        // Page down should move cursor down by PAGE_SIZE lines
        page_down(&mut sim, 1).unwrap();

        // Should be on a later line
        let new_line = sim.doc.char_to_line(sim.selection.primary().head);
        assert!(new_line > 0);
    }

    #[test]
    fn test_page_up() {
        // Create content with many lines
        let content = (0..50)
            .map(|i| format!("line {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let mut sim = HelixSimulator::new(content);

        // Start near the end
        let last_line_start = sim.doc.line_to_char(40);
        sim.selection = Selection::point(last_line_start);

        // Page up should move cursor up
        page_up(&mut sim, 1).unwrap();

        // Should be on an earlier line
        let new_line = sim.doc.char_to_line(sim.selection.primary().head);
        assert!(new_line < 40);
    }

    #[test]
    fn test_half_page_down() {
        let content = (0..50)
            .map(|i| format!("line {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let mut sim = HelixSimulator::new(content);

        half_page_down(&mut sim, 1).unwrap();

        let new_line = sim.doc.char_to_line(sim.selection.primary().head);
        assert!(new_line > 0);
    }

    #[test]
    fn test_half_page_up() {
        let content = (0..50)
            .map(|i| format!("line {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let mut sim = HelixSimulator::new(content);

        let line_30_start = sim.doc.line_to_char(30);
        sim.selection = Selection::point(line_30_start);

        half_page_up(&mut sim, 1).unwrap();

        let new_line = sim.doc.char_to_line(sim.selection.primary().head);
        assert!(new_line < 30);
    }

    #[test]
    fn test_page_up_at_top() {
        let mut sim = HelixSimulator::new("line 1\nline 2".to_string());
        sim.selection = Selection::point(0);

        // Page up at top should stay at top
        page_up(&mut sim, 1).unwrap();

        assert_eq!(sim.selection.primary().head, 0);
    }

    #[test]
    fn test_page_down_at_bottom() {
        let mut sim = HelixSimulator::new("line 1\nline 2".to_string());
        let last_line_start = sim.doc.line_to_char(1);
        sim.selection = Selection::point(last_line_start);

        // Page down at bottom should stay near bottom
        page_down(&mut sim, 1).unwrap();

        // Should be on last line
        let new_line = sim.doc.char_to_line(sim.selection.primary().head);
        assert_eq!(new_line, 1);
    }

    /// Test to verify `helix_core::search` behavior across lines
    /// This documents why we need line boundary checks in f/F/t/T commands
    #[test]
    fn test_helix_core_search_crosses_lines() {
        use helix_core::search::{find_nth_next, find_nth_prev};

        let text = helix_core::Rope::from("hello world\nfoo bar\n");
        let slice = text.slice(..);

        // helix_core::search::find_nth_next searches across the entire document,
        // not just the current line. The 'f' at the start of "foo" is at position 12.
        let result = find_nth_next(slice, 'f', 0, 1);
        assert_eq!(
            result,
            Some(12),
            "helix-core find_nth_next crosses line boundaries"
        );

        // Same for find_nth_prev - it searches backward across all lines
        // The 'e' in "hello" is at position 1
        let result = find_nth_prev(slice, 'e', 15, 1);
        assert_eq!(
            result,
            Some(1),
            "helix-core find_nth_prev crosses line boundaries"
        );
    }

    #[test]
    fn test_find_next_char_crosses_lines() {
        let mut sim = HelixSimulator::new("abc\nxyz".to_string());
        find_next_char(&mut sim, 'x', 1).unwrap();
        assert_eq!(bounds(&sim), (0, 5));
    }

    #[test]
    fn test_find_prev_char_crosses_lines() {
        let mut sim = HelixSimulator::new("abc\nxyz".to_string());
        sim.selection = Selection::point(6);
        find_prev_char(&mut sim, 'a', 1).unwrap();
        assert_eq!(bounds(&sim), (7, 0));
    }

    #[test]
    fn test_find_next_char_with_count() {
        // "aXaXaXa" - positions: a=0, X=1, a=2, X=3, a=4, X=5, a=6
        let mut sim = HelixSimulator::new("aXaXaXa".to_string());
        // Find 2nd 'a' starting from position 1 (search starts at head + 1)
        find_next_char(&mut sim, 'a', 2).unwrap();
        // 1st 'a' after position 0 is at position 2
        // 2nd 'a' after position 0 is at position 4
        assert_eq!(cursor(&sim), 4);
    }

    #[test]
    fn test_find_prev_char_with_count() {
        // "cXcXcXc" - positions: c=0, X=1, c=2, X=3, c=4, X=5, c=6
        let mut sim = HelixSimulator::new("cXcXcXc".to_string());
        sim.selection = Selection::point(6); // at last 'c'
        // Find 2nd 'c' backward starting from position 5 (search starts at head - 1)
        find_prev_char(&mut sim, 'c', 2).unwrap();
        // 1st 'c' before position 6 is at position 4
        // 2nd 'c' before position 6 is at position 2
        assert_eq!(cursor(&sim), 2);
    }

    #[test]
    fn test_till_next_char_stops_before() {
        let mut sim = HelixSimulator::new("hello".to_string());
        // Till 'o' should stop at 'l' (position 3), one before 'o' (position 4)
        till_next_char(&mut sim, 'o', 1).unwrap();
        assert_eq!(cursor(&sim), 3);
    }

    #[test]
    fn test_till_prev_char_stops_after() {
        let mut sim = HelixSimulator::new("hello".to_string());
        sim.selection = Selection::point(4); // at 'o'
        // Till 'e' backward should stop at 'l' (position 2), one after 'e' (position 1)
        till_prev_char(&mut sim, 'e', 1).unwrap();
        assert_eq!(cursor(&sim), 2);
    }

    #[test]
    fn test_repeat_last_motion_find_backward() {
        // "abcabc" - test repeat for F (find backward)
        let mut sim = HelixSimulator::new("abcabc".to_string());
        sim.selection = Selection::point(5); // at last 'c'
        find_prev_char(&mut sim, 'b', 1).unwrap();
        assert_eq!(cursor(&sim), 4); // found 'b' at position 4

        repeat_last_motion(&mut sim).unwrap();
        assert_eq!(cursor(&sim), 1); // found earlier 'b' at position 1
    }

    #[test]
    fn test_repeat_last_motion_till_backward() {
        // "axbxc" - test repeat for T (till backward)
        let mut sim = HelixSimulator::new("axbxc".to_string());
        sim.selection = Selection::point(4); // at 'c'
        till_prev_char(&mut sim, 'x', 1).unwrap();
        // 'x' at 3 is adjacent, so hx skips it and tills the next 'x' (at 1),
        // landing right after it.
        assert_eq!(cursor(&sim), 2);
    }

    #[test]
    fn test_find_char_not_found() {
        let mut sim = HelixSimulator::new("hello".to_string());
        find_next_char(&mut sim, 'z', 1).unwrap();
        assert_eq!(
            sim.selection.primary().head,
            0,
            "cursor should not move when char not found"
        );
    }

    #[test]
    fn test_till_next_char_crosses_lines() {
        let mut sim = HelixSimulator::new("abc\nxyz".to_string());
        till_next_char(&mut sim, 'x', 1).unwrap();
        assert_eq!(bounds(&sim), (0, 4));
    }

    #[test]
    fn test_till_prev_char_crosses_lines() {
        let mut sim = HelixSimulator::new("abc\nxyz".to_string());
        sim.selection = Selection::point(6);
        till_prev_char(&mut sim, 'a', 1).unwrap();
        assert_eq!(bounds(&sim), (7, 1));
    }

    #[test]
    fn test_find_prev_char_not_found() {
        let mut sim = HelixSimulator::new("hello".to_string());
        sim.selection = Selection::point(4); // at 'o'
        find_prev_char(&mut sim, 'z', 1).unwrap();
        assert_eq!(
            sim.selection.primary().head,
            4,
            "cursor should not move when char not found"
        );
    }

    #[test]
    fn test_find_forward_extends_from_cursor() {
        let mut sim = HelixSimulator::new("fn add(a: i32)".to_string());
        find_next_char(&mut sim, 'a', 1).unwrap();
        assert_eq!(bounds(&sim), (0, 4));
    }

    #[test]
    fn test_till_forward_extends_to_before_target() {
        let mut sim = HelixSimulator::new("fn add(a: i32)".to_string());
        till_next_char(&mut sim, '(', 1).unwrap();
        assert_eq!(bounds(&sim), (0, 6));
    }

    #[test]
    fn test_find_backward_selects_backwards() {
        let mut sim = HelixSimulator::new("abcabc".to_string());
        sim.selection = Selection::point(5);
        find_prev_char(&mut sim, 'b', 1).unwrap();
        assert_eq!(bounds(&sim), (6, 4));
    }

    #[test]
    fn test_till_backward_selects_backwards() {
        let mut sim = HelixSimulator::new("abcabc".to_string());
        sim.selection = Selection::point(5);
        till_prev_char(&mut sim, 'a', 1).unwrap();
        assert_eq!(bounds(&sim), (6, 4));
    }

    #[test]
    fn test_find_counted_selects_nth_match() {
        let mut sim = HelixSimulator::new("banana".to_string());
        find_next_char(&mut sim, 'a', 3).unwrap();
        assert_eq!(bounds(&sim), (0, 6));
    }

    #[test]
    fn test_find_counted_beyond_matches_is_unchanged() {
        let mut sim = HelixSimulator::new("banana".to_string());
        find_next_char(&mut sim, 'a', 9).unwrap();
        assert_eq!(bounds(&sim), (0, 0));
    }

    #[test]
    fn test_find_extends_every_range() {
        let mut sim = HelixSimulator::new("xa\nya".to_string());
        sim.selection = Selection::new(helix_core::smallvec![Range::point(0), Range::point(3)], 0);
        find_next_char(&mut sim, 'a', 1).unwrap();
        let ranges: Vec<_> = sim
            .selection
            .ranges()
            .iter()
            .map(|r| (r.anchor, r.head))
            .collect();
        assert_eq!(ranges, vec![(0, 2), (3, 5)]);
    }

    #[test]
    fn test_goto_line_end_lands_on_last_char() {
        let mut sim = HelixSimulator::new("abc\ndef".to_string());
        move_line_end(&mut sim).unwrap();
        assert_eq!(bounds(&sim), (2, 2));
    }

    #[test]
    fn test_goto_line_end_on_last_line_without_newline() {
        let mut sim = HelixSimulator::new("abc\ndef".to_string());
        sim.selection = Selection::point(4);
        move_line_end(&mut sim).unwrap();
        assert_eq!(bounds(&sim), (6, 6));
    }

    #[test]
    fn test_goto_line_end_on_empty_line_stays() {
        let mut sim = HelixSimulator::new("abc\n\ndef".to_string());
        sim.selection = Selection::point(4);
        move_line_end(&mut sim).unwrap();
        assert_eq!(bounds(&sim), (4, 4));
    }

    #[test]
    fn test_goto_line_clamps_and_skips_trailing_blank_line() {
        let mut sim = HelixSimulator::new("a\nb\nc\n".to_string());
        goto_line(&mut sim, NonZeroUsize::new(2).unwrap());
        assert_eq!(bounds(&sim), (2, 2));
        goto_line(&mut sim, NonZeroUsize::new(99).unwrap());
        assert_eq!(bounds(&sim), (4, 4));
    }
}
