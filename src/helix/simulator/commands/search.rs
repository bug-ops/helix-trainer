//! Search commands (/, ?, n, N, *, Alt-*)
//!
//! As in Helix, `n` always searches forward and `N` always backward, whichever
//! of `/` or `?` set the pattern. `*` turns the current selection into the
//! pattern without moving it.

use crate::helix::error::SimulatorError;
use crate::helix::simulator::HelixSimulator;
use crate::helix::simulator::search_state::SearchDirection;
use helix_core::chars::char_is_word;
use helix_core::graphemes::{
    ensure_grapheme_boundary_next, ensure_grapheme_boundary_prev, next_grapheme_boundary,
};
use helix_core::regex;
use helix_core::ropey::RopeSlice;
use helix_core::{Range, Rope, Selection};

/// Select the found `range` (byte offsets) as Helix's `search_impl` does: the
/// match takes the direction of the primary range and replaces it, or in
/// Select mode is added as the new primary. A zero-width match is widened to
/// one grapheme and a match ending at the document start is skipped. The view
/// is centered on the match when it was outside the view.
fn place_match<M: crate::helix::simulator::EditorMode>(
    sim: &mut HelixSimulator<M>,
    range: std::ops::Range<usize>,
) {
    let text = sim.doc.slice(..);
    let start = text.byte_to_char(range.start);
    let end = text.byte_to_char(range.end);
    if end == 0 {
        return;
    }
    let end = if start == end {
        next_grapheme_boundary(text, end)
    } else {
        end
    };
    let primary = sim.selection.primary();
    let found = Range::new(start, end).with_direction(primary.direction());
    sim.selection = if sim.select_mode {
        sim.selection.clone().push(found)
    } else {
        sim.selection
            .clone()
            .replace(sim.selection.primary_index(), found)
    };
    super::view::center_if_outside_view(sim);
}

/// Port of Helix's `search_impl`: look for the pattern from the primary range
/// in `direction` (forward from the grapheme after its end, backward from the
/// grapheme boundary before its start), wrapping around the document.
fn search_impl<M: crate::helix::simulator::EditorMode>(
    sim: &mut HelixSimulator<M>,
    direction: SearchDirection,
) {
    let text = sim.doc.slice(..);
    let primary = sim.selection.primary();
    let from = match direction {
        SearchDirection::Forward => {
            let to = if primary.is_empty() {
                next_grapheme_boundary(text, primary.head)
            } else {
                primary.to()
            };
            ensure_grapheme_boundary_next(text, to)
        }
        SearchDirection::Backward => ensure_grapheme_boundary_prev(text, primary.from()),
    };
    if let Some(range) = sim
        .search_state
        .find(text, text.char_to_byte(from), direction)
    {
        place_match(sim, range);
    }
}

/// Search forward (/) command
///
/// # Errors
///
/// Never fails: always returns `Ok(())`. The `Result` keeps the signature uniform with the
/// fallible commands that share the registry's function type.
pub fn search_forward<M: crate::helix::simulator::EditorMode>(
    sim: &mut HelixSimulator<M>,
) -> Result<(), SimulatorError> {
    search_impl(sim, SearchDirection::Forward);
    Ok(())
}

/// Search forward for `pattern` (`/ <pattern>` prompt invocation)
///
/// Stores the pattern so `n`/`N` continue from it, then selects the next
/// match after the cursor, wrapping at the end of the buffer. An empty
/// pattern is a no-op.
///
/// # Errors
///
/// Returns a [`SimulatorError`] if `pattern` is not a valid regex or exceeds the
/// maximum pattern length; the selection is left unchanged.
pub(crate) fn search_pattern_forward<M: crate::helix::simulator::EditorMode>(
    sim: &mut HelixSimulator<M>,
    pattern: &str,
) -> Result<(), SimulatorError> {
    if pattern.is_empty() {
        return Ok(());
    }
    sim.search_state
        .set_pattern(pattern, SearchDirection::Forward)?;
    search_forward(sim)
}

/// Search backward for `pattern` (`? <pattern>` prompt invocation)
///
/// Stores the pattern so `n`/`N` continue from it in the backward direction,
/// then selects the previous match before the cursor, wrapping at the start
/// of the buffer. An empty pattern is a no-op.
///
/// # Errors
///
/// Returns a [`SimulatorError`] if `pattern` is not a valid regex or exceeds the
/// maximum pattern length; the selection is left unchanged.
pub(crate) fn search_pattern_backward<M: crate::helix::simulator::EditorMode>(
    sim: &mut HelixSimulator<M>,
    pattern: &str,
) -> Result<(), SimulatorError> {
    if pattern.is_empty() {
        return Ok(());
    }
    sim.search_state
        .set_pattern(pattern, SearchDirection::Backward)?;
    search_backward(sim)
}

/// Search backward (?) command
///
/// # Errors
///
/// Never fails: always returns `Ok(())`. The `Result` keeps the signature uniform with the
/// fallible commands that share the registry's function type.
pub fn search_backward<M: crate::helix::simulator::EditorMode>(
    sim: &mut HelixSimulator<M>,
) -> Result<(), SimulatorError> {
    search_impl(sim, SearchDirection::Backward);
    Ok(())
}

