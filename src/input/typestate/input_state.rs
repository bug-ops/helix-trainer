//! Runtime input state enum
//!
//! This module provides the runtime representation of input state, wrapping the typestate
//! pattern for runtime use while still benefiting from the type-safe design.

use super::state_types::FindType;
use crate::input::keymap::KeyContext;

/// Which `s`/`S` regex-selection command a `RegexPromptPending` buffer is for
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegexPromptKind {
    /// `s` - select all regex matches within the current selection
    SelectRegex,
    /// `S` - split the current selection on a regex delimiter
    SplitSelection,
}

/// Runtime representation of input state
///
/// This enum wraps the typestate pattern for runtime use, allowing dynamic
/// dispatch while still benefiting from the type-safe design.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum InputState {
    /// No prefix, normal input mode
    #[default]
    Base,
    /// After 'g' - waiting for goto command second key
    GotoPending,
    /// After 'z' - waiting for view command second key
    ViewPending,
    /// After 'm' - waiting for match command second key
    MatchPending,
    /// After 'ms' - waiting for surround add character
    SurroundAddPending,
    /// After 'md' - waiting for surround delete character
    SurroundDeletePending,
    /// After 'mr' - waiting for surround replace from character
    SurroundReplaceFromPending,
    /// After 'mr{char}' - waiting for surround replace to character
    SurroundReplaceToPending { from_char: char },
    /// After 'ma' - waiting for text object (around)
    TextObjectAroundPending,
    /// After 'mi' - waiting for text object (inside)
    TextObjectInsidePending,
    /// After 'f'/'F'/'t'/'T' - waiting for character
    FindCharPending { find_type: FindType },
    /// After 'r' - waiting for replacement character
    ReplaceCharPending,
    /// After '"' - waiting for register character
    RegisterPending,
    /// After '"{register}' - waiting for operator character (y/p/P/R/d/c)
    RegisterOpPending { register: char },
    /// After ':' - accumulating a command-line buffer
    CommandLinePending { buffer: String },
    /// After 's'/'S' - accumulating a regex-selection prompt buffer
    RegexPromptPending {
        kind: RegexPromptKind,
        buffer: String,
    },
    /// After digit 1-9 - building count prefix
    CountPending { count: usize },
    /// After '[' - waiting for unmatched previous command second key
    UnmatchedPrevPending,
    /// After ']' - waiting for unmatched next command second key
    UnmatchedNextPending,
}

impl InputState {
    /// Check if this is the base state
    pub fn is_base(&self) -> bool {
        matches!(self, Self::Base)
    }

    /// Check if this is goto pending state
    pub fn is_goto_pending(&self) -> bool {
        matches!(self, Self::GotoPending)
    }

    /// Check if this is view pending state
    pub fn is_view_pending(&self) -> bool {
        matches!(self, Self::ViewPending)
    }

    /// Check if this is match pending state
    pub fn is_match_pending(&self) -> bool {
        matches!(self, Self::MatchPending)
    }

    /// Check if this is find char pending state
    pub fn is_find_char_pending(&self) -> bool {
        matches!(self, Self::FindCharPending { .. })
    }

    /// Check if this is replace char pending state
    pub fn is_replace_char_pending(&self) -> bool {
        matches!(self, Self::ReplaceCharPending)
    }

    /// Check if this is count pending state
    pub fn is_count_pending(&self) -> bool {
        matches!(self, Self::CountPending { .. })
    }

    /// Check if this is a regex-selection prompt pending state
    pub fn is_regex_prompt_pending(&self) -> bool {
        matches!(self, Self::RegexPromptPending { .. })
    }

    /// Check if this state is waiting for a character argument
    pub fn is_waiting_for_char(&self) -> bool {
        matches!(
            self,
            Self::FindCharPending { .. }
                | Self::ReplaceCharPending
                | Self::SurroundAddPending
                | Self::SurroundDeletePending
                | Self::SurroundReplaceFromPending
                | Self::SurroundReplaceToPending { .. }
                | Self::RegisterPending
                | Self::RegisterOpPending { .. }
        )
    }

    /// Check if this state is a prefix state (waiting for more input)
    pub fn is_prefix_state(&self) -> bool {
        !matches!(self, Self::Base)
    }

    /// Check if this is a surround pending state
    pub fn is_surround_pending(&self) -> bool {
        matches!(
            self,
            Self::SurroundAddPending
                | Self::SurroundDeletePending
                | Self::SurroundReplaceFromPending
                | Self::SurroundReplaceToPending { .. }
        )
    }

    /// Check if this is a text object pending state (waiting for text object type)
    pub fn is_text_object_pending(&self) -> bool {
        matches!(
            self,
            Self::TextObjectAroundPending | Self::TextObjectInsidePending
        )
    }

    /// Check if this is unmatched prev pending state
    pub fn is_unmatched_prev_pending(&self) -> bool {
        matches!(self, Self::UnmatchedPrevPending)
    }

    /// Check if this is unmatched next pending state
    pub fn is_unmatched_next_pending(&self) -> bool {
        matches!(self, Self::UnmatchedNextPending)
    }

