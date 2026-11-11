//! Find state for f/F/t/T commands and Alt-. repeat
//!
//! Tracks the last find/till motion to enable repeating with Alt-.

use crate::grammar::find::{FindDirection, FindKind};

/// State for the last find/till motion
#[derive(Debug, Clone, Default)]
pub struct FindState {
    /// Last searched character
    ch: Option<char>,
    /// Type of last motion (find or till)
    find_kind: Option<FindKind>,
    /// Direction of last motion
    direction: Option<FindDirection>,
}

impl FindState {
    /// Create a new empty find state
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a find/till motion
    pub fn set(&mut self, ch: char, find_kind: FindKind, direction: FindDirection) {
        self.ch = Some(ch);
        self.find_kind = Some(find_kind);
        self.direction = Some(direction);
    }

    /// Get the last motion if available
    #[must_use]
    pub fn get(&self) -> Option<(char, FindKind, FindDirection)> {
        match (self.ch, self.find_kind, self.direction) {
            (Some(ch), Some(ft), Some(dir)) => Some((ch, ft, dir)),
            _ => None,
        }
    }

    /// Check if there is a recorded motion
    #[must_use]
    pub fn has_motion(&self) -> bool {
        self.ch.is_some()
    }

    /// Clear the stored motion
    pub fn clear(&mut self) {
        self.ch = None;
        self.find_kind = None;
        self.direction = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_state_new() {
        let state = FindState::new();
        assert!(!state.has_motion());
        assert!(state.get().is_none());
    }

    #[test]
    fn test_find_state_set_and_get() {
        let mut state = FindState::new();
        state.set('x', FindKind::Find, FindDirection::Forward);

        assert!(state.has_motion());
        let (ch, ft, dir) = state.get().unwrap();
        assert_eq!(ch, 'x');
        assert_eq!(ft, FindKind::Find);
        assert_eq!(dir, FindDirection::Forward);
    }

    #[test]
    fn test_find_state_till_backward() {
        let mut state = FindState::new();
        state.set('(', FindKind::Till, FindDirection::Backward);

        let (ch, ft, dir) = state.get().unwrap();
        assert_eq!(ch, '(');
        assert_eq!(ft, FindKind::Till);
        assert_eq!(dir, FindDirection::Backward);
    }

    #[test]
    fn test_find_state_clear() {
        let mut state = FindState::new();
        state.set('a', FindKind::Find, FindDirection::Forward);
        assert!(state.has_motion());

        state.clear();
        assert!(!state.has_motion());
        assert!(state.get().is_none());
    }

    #[test]
    fn test_find_direction_reverse() {
        assert_eq!(FindDirection::Forward.reverse(), FindDirection::Backward);
        assert_eq!(FindDirection::Backward.reverse(), FindDirection::Forward);
    }

    #[test]
    fn test_find_state_overwrite() {
        let mut state = FindState::new();
        state.set('a', FindKind::Find, FindDirection::Forward);
        state.set('b', FindKind::Till, FindDirection::Backward);

        let (ch, ft, dir) = state.get().unwrap();
        assert_eq!(ch, 'b');
        assert_eq!(ft, FindKind::Till);
        assert_eq!(dir, FindDirection::Backward);
    }
}
