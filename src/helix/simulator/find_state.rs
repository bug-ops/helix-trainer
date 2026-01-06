//! Find state for f/F/t/T commands and Alt-./Alt-, repeat
//!
//! Tracks the last find/till motion to enable repeating with Alt-. and Alt-,

/// Type of find motion
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FindType {
    /// Find character (f/F commands)
    Find,
    /// Till character (t/T commands)
    Till,
}

/// Direction of find motion
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FindDirection {
    /// Forward search (f/t commands)
    Forward,
    /// Backward search (F/T commands)
    Backward,
}

impl FindDirection {
    /// Return the opposite direction
    #[must_use]
    pub fn reverse(&self) -> Self {
        match self {
            FindDirection::Forward => FindDirection::Backward,
            FindDirection::Backward => FindDirection::Forward,
        }
    }
}

/// State for the last find/till motion
#[derive(Debug, Clone, Default)]
pub struct FindState {
    /// Last searched character
    last_char: Option<char>,
    /// Type of last motion (find or till)
    last_type: Option<FindType>,
    /// Direction of last motion
    last_direction: Option<FindDirection>,
}

impl FindState {
    /// Create a new empty find state
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a find/till motion
    pub fn set(&mut self, ch: char, find_type: FindType, direction: FindDirection) {
        self.last_char = Some(ch);
        self.last_type = Some(find_type);
        self.last_direction = Some(direction);
    }

    /// Get the last motion if available
    #[must_use]
    pub fn get(&self) -> Option<(char, FindType, FindDirection)> {
        match (self.last_char, self.last_type, self.last_direction) {
            (Some(ch), Some(ft), Some(dir)) => Some((ch, ft, dir)),
            _ => None,
        }
    }

    /// Check if there is a recorded motion
    #[must_use]
    pub fn has_motion(&self) -> bool {
        self.last_char.is_some()
    }

    /// Clear the stored motion
    pub fn clear(&mut self) {
        self.last_char = None;
        self.last_type = None;
        self.last_direction = None;
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
        state.set('x', FindType::Find, FindDirection::Forward);

        assert!(state.has_motion());
        let (ch, ft, dir) = state.get().unwrap();
        assert_eq!(ch, 'x');
        assert_eq!(ft, FindType::Find);
        assert_eq!(dir, FindDirection::Forward);
    }

    #[test]
    fn test_find_state_till_backward() {
        let mut state = FindState::new();
        state.set('(', FindType::Till, FindDirection::Backward);

        let (ch, ft, dir) = state.get().unwrap();
        assert_eq!(ch, '(');
        assert_eq!(ft, FindType::Till);
        assert_eq!(dir, FindDirection::Backward);
    }

    #[test]
    fn test_find_state_clear() {
        let mut state = FindState::new();
        state.set('a', FindType::Find, FindDirection::Forward);
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
        state.set('a', FindType::Find, FindDirection::Forward);
        state.set('b', FindType::Till, FindDirection::Backward);

        let (ch, ft, dir) = state.get().unwrap();
        assert_eq!(ch, 'b');
        assert_eq!(ft, FindType::Till);
        assert_eq!(dir, FindDirection::Backward);
    }
}
