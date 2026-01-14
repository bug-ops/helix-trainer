//! Typestate-based input handling for compile-time safe state transitions
//!
//! This module implements the typestate pattern for input handling, providing
//! compile-time guarantees about which mode the editor is in and what kind of
//! input is expected.
//!
//! # Architecture
//!
//! The system uses zero-sized marker types to encode input state:
//!
//! ```text
//! BaseState        -> GotoPending (after 'g')
//!                  -> ViewPending (after 'z')
//!                  -> MatchPending (after 'm')
//!                  -> FindCharPending (after 'f'/'F'/'t'/'T')
//!                  -> ReplaceCharPending (after 'r')
//!                  -> CountPending (after digit 1-9)
//!
//! GotoPending      -> BaseState (after 'g'/'h'/'l'/'s'/'e' or cancel)
//! ViewPending      -> BaseState (after 'z'/'t'/'b'/'m'/'j'/'k' or cancel)
//! MatchPending     -> BaseState (after 'm' or cancel)
//! FindCharPending  -> BaseState (after any char or cancel)
//! ReplaceCharPending -> BaseState (after any char or cancel)
//! CountPending     -> BaseState (after command char or cancel)
//! ```
//!
//! # Example
//!
//! ```ignore
//! use helix_trainer::input::typestate::{InputState, InputStateMachine};
//!
//! let mut state_machine = InputStateMachine::new();
//! assert!(state_machine.state().is_base());
//!
//! // Press 'g' - transition to GotoPending
//! let result = state_machine.process_key(KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE));
//! assert!(state_machine.state().is_goto_pending());
//!
//! // Press 'g' again - complete "gg" command
//! let result = state_machine.process_key(KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE));
//! assert!(matches!(result, HandlerResult::Execute(_)));
//! assert!(state_machine.state().is_base());
//! ```

pub mod handler_result;
pub mod handlers;
pub mod input_state;
pub mod key_mapping;
pub mod state_machine;
pub mod state_types;
pub mod typestate_wrapper;

#[cfg(test)]
mod tests;

// Re-export public API
pub use handler_result::HandlerResult;
pub use handlers::{InputHandler, KeyHandler};
pub use input_state::InputState;
pub use key_mapping::{
    command_to_key_event, handle_insert_mode_input, map_key_to_helix_command, normalize_key_event,
    parse_helix_key_string,
};
pub use state_machine::{InputStateMachine, SurroundPreview};
pub use state_types::{
    BaseState, CountPending, FindCharPending, FindType, GotoPending, HandlerState, MatchPending,
    ReplaceCharPending, SurroundAddPending, SurroundDeletePending, SurroundReplaceFromPending,
    SurroundReplaceToPending, TextObjectAroundPending, TextObjectInsidePending,
    UnmatchedNextPending, UnmatchedPrevPending, ViewPending,
};
pub use typestate_wrapper::{TypestateHandler, TypestateHandlerState};
