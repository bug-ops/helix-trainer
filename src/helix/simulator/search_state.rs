//! Search state management for Helix simulator
//!
//! Provides search pattern storage, direction tracking, and match caching
//! for search commands (/, ?, n, N, *, Alt-*).

use helix_core::regex::{self, Regex};
use std::ops::Range;

/// Maximum number of matches to collect (prevents unbounded memory usage)
const MAX_MATCHES: usize = 10_000;

/// Maximum pattern length (prevents regex DoS attacks)
const MAX_PATTERN_LENGTH: usize = 1000;

/// Search direction for forward or backward search
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SearchDirection {
    /// Search forward from cursor (/)
    #[default]
    Forward,
    /// Search backward from cursor (?)
    Backward,
}

/// Search state for the editor
///
/// Stores the current search pattern, direction, compiled regex,
/// and cached match positions for efficient navigation.
#[derive(Debug, Default)]
pub struct SearchState {
    /// The raw search pattern string
    pattern: Option<String>,
    /// Search direction (forward or backward)
    direction: SearchDirection,
    /// Compiled regex (cached for performance)
    regex: Option<Regex>,
    /// Cached match positions (start, end) in document (byte offsets)
    matches: Vec<Range<usize>>,
    /// Current match index (for n/N navigation)
    current_match: Option<usize>,
    /// Document version when matches were last computed (for cache invalidation)
    cached_doc_version: Option<u64>,
}

impl SearchState {
    /// Create a new empty search state
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the current search pattern
    pub fn pattern(&self) -> Option<&str> {
        self.pattern.as_deref()
    }

    /// Get the search direction
    pub fn direction(&self) -> SearchDirection {
        self.direction
    }

    /// Get the compiled regex
    pub fn regex(&self) -> Option<&Regex> {
        self.regex.as_ref()
    }

    /// Get cached matches
    pub fn matches(&self) -> &[Range<usize>] {
        &self.matches
    }

    /// Get current match index
    pub fn current_match(&self) -> Option<usize> {
        self.current_match
    }

    /// Set the current match index
    pub fn set_current_match(&mut self, index: Option<usize>) {
        self.current_match = index;
    }

    /// Set search pattern and direction
    ///
    /// Compiles the regex and clears cached matches.
    /// Returns an error if the pattern is invalid regex or exceeds length limit.
    pub fn set_pattern(
        &mut self,
        pattern: &str,
        direction: SearchDirection,
    ) -> Result<(), regex::Error> {
        // Security: limit pattern length to prevent regex DoS
        if pattern.len() > MAX_PATTERN_LENGTH {
            return Err(regex::Error::Syntax(format!(
                "pattern exceeds maximum length of {} characters",
                MAX_PATTERN_LENGTH
            )));
        }

        let regex = Regex::new(pattern)?;
        self.pattern = Some(pattern.to_string());
        self.direction = direction;
        self.regex = Some(regex);
        self.matches.clear();
        self.current_match = None;
        self.cached_doc_version = None; // Invalidate cache
        Ok(())
    }

    /// Set search pattern with word boundaries (for * command)
    ///
    /// Wraps the word in \b...\b for whole-word matching.
    pub fn set_word_pattern(
        &mut self,
        word: &str,
        direction: SearchDirection,
    ) -> Result<(), regex::Error> {
        let pattern = format!(r"\b{}\b", regex::escape(word));
        self.set_pattern(&pattern, direction)
    }

    /// Set search pattern without word boundaries (for Alt-* command)
    ///
    /// Uses the selection text as-is (escaped for regex safety).
    pub fn set_selection_pattern(
        &mut self,
        selection: &str,
        direction: SearchDirection,
    ) -> Result<(), regex::Error> {
        let pattern = regex::escape(selection);
        self.set_pattern(&pattern, direction)
    }

    /// Update cached matches from document content
    ///
    /// Scans the document and caches all match positions.
    /// Uses a document version to skip re-scanning when document is unchanged.
    /// Limits matches to MAX_MATCHES to prevent unbounded memory usage.
    pub fn update_matches_with_version(&mut self, content: &str, doc_version: u64) {
        // Skip re-scan if document hasn't changed since last update
        if self.cached_doc_version == Some(doc_version) && !self.matches.is_empty() {
            return;
        }

        self.matches.clear();
        if let Some(regex) = &self.regex {
            // Limit matches to prevent unbounded memory usage
            for m in regex.find_iter(content).take(MAX_MATCHES) {
                self.matches.push(m.start()..m.end());
            }
        }
        self.cached_doc_version = Some(doc_version);
    }

