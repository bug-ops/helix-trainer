//! Handler result type for input state transitions
//!
//! Defines the result of handling a key event, which can be staying in the current state,
//! transitioning to a new state, executing a command, or cancelling.

use std::borrow::Cow;

use super::input_state::InputState;

/// Result of handling a key event
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HandlerResult {
    /// Stay in current state, no command to execute
    Stay,
    /// Transition to a new state
    Transition(InputState),
    /// Execute a command and return to base state
    ///
    /// Uses `Cow<'static, str>` to avoid allocations for static command strings
    /// while still supporting dynamic commands (with count prefix or character arguments).
    Execute(Cow<'static, str>),
    /// Cancel current state and return to base
    Cancel,
}

impl HandlerResult {
    /// Check if this result indicates staying in the same state
    pub fn is_stay(&self) -> bool {
        matches!(self, Self::Stay)
    }

    /// Check if this result indicates a state transition
    pub fn is_transition(&self) -> bool {
        matches!(self, Self::Transition(_))
    }

    /// Check if this result indicates command execution
    pub fn is_execute(&self) -> bool {
        matches!(self, Self::Execute(_))
    }

    /// Check if this result indicates cancellation
    pub fn is_cancel(&self) -> bool {
        matches!(self, Self::Cancel)
    }

    /// Get the command to execute, if any
    pub fn command(&self) -> Option<&str> {
        match self {
            Self::Execute(cmd) => Some(cmd),
            _ => None,
        }
    }
}
