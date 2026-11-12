//! Search state management for Helix simulator
//!
//! Provides search pattern storage and the match lookup of Helix's
//! `search_impl` for the search commands (/, ?, n, N, *, Alt-*).

use crate::helix::error::SimulatorError;
use helix_core::ropey::RopeSlice;
use helix_stdx::rope::{Config, Regex, RegexBuilder, RopeSliceExt};
use std::ops::Range;

/// Maximum pattern length (prevents regex `DoS` attacks)
const MAX_PATTERN_LENGTH: usize = 1000;

/// Compile `pattern` as Helix's regex prompt does with the default
/// `search.smart-case`: per-line anchors, and case-insensitive unless the
/// pattern contains an uppercase character.
///
/// Shared by search and the selection commands (`s`, `S`, `K`, `Alt-K`).
///
/// # Errors
///
/// Returns a [`SimulatorError`] if `pattern` is not a valid regex.
pub(crate) fn compile_smart_case(pattern: &str) -> Result<Regex, SimulatorError> {
    RegexBuilder::new()
        .syntax(
            Config::new()
                .multi_line(true)
                .case_insensitive(!pattern.chars().any(char::is_uppercase)),
        )
        .build(pattern)
        .map_err(|e| SimulatorError::invalid_pattern(&e))
}

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
/// Stores the current search pattern, direction and compiled regex. Patterns
/// match per line (`^` and `$` anchor at line ends), as in Helix.
#[derive(Debug, Default)]
pub struct SearchState {
    /// The raw search pattern string
    pattern: Option<String>,
    /// Search direction (forward or backward)
    direction: SearchDirection,
    /// Compiled regex
    regex: Option<Regex>,
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

    /// Set search pattern and direction
    ///
    /// Compiles the regex with Helix's default `search.smart-case`: a pattern
    /// without uppercase characters matches case-insensitively.
    ///
    /// # Errors
    ///
    /// Returns a [`SimulatorError`] if `pattern` exceeds the length limit or is not a valid regex.
    pub fn set_pattern(
        &mut self,
        pattern: &str,
        direction: SearchDirection,
    ) -> Result<(), SimulatorError> {
        // Security: limit pattern length to prevent regex DoS
        if pattern.len() > MAX_PATTERN_LENGTH {
            return Err(SimulatorError::PatternTooLong {
                max: MAX_PATTERN_LENGTH,
            });
        }

        let regex = compile_smart_case(pattern)?;
        self.pattern = Some(pattern.to_string());
        self.direction = direction;
        self.regex = Some(regex);
        Ok(())
    }

    /// Find the match Helix's `search_impl` would select, as a byte range
    ///
    /// Searching forward finds the first match that starts at or after the
    /// byte `from`; searching backward finds the last match that ends at or
    /// before it. With no match that way the search wraps around the
    /// document. `None` when there is no pattern or no match at all.
    pub fn find(
        &self,
        text: RopeSlice,
        from: usize,
        direction: SearchDirection,
    ) -> Option<Range<usize>> {
        let regex = self.regex.as_ref()?;
        let found = match direction {
            SearchDirection::Forward => regex
                .find(text.regex_input_at_bytes(from..))
                .or_else(|| regex.find(text.regex_input())),
            SearchDirection::Backward => regex
                .find_iter(text.regex_input_at_bytes(..from))
                .last()
                .or_else(|| regex.find_iter(text.regex_input_at_bytes(from..)).last()),
        };
        found.map(|found| found.start()..found.end())
    }

    /// Clear the search state
    pub fn clear(&mut self) {
        self.pattern = None;
        self.direction = SearchDirection::default();
        self.regex = None;
    }

