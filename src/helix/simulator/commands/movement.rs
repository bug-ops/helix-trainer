//! Movement commands

use crate::helix::simulator::{EditorMode, FindDirection, FindType, HelixSimulator};
use crate::security::UserError;
use helix_core::{
    Selection,
    doc_formatter::TextFormat,
    movement::{self, Movement},
    text_annotations::TextAnnotations,
};

/// Move left by count characters (works in any mode)
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
pub fn move_line_start<M: EditorMode>(sim: &mut HelixSimulator<M>) -> Result<(), UserError> {
    let head = sim.selection.primary().head;
    let line = sim.doc.char_to_line(head);
    let line_start = sim.doc.line_to_char(line);

    sim.selection = Selection::point(line_start);
    Ok(())
}

/// Move to end of current line
pub fn move_line_end<M: EditorMode>(sim: &mut HelixSimulator<M>) -> Result<(), UserError> {
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
pub fn move_document_start<M: EditorMode>(sim: &mut HelixSimulator<M>) -> Result<(), UserError> {
    sim.selection = Selection::point(0);
    Ok(())
}

/// Move to end of document
pub fn move_document_end<M: EditorMode>(sim: &mut HelixSimulator<M>) -> Result<(), UserError> {
    let end = sim.doc.len_chars();
    sim.selection = Selection::point(end);
    Ok(())
}

/// Find next occurrence of character on current line (Helix 'f' command)
pub fn find_next_char<M: EditorMode>(
    sim: &mut HelixSimulator<M>,
    ch: char,
    count: usize,
) -> Result<(), UserError> {
    // Record this motion for Alt-. repeat
    sim.find_state
        .set(ch, FindType::Find, FindDirection::Forward);

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
pub fn find_prev_char<M: EditorMode>(
    sim: &mut HelixSimulator<M>,
    ch: char,
    count: usize,
) -> Result<(), UserError> {
    // Record this motion for Alt-. repeat
    sim.find_state
        .set(ch, FindType::Find, FindDirection::Backward);

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
pub fn till_next_char<M: EditorMode>(
    sim: &mut HelixSimulator<M>,
    ch: char,
    count: usize,
) -> Result<(), UserError> {
    // Record this motion for Alt-. repeat
    sim.find_state
        .set(ch, FindType::Till, FindDirection::Forward);

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
pub fn till_prev_char<M: EditorMode>(
    sim: &mut HelixSimulator<M>,
    ch: char,
    count: usize,
) -> Result<(), UserError> {
    // Record this motion for Alt-. repeat
    sim.find_state
        .set(ch, FindType::Till, FindDirection::Backward);

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
/// Moves cursor to the start of the last line with content.
/// If the document ends with a newline, goes to the line before the empty final line.
pub fn goto_last_line<M: EditorMode>(sim: &mut HelixSimulator<M>) -> Result<(), UserError> {
    let total_lines = sim.doc.len_lines();
    let doc_len = sim.doc.len_chars();

    // If document is empty, stay at position 0
    if doc_len == 0 {
        sim.selection = Selection::point(0);
        return Ok(());
    }

    // Find the last line with content
    // If the last line is empty (document ends with newline), go to the line before it
    let mut last_line = total_lines.saturating_sub(1);

    // Check if the last line is empty
    let last_line_start = sim.doc.line_to_char(last_line);
    let last_line_len = if last_line + 1 < total_lines {
        sim.doc.line_to_char(last_line + 1) - last_line_start
    } else {
        doc_len - last_line_start
    };

    // If the last line is empty (0 characters or just a newline position at end), go to previous line
    if last_line_len == 0 && last_line > 0 {
        last_line = last_line.saturating_sub(1);
    }

    let line_start = sim.doc.line_to_char(last_line);
    sim.selection = Selection::point(line_start);
    Ok(())
}

/// Match brackets - find matching bracket pair (Helix 'm' command)
pub fn match_brackets<M: EditorMode>(sim: &mut HelixSimulator<M>) -> Result<(), UserError> {
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
pub fn flip_selections<M: EditorMode>(sim: &mut HelixSimulator<M>) -> Result<(), UserError> {
    let range = sim.selection.primary();
    // Swap anchor and head
    sim.selection = Selection::single(range.head, range.anchor);
    Ok(())
}

/// Move to previous paragraph (Helix '{' command)
///
/// A paragraph boundary is defined as a blank line (line containing only whitespace).
/// Moves cursor to the start of the previous paragraph.
pub fn goto_prev_paragraph<M: EditorMode>(
    sim: &mut HelixSimulator<M>,
    count: usize,
) -> Result<(), UserError> {
    let head = sim.selection.primary().head;
    let mut current_line = sim.doc.char_to_line(head);
    let slice = sim.doc.slice(..);

    let mut found = 0;
    let mut in_blank_region = is_line_blank(&slice, current_line);

    // Move up through lines looking for paragraph boundaries
    while current_line > 0 && found < count {
        current_line -= 1;
        let is_blank = is_line_blank(&slice, current_line);

        // Transition from non-blank to blank means we found a paragraph boundary
        if !in_blank_region && is_blank {
            found += 1;
            if found >= count {
                break;
            }
        }
        in_blank_region = is_blank;
    }

    // Move to the start of the target line
    let pos = sim.doc.line_to_char(current_line);
    sim.selection = Selection::point(pos);
    Ok(())
}

/// Move to next paragraph (Helix '}' command)
///
/// A paragraph boundary is defined as a blank line (line containing only whitespace).
/// Moves cursor to the start of the next paragraph.
pub fn goto_next_paragraph<M: EditorMode>(
    sim: &mut HelixSimulator<M>,
    count: usize,
) -> Result<(), UserError> {
    let head = sim.selection.primary().head;
    let mut current_line = sim.doc.char_to_line(head);
    let total_lines = sim.doc.len_lines();
    let slice = sim.doc.slice(..);

    let mut found = 0;
    let mut in_blank_region = is_line_blank(&slice, current_line);

    // Move down through lines looking for paragraph boundaries
    while current_line + 1 < total_lines && found < count {
        current_line += 1;
        let is_blank = is_line_blank(&slice, current_line);

        // Transition from blank to non-blank means we entered a new paragraph
        if in_blank_region && !is_blank {
            found += 1;
            if found >= count {
                break;
            }
        }
        in_blank_region = is_blank;
    }

    // Move to the start of the target line
    let pos = sim.doc.line_to_char(current_line);
    sim.selection = Selection::point(pos);
    Ok(())
}

/// Repeat last f/F/t/T motion in the same direction (Helix 'Alt-.' command)
pub fn repeat_last_motion<M: EditorMode>(sim: &mut HelixSimulator<M>) -> Result<(), UserError> {
    if let Some((ch, find_type, direction)) = sim.find_state.get() {
        match (find_type, direction) {
            (FindType::Find, FindDirection::Forward) => {
                // Don't re-record the motion
                let head = sim.selection.primary().head;
                let line = sim.doc.char_to_line(head);
                let line_end = if line + 1 < sim.doc.len_lines() {
                    sim.doc.line_to_char(line + 1)
                } else {
                    sim.doc.len_chars()
                };
                let slice = sim.doc.slice(..);
                let mut pos = head + 1;
                while pos < line_end {
                    if let Some(c) = slice.get_char(pos)
                        && c == ch
                    {
                        sim.selection = Selection::point(pos);
                        return Ok(());
                    }
                    pos += 1;
                }
            }
            (FindType::Find, FindDirection::Backward) => {
                let head = sim.selection.primary().head;
                let line = sim.doc.char_to_line(head);
                let line_start = sim.doc.line_to_char(line);
                let slice = sim.doc.slice(..);
                if head > line_start {
                    let mut pos = head - 1;
                    loop {
                        if let Some(c) = slice.get_char(pos)
                            && c == ch
                        {
                            sim.selection = Selection::point(pos);
                            return Ok(());
                        }
                        if pos == line_start {
                            break;
                        }
                        pos -= 1;
                    }
                }
            }
            (FindType::Till, FindDirection::Forward) => {
                let head = sim.selection.primary().head;
                let line = sim.doc.char_to_line(head);
                let line_end = if line + 1 < sim.doc.len_lines() {
                    sim.doc.line_to_char(line + 1)
                } else {
                    sim.doc.len_chars()
                };
                let slice = sim.doc.slice(..);
                let mut pos = head + 1;
                while pos < line_end {
                    if let Some(c) = slice.get_char(pos)
                        && c == ch
                    {
                        if pos > head + 1 {
                            sim.selection = Selection::point(pos - 1);
                        }
                        return Ok(());
                    }
                    pos += 1;
                }
            }
            (FindType::Till, FindDirection::Backward) => {
                let head = sim.selection.primary().head;
                let line = sim.doc.char_to_line(head);
                let line_start = sim.doc.line_to_char(line);
                let slice = sim.doc.slice(..);
                if head > line_start {
                    let mut pos = head - 1;
                    loop {
                        if let Some(c) = slice.get_char(pos)
                            && c == ch
                        {
                            sim.selection = Selection::point(pos + 1);
                            return Ok(());
                        }
                        if pos == line_start {
                            break;
                        }
                        pos -= 1;
                    }
                }
            }
        }
    }
    Ok(())
}

/// Default number of lines to move for page movement
const PAGE_SIZE: usize = 20;

/// Page up movement (Ctrl-b command)
///
/// Moves the cursor up by a full page (PAGE_SIZE lines).
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
/// Moves the cursor down by a full page (PAGE_SIZE lines).
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
/// Moves the cursor up by half a page (PAGE_SIZE / 2 lines).
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
/// Moves the cursor down by half a page (PAGE_SIZE / 2 lines).
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

/// Check if a line is blank (contains only whitespace)
fn is_line_blank(slice: &helix_core::RopeSlice, line: usize) -> bool {
    let line_start = slice.line_to_char(line);
    let line_end = if line + 1 < slice.len_lines() {
        slice.line_to_char(line + 1)
    } else {
        slice.len_chars()
    };

    for i in line_start..line_end {
        if let Some(c) = slice.get_char(i)
            && !c.is_whitespace()
        {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::helix::simulator::HelixSimulator;

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
        assert_eq!(sim.selection.primary().head, 1);

        // Repeat should find next 'b'
        repeat_last_motion(&mut sim).unwrap();
        assert_eq!(sim.selection.primary().head, 4);
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
}
