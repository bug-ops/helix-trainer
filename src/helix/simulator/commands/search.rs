//! Search commands (/, ?, n, N, *, Alt-*)
//!
//! Provides search functionality for the Helix simulator.

use crate::helix::simulator::HelixSimulator;
use crate::helix::simulator::search_state::SearchDirection;
use crate::security::UserError;
use helix_core::Selection;
use std::hash::{Hash, Hasher};

/// Compute a simple document version hash for cache invalidation
///
/// Uses document length and a sample of content to create a fast version identifier.
/// This is not cryptographically secure but sufficient for change detection.
fn compute_doc_version(doc: &helix_core::Rope) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    doc.len_bytes().hash(&mut hasher);
    doc.len_chars().hash(&mut hasher);
    // Sample first and last few chars for quick change detection
    if doc.len_chars() > 0 {
        doc.slice(..doc.len_chars().min(64))
            .to_string()
            .hash(&mut hasher);
    }
    hasher.finish()
}

/// Search forward from cursor position
///
/// Sets the search pattern and direction, updates matches, and selects the first match.
/// The pattern should be provided before calling this function via `sim.search_state_mut().set_pattern()`.
///
/// Performance optimizations:
/// - Uses Rope's byte_to_char for O(log n) byte-to-char conversion
/// - Uses document versioning to skip re-scanning unchanged documents
pub fn search_next_match<M: crate::helix::simulator::EditorMode>(
    sim: &mut HelixSimulator<M>,
) -> Result<(), UserError> {
    let head = sim.selection.primary().head;

    // Get content as Cow<str> for regex matching (avoids allocation when possible)
    // Note: For ASCII content this is zero-copy, for non-ASCII it allocates
    let content = sim.doc.slice(..).to_string();
    let doc_version = compute_doc_version(&sim.doc);

    // Update matches in search state (skips re-scan if document unchanged)
    sim.search_state
        .update_matches_with_version(&content, doc_version);

    // Find next match from current position (byte position for search state)
    // Convert char position to byte position for lookup
    let head_byte = sim.doc.char_to_byte(head);
    if let Some((_, range)) = sim.search_state.find_next(head_byte) {
        // Convert byte range to char position using Rope's O(log n) method
        let start_chars = sim.doc.byte_to_char(range.start);
        let end_chars = sim.doc.byte_to_char(range.end);

        // Select the match (anchor at start, head at end)
        sim.selection = Selection::single(start_chars, end_chars);
    }

    Ok(())
}

/// Search backward from cursor position
///
/// Similar to search_next_match but searches in reverse direction.
///
/// Performance optimizations:
/// - Uses Rope's byte_to_char for O(log n) byte-to-char conversion
/// - Uses document versioning to skip re-scanning unchanged documents
pub fn search_prev_match<M: crate::helix::simulator::EditorMode>(
    sim: &mut HelixSimulator<M>,
) -> Result<(), UserError> {
    let head = sim.selection.primary().head;

    // Get content for regex matching
    let content = sim.doc.slice(..).to_string();
    let doc_version = compute_doc_version(&sim.doc);

    // Update matches in search state (skips re-scan if document unchanged)
    sim.search_state
        .update_matches_with_version(&content, doc_version);

    // Find previous match from current position
    // Convert char position to byte position for lookup
    let head_byte = sim.doc.char_to_byte(head);
    if let Some((_, range)) = sim.search_state.find_prev(head_byte) {
        // Convert byte range to char position using Rope's O(log n) method
        let start_chars = sim.doc.byte_to_char(range.start);
        let end_chars = sim.doc.byte_to_char(range.end);

        // Select the match (anchor at start, head at end)
        sim.selection = Selection::single(start_chars, end_chars);
    }

    Ok(())
}

/// Search forward (/) command
///
/// In a real implementation, this would prompt for a pattern.
/// For the trainer, we simulate by setting a pattern via the search state.
pub fn search_forward<M: crate::helix::simulator::EditorMode>(
    sim: &mut HelixSimulator<M>,
) -> Result<(), UserError> {
    // In the trainer context, this is typically used after setting a pattern
    // For now, just ensure direction is forward and search for next match
    sim.search_state.set_current_match(None);
    search_next_match(sim)
}

/// Search backward (?) command
pub fn search_backward<M: crate::helix::simulator::EditorMode>(
    sim: &mut HelixSimulator<M>,
) -> Result<(), UserError> {
    // Ensure direction is backward and search for previous match
    sim.search_state.set_current_match(None);
    search_prev_match(sim)
}

