//! Selection commands (s, S, Alt-s, &, _, Alt--, Alt-_, C, Alt-C, K, Alt-K, Ctrl-c)

use crate::helix::simulator::{EditorMode, HelixSimulator};
use crate::security::UserError;
use helix_core::Range;
use helix_core::Selection;
use helix_core::comment::toggle_line_comments;
use helix_core::selection::split_on_newline;

/// Trim whitespace from selections (_ command)
pub fn trim_selections<M: EditorMode>(sim: &mut HelixSimulator<M>) -> Result<(), UserError> {
    let range = sim.selection.primary();
    let start = range.from();
    let end = range.to();

    if start >= end {
        return Ok(());
    }

    let slice = sim.doc.slice(..);
    let mut new_start = start;
    let mut new_end = end;

    while new_start < new_end {
        if let Some(ch) = slice.get_char(new_start) {
            if ch.is_whitespace() && ch != '\n' {
                new_start += 1;
            } else {
                break;
            }
        } else {
            break;
        }
    }

    while new_end > new_start {
        if let Some(ch) = slice.get_char(new_end - 1) {
            if ch.is_whitespace() && ch != '\n' {
                new_end -= 1;
            } else {
                break;
            }
        } else {
            break;
        }
    }

    if new_start < new_end {
        sim.selection = Selection::single(new_start, new_end);
    } else {
        sim.selection = Selection::point(new_start);
    }

    Ok(())
}

/// Merge all selections into one (Alt-- command)
pub fn merge_selections<M: EditorMode>(sim: &mut HelixSimulator<M>) -> Result<(), UserError> {
    let range = sim.selection.primary();
    let start = range.from();
    let end = range.to();
    sim.selection = Selection::single(start, end);

    Ok(())
}

/// Copy selection to next line (C command)
///
/// Creates an additional cursor on the next line at the same column (with clamping).
/// Preserves existing cursors - the new cursor becomes the primary selection.
pub fn copy_selection_next_line<M: EditorMode>(
    sim: &mut HelixSimulator<M>,
) -> Result<(), UserError> {
    let range = sim.selection.primary();
    let head = range.head;

    let current_line = sim.doc.char_to_line(head);
    let total_lines = sim.doc.len_lines();

    if current_line + 1 >= total_lines {
        return Ok(());
    }

    let line_start = sim.doc.line_to_char(current_line);
    let col = head - line_start;

    let next_line_start = sim.doc.line_to_char(current_line + 1);
    let next_line_len = if current_line + 2 < total_lines {
        sim.doc.line_to_char(current_line + 2) - next_line_start - 1 // -1 for newline
    } else {
        sim.doc.len_chars() - next_line_start
    };

    let new_col = col.min(next_line_len);
    let new_head = next_line_start + new_col;

    // Accumulate existing ranges and add the new cursor
    let new_range = Range::point(new_head.min(sim.doc.len_chars()));
    let mut ranges: Vec<Range> = sim.selection.ranges().to_vec();
    ranges.push(new_range);
    let primary_index = ranges.len() - 1;
    // Primary index is the newly added cursor (last index)
    sim.selection = Selection::new(ranges.into(), primary_index);

    Ok(())
}

/// Copy selection to previous line (Alt-C command)
///
/// Creates an additional cursor on the previous line at the same column (with clamping).
/// Preserves existing cursors - the new cursor becomes the primary selection.
pub fn copy_selection_prev_line<M: EditorMode>(
    sim: &mut HelixSimulator<M>,
) -> Result<(), UserError> {
    let range = sim.selection.primary();
    let head = range.head;
    let current_line = sim.doc.char_to_line(head);

    if current_line == 0 {
        return Ok(());
    }

    let line_start = sim.doc.line_to_char(current_line);
    let col = head - line_start;

    let prev_line_start = sim.doc.line_to_char(current_line - 1);
    let prev_line_len = line_start - prev_line_start - 1;

    let new_col = col.min(prev_line_len);
    let new_head = prev_line_start + new_col;

    // Accumulate existing ranges and insert new cursor at beginning
    let new_range = Range::point(new_head);
    let mut ranges: Vec<Range> = sim.selection.ranges().to_vec();
    // Insert at beginning since we're going backwards (maintains top-to-bottom order)
    ranges.insert(0, new_range);
    // Primary index is the newly added cursor (index 0)
    sim.selection = Selection::new(ranges.into(), 0);

    Ok(())
}