    /// Update cached matches from document content (legacy, no version tracking)
    ///
    /// Scans the document and caches all match positions.
    /// Prefer `update_matches_with_version` for better performance.
    pub fn update_matches(&mut self, content: &str) {
        self.matches.clear();
        if let Some(regex) = &self.regex {
            // Limit matches to prevent unbounded memory usage
            for m in regex.find_iter(content).take(MAX_MATCHES) {
                self.matches.push(m.start()..m.end());
            }
        }
        self.cached_doc_version = None; // No version tracking
    }

    /// Find the next match from the given position
    ///
    /// Returns the match index and range, wrapping around if needed.
    /// Uses binary search for O(log n) performance since matches are sorted.
    pub fn find_next(&mut self, pos: usize) -> Option<(usize, Range<usize>)> {
        if self.matches.is_empty() {
            return None;
        }

        // Binary search for the first match starting after pos
        // partition_point returns the index where all elements before satisfy the predicate
        let idx = self.matches.partition_point(|range| range.start <= pos);

        if idx < self.matches.len() {
            self.current_match = Some(idx);
            // Range is Copy, no need for clone()
            Some((idx, self.matches[idx].start..self.matches[idx].end))
        } else {
            // Wrap around to first match
            self.current_match = Some(0);
            Some((0, self.matches[0].start..self.matches[0].end))
        }
    }

    /// Find the previous match from the given position
    ///
    /// Returns the match index and range, wrapping around if needed.
    /// If the position is within a match, returns the match before that one.
    /// Uses binary search for O(log n) performance since matches are sorted.
    pub fn find_prev(&mut self, pos: usize) -> Option<(usize, Range<usize>)> {
        if self.matches.is_empty() {
            return None;
        }

        // Binary search for the last match ending strictly before pos
        // We want the rightmost match where range.end < pos
        // This ensures we skip the current match when cursor is at its end
        let idx = self.matches.partition_point(|range| range.end < pos);

        if idx > 0 {
            let i = idx - 1;
            self.current_match = Some(i);
            // Range is Copy, no need for clone()
            Some((i, self.matches[i].start..self.matches[i].end))
        } else {
            // Wrap around to last match
            let last = self.matches.len() - 1;
            self.current_match = Some(last);
            Some((last, self.matches[last].start..self.matches[last].end))
        }
    }

    /// Clear the search state
    pub fn clear(&mut self) {
        self.pattern = None;
        self.direction = SearchDirection::default();
        self.regex = None;
        self.matches.clear();
        self.current_match = None;
        self.cached_doc_version = None;
    }

