//! The find/till grammar: `f`, `F`, `t`, `T`, and the keys that take a
//! character argument

/// Whether a motion lands on the character or stops before it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FindKind {
    /// Land on the character (`f`, `F`).
    Find,
    /// Stop next to the character (`t`, `T`).
    Till,
}

/// Which way a find motion searches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FindDirection {
    /// Towards the end of the document (`f`, `t`).
    Forward,
    /// Towards the start of the document (`F`, `T`).
    Backward,
}

impl FindDirection {
    /// The opposite direction.
    #[must_use]
    pub fn reverse(self) -> Self {
        match self {
            Self::Forward => Self::Backward,
            Self::Backward => Self::Forward,
        }
    }
}

/// One of the four find motions, before its character argument.
///
/// # Examples
///
/// ```
/// use helix_trainer::grammar::find::{FindDirection, FindKind, FindMotion};
///
/// let till_back = FindMotion::from_key('T').unwrap();
/// assert_eq!(till_back, FindMotion::new(FindKind::Till, FindDirection::Backward));
/// assert_eq!(till_back.key(), 'T');
/// assert_eq!(FindMotion::from_key('x'), None);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FindMotion {
    kind: FindKind,
    direction: FindDirection,
}

impl FindMotion {
    /// `f`: find forward.
    pub const FIND_FORWARD: Self = Self::new(FindKind::Find, FindDirection::Forward);
    /// `F`: find backward.
    pub const FIND_BACKWARD: Self = Self::new(FindKind::Find, FindDirection::Backward);
    /// `t`: till forward.
    pub const TILL_FORWARD: Self = Self::new(FindKind::Till, FindDirection::Forward);
    /// `T`: till backward.
    pub const TILL_BACKWARD: Self = Self::new(FindKind::Till, FindDirection::Backward);

    /// The motion with this kind and direction.
    #[must_use]
    pub const fn new(kind: FindKind, direction: FindDirection) -> Self {
        Self { kind, direction }
    }

    /// The motion bound to `key` in Normal mode (`f`, `F`, `t`, `T`).
    #[must_use]
    pub fn from_key(key: char) -> Option<Self> {
        match key {
            'f' => Some(Self::FIND_FORWARD),
            'F' => Some(Self::FIND_BACKWARD),
            't' => Some(Self::TILL_FORWARD),
            'T' => Some(Self::TILL_BACKWARD),
            _ => None,
        }
    }

    /// The key that starts this motion.
    #[must_use]
    pub fn key(self) -> char {
        match (self.kind, self.direction) {
            (FindKind::Find, FindDirection::Forward) => 'f',
            (FindKind::Find, FindDirection::Backward) => 'F',
            (FindKind::Till, FindDirection::Forward) => 't',
            (FindKind::Till, FindDirection::Backward) => 'T',
        }
    }

    /// Whether the motion lands on or stops before the character.
    #[must_use]
    pub fn kind(self) -> FindKind {
        self.kind
    }

    /// Which way the motion searches.
    #[must_use]
    pub fn direction(self) -> FindDirection {
        self.direction
    }
}

/// Whether the normal-mode key `key` waits for a character argument: the four
/// find motions and `r`.
///
/// # Examples
///
/// ```
/// use helix_trainer::grammar::find::takes_char_argument;
///
/// assert!(takes_char_argument('f'));
/// assert!(takes_char_argument('r'));
/// assert!(!takes_char_argument('d'));
/// ```
#[must_use]
pub fn takes_char_argument(key: char) -> bool {
    key == 'r' || FindMotion::from_key(key).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_motion_key_round_trips() {
        for key in ['f', 'F', 't', 'T'] {
            assert_eq!(FindMotion::from_key(key).map(FindMotion::key), Some(key));
        }
    }

    #[test]
    fn reverse_flips_the_direction_only() {
        assert_eq!(FindDirection::Forward.reverse(), FindDirection::Backward);
        assert_eq!(FindDirection::Backward.reverse(), FindDirection::Forward);
    }
}
