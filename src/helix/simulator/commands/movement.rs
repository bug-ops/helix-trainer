//! Movement commands

use crate::helix::simulator::{EditorMode, HelixSimulator};
use crate::security::UserError;
use helix_core::{
    Selection,
    doc_formatter::TextFormat,
    movement::{self, Movement},
    text_annotations::TextAnnotations,
};

/// Move left by count characters (works in any mode)
pub(super) fn move_left<M: EditorMode>(
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
pub(super) fn move_right<M: EditorMode>(
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
pub(super) fn move_down<M: EditorMode>(
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
pub(super) fn move_up<M: EditorMode>(
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
pub(super) fn move_next_word_start(
    sim: &mut HelixSimulator,
    count: usize,
) -> Result<(), UserError> {
    let slice = sim.doc.slice(..);
    let new_selection = sim
        .selection
        .clone()
        .transform(|range| movement::move_next_word_start(slice, range, count));

    sim.selection = new_selection;
    Ok(())
}

/// Move to start of previous word
pub(super) fn move_prev_word_start(
    sim: &mut HelixSimulator,
    count: usize,
) -> Result<(), UserError> {
    let slice = sim.doc.slice(..);
    let new_selection = sim
        .selection
        .clone()
        .transform(|range| movement::move_prev_word_start(slice, range, count));

    sim.selection = new_selection;
    Ok(())
}

/// Move to end of next word
pub(super) fn move_next_word_end<M: EditorMode>(
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
pub(super) fn move_next_long_word_start<M: EditorMode>(
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
pub(super) fn move_prev_long_word_start<M: EditorMode>(
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
pub(super) fn move_next_long_word_end<M: EditorMode>(
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
pub(super) fn move_line_start<M: EditorMode>(sim: &mut HelixSimulator<M>) -> Result<(), UserError> {
    let head = sim.selection.primary().head;
    let line = sim.doc.char_to_line(head);
    let line_start = sim.doc.line_to_char(line);

    sim.selection = Selection::point(line_start);
    Ok(())
}

/// Move to end of current line
pub(super) fn move_line_end<M: EditorMode>(sim: &mut HelixSimulator<M>) -> Result<(), UserError> {
    let head = sim.selection.primary().head;
    let line = sim.doc.char_to_line(head);

    // Get position of next line, or end of document
    let line_end = if line + 1 < sim.doc.len_lines() {
        sim.doc.line_to_char(line + 1) - 1
    } else {
        sim.doc.len_chars()
    };

    sim.selection = Selection::point(line_end);
    Ok(())
}

/// Move to start of document
pub(super) fn move_document_start<M: EditorMode>(
    sim: &mut HelixSimulator<M>,
) -> Result<(), UserError> {
    sim.selection = Selection::point(0);
    Ok(())
}

/// Move to end of document
pub(super) fn move_document_end<M: EditorMode>(
    sim: &mut HelixSimulator<M>,
) -> Result<(), UserError> {
    let end = sim.doc.len_chars();
    sim.selection = Selection::point(end);
    Ok(())
}

/// Find next occurrence of character on current line (Helix 'f' command)
pub(super) fn find_next_char<M: EditorMode>(
    sim: &mut HelixSimulator<M>,
    ch: char,
    count: usize,
) -> Result<(), UserError> {
    let head = sim.selection.primary().head;
    let line = sim.doc.char_to_line(head);
    let line_end = if line + 1 < sim.doc.len_lines() {
        sim.doc.line_to_char(line + 1)
    } else {
        sim.doc.len_chars()
    };

    // Search forward from cursor position
    let slice = sim.doc.slice(..);
    let mut found_count = 0;
    let mut pos = head + 1; // Start after current position

    while pos < line_end {
        if let Some(c) = slice.get_char(pos)
            && c == ch
        {
            found_count += 1;
            if found_count >= count {
                sim.selection = Selection::point(pos);
                return Ok(());
            }
        }
        pos += 1;
    }

    // Character not found - don't move cursor (Helix behavior)
    Ok(())
}

/// Find previous occurrence of character on current line (Helix 'F' command)
pub(super) fn find_prev_char<M: EditorMode>(
    sim: &mut HelixSimulator<M>,
    ch: char,
    count: usize,
) -> Result<(), UserError> {
    let head = sim.selection.primary().head;
    let line = sim.doc.char_to_line(head);
    let line_start = sim.doc.line_to_char(line);

    // Search backward from cursor position
    let slice = sim.doc.slice(..);
    let mut found_count = 0;

    if head > line_start {
        let mut pos = head - 1;
        loop {
            if let Some(c) = slice.get_char(pos)
                && c == ch
            {
                found_count += 1;
                if found_count >= count {
                    sim.selection = Selection::point(pos);
                    return Ok(());
                }
            }
            if pos == line_start {
                break;
            }
            pos -= 1;
        }
    }

    // Character not found - don't move cursor
    Ok(())
}

/// Move till (before) next occurrence of character (Helix 't' command)
pub(super) fn till_next_char<M: EditorMode>(
    sim: &mut HelixSimulator<M>,
    ch: char,
    count: usize,
) -> Result<(), UserError> {
    let head = sim.selection.primary().head;
    let line = sim.doc.char_to_line(head);
    let line_end = if line + 1 < sim.doc.len_lines() {
        sim.doc.line_to_char(line + 1)
    } else {
        sim.doc.len_chars()
    };

    // Search forward from cursor position
    let slice = sim.doc.slice(..);
    let mut found_count = 0;
    let mut pos = head + 1;

    while pos < line_end {
        if let Some(c) = slice.get_char(pos)
            && c == ch
        {
            found_count += 1;
            if found_count >= count {
                // Stop one position before the character
                if pos > head + 1 {
                    sim.selection = Selection::point(pos - 1);
                }
                return Ok(());
            }
        }
        pos += 1;
    }

    Ok(())
}

/// Move till (after) previous occurrence of character (Helix 'T' command)
pub(super) fn till_prev_char<M: EditorMode>(
    sim: &mut HelixSimulator<M>,
    ch: char,
    count: usize,
) -> Result<(), UserError> {
    let head = sim.selection.primary().head;
    let line = sim.doc.char_to_line(head);
    let line_start = sim.doc.line_to_char(line);

    // Search backward from cursor position
    let slice = sim.doc.slice(..);
    let mut found_count = 0;

    if head > line_start {
        let mut pos = head - 1;
        loop {
            if let Some(c) = slice.get_char(pos)
                && c == ch
            {
                found_count += 1;
                if found_count >= count {
                    // Stop one position after the character
                    sim.selection = Selection::point(pos + 1);
                    return Ok(());
                }
            }
            if pos == line_start {
                break;
            }
            pos -= 1;
        }
    }

    Ok(())
}

/// Move to first non-whitespace character on line (Helix 'gs' command)
pub(super) fn goto_first_nonwhitespace<M: EditorMode>(
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
pub(super) fn goto_last_line<M: EditorMode>(sim: &mut HelixSimulator<M>) -> Result<(), UserError> {
    let last_line = sim.doc.len_lines().saturating_sub(1);
    let line_start = sim.doc.line_to_char(last_line);
    sim.selection = Selection::point(line_start);
    Ok(())
}

/// Match brackets - find matching bracket pair (Helix 'm' command)
pub(super) fn match_brackets<M: EditorMode>(sim: &mut HelixSimulator<M>) -> Result<(), UserError> {
    let head = sim.selection.primary().head;
    let slice = sim.doc.slice(..);

    // Get character at cursor
    let Some(ch) = slice.get_char(head) else {
        return Ok(());
    };

    // Determine bracket type and direction
    let (open, close, forward) = match ch {
        '(' => ('(', ')', true),
        ')' => ('(', ')', false),
        '[' => ('[', ']', true),
        ']' => ('[', ']', false),
        '{' => ('{', '}', true),
        '}' => ('{', '}', false),
        '<' => ('<', '>', true),
        '>' => ('<', '>', false),
        _ => return Ok(()), // Not on a bracket
    };

    let doc_len = sim.doc.len_chars();
    let mut depth = 1;

    if forward {
        let mut pos = head + 1;
        while pos < doc_len && depth > 0 {
            if let Some(c) = slice.get_char(pos) {
                if c == close {
                    depth -= 1;
                } else if c == open {
                    depth += 1;
                }
            }
            if depth == 0 {
                sim.selection = Selection::point(pos);
                return Ok(());
            }
            pos += 1;
        }
    } else if head > 0 {
        let mut pos = head - 1;
        loop {
            if let Some(c) = slice.get_char(pos) {
                if c == open {
                    depth -= 1;
                } else if c == close {
                    depth += 1;
                }
            }
            if depth == 0 {
                sim.selection = Selection::point(pos);
                return Ok(());
            }
            if pos == 0 {
                break;
            }
            pos -= 1;
        }
    }

    Ok(())
}

/// Flip selection direction (swap anchor and head) - Helix 'Alt-;' command
pub(super) fn flip_selections<M: EditorMode>(sim: &mut HelixSimulator<M>) -> Result<(), UserError> {
    let range = sim.selection.primary();
    // Swap anchor and head
    sim.selection = Selection::single(range.head, range.anchor);
    Ok(())
}