/// Go to next match (n command)
///
/// # Errors
///
/// Never fails: always returns `Ok(())`. The `Result` keeps the signature uniform with the
/// fallible commands that share the registry's function type.
pub(crate) fn goto_next_match<M: crate::helix::simulator::EditorMode>(
    sim: &mut HelixSimulator<M>,
) -> Result<(), SimulatorError> {
    if !sim.search_state.has_pattern() {
        return Ok(());
    }

    search_forward(sim)
}

/// Go to previous match (N command)
///
/// # Errors
///
/// Never fails: always returns `Ok(())`. The `Result` keeps the signature uniform with the
/// fallible commands that share the registry's function type.
pub(crate) fn goto_prev_match<M: crate::helix::simulator::EditorMode>(
    sim: &mut HelixSimulator<M>,
) -> Result<(), SimulatorError> {
    if !sim.search_state.has_pattern() {
        return Ok(());
    }

    search_backward(sim)
}

fn is_at_word_start(text: RopeSlice, index: usize) -> bool {
    index < text.len_chars()
        && char_is_word(text.char(index))
        && (index == 0 || !char_is_word(text.char(index - 1)))
}

fn is_at_word_end(text: RopeSlice, index: usize) -> bool {
    index != 0
        && index < text.len_chars()
        && char_is_word(text.char(index - 1))
        && !char_is_word(text.char(index))
}

/// Whether the selection pattern gets `\b` where a range sits on a word edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WordBoundaries {
    /// Add `\b` at each end of a range that starts or ends on a word edge.
    Detect,
    /// Use the escaped text as it is.
    Ignore,
}

/// The pattern `*` and `Alt-*` search for: every range's text escaped (with
/// `\b` where `boundaries` asks for it), distinct alternatives joined by `|`.
///
/// A point cursor stands for the one character under it, as in Helix.
fn selection_pattern(doc: &Rope, selection: &Selection, boundaries: WordBoundaries) -> String {
    let text = doc.slice(..);
    let mut alternatives: Vec<String> = Vec::new();
    for range in selection.ranges() {
        let from = range.from();
        let to = if range.is_empty() {
            (from + 1).min(text.len_chars())
        } else {
            range.to()
        };
        let detect = boundaries == WordBoundaries::Detect;
        let prefix = if detect && is_at_word_start(text, from) {
            "\\b"
        } else {
            ""
        };
        let suffix = if detect && is_at_word_end(text, to) {
            "\\b"
        } else {
            ""
        };
        let escaped = regex::escape(&text.slice(from..to).to_string());
        let alternative = format!("{prefix}{escaped}{suffix}");
        if !alternatives.contains(&alternative) {
            alternatives.push(alternative);
        }
    }
    alternatives.join("|")
}

/// Use the selection as the search pattern (`*`, Helix
/// `search_selection_detect_word_boundaries`)
///
/// The escaped selection becomes the pattern, with `\b` at each end that
/// sits on a word boundary. The selection does not move; `n` jumps to the
/// next match. An empty pattern (a cursor past the last character) is ignored.
///
/// # Errors
///
/// Returns a [`SimulatorError`] if the pattern exceeds the maximum pattern
/// length; the previous pattern is kept.
pub(crate) fn search_selection_detect_word_boundaries<M: crate::helix::simulator::EditorMode>(
    sim: &mut HelixSimulator<M>,
) -> Result<(), SimulatorError> {
    set_selection_pattern(sim, WordBoundaries::Detect)
}

fn set_selection_pattern<M: crate::helix::simulator::EditorMode>(
    sim: &mut HelixSimulator<M>,
    boundaries: WordBoundaries,
) -> Result<(), SimulatorError> {
    let pattern = selection_pattern(&sim.doc, &sim.selection, boundaries);
    if pattern.is_empty() {
        return Ok(());
    }
    sim.search_state
        .set_pattern(&pattern, SearchDirection::Forward)
}