    /// Check if there is an active search pattern
    pub fn has_pattern(&self) -> bool {
        self.pattern.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use helix_core::Rope;
    use std::assert_matches;

    fn find(
        pattern: &str,
        text: &str,
        from: usize,
        direction: SearchDirection,
    ) -> Option<Range<usize>> {
        let mut state = SearchState::new();
        state
            .set_pattern(pattern, SearchDirection::Forward)
            .unwrap();
        state.find(Rope::from_str(text).slice(..), from, direction)
    }

    #[test]
    fn test_search_state_new() {
        let state = SearchState::new();
        assert!(state.pattern().is_none());
        assert_eq!(state.direction(), SearchDirection::Forward);
        assert!(!state.has_pattern());
    }

    #[test]
    fn test_set_pattern_valid() {
        let mut state = SearchState::new();
        assert!(state.set_pattern("foo", SearchDirection::Forward).is_ok());
        assert_eq!(state.pattern(), Some("foo"));
        assert_eq!(state.direction(), SearchDirection::Forward);
    }

    #[test]
    fn test_set_pattern_invalid_regex() {
        let mut state = SearchState::new();
        assert_matches!(
            state.set_pattern("[invalid", SearchDirection::Forward),
            Err(SimulatorError::InvalidRegex(_))
        );
    }

    #[test]
    fn forward_finds_the_first_match_at_or_after_the_start() {
        let text = "foo bar foo baz foo";
        assert_eq!(find("foo", text, 0, SearchDirection::Forward), Some(0..3));
        assert_eq!(find("foo", text, 1, SearchDirection::Forward), Some(8..11));
        assert_eq!(
            find("foo", text, 12, SearchDirection::Forward),
            Some(16..19)
        );
    }

    #[test]
    fn forward_wraps_to_the_first_match() {
        assert_eq!(
            find("foo", "foo bar foo", 9, SearchDirection::Forward),
            Some(0..3)
        );
    }

    #[test]
    fn backward_finds_the_last_match_ending_at_or_before_the_start() {
        let text = "foo bar foo baz foo";
        assert_eq!(
            find("foo", text, 19, SearchDirection::Backward),
            Some(16..19)
        );
        assert_eq!(
            find("foo", text, 16, SearchDirection::Backward),
            Some(8..11)
        );
        assert_eq!(
            find("foo", text, 11, SearchDirection::Backward),
            Some(8..11)
        );
        assert_eq!(find("foo", text, 10, SearchDirection::Backward), Some(0..3));
    }

    #[test]
    fn backward_wraps_to_the_last_match() {
        let text = "foo bar foo baz foo";
        assert_eq!(find("foo", text, 3, SearchDirection::Backward), Some(0..3));
        assert_eq!(
            find("foo", text, 2, SearchDirection::Backward),
            Some(16..19)
        );
    }

    #[test]
    fn lowercase_pattern_is_case_insensitive() {
        assert_eq!(
            find("foo", "xx Foo", 0, SearchDirection::Forward),
            Some(3..6)
        );
    }

    #[test]
    fn pattern_with_uppercase_is_case_sensitive() {
        assert_eq!(
            find("Foo", "foo Foo", 0, SearchDirection::Forward),
            Some(4..7)
        );
        assert_eq!(find("Foo", "foo FOO", 0, SearchDirection::Forward), None);
    }

    #[test]
    fn anchors_match_at_line_ends() {
        let text = "ab\ncd\n";
        assert_eq!(find("$", text, 0, SearchDirection::Forward), Some(2..2));
        assert_eq!(find("^", text, 1, SearchDirection::Forward), Some(3..3));
    }

    #[test]
    fn no_pattern_and_no_match_find_nothing() {
        let rope = Rope::from_str("abc");
        assert_eq!(
            SearchState::new().find(rope.slice(..), 0, SearchDirection::Forward),
            None
        );
        assert_eq!(find("zzz", "abc", 0, SearchDirection::Forward), None);
        assert_eq!(find("zzz", "abc", 0, SearchDirection::Backward), None);
    }

    #[test]
    fn test_clear() {
        let mut state = SearchState::new();
        state.set_pattern("foo", SearchDirection::Forward).unwrap();
        state.clear();

        assert!(state.pattern().is_none());
        assert!(!state.has_pattern());
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
        assert_matches!(
            result,
            Err(SimulatorError::PatternTooLong {
                max: MAX_PATTERN_LENGTH
            })
        );
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
}
