//! Handler result type for input state transitions
//!
//! Defines the result of handling a key event, which can be staying in the current state,
//! transitioning to a new state, executing a command, or cancelling.

use super::input_state::InputState;
use crate::helix::normal_command::NormalCommand;

/// Result of handling a key event
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HandlerResult {
    /// Stay in current state, no command to execute
    Stay,
    /// Transition to a new state
    Transition(InputState),
    /// Execute a command and return to base state
    ///
    /// The command is the typed [`NormalCommand`] the state machine resolved the
    /// keys to; its `Display` is the canonical string form.
    Execute(NormalCommand),
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
    pub fn command(&self) -> Option<&NormalCommand> {
        match self {
            Self::Execute(cmd) => Some(cmd),
            _ => None,
        }
    }
}