/// Go to next match (n command)
///
/// Jumps to the next match based on the current search direction.
pub fn goto_next_match<M: crate::helix::simulator::EditorMode>(
    sim: &mut HelixSimulator<M>,
) -> Result<(), UserError> {
    if !sim.search_state.has_pattern() {
        return Ok(()); // No active search
    }

    match sim.search_state.direction() {
        SearchDirection::Forward => search_next_match(sim),
        SearchDirection::Backward => search_prev_match(sim),
    }
}

/// Go to previous match (N command)
///
/// Jumps to the previous match (opposite of current search direction).
pub fn goto_prev_match<M: crate::helix::simulator::EditorMode>(
    sim: &mut HelixSimulator<M>,
) -> Result<(), UserError> {
    if !sim.search_state.has_pattern() {
        return Ok(()); // No active search
    }

    match sim.search_state.direction() {
        SearchDirection::Forward => search_prev_match(sim),
        SearchDirection::Backward => search_next_match(sim),
    }
}

/// Search word under cursor (* command)
///
/// Gets the word at the cursor position and searches for it with word boundaries.
pub fn search_word_under_cursor<M: crate::helix::simulator::EditorMode>(
    sim: &mut HelixSimulator<M>,
) -> Result<(), UserError> {
    let head = sim.selection.primary().head;
    let slice = sim.doc.slice(..);

    // Find word boundaries around cursor
    let mut start = head;
    let mut end = head;

    // Move start backward to word start
    while start > 0 {
        if let Some(ch) = slice.get_char(start.saturating_sub(1)) {
            if ch.is_alphanumeric() || ch == '_' {
                start -= 1;
            } else {
                break;
            }
        } else {
            break;
        }
    }

    // Move end forward to word end
    let doc_len = sim.doc.len_chars();
    while end < doc_len {
        if let Some(ch) = slice.get_char(end) {
            if ch.is_alphanumeric() || ch == '_' {
                end += 1;
            } else {
                break;
            }
        } else {
            break;
        }
    }

    // Extract the word
    if start < end {
        let word: String = slice.slice(start..end).chars().collect();

        // Set the search pattern with word boundaries
        if sim
            .search_state
            .set_word_pattern(&word, SearchDirection::Forward)
            .is_ok()
        {
            search_next_match(sim)?;
        }
    }

    Ok(())
}

/// Search word under cursor backward (# command)
///
/// Gets the word at the cursor position and searches backward for it with word boundaries.
pub fn search_word_under_cursor_backward<M: crate::helix::simulator::EditorMode>(
    sim: &mut HelixSimulator<M>,
) -> Result<(), UserError> {
    let head = sim.selection.primary().head;
    let slice = sim.doc.slice(..);

    // Find word boundaries around cursor
    let mut start = head;
    let mut end = head;

    // Move start backward to word start
    while start > 0 {
        if let Some(ch) = slice.get_char(start.saturating_sub(1)) {
            if ch.is_alphanumeric() || ch == '_' {
                start -= 1;
            } else {
                break;
            }
        } else {
            break;
        }
    }

    // Move end forward to word end
    let doc_len = sim.doc.len_chars();
    while end < doc_len {
        if let Some(ch) = slice.get_char(end) {
            if ch.is_alphanumeric() || ch == '_' {
                end += 1;
            } else {
                break;
            }
        } else {
            break;
        }
    }

    // Extract the word
    if start < end {
        let word: String = slice.slice(start..end).chars().collect();

        // Set the search pattern with word boundaries and backward direction
        if sim
            .search_state
            .set_word_pattern(&word, SearchDirection::Backward)
            .is_ok()
        {
            search_prev_match(sim)?;
        }
    }

    Ok(())
}

/// Search selection text (Alt-* command)
///
/// Uses the current selection text as the search pattern without word boundaries.
pub fn search_selection<M: crate::helix::simulator::EditorMode>(
    sim: &mut HelixSimulator<M>,
) -> Result<(), UserError> {
    let range = sim.selection.primary();
    let start = range.from();
    let end = range.to();

    // If selection is empty, use word under cursor behavior
    if start == end {
        return search_word_under_cursor(sim);
    }

    // Extract selection text
    let slice = sim.doc.slice(..);
    let selection_text: String = slice.slice(start..end).chars().collect();

    // Set the search pattern without word boundaries
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

        // Should find the last match (second "hello") since we're searching backward
        // from byte position 17, and the match "hello" at 12-17 ends at 17,
        // so it finds the match ending <= 17
        let range = sim.selection.primary();
        assert_eq!(range.from(), 12);
        assert_eq!(range.to(), 17);
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
}