    /// Check if there is an active search pattern
    pub fn has_pattern(&self) -> bool {
        self.pattern.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_state_new() {
        let state = SearchState::new();
        assert!(state.pattern().is_none());
        assert_eq!(state.direction(), SearchDirection::Forward);
        assert!(state.regex().is_none());
        assert!(state.matches().is_empty());
    }

    #[test]
    fn test_set_pattern_valid() {
        let mut state = SearchState::new();
        assert!(state.set_pattern("foo", SearchDirection::Forward).is_ok());
        assert_eq!(state.pattern(), Some("foo"));
        assert_eq!(state.direction(), SearchDirection::Forward);
        assert!(state.regex().is_some());
    }

    #[test]
    fn test_set_pattern_invalid_regex() {
        let mut state = SearchState::new();
        assert!(
            state
                .set_pattern("[invalid", SearchDirection::Forward)
                .is_err()
        );
    }

    #[test]
    fn test_set_word_pattern() {
        let mut state = SearchState::new();
        assert!(
            state
                .set_word_pattern("hello", SearchDirection::Forward)
                .is_ok()
        );
        assert!(state.pattern().unwrap().contains(r"\b"));
    }

    #[test]
    fn test_set_selection_pattern() {
        let mut state = SearchState::new();
        assert!(
            state
                .set_selection_pattern("foo.bar", SearchDirection::Forward)
                .is_ok()
        );
        // The dot should be escaped
        assert!(state.pattern().unwrap().contains(r"\."));
    }

    #[test]
    fn test_update_matches() {
        let mut state = SearchState::new();
        state.set_pattern("foo", SearchDirection::Forward).unwrap();
        state.update_matches("foo bar foo baz foo");
        assert_eq!(state.matches().len(), 3);
        assert_eq!(state.matches()[0], 0..3);
        assert_eq!(state.matches()[1], 8..11);
        assert_eq!(state.matches()[2], 16..19);
    }

    #[test]
    fn test_find_next() {
        let mut state = SearchState::new();
        state.set_pattern("foo", SearchDirection::Forward).unwrap();
        state.update_matches("foo bar foo baz foo");

        let (idx, range) = state.find_next(0).unwrap();
        assert_eq!(idx, 1);
        assert_eq!(range, 8..11);

        let (idx, range) = state.find_next(12).unwrap();
        assert_eq!(idx, 2);
        assert_eq!(range, 16..19);

        // Wrap around
        let (idx, range) = state.find_next(20).unwrap();
        assert_eq!(idx, 0);
        assert_eq!(range, 0..3);
    }

    #[test]
    fn test_find_prev() {
        let mut state = SearchState::new();
        state.set_pattern("foo", SearchDirection::Forward).unwrap();
        state.update_matches("foo bar foo baz foo");
        // Matches are at positions: 0..3, 8..11, 16..19

        // At position 19 (end of last match), find_prev returns PREVIOUS match (not current)
        // Match at 16..19 ends at 19, but 19 < 19 is false, so it's excluded
        let (idx, range) = state.find_prev(19).unwrap();
        assert_eq!(idx, 1);
        assert_eq!(range, 8..11);

        // At position 16 (start of last match), matches ending < 16 are 0..3 and 8..11
        let (idx, range) = state.find_prev(16).unwrap();
        assert_eq!(idx, 1);
        assert_eq!(range, 8..11);

        // At position 11 (end of second match), find_prev returns first match
        let (idx, range) = state.find_prev(11).unwrap();
        assert_eq!(idx, 0);
        assert_eq!(range, 0..3);

        // At position 8 (start of second match), matches ending < 8 is only 0..3
        let (idx, range) = state.find_prev(8).unwrap();
        assert_eq!(idx, 0);
        assert_eq!(range, 0..3);

        // At position 3 (end of first match), no match ends < 3, wraps to last match
        let (idx, range) = state.find_prev(3).unwrap();
        assert_eq!(idx, 2);
        assert_eq!(range, 16..19);

        // Wrap around: at position 0, no match ends < 0, wraps to last match
        let (idx, range) = state.find_prev(0).unwrap();
        assert_eq!(idx, 2);
        assert_eq!(range, 16..19);
    }

    #[test]
    fn test_clear() {
        let mut state = SearchState::new();
        state.set_pattern("foo", SearchDirection::Forward).unwrap();
        state.update_matches("foo bar foo");
        state.clear();

        assert!(state.pattern().is_none());
        assert!(state.regex().is_none());
        assert!(state.matches().is_empty());
    }

    #[test]
    fn test_has_pattern() {
        let mut state = SearchState::new();
        assert!(!state.has_pattern());

        state.set_pattern("foo", SearchDirection::Forward).unwrap();
        assert!(state.has_pattern());

        state.clear();
        assert!(!state.has_pattern());
    }

    #[test]
    fn test_search_direction_default() {
        assert_eq!(SearchDirection::default(), SearchDirection::Forward);
    }

    #[test]
    fn test_pattern_length_limit() {
        let mut state = SearchState::new();
        // Create a pattern longer than MAX_PATTERN_LENGTH (1000)
        let long_pattern = "a".repeat(1001);
        let result = state.set_pattern(&long_pattern, SearchDirection::Forward);
        assert!(result.is_err());
        assert!(state.pattern().is_none());
    }

    #[test]
    fn test_pattern_at_length_limit() {
        let mut state = SearchState::new();
        // Create a pattern exactly at MAX_PATTERN_LENGTH (1000)
        let pattern = "a".repeat(1000);
        let result = state.set_pattern(&pattern, SearchDirection::Forward);
        assert!(result.is_ok());
        assert!(state.pattern().is_some());
    }

    #[test]
    fn test_max_matches_limit() {
        let mut state = SearchState::new();
        state.set_pattern("a", SearchDirection::Forward).unwrap();

        // Create content with more than MAX_MATCHES (10_000) matches
        let content = "a".repeat(15_000);
        state.update_matches(&content);

        // Should be limited to MAX_MATCHES
        assert_eq!(state.matches().len(), 10_000);
    }

    #[test]
    fn test_update_matches_with_version_caching() {
        let mut state = SearchState::new();
        state.set_pattern("foo", SearchDirection::Forward).unwrap();

        let content = "foo bar foo";
        let version = 12345u64;

        // First update should populate matches
        state.update_matches_with_version(content, version);
        assert_eq!(state.matches().len(), 2);

        // Second update with same version should be no-op (cached)
        // We can't directly test this, but we can verify matches are unchanged
        state.update_matches_with_version(content, version);
        assert_eq!(state.matches().len(), 2);

        // Update with different version should re-scan
        state.update_matches_with_version("foo foo foo foo", version + 1);
        assert_eq!(state.matches().len(), 4);
    }

    #[test]
    fn test_binary_search_find_next() {
        let mut state = SearchState::new();
        state.set_pattern("x", SearchDirection::Forward).unwrap();

        // Create many matches to test binary search
        let content = (0..100).map(|i| format!("x{}", i)).collect::<String>();
        state.update_matches(&content);

        // Test finding next at various positions
        let (idx, _) = state.find_next(0).unwrap();
        assert!(idx > 0 || state.matches().len() == 1);

        // Position at end should wrap
        let (idx, range) = state.find_next(content.len()).unwrap();
        assert_eq!(idx, 0);
        assert_eq!(range.start, 0);
    }

    #[test]
    fn test_binary_search_find_prev() {
        let mut state = SearchState::new();
        state.set_pattern("x", SearchDirection::Forward).unwrap();

        // Create many matches to test binary search
        let content = (0..100).map(|i| format!("x{}", i)).collect::<String>();
        state.update_matches(&content);

        // Position at 0 should wrap to last
        let (idx, _) = state.find_prev(0).unwrap();
        assert_eq!(idx, state.matches().len() - 1);
    }

    #[test]
    fn test_clear_resets_doc_version() {
        let mut state = SearchState::new();
        state.set_pattern("foo", SearchDirection::Forward).unwrap();
        state.update_matches_with_version("foo bar", 12345);

        state.clear();

        // After clear, version should be reset (None)
        // Next update should not use cache
        state.set_pattern("foo", SearchDirection::Forward).unwrap();
        state.update_matches_with_version("foo foo foo", 12345);
        // Should find 3 matches, not the cached 1
        assert_eq!(state.matches().len(), 3);
    }

    #[test]
    fn test_find_next_empty_matches() {
        let mut state = SearchState::new();
        state.set_pattern("xyz", SearchDirection::Forward).unwrap();
        state.update_matches("abc def");

        // No matches
        assert!(state.find_next(0).is_none());
    }

    #[test]
    fn test_find_prev_empty_matches() {
        let mut state = SearchState::new();
        state.set_pattern("xyz", SearchDirection::Forward).unwrap();
        state.update_matches("abc def");

        // No matches
        assert!(state.find_prev(0).is_none());
    }

    #[test]
    fn test_find_prev_cursor_at_match_end() {
        let mut state = SearchState::new();
        state.set_pattern("abc", SearchDirection::Forward).unwrap();
        state.update_matches("abc def abc ghi abc");
        // Matches at: 0..3, 8..11, 16..19

        // When cursor is at END of a match, find_prev should NOT return that match
        // Position 3 = end of first match, should wrap to last
        let (idx, range) = state.find_prev(3).unwrap();
        assert_eq!(idx, 2);
        assert_eq!(range, 16..19);

        // Position 11 = end of second match, should return first
        let (idx, range) = state.find_prev(11).unwrap();
        assert_eq!(idx, 0);
        assert_eq!(range, 0..3);

        // Position 19 = end of third match, should return second
        let (idx, range) = state.find_prev(19).unwrap();
        assert_eq!(idx, 1);
        assert_eq!(range, 8..11);
    }

    #[test]
    fn test_find_next_and_prev_symmetry() {
        let mut state = SearchState::new();
        state.set_pattern("x", SearchDirection::Forward).unwrap();
        state.update_matches("x y x z x");
        // Matches at: 0..1, 4..5, 8..9

        // Starting at position 4 (start of second match)
        // find_next should return match at 8..9
        let (next_idx, _) = state.find_next(4).unwrap();
        assert_eq!(next_idx, 2);

        // find_prev should return match at 0..1
        let (prev_idx, _) = state.find_prev(4).unwrap();
        assert_eq!(prev_idx, 0);

        // At position 5 (end of second match)
        // find_next should return match at 8..9
        let (next_idx, _) = state.find_next(5).unwrap();
        assert_eq!(next_idx, 2);

        // find_prev should return match at 0..1 (NOT 4..5!)
        let (prev_idx, _) = state.find_prev(5).unwrap();
        assert_eq!(prev_idx, 0);
    }
}
