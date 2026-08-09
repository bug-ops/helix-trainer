//! Runtime input state enum
//!
//! This module provides the runtime representation of input state, wrapping the typestate
//! pattern for runtime use while still benefiting from the type-safe design.

use super::state_types::FindType;

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
    /// After '"{register}' - waiting for operator character (y/p/P/R)
    RegisterOpPending { register: char },
    /// After ':' - accumulating a command-line buffer
    CommandLinePending { buffer: String },
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
            Self::CountPending { .. } => "COUNT_PENDING",
            Self::UnmatchedPrevPending => "UNMATCHED_PREV_PENDING",
            Self::UnmatchedNextPending => "UNMATCHED_NEXT_PENDING",
        }
    }
}
