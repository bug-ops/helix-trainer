//! Keymap overlay lookup context

/// Which keymap sub-table a translatable [`InputState`](crate::input::typestate::InputState)
/// corresponds to.
///
/// This is the first half of a [`KeymapOverlay`](super::overlay::KeymapOverlay)
/// lookup key. `InputState::key_context` maps every state to one of these
/// variants, or to `None` for states that consume the next key literally
/// (find-char targets, replace targets, register names, surround/text-object
/// characters, command-line buffer input) — those must never be translated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyContext {
    /// `InputState::Base`, plus `CountPending` and `RegisterOpPending`,
    /// which share the top-level `[keys.normal]` table.
    Base,
    /// `InputState::GotoPending` (after `g`) -> `[keys.normal.g]`.
    Goto,
    /// `InputState::ViewPending` (after `z`) -> `[keys.normal.z]`.
    View,
    /// `InputState::MatchPending` (after `m`) -> `[keys.normal.m]`.
    Match,
    /// `InputState::UnmatchedPrevPending` (after `[`) -> `[keys.normal.'[']`.
    UnmatchedPrev,
    /// `InputState::UnmatchedNextPending` (after `]`) -> `[keys.normal.']']`.
    UnmatchedNext,
}

impl KeyContext {
    /// The physical prefix character that leads into this context from
    /// `Base`, or `None` for [`KeyContext::Base`] itself (which has no
    /// prefix).
    ///
    /// Used to validate minor-mode remaps: a `[keys.normal.g]` binding may
    /// only retarget a command whose canonical key is exactly this prefix
    /// followed by one more token (see `src/config/keymap/parse.rs`).
    pub fn prefix_char(self) -> Option<&'static str> {
        match self {
            KeyContext::Base => None,
            KeyContext::Goto => Some("g"),
            KeyContext::View => Some("z"),
            KeyContext::Match => Some("m"),
            KeyContext::UnmatchedPrev => Some("["),
            KeyContext::UnmatchedNext => Some("]"),
        }
    }

    /// The `[keys.normal.<p>]` minor-mode prefix this context corresponds
    /// to, if any is bound in the raw TOML at that key.
    pub fn from_prefix_char(prefix: &str) -> Option<Self> {
        match prefix {
            "g" => Some(KeyContext::Goto),
            "z" => Some(KeyContext::View),
            "m" => Some(KeyContext::Match),
            "[" => Some(KeyContext::UnmatchedPrev),
            "]" => Some(KeyContext::UnmatchedNext),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefix_char_round_trips_through_from_prefix_char() {
        for ctx in [
            KeyContext::Goto,
            KeyContext::View,
            KeyContext::Match,
            KeyContext::UnmatchedPrev,
            KeyContext::UnmatchedNext,
        ] {
            let prefix = ctx.prefix_char().unwrap();
            assert_eq!(KeyContext::from_prefix_char(prefix), Some(ctx));
        }
    }

    #[test]
    fn base_has_no_prefix() {
        assert_eq!(KeyContext::Base.prefix_char(), None);
    }

    #[test]
    fn unknown_prefix_is_none() {
        assert_eq!(KeyContext::from_prefix_char("space"), None);
    }
}