/// Search selection text (`Alt-*`, Helix `search_selection`)
///
/// The escaped selection becomes the pattern with no word boundaries; a point
/// cursor stands for the one character under it. The selection does not move.
///
/// # Errors
///
/// Returns a [`SimulatorError`] if the pattern exceeds the maximum pattern
/// length; the previous pattern is kept.
pub fn search_selection<M: crate::helix::simulator::EditorMode>(
    sim: &mut HelixSimulator<M>,
) -> Result<(), SimulatorError> {
    set_selection_pattern(sim, WordBoundaries::Ignore)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::helix::simulator::NormalMode;

    #[test]
    fn star_uses_the_selection_with_word_boundaries_and_keeps_it() {
        let mut sim: HelixSimulator<NormalMode> =
            HelixSimulator::new("hello world hello".to_string());
        sim.selection = Selection::single(0, 5);

        search_selection_detect_word_boundaries(&mut sim).unwrap();

        assert_eq!(sim.search_state.pattern(), Some(r"\bhello\b"));
        assert_eq!(sim.selection, Selection::single(0, 5));
    }

    #[test]
    fn star_adds_boundaries_only_where_the_selection_touches_a_word_edge() {
        let mut sim: HelixSimulator<NormalMode> = HelixSimulator::new("fn add(a)".to_string());

        sim.selection = Selection::single(0, 3); // "fn "
        search_selection_detect_word_boundaries(&mut sim).unwrap();
        assert_eq!(sim.search_state.pattern(), Some(r"\bfn "));

        sim.selection = Selection::single(4, 6); // "dd"
        search_selection_detect_word_boundaries(&mut sim).unwrap();
        assert_eq!(sim.search_state.pattern(), Some(r"dd\b"));

        sim.selection = Selection::single(6, 8); // "(a"
        search_selection_detect_word_boundaries(&mut sim).unwrap();
        assert_eq!(sim.search_state.pattern(), Some(r"\(a\b"));
    }

    #[test]
    fn star_on_a_point_cursor_searches_the_character_under_it() {
        let mut sim: HelixSimulator<NormalMode> = HelixSimulator::new("fn fn".to_string());
        sim.selection = Selection::point(0);

        search_selection_detect_word_boundaries(&mut sim).unwrap();

        assert_eq!(sim.search_state.pattern(), Some(r"\bf"));
        assert_eq!(sim.selection, Selection::point(0));
    }

    #[test]
    fn star_joins_distinct_ranges_and_deduplicates_equal_ones() {
        let mut sim: HelixSimulator<NormalMode> = HelixSimulator::new("ab cd ab!".to_string());
        let ranges = vec![Range::new(0, 2), Range::new(3, 5), Range::new(6, 8)];
        sim.selection = Selection::new(ranges.into(), 0);

        search_selection_detect_word_boundaries(&mut sim).unwrap();

        assert_eq!(sim.search_state.pattern(), Some(r"\bab\b|\bcd\b"));
    }

    #[test]
    fn n_moves_forward_and_wraps_after_a_backward_search() {
        let mut sim: HelixSimulator<NormalMode> = HelixSimulator::new("a-a-a".to_string());
        search_pattern_backward(&mut sim, "a").unwrap();
        assert_eq!(sim.selection, Selection::single(4, 5));

        goto_next_match(&mut sim).unwrap();
        assert_eq!(sim.selection, Selection::single(0, 1));

        goto_prev_match(&mut sim).unwrap();
        assert_eq!(sim.selection, Selection::single(4, 5));
    }

    #[test]
    fn n_and_big_n_ignore_the_direction_of_the_last_search() {
        let mut forward: HelixSimulator<NormalMode> = HelixSimulator::new("a-a-a".to_string());
        search_pattern_forward(&mut forward, "a").unwrap();
        let mut backward: HelixSimulator<NormalMode> = HelixSimulator::new("a-a-a".to_string());
        search_pattern_backward(&mut backward, "a").unwrap();
        for sim in [&mut forward, &mut backward] {
            sim.selection = Selection::single(2, 3);
            goto_next_match(sim).unwrap();
            assert_eq!(sim.selection, Selection::single(4, 5));
            sim.selection = Selection::single(2, 3);
            goto_prev_match(sim).unwrap();
            assert_eq!(sim.selection, Selection::single(0, 1));
        }
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

        // Selection should remain unchanged (already set at 0..3)
        let range = sim.selection.primary();
        assert_eq!(range.from(), 0);
        assert_eq!(range.to(), 3);
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
    fn alt_star_on_a_point_searches_the_escaped_character_under_it() {
        let mut sim: HelixSimulator<NormalMode> = HelixSimulator::new("a.b a.b".to_string());
        sim.selection = Selection::point(1);

        search_selection(&mut sim).unwrap();

        assert_eq!(sim.search_state.pattern(), Some(r"\."));
        assert_eq!(sim.selection, Selection::point(1));
    }

    #[test]
    fn alt_star_adds_no_word_boundaries() {
        let mut sim: HelixSimulator<NormalMode> =
            HelixSimulator::new("hello world hello".to_string());
        sim.selection = Selection::single(0, 5);

        search_selection(&mut sim).unwrap();

        assert_eq!(sim.search_state.pattern(), Some("hello"));
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

        // A match ending exactly at the search start is found, as in Helix.
        let range = sim.selection.primary();
        assert_eq!(range.from(), 12);
        assert_eq!(range.to(), 17);
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
        assert_eq!(sim.selection.primary().from(), 6);
    }

    #[test]
    fn repeated_search_navigates_through_different_matches() {
        let mut sim: HelixSimulator<NormalMode> = HelixSimulator::new("abc abc abc".to_string());

        sim.search_state
            .set_pattern("abc", SearchDirection::Forward)
            .unwrap();

        search_forward(&mut sim).unwrap();
        let first_match = sim.selection.primary().from();
        search_forward(&mut sim).unwrap();
        let second_match = sim.selection.primary().from();

        assert_ne!(first_match, second_match);
    }

    // ========== Helper function tests ==========
}
