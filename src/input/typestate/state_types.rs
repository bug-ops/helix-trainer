//! State marker types for typestate-based input handling
//!
//! This module contains zero-sized marker types that encode input state at the type level,
//! providing compile-time guarantees about state transitions.

// ============================================================================
// Handler state marker types (zero-sized)
// ============================================================================

/// Base state - no prefix, accepting normal input
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BaseState;

/// Waiting for second key after 'g' (goto commands)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GotoPending;

/// Waiting for second key after 'z' (view commands)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ViewPending;

/// Waiting for second key after 'm' (match commands)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MatchPending;

/// Waiting for character after 'ms' (surround add)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SurroundAddPending;

/// Waiting for character after 'md' (surround delete)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SurroundDeletePending;

/// Waiting for first character after 'mr' (surround replace from char)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SurroundReplaceFromPending;

/// Waiting for second character after 'mr{from}' (surround replace to char)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SurroundReplaceToPending {
    /// The character to replace from
    pub from_char: char,
}

/// Waiting for text object after 'ma' (select around)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextObjectAroundPending;

/// Waiting for text object after 'mi' (select inside)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextObjectInsidePending;

/// Waiting for character after 'f'/'F'/'t'/'T' (find/till commands)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FindCharPending {
    /// The direction and type of find operation
    pub find_type: FindType,
}

/// Type of find/till operation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FindType {
    /// Find forward ('f')
    FindForward,
    /// Find backward ('F')
    FindBackward,
    /// Till forward ('t')
    TillForward,
    /// Till backward ('T')
    TillBackward,
}

impl FindType {
    /// Get the command prefix for this find type
    pub fn prefix(&self) -> &'static str {
        match self {
            Self::FindForward => "f",
            Self::FindBackward => "F",
            Self::TillForward => "t",
            Self::TillBackward => "T",
        }
    }
}

/// Waiting for character after 'r' (replace command)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReplaceCharPending;

/// Waiting for the register character after '"' (named register selection)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RegisterPending;

/// Waiting for the operator character after '"{register}' (y/p/P/R)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RegisterOpPending {
    /// The register selected to scope the upcoming operator
    pub register: char,
}

/// Accumulating a `:`-prefixed command-line buffer
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandLinePending {
    /// Buffer contents typed after the leading ':'
    pub buffer: String,
}

/// Accumulating an `s`/`S` regex-selection prompt buffer
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegexPromptPending {
    /// Which command (`s` or `S`) this prompt was opened for
    pub kind: super::input_state::RegexPromptKind,
    /// Buffer contents typed after the leading 's'/'S'
    pub buffer: String,
}

/// Building a count prefix (digits 1-9, then 0-9)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CountPending {
    /// The accumulated count value
    pub count: usize,
}

/// Waiting for second key after '[' (unmatched previous commands)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnmatchedPrevPending;

/// Waiting for second key after ']' (unmatched next commands)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnmatchedNextPending;

// ============================================================================
// Sealed trait for handler states
// ============================================================================

mod private {
    pub trait Sealed {}
}

impl private::Sealed for BaseState {}
impl private::Sealed for GotoPending {}
impl private::Sealed for ViewPending {}
impl private::Sealed for MatchPending {}
impl private::Sealed for SurroundAddPending {}
impl private::Sealed for SurroundDeletePending {}
impl private::Sealed for SurroundReplaceFromPending {}
impl private::Sealed for SurroundReplaceToPending {}
impl private::Sealed for TextObjectAroundPending {}
impl private::Sealed for TextObjectInsidePending {}
impl private::Sealed for FindCharPending {}
impl private::Sealed for ReplaceCharPending {}
impl private::Sealed for RegisterPending {}
impl private::Sealed for RegisterOpPending {}
impl private::Sealed for CommandLinePending {}
impl private::Sealed for RegexPromptPending {}
impl private::Sealed for CountPending {}
impl private::Sealed for UnmatchedPrevPending {}
impl private::Sealed for UnmatchedNextPending {}

/// Marker trait for handler state types
///
/// This trait is sealed to ensure only valid states can be used.
pub trait HandlerState: private::Sealed {
    /// Human-readable name of this state
    fn state_name() -> &'static str;
}

impl HandlerState for BaseState {
    fn state_name() -> &'static str {
        "BASE"
    }
}

impl HandlerState for GotoPending {
    fn state_name() -> &'static str {
        "GOTO_PENDING"
    }
}

impl HandlerState for ViewPending {
    fn state_name() -> &'static str {
        "VIEW_PENDING"
    }
}

impl HandlerState for MatchPending {
    fn state_name() -> &'static str {
        "MATCH_PENDING"
    }
}

impl HandlerState for SurroundAddPending {
    fn state_name() -> &'static str {
        "SURROUND_ADD_PENDING"
    }
}

impl HandlerState for SurroundDeletePending {
    fn state_name() -> &'static str {
        "SURROUND_DELETE_PENDING"
    }
}

impl HandlerState for SurroundReplaceFromPending {
    fn state_name() -> &'static str {
        "SURROUND_REPLACE_FROM_PENDING"
    }
}

impl HandlerState for SurroundReplaceToPending {
    fn state_name() -> &'static str {
        "SURROUND_REPLACE_TO_PENDING"
    }
}

impl HandlerState for TextObjectAroundPending {
    fn state_name() -> &'static str {
        "TEXT_OBJECT_AROUND_PENDING"
    }
}

impl HandlerState for TextObjectInsidePending {
    fn state_name() -> &'static str {
        "TEXT_OBJECT_INSIDE_PENDING"
    }
}

impl HandlerState for FindCharPending {
    fn state_name() -> &'static str {
        "FIND_CHAR_PENDING"
    }
}

impl HandlerState for ReplaceCharPending {
    fn state_name() -> &'static str {
        "REPLACE_CHAR_PENDING"
    }
}

impl HandlerState for RegisterPending {
    fn state_name() -> &'static str {
        "REGISTER_PENDING"
    }
}

impl HandlerState for RegisterOpPending {
    fn state_name() -> &'static str {
        "REGISTER_OP_PENDING"
    }
}

impl HandlerState for CommandLinePending {
    fn state_name() -> &'static str {
        "COMMAND_LINE_PENDING"
    }
}

impl HandlerState for RegexPromptPending {
    fn state_name() -> &'static str {
        "REGEX_PROMPT_PENDING"
    }
}

impl HandlerState for CountPending {
    fn state_name() -> &'static str {
        "COUNT_PENDING"
    }
}

impl HandlerState for UnmatchedPrevPending {
    fn state_name() -> &'static str {
        "UNMATCHED_PREV_PENDING"
    }
}

impl HandlerState for UnmatchedNextPending {
    fn state_name() -> &'static str {
        "UNMATCHED_NEXT_PENDING"
    }
}