    /// Which [`KeyContext`] this state's next keypress should be translated
    /// under, or `None` if this state consumes the next key literally and
    /// must never be translated by the keymap overlay.
    ///
    /// `CountPending` and `RegisterOpPending` map to [`KeyContext::Base`]:
    /// both dispatch through the same top-level `[keys.normal]` table as
    /// `Base` itself (a count prefix or a register selector doesn't change
    /// which command a key invokes, only how it's recorded). States that
    /// consume a literal character argument (find/replace targets,
    /// register names, surround/text-object characters, the command-line
    /// and regex-prompt buffers) return `None`.
    pub fn key_context(&self) -> Option<KeyContext> {
        match self {
            Self::Base | Self::CountPending { .. } | Self::RegisterOpPending { .. } => {
                Some(KeyContext::Base)
            }
            Self::GotoPending => Some(KeyContext::Goto),
            Self::ViewPending => Some(KeyContext::View),
            Self::MatchPending => Some(KeyContext::Match),
            Self::UnmatchedPrevPending => Some(KeyContext::UnmatchedPrev),
            Self::UnmatchedNextPending => Some(KeyContext::UnmatchedNext),
            Self::SurroundAddPending
            | Self::SurroundDeletePending
            | Self::SurroundReplaceFromPending
            | Self::SurroundReplaceToPending { .. }
            | Self::TextObjectAroundPending
            | Self::TextObjectInsidePending
            | Self::FindCharPending { .. }
            | Self::ReplaceCharPending
            | Self::RegisterPending
            | Self::CommandLinePending { .. }
            | Self::RegexPromptPending { .. } => None,
        }
    }

    /// Get the state name for display
    pub fn name(&self) -> &'static str {
        match self {
            Self::Base => "BASE",
            Self::GotoPending => "GOTO_PENDING",
            Self::ViewPending => "VIEW_PENDING",
            Self::MatchPending => "MATCH_PENDING",
            Self::SurroundAddPending => "SURROUND_ADD_PENDING",
            Self::SurroundDeletePending => "SURROUND_DELETE_PENDING",
            Self::SurroundReplaceFromPending => "SURROUND_REPLACE_FROM_PENDING",
            Self::SurroundReplaceToPending { .. } => "SURROUND_REPLACE_TO_PENDING",
            Self::TextObjectAroundPending => "TEXT_OBJECT_AROUND_PENDING",
            Self::TextObjectInsidePending => "TEXT_OBJECT_INSIDE_PENDING",
            Self::FindCharPending { .. } => "FIND_CHAR_PENDING",
            Self::ReplaceCharPending => "REPLACE_CHAR_PENDING",
            Self::RegisterPending => "REGISTER_PENDING",
            Self::RegisterOpPending { .. } => "REGISTER_OP_PENDING",
            Self::CommandLinePending { .. } => "COMMAND_LINE_PENDING",
            Self::RegexPromptPending { .. } => "REGEX_PROMPT_PENDING",
            Self::CountPending { .. } => "COUNT_PENDING",
            Self::UnmatchedPrevPending => "UNMATCHED_PREV_PENDING",
            Self::UnmatchedNextPending => "UNMATCHED_NEXT_PENDING",
        }
    }
}

#[cfg(test)]
mod key_context_tests {
    use super::*;

    #[test]
    fn base_and_count_and_register_op_map_to_base_context() {
        assert_eq!(InputState::Base.key_context(), Some(KeyContext::Base));
        assert_eq!(
            InputState::CountPending { count: 3 }.key_context(),
            Some(KeyContext::Base)
        );
        assert_eq!(
            InputState::RegisterOpPending { register: 'a' }.key_context(),
            Some(KeyContext::Base)
        );
    }

    #[test]
    fn minor_mode_pending_states_map_to_their_context() {
        assert_eq!(
            InputState::GotoPending.key_context(),
            Some(KeyContext::Goto)
        );
        assert_eq!(
            InputState::ViewPending.key_context(),
            Some(KeyContext::View)
        );
        assert_eq!(
            InputState::MatchPending.key_context(),
            Some(KeyContext::Match)
        );
        assert_eq!(
            InputState::UnmatchedPrevPending.key_context(),
            Some(KeyContext::UnmatchedPrev)
        );
        assert_eq!(
            InputState::UnmatchedNextPending.key_context(),
            Some(KeyContext::UnmatchedNext)
        );
    }

    #[test]
    fn literal_consuming_states_are_never_translated() {
        let literal_states = [
            InputState::SurroundAddPending,
            InputState::SurroundDeletePending,
            InputState::SurroundReplaceFromPending,
            InputState::SurroundReplaceToPending { from_char: '(' },
            InputState::TextObjectAroundPending,
            InputState::TextObjectInsidePending,
            InputState::FindCharPending {
                find_type: FindType::FindForward,
            },
            InputState::ReplaceCharPending,
            InputState::RegisterPending,
            InputState::CommandLinePending {
                buffer: String::new(),
            },
        ];
        for state in literal_states {
            assert_eq!(state.key_context(), None, "{:?} must not translate", state);
        }
    }
}