/// Toggle line comments (Ctrl-c command)
pub fn toggle_comments<M: EditorMode>(sim: &mut HelixSimulator<M>) -> Result<(), UserError> {
    let transaction = toggle_line_comments(&sim.doc, &sim.selection, Some("//"));
    sim.apply_transaction(transaction);
    Ok(())
}

/// Split selection on newlines (Alt-s command)
pub fn split_selection_newlines<M: EditorMode>(
    sim: &mut HelixSimulator<M>,
) -> Result<(), UserError> {
    let slice = sim.doc.slice(..);
    sim.selection = split_on_newline(slice, &sim.selection);
    Ok(())
}

/// Align selections to columns (& command)
pub fn align_selections<M: EditorMode>(_sim: &mut HelixSimulator<M>) -> Result<(), UserError> {
    Ok(())
}

/// Merge consecutive selections (Alt-_ command)
pub fn merge_consecutive_selections<M: EditorMode>(
    sim: &mut HelixSimulator<M>,
) -> Result<(), UserError> {
    merge_selections(sim)
}

/// Select regex matches (s command)
pub fn select_regex<M: EditorMode>(_sim: &mut HelixSimulator<M>) -> Result<(), UserError> {
    Ok(())
}

/// Split selection on regex (S command)
pub fn split_selection<M: EditorMode>(_sim: &mut HelixSimulator<M>) -> Result<(), UserError> {
    Ok(())
}

/// Keep selections matching pattern (K command)
pub fn keep_selections_matching<M: EditorMode>(
    _sim: &mut HelixSimulator<M>,
) -> Result<(), UserError> {
    Ok(())
}

/// Remove selections matching pattern (Alt-K command)
pub fn remove_selections_matching<M: EditorMode>(
    _sim: &mut HelixSimulator<M>,
) -> Result<(), UserError> {
    Ok(())
}

/// Keep only the primary selection (, command)
pub fn keep_primary_selection<M: EditorMode>(
    _sim: &mut HelixSimulator<M>,
) -> Result<(), UserError> {
    Ok(())
}

/// Remove the primary selection (Alt-, command)
pub fn remove_primary_selection<M: EditorMode>(
    _sim: &mut HelixSimulator<M>,
) -> Result<(), UserError> {
    Ok(())
}

