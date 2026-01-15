//! Search commands (/, ?, n, N, *, Alt-*)
//!
//! Note: Helix provides `*` for word search (forward). To search backward
//! for word under cursor, use `*` followed by `N` to go to previous match.

use crate::helix::simulator::HelixSimulator;
use crate::helix::simulator::search_state::SearchDirection;
use crate::security::UserError;
use helix_core::Selection;
use helix_core::ropey::RopeSlice;
use std::hash::{Hash, Hasher};

#[inline]
fn is_word_char(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_'
}

fn extract_word_at_cursor(slice: RopeSlice, cursor: usize) -> Option<(usize, usize)> {
    let doc_len = slice.len_chars();

    if cursor >= doc_len {
        return None;
    }

    let ch_at_cursor = slice.get_char(cursor)?;
    if !is_word_char(ch_at_cursor) {
        return None;
    }

    let mut start = cursor;
    let mut end = cursor;

    while start > 0 {
        if let Some(ch) = slice.get_char(start.saturating_sub(1)) {
            if is_word_char(ch) {
                start -= 1;
            } else {
                break;
            }
        } else {
            break;
        }
    }

    while end < doc_len {
        if let Some(ch) = slice.get_char(end) {
            if is_word_char(ch) {
                end += 1;
            } else {
                break;
            }
        } else {
            break;
        }
    }

    if start < end {
        Some((start, end))
    } else {
        None
    }
}

fn compute_doc_version(doc: &helix_core::Rope) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    doc.len_bytes().hash(&mut hasher);
    doc.len_chars().hash(&mut hasher);
    if doc.len_chars() > 0 {
        doc.slice(..doc.len_chars().min(64))
            .to_string()
            .hash(&mut hasher);
    }
    hasher.finish()
}

/// Search forward from cursor position
pub fn search_next_match<M: crate::helix::simulator::EditorMode>(
    sim: &mut HelixSimulator<M>,
) -> Result<(), UserError> {
    let head = sim.selection.primary().head;
    let content = sim.doc.slice(..).to_string();
    let doc_version = compute_doc_version(&sim.doc);

    sim.search_state
        .update_matches_with_version(&content, doc_version);

    let head_byte = sim.doc.char_to_byte(head);
    if let Some((_, range)) = sim.search_state.find_next(head_byte) {
        let start_chars = sim.doc.byte_to_char(range.start);
        let end_chars = sim.doc.byte_to_char(range.end);
        sim.selection = Selection::single(start_chars, end_chars);
    }

    Ok(())
}

/// Search backward from cursor position
pub fn search_prev_match<M: crate::helix::simulator::EditorMode>(
    sim: &mut HelixSimulator<M>,
) -> Result<(), UserError> {
    let head = sim.selection.primary().head;
    let content = sim.doc.slice(..).to_string();
    let doc_version = compute_doc_version(&sim.doc);

    sim.search_state
        .update_matches_with_version(&content, doc_version);

    let head_byte = sim.doc.char_to_byte(head);
    if let Some((_, range)) = sim.search_state.find_prev(head_byte) {
        let start_chars = sim.doc.byte_to_char(range.start);
        let end_chars = sim.doc.byte_to_char(range.end);
        sim.selection = Selection::single(start_chars, end_chars);
    }

    Ok(())
}

/// Search forward (/) command
pub fn search_forward<M: crate::helix::simulator::EditorMode>(
    sim: &mut HelixSimulator<M>,
) -> Result<(), UserError> {
    sim.search_state.set_current_match(None);
    search_next_match(sim)
}

/// Search backward (?) command
pub fn search_backward<M: crate::helix::simulator::EditorMode>(
    sim: &mut HelixSimulator<M>,
) -> Result<(), UserError> {
    sim.search_state.set_current_match(None);
    search_prev_match(sim)
}

/// Go to next match (n command)
pub fn goto_next_match<M: crate::helix::simulator::EditorMode>(
    sim: &mut HelixSimulator<M>,
) -> Result<(), UserError> {
    if !sim.search_state.has_pattern() {
        return Ok(());
    }

    match sim.search_state.direction() {
        SearchDirection::Forward => search_next_match(sim),
        SearchDirection::Backward => search_prev_match(sim),
    }
}

/// Go to previous match (N command)
pub fn goto_prev_match<M: crate::helix::simulator::EditorMode>(
    sim: &mut HelixSimulator<M>,
) -> Result<(), UserError> {
    if !sim.search_state.has_pattern() {
        return Ok(());
    }

    match sim.search_state.direction() {
        SearchDirection::Forward => search_prev_match(sim),
        SearchDirection::Backward => search_next_match(sim),
    }
}

/// Search word under cursor (* command)
pub fn search_word_under_cursor<M: crate::helix::simulator::EditorMode>(
    sim: &mut HelixSimulator<M>,
) -> Result<(), UserError> {
    let head = sim.selection.primary().head;
    let slice = sim.doc.slice(..);

    let Some((start, end)) = extract_word_at_cursor(slice, head) else {
        return Ok(());
    };

    let word: String = slice.slice(start..end).chars().collect();

    if sim
        .search_state
        .set_word_pattern(&word, SearchDirection::Forward)
        .is_ok()
    {
        search_next_match(sim)?;
    }

    Ok(())
}

/// Search selection text (Alt-* command)
pub fn search_selection<M: crate::helix::simulator::EditorMode>(
    sim: &mut HelixSimulator<M>,
) -> Result<(), UserError> {
    let range = sim.selection.primary();
    let start = range.from();
    let end = range.to();

    if start == end {
        return search_word_under_cursor(sim);
    }

    let slice = sim.doc.slice(..);
    let selection_text: String = slice.slice(start..end).chars().collect();

    if sim
        .search_state
        .set_selection_pattern(&selection_text, SearchDirection::Forward)
        .is_ok()
    {
        search_next_match(sim)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::helix::simulator::NormalMode;

    #[test]
    fn test_search_word_under_cursor() {
        let mut sim: HelixSimulator<NormalMode> =
            HelixSimulator::new("hello world hello".to_string());

        // Move cursor to "hello"
        sim.selection = Selection::point(0);

        // Search for word under cursor
        search_word_under_cursor(&mut sim).unwrap();

        // Should have found the word pattern
        assert!(sim.search_state.has_pattern());

        // Should have selected the next match (second "hello")
        let range = sim.selection.primary();
        assert_eq!(range.from(), 12);
        assert_eq!(range.to(), 17);
    }

    #[test]
    fn test_search_word_under_cursor_middle() {
        let mut sim: HelixSimulator<NormalMode> =
            HelixSimulator::new("hello world hello".to_string());

        // Move cursor to middle of "hello"
        sim.selection = Selection::point(2);

        search_word_under_cursor(&mut sim).unwrap();

        // Should still find the word and jump to next occurrence
        assert!(sim.search_state.has_pattern());
    }

    #[test]
    fn test_search_selection() {
        let mut sim: HelixSimulator<NormalMode> =
            HelixSimulator::new("foo bar foo baz foo".to_string());

        // Select "foo"
        sim.selection = Selection::single(0, 3);

        search_selection(&mut sim).unwrap();

        // Should have pattern set
        assert!(sim.search_state.has_pattern());

        // Should have found next match
        let range = sim.selection.primary();
        assert_eq!(range.from(), 8);
        assert_eq!(range.to(), 11);
    }

    #[test]
    fn test_goto_next_match() {
        let mut sim: HelixSimulator<NormalMode> = HelixSimulator::new("abc abc abc".to_string());

        // Set pattern manually
        sim.search_state
            .set_pattern("abc", SearchDirection::Forward)
            .unwrap();

        // Position at first match
        sim.selection = Selection::point(0);

        // Go to next match
        goto_next_match(&mut sim).unwrap();

        let range = sim.selection.primary();
        assert_eq!(range.from(), 4);
        assert_eq!(range.to(), 7);

        // Go to next again
        goto_next_match(&mut sim).unwrap();

        let range = sim.selection.primary();
        assert_eq!(range.from(), 8);
        assert_eq!(range.to(), 11);
    }

    #[test]
    fn test_goto_prev_match() {
        let mut sim: HelixSimulator<NormalMode> = HelixSimulator::new("abc abc abc".to_string());

        // Set pattern
        sim.search_state
            .set_pattern("abc", SearchDirection::Forward)
            .unwrap();

        // Position at last match
        sim.selection = Selection::point(10);

        // Go to previous match
        goto_prev_match(&mut sim).unwrap();

        let range = sim.selection.primary();
        assert_eq!(range.from(), 4);
        assert_eq!(range.to(), 7);
    }

    #[test]
    fn test_search_wrap_around() {
        let mut sim: HelixSimulator<NormalMode> = HelixSimulator::new("abc xyz abc".to_string());

        sim.search_state
            .set_pattern("abc", SearchDirection::Forward)
            .unwrap();

        // Position past last match
        sim.selection = Selection::point(10);

        // Should wrap around to first match
        goto_next_match(&mut sim).unwrap();

        let range = sim.selection.primary();
        assert_eq!(range.from(), 0);
        assert_eq!(range.to(), 3);
    }

    #[test]
    fn test_no_pattern_no_op() {
        let mut sim: HelixSimulator<NormalMode> = HelixSimulator::new("hello world".to_string());

        // No pattern set, should be no-op
        let result = goto_next_match(&mut sim);
        assert!(result.is_ok());

        // Selection unchanged
        assert_eq!(sim.selection.primary().head, 0);
    }

    #[test]
    fn test_search_empty_selection_uses_word() {
        let mut sim: HelixSimulator<NormalMode> = HelixSimulator::new("test test test".to_string());

        // Empty selection at "test"
        sim.selection = Selection::point(0);

        search_selection(&mut sim).unwrap();

        // Should behave like word search
        assert!(sim.search_state.has_pattern());
    }

    #[test]
    fn test_search_forward() {
        let mut sim: HelixSimulator<NormalMode> =
            HelixSimulator::new("hello world hello".to_string());

        // Set pattern first
        sim.search_state
            .set_pattern("hello", SearchDirection::Forward)
            .unwrap();

        // Call search_forward
        search_forward(&mut sim).unwrap();

        // Should find the next match (second "hello")
        let range = sim.selection.primary();
        assert_eq!(range.from(), 12);
        assert_eq!(range.to(), 17);
    }

    #[test]
    fn test_search_forward_resets_current_match() {
        let mut sim: HelixSimulator<NormalMode> = HelixSimulator::new("abc abc abc".to_string());

        sim.search_state
            .set_pattern("abc", SearchDirection::Forward)
            .unwrap();

        // Set a current match
        sim.search_state.set_current_match(Some(2));

        // search_forward should reset current match
        search_forward(&mut sim).unwrap();

        // Should still find a match
        assert!(sim.search_state.current_match().is_some());
    }

    #[test]
    fn test_search_backward() {
        let mut sim: HelixSimulator<NormalMode> =
            HelixSimulator::new("hello world hello".to_string());

        // Position at end (past the last "hello")
        sim.selection = Selection::point(17);

        // Set pattern
        sim.search_state
            .set_pattern("hello", SearchDirection::Backward)
            .unwrap();

        // Call search_backward
        search_backward(&mut sim).unwrap();

        // At position 17, the match "hello" at 12..17 ends exactly at 17.
        // Since position 17 is the exclusive end of the match, find_prev uses
        // strict inequality (range.end < pos), so 17 < 17 is false.
        // This means the match at 12..17 is excluded, and we get the previous match.
        let range = sim.selection.primary();
        assert_eq!(range.from(), 0);
        assert_eq!(range.to(), 5);
    }

    #[test]
    fn test_search_backward_resets_current_match() {
        let mut sim: HelixSimulator<NormalMode> = HelixSimulator::new("abc abc abc".to_string());

        // Position at end
        sim.selection = Selection::point(11);

        sim.search_state
            .set_pattern("abc", SearchDirection::Backward)
            .unwrap();

        // Set a current match
        sim.search_state.set_current_match(Some(0));

        // search_backward should reset current match
        search_backward(&mut sim).unwrap();

        // Should still find a match
        assert!(sim.search_state.current_match().is_some());
    }

    #[test]
    fn test_search_forward_no_pattern() {
        let mut sim: HelixSimulator<NormalMode> = HelixSimulator::new("hello world".to_string());

        // No pattern set
        let result = search_forward(&mut sim);

        // Should succeed (no-op)
        assert!(result.is_ok());
    }

    #[test]
    fn test_search_backward_no_pattern() {
        let mut sim: HelixSimulator<NormalMode> = HelixSimulator::new("hello world".to_string());

        // No pattern set
        let result = search_backward(&mut sim);

        // Should succeed (no-op)
        assert!(result.is_ok());
    }

    #[test]
    fn test_search_with_unicode() {
        let mut sim: HelixSimulator<NormalMode> =
            HelixSimulator::new("hello \u{1F600} world \u{1F600} end".to_string());

        // Set pattern for emoji
        sim.search_state
            .set_pattern("\u{1F600}", SearchDirection::Forward)
            .unwrap();

        search_forward(&mut sim).unwrap();

        // Should find the first emoji
        assert!(sim.search_state.has_pattern());
        assert!(sim.search_state.current_match().is_some());
    }

    #[test]
    fn test_document_version_caching() {
        let mut sim: HelixSimulator<NormalMode> = HelixSimulator::new("abc abc abc".to_string());

        sim.search_state
            .set_pattern("abc", SearchDirection::Forward)
            .unwrap();

        // First search populates cache
        search_next_match(&mut sim).unwrap();
        let first_match = sim.selection.primary().from();

        // Second search should use cached matches
        search_next_match(&mut sim).unwrap();
        let second_match = sim.selection.primary().from();

        // Should navigate through different matches
        assert_ne!(first_match, second_match);
    }

    // ========== Helper function tests ==========

    #[test]
    fn test_is_word_char() {
        // Alphabetic characters
        assert!(is_word_char('a'));
        assert!(is_word_char('z'));
        assert!(is_word_char('A'));
        assert!(is_word_char('Z'));

        // Numeric characters
        assert!(is_word_char('0'));
        assert!(is_word_char('9'));

        // Underscore
        assert!(is_word_char('_'));

        // Non-word characters
        assert!(!is_word_char(' '));
        assert!(!is_word_char('-'));
        assert!(!is_word_char('.'));
        assert!(!is_word_char('\n'));
        assert!(!is_word_char('\t'));
        assert!(!is_word_char('!'));
        assert!(!is_word_char('@'));
    }

    #[test]
    fn test_extract_word_at_cursor_basic() {
        let rope = helix_core::Rope::from("hello world");
        let slice = rope.slice(..);

        // Cursor at start of "hello"
        let result = extract_word_at_cursor(slice, 0);
        assert_eq!(result, Some((0, 5)));

        // Cursor at middle of "hello"
        let result = extract_word_at_cursor(slice, 2);
        assert_eq!(result, Some((0, 5)));

        // Cursor at end of "hello"
        let result = extract_word_at_cursor(slice, 4);
        assert_eq!(result, Some((0, 5)));

        // Cursor at start of "world"
        let result = extract_word_at_cursor(slice, 6);
        assert_eq!(result, Some((6, 11)));
    }

    #[test]
    fn test_extract_word_at_cursor_underscore() {
        let rope = helix_core::Rope::from("my_variable_name foo");
        let slice = rope.slice(..);

        // Cursor at start
        let result = extract_word_at_cursor(slice, 0);
        assert_eq!(result, Some((0, 16)));

        // Cursor at first underscore
        let result = extract_word_at_cursor(slice, 2);
        assert_eq!(result, Some((0, 16)));

        // Cursor at middle
        let result = extract_word_at_cursor(slice, 8);
        assert_eq!(result, Some((0, 16)));

        // Cursor at last char before space
        let result = extract_word_at_cursor(slice, 15);
        assert_eq!(result, Some((0, 16)));
    }

    #[test]
    fn test_extract_word_at_cursor_start_of_doc() {
        let rope = helix_core::Rope::from("word");
        let slice = rope.slice(..);

        // Cursor at position 0
        let result = extract_word_at_cursor(slice, 0);
        assert_eq!(result, Some((0, 4)));
    }

    #[test]
    fn test_extract_word_at_cursor_end_of_doc() {
        let rope = helix_core::Rope::from("word");
        let slice = rope.slice(..);

        // Cursor at last character
        let result = extract_word_at_cursor(slice, 3);
        assert_eq!(result, Some((0, 4)));

        // Cursor past end of document
        let result = extract_word_at_cursor(slice, 4);
        assert_eq!(result, None);
    }

    #[test]
    fn test_extract_word_at_cursor_on_whitespace() {
        let rope = helix_core::Rope::from("hello world");
        let slice = rope.slice(..);

        // Cursor on space
        let result = extract_word_at_cursor(slice, 5);
        assert_eq!(result, None);
    }

    #[test]
    fn test_extract_word_at_cursor_single_char() {
        let rope = helix_core::Rope::from("a b c");
        let slice = rope.slice(..);

        // Single char word at start
        let result = extract_word_at_cursor(slice, 0);
        assert_eq!(result, Some((0, 1)));

        // Single char word in middle
        let result = extract_word_at_cursor(slice, 2);
        assert_eq!(result, Some((2, 3)));

        // Single char word at end
        let result = extract_word_at_cursor(slice, 4);
        assert_eq!(result, Some((4, 5)));
    }

    #[test]
    fn test_extract_word_at_cursor_empty_doc() {
        let rope = helix_core::Rope::from("");
        let slice = rope.slice(..);

        let result = extract_word_at_cursor(slice, 0);
        assert_eq!(result, None);
    }

    #[test]
    fn test_extract_word_at_cursor_with_numbers() {
        let rope = helix_core::Rope::from("var123 test2go 456");
        let slice = rope.slice(..);

        // Word starting with letter, containing numbers
        let result = extract_word_at_cursor(slice, 0);
        assert_eq!(result, Some((0, 6)));

        // Cursor on digit within word
        let result = extract_word_at_cursor(slice, 4);
        assert_eq!(result, Some((0, 6)));

        // Word with numbers in middle
        let result = extract_word_at_cursor(slice, 7);
        assert_eq!(result, Some((7, 14)));

        // Pure number
        let result = extract_word_at_cursor(slice, 15);
        assert_eq!(result, Some((15, 18)));
    }

    #[test]
    fn test_extract_word_at_cursor_punctuation() {
        let rope = helix_core::Rope::from("hello, world! test");
        let slice = rope.slice(..);

        // Cursor on comma
        let result = extract_word_at_cursor(slice, 5);
        assert_eq!(result, None);

        // Cursor on exclamation mark
        let result = extract_word_at_cursor(slice, 12);
        assert_eq!(result, None);
    }

    #[test]
    fn test_extract_word_at_cursor_newlines() {
        let rope = helix_core::Rope::from("hello\nworld");
        let slice = rope.slice(..);

        // Word before newline
        let result = extract_word_at_cursor(slice, 0);
        assert_eq!(result, Some((0, 5)));

        // Cursor on newline
        let result = extract_word_at_cursor(slice, 5);
        assert_eq!(result, None);

        // Word after newline
        let result = extract_word_at_cursor(slice, 6);
        assert_eq!(result, Some((6, 11)));
    }

    #[test]
    fn test_extract_word_at_cursor_tabs() {
        let rope = helix_core::Rope::from("hello\tworld");
        let slice = rope.slice(..);

        // Word before tab
        let result = extract_word_at_cursor(slice, 0);
        assert_eq!(result, Some((0, 5)));

        // Cursor on tab
        let result = extract_word_at_cursor(slice, 5);
        assert_eq!(result, None);

        // Word after tab
        let result = extract_word_at_cursor(slice, 6);
        assert_eq!(result, Some((6, 11)));
    }

    #[test]
    fn test_extract_word_at_cursor_unicode() {
        let rope = helix_core::Rope::from("cafe resume naive");
        let slice = rope.slice(..);

        // ASCII word
        let result = extract_word_at_cursor(slice, 0);
        assert_eq!(result, Some((0, 4)));

        // Unicode is_alphanumeric includes accented chars if present
        let result = extract_word_at_cursor(slice, 5);
        assert_eq!(result, Some((5, 11)));
    }

    #[test]
    fn test_is_word_char_unicode() {
        // Unicode letters are alphanumeric
        assert!(is_word_char('e')); // ASCII e with accent in some representations
        assert!(is_word_char('a'));

        // CJK characters are alphanumeric per Unicode
        // Note: This tests the actual behavior of is_alphanumeric()
        assert!(!is_word_char(' '));
        assert!(!is_word_char('-'));
    }
}