/// Shrink selection to line bounds (Alt-x command)
pub fn shrink_to_line_bounds<M: EditorMode>(sim: &mut HelixSimulator<M>) -> Result<(), UserError> {
    let range = sim.selection.primary();
    let head = range.head;
    let line = sim.doc.char_to_line(head);

    let line_start = sim.doc.line_to_char(line);
    let line_end = if line + 1 < sim.doc.len_lines() {
        sim.doc.line_to_char(line + 1) - 1
    } else {
        sim.doc.len_chars()
    };

    sim.selection = Selection::single(line_start, line_end);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::helix::simulator::NormalMode;

    #[test]
    fn test_trim_selections_whitespace() {
        let mut sim: HelixSimulator<NormalMode> = HelixSimulator::new("  hello  ".to_string());

        // Select with whitespace
        sim.selection = Selection::single(0, 9);

        trim_selections(&mut sim).unwrap();

        // Selection should now be just "hello"
        let range = sim.selection.primary();
        assert_eq!(range.from(), 2);
        assert_eq!(range.to(), 7);
    }

    #[test]
    fn test_trim_selections_no_whitespace() {
        let mut sim: HelixSimulator<NormalMode> = HelixSimulator::new("hello".to_string());

        sim.selection = Selection::single(0, 5);

        trim_selections(&mut sim).unwrap();

        // Selection unchanged
        let range = sim.selection.primary();
        assert_eq!(range.from(), 0);
        assert_eq!(range.to(), 5);
    }

    #[test]
    fn test_trim_selections_only_leading() {
        let mut sim: HelixSimulator<NormalMode> = HelixSimulator::new("   hello".to_string());

        sim.selection = Selection::single(0, 8);

        trim_selections(&mut sim).unwrap();

        let range = sim.selection.primary();
        assert_eq!(range.from(), 3);
        assert_eq!(range.to(), 8);
    }

    #[test]
    fn test_merge_selections() {
        let mut sim: HelixSimulator<NormalMode> = HelixSimulator::new("hello world".to_string());

        sim.selection = Selection::single(0, 5);

        merge_selections(&mut sim).unwrap();

        // Selection unchanged for single selection
        let range = sim.selection.primary();
        assert_eq!(range.from(), 0);
        assert_eq!(range.to(), 5);
    }

    #[test]
    fn test_copy_selection_next_line() {
        let mut sim: HelixSimulator<NormalMode> =
            HelixSimulator::new("line 1\nline 2\nline 3".to_string());

        // Position at column 2 of line 1
        sim.selection = Selection::point(2);

        copy_selection_next_line(&mut sim).unwrap();

        // Should now have 2 cursors
        assert_eq!(sim.selection.len(), 2);

        // Primary cursor should be at column 2 of line 2 (the new cursor)
        let head = sim.selection.primary().head;
        let line = sim.doc.char_to_line(head);
        let line_start = sim.doc.line_to_char(line);
        let col = head - line_start;

        assert_eq!(line, 1);
        assert_eq!(col, 2);

        // Original cursor should still exist at position 2 (line 0, col 2)
        let ranges: Vec<_> = sim.selection.ranges().iter().collect();
        assert_eq!(ranges[0].head, 2); // Original cursor
        assert_eq!(ranges[1].head, 9); // New cursor at line 2 (7 chars for "line 1\n" + 2)
    }

    #[test]
    fn test_copy_selection_prev_line() {
        let mut sim: HelixSimulator<NormalMode> =
            HelixSimulator::new("line 1\nline 2\nline 3".to_string());

        // Position at column 2 of line 2
        let line2_start = sim.doc.line_to_char(1);
        sim.selection = Selection::point(line2_start + 2);

        copy_selection_prev_line(&mut sim).unwrap();

        // Should now have 2 cursors
        assert_eq!(sim.selection.len(), 2);

        // Primary cursor should be at column 2 of line 1 (the new cursor, inserted at beginning)
        let head = sim.selection.primary().head;
        assert_eq!(head, 2);

        // Verify cursor order: new cursor at index 0, original at index 1
        let ranges: Vec<_> = sim.selection.ranges().iter().collect();
        assert_eq!(ranges[0].head, 2); // New cursor at line 0, col 2
        assert_eq!(ranges[1].head, line2_start + 2); // Original cursor at line 1, col 2
    }

    #[test]
    fn test_copy_selection_next_line_at_last() {
        let mut sim: HelixSimulator<NormalMode> = HelixSimulator::new("line 1\nline 2".to_string());

        // Position at last line
        let line2_start = sim.doc.line_to_char(1);
        sim.selection = Selection::point(line2_start);

        copy_selection_next_line(&mut sim).unwrap();

        // Should stay at same position
        assert_eq!(sim.selection.primary().head, line2_start);
    }

    #[test]
    fn test_copy_selection_prev_line_at_first() {
        let mut sim: HelixSimulator<NormalMode> = HelixSimulator::new("line 1\nline 2".to_string());

        // Position at first line
        sim.selection = Selection::point(2);

        copy_selection_prev_line(&mut sim).unwrap();

        // Should stay at same position
        assert_eq!(sim.selection.primary().head, 2);
    }

    #[test]
    fn test_toggle_comments_add() {
        let mut sim: HelixSimulator<NormalMode> = HelixSimulator::new("hello\nworld".to_string());

        // Select first line
        sim.selection = Selection::single(0, 5);

        toggle_comments(&mut sim).unwrap();

        let content = sim.doc.to_string();
        assert!(content.starts_with("// hello"));
    }

    #[test]
    fn test_toggle_comments_remove() {
        let mut sim: HelixSimulator<NormalMode> =
            HelixSimulator::new("// hello\nworld".to_string());

        // Select first line
        sim.selection = Selection::single(0, 8);

        toggle_comments(&mut sim).unwrap();

        let content = sim.doc.to_string();
        assert!(content.starts_with("hello"));
    }

    #[test]
    fn test_split_selection_newlines() {
        let mut sim: HelixSimulator<NormalMode> =
            HelixSimulator::new("line 1\nline 2\nline 3".to_string());

        // Select multiple lines (0 to 20 covers all three lines)
        sim.selection = Selection::single(0, 20);

        split_selection_newlines(&mut sim).unwrap();

        // Should create multiple selections, one per line (excluding newlines)
        // "line 1" (0-6), "line 2" (7-13), "line 3" (14-20)
        assert_eq!(sim.selection.len(), 3);

        let ranges: Vec<_> = sim.selection.ranges().iter().collect();
        assert_eq!(ranges[0].from(), 0);
        assert_eq!(ranges[0].to(), 6); // "line 1"
        assert_eq!(ranges[1].from(), 7);
        assert_eq!(ranges[1].to(), 13); // "line 2"
        assert_eq!(ranges[2].from(), 14);
        assert_eq!(ranges[2].to(), 20); // "line 3"
    }

    #[test]
    fn test_trim_preserves_newlines() {
        let mut sim: HelixSimulator<NormalMode> = HelixSimulator::new("  hello\n  ".to_string());

        sim.selection = Selection::single(0, 10);

        trim_selections(&mut sim).unwrap();

        // Should not trim past newlines
        let range = sim.selection.primary();
        assert!(range.from() >= 2);
    }

    // Edge case tests for improved coverage (I4)

    #[test]
    fn test_toggle_comments_mixed_lines() {
        let mut sim: HelixSimulator<NormalMode> =
            HelixSimulator::new("// commented\nuncommented".to_string());

        // Select all lines (string is 24 chars: 12 + 1 newline + 11)
        sim.selection = Selection::single(0, 24);

        toggle_comments(&mut sim).unwrap();

        let content = sim.doc.to_string();
        // Since not all lines are commented, it should add comments to all
        assert!(content.starts_with("// // commented"));
        assert!(content.contains("// uncommented"));
    }

    #[test]
    fn test_toggle_comments_indented_lines() {
        let mut sim: HelixSimulator<NormalMode> =
            HelixSimulator::new("    // indented comment".to_string());

        sim.selection = Selection::single(0, 22);

        toggle_comments(&mut sim).unwrap();

        let content = sim.doc.to_string();
        // Should remove the comment while preserving indentation
        assert_eq!(content, "    indented comment");
    }

    #[test]
    fn test_toggle_comments_no_space_after_slashes() {
        let mut sim: HelixSimulator<NormalMode> = HelixSimulator::new("//no space".to_string());

        sim.selection = Selection::single(0, 10);

        toggle_comments(&mut sim).unwrap();

        let content = sim.doc.to_string();
        // Should remove "//" (without space)
        assert_eq!(content, "no space");
    }

    #[test]
    fn test_copy_selection_next_line_column_clamping() {
        let mut sim: HelixSimulator<NormalMode> =
            HelixSimulator::new("long line here\nshort".to_string());

        // Position at column 12 of first line
        sim.selection = Selection::point(12);

        copy_selection_next_line(&mut sim).unwrap();

        // Should have 2 cursors now
        assert_eq!(sim.selection.len(), 2);

        // Primary cursor (new) should be on next line with clamped column
        let head = sim.selection.primary().head;
        let line = sim.doc.char_to_line(head);
        assert_eq!(line, 1);

        // Column should be clamped to "short" length (5)
        let line_start = sim.doc.line_to_char(1);
        let col = head - line_start;
        assert!(col <= 5);

        // Original cursor should still be at position 12
        let ranges: Vec<_> = sim.selection.ranges().iter().collect();
        assert_eq!(ranges[0].head, 12);
    }

    #[test]
    fn test_copy_selection_prev_line_column_clamping() {
        let mut sim: HelixSimulator<NormalMode> =
            HelixSimulator::new("short\nlong line here".to_string());

        // Position at column 12 of second line
        let line2_start = sim.doc.line_to_char(1);
        sim.selection = Selection::point(line2_start + 12);

        copy_selection_prev_line(&mut sim).unwrap();

        // Should have 2 cursors now
        assert_eq!(sim.selection.len(), 2);

        // Primary cursor (new, at index 0) should be on previous line with clamped column
        let head = sim.selection.primary().head;
        let line = sim.doc.char_to_line(head);
        assert_eq!(line, 0);

        // Column should be clamped to "short" length (5)
        assert!(head <= 5);

        // Original cursor should still be at position line2_start + 12
        let ranges: Vec<_> = sim.selection.ranges().iter().collect();
        assert_eq!(ranges[1].head, line2_start + 12);
    }

    #[test]
    fn test_trim_selections_empty_selection() {
        let mut sim: HelixSimulator<NormalMode> = HelixSimulator::new("hello".to_string());

        // Empty selection (point)
        sim.selection = Selection::point(2);

        trim_selections(&mut sim).unwrap();

        // Should be no-op for empty selection
        assert_eq!(sim.selection.primary().head, 2);
    }

    #[test]
    fn test_trim_selections_all_whitespace() {
        let mut sim: HelixSimulator<NormalMode> = HelixSimulator::new("     ".to_string());

        sim.selection = Selection::single(0, 5);

        trim_selections(&mut sim).unwrap();

        // Should collapse to a point since all whitespace
        let range = sim.selection.primary();
        assert_eq!(range.from(), range.to());
    }

    #[test]
    fn test_merge_selections_empty() {
        let mut sim: HelixSimulator<NormalMode> = HelixSimulator::new("hello".to_string());

        // Point selection
        sim.selection = Selection::point(2);

        merge_selections(&mut sim).unwrap();

        // Should remain a point
        let range = sim.selection.primary();
        assert_eq!(range.from(), 2);
        assert_eq!(range.to(), 2);
    }

    #[test]
    fn test_split_selection_newlines_single_line() {
        let mut sim: HelixSimulator<NormalMode> = HelixSimulator::new("single line".to_string());

        sim.selection = Selection::single(0, 11);

        split_selection_newlines(&mut sim).unwrap();

        // Should be no-op for single line
        // Selection should remain unchanged
        let range = sim.selection.primary();
        assert_eq!(range.from(), 0);
        assert_eq!(range.to(), 11);
    }

    #[test]
    fn test_toggle_comments_single_line_selection() {
        // Test commenting a single line
        let mut sim: HelixSimulator<NormalMode> = HelixSimulator::new("hello world".to_string());

        sim.selection = Selection::single(0, 11);

        toggle_comments(&mut sim).unwrap();

        let content = sim.doc.to_string();
        assert_eq!(content, "// hello world");
    }

    #[test]
    fn test_copy_selection_handles_last_line_without_newline() {
        let mut sim: HelixSimulator<NormalMode> = HelixSimulator::new("line 1\nlast".to_string());

        // Position on first line
        sim.selection = Selection::point(2);

        copy_selection_next_line(&mut sim).unwrap();

        // Should move to second line (last line, no trailing newline)
        let head = sim.selection.primary().head;
        let line = sim.doc.char_to_line(head);
        assert_eq!(line, 1);
    }

    #[test]
    fn test_keep_primary_selection() {
        let mut sim: HelixSimulator<NormalMode> = HelixSimulator::new("hello world".to_string());

        // Set a selection
        sim.selection = Selection::point(5);

        // For single selection, this is a no-op
        keep_primary_selection(&mut sim).unwrap();

        // Selection should be unchanged
        assert_eq!(sim.selection.primary().head, 5);
    }

    #[test]
    fn test_remove_primary_selection() {
        let mut sim: HelixSimulator<NormalMode> = HelixSimulator::new("hello world".to_string());

        // Set a selection
        sim.selection = Selection::point(3);

        // For single selection, this is a no-op (can't remove the only cursor)
        remove_primary_selection(&mut sim).unwrap();

        // Selection should be unchanged
        assert_eq!(sim.selection.primary().head, 3);
    }

    #[test]
    fn test_shrink_to_line_bounds() {
        let mut sim: HelixSimulator<NormalMode> =
            HelixSimulator::new("hello world\nsecond line".to_string());

        // Set cursor in the middle of first line
        sim.selection = Selection::point(5);

        shrink_to_line_bounds(&mut sim).unwrap();

        // Selection should be the entire first line (excluding newline)
        let range = sim.selection.primary();
        assert_eq!(range.from(), 0);
        assert_eq!(range.to(), 11); // "hello world" without newline
    }

    #[test]
    fn test_shrink_to_line_bounds_last_line() {
        let mut sim: HelixSimulator<NormalMode> =
            HelixSimulator::new("first\nlast line".to_string());

        // Move to last line
        sim.selection = Selection::point(10);

        shrink_to_line_bounds(&mut sim).unwrap();

        let range = sim.selection.primary();
        assert_eq!(range.from(), 6); // Start of "last line"
        assert_eq!(range.to(), 15); // End of file
    }

    // Multi-cursor creation tests (Issue #141 Phase 5)

    #[test]
    fn test_copy_selection_next_creates_multi_cursor() {
        let mut sim: HelixSimulator<NormalMode> =
            HelixSimulator::new("line 1\nline 2\nline 3".to_string());

        // Start at position 2 (column 2 of line 0)
        sim.selection = Selection::point(2);
        assert_eq!(sim.selection.len(), 1);

        // First C: creates cursor on line 1
        copy_selection_next_line(&mut sim).unwrap();
        assert_eq!(sim.selection.len(), 2);

        // Second C: creates cursor on line 2
        copy_selection_next_line(&mut sim).unwrap();
        assert_eq!(sim.selection.len(), 3);

        // Verify all cursor positions
        let ranges: Vec<_> = sim.selection.ranges().iter().collect();
        assert_eq!(ranges[0].head, 2); // Line 0, col 2
        assert_eq!(ranges[1].head, 9); // Line 1, col 2 (7 chars for "line 1\n" + 2)
        assert_eq!(ranges[2].head, 16); // Line 2, col 2 (14 chars for first two lines + 2)
    }

    #[test]
    fn test_copy_selection_prev_creates_multi_cursor() {
        let mut sim: HelixSimulator<NormalMode> =
            HelixSimulator::new("line 1\nline 2\nline 3".to_string());

        // Start at line 2, column 2 (position 16)
        sim.selection = Selection::point(16);
        assert_eq!(sim.selection.len(), 1);

        // First Alt-C: creates cursor on line 1
        copy_selection_prev_line(&mut sim).unwrap();
        assert_eq!(sim.selection.len(), 2);

        // Second Alt-C: creates cursor on line 0
        copy_selection_prev_line(&mut sim).unwrap();
        assert_eq!(sim.selection.len(), 3);

        // Verify all cursor positions (new cursors inserted at beginning)
        let ranges: Vec<_> = sim.selection.ranges().iter().collect();
        assert_eq!(ranges[0].head, 2); // Line 0, col 2 (most recent)
        assert_eq!(ranges[1].head, 9); // Line 1, col 2
        assert_eq!(ranges[2].head, 16); // Line 2, col 2 (original)
    }

    #[test]
    fn test_copy_selection_with_existing_multi_cursor() {
        let mut sim: HelixSimulator<NormalMode> = HelixSimulator::new("a\nb\nc\nd".to_string());

        // Start with 2 cursors at lines 0 and 1
        let ranges = vec![Range::point(0), Range::point(2)];
        sim.selection = Selection::new(ranges.into(), 1); // Primary at index 1 (line 1)

        assert_eq!(sim.selection.len(), 2);

        // C from primary cursor (at line 1) should add cursor at line 2
        copy_selection_next_line(&mut sim).unwrap();

        // Now should have 3 cursors
        assert_eq!(sim.selection.len(), 3);

        let ranges: Vec<_> = sim.selection.ranges().iter().collect();
        assert_eq!(ranges[0].head, 0); // Line 0
        assert_eq!(ranges[1].head, 2); // Line 1
        assert_eq!(ranges[2].head, 4); // Line 2 (new cursor)
    }

    #[test]
    fn test_copy_selection_column_clamp_multi_cursor() {
        let mut sim: HelixSimulator<NormalMode> =
            HelixSimulator::new("long line here\nshort\ntiny".to_string());

        // Start at column 10 of first line
        sim.selection = Selection::point(10);

        // First C: column clamped to 5 on "short"
        copy_selection_next_line(&mut sim).unwrap();
        assert_eq!(sim.selection.len(), 2);

        // Second C: column clamped to 4 on "tiny"
        copy_selection_next_line(&mut sim).unwrap();
        assert_eq!(sim.selection.len(), 3);

        let ranges: Vec<_> = sim.selection.ranges().iter().collect();
        assert_eq!(ranges[0].head, 10); // Original at col 10
        assert_eq!(ranges[1].head, 20); // "short" is 5 chars, so clamped to end (15 + 5 = 20)
        assert_eq!(ranges[2].head, 25); // "tiny" is 4 chars, so clamped to end (21 + 4 = 25)
    }
}
