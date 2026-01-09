//! Input handler implementations for each state
//!
//! Each state has its own handler implementing the InputHandler trait.
//! The handlers are dispatched based on the current state.

mod base;
pub mod count;
mod find;
mod goto;
mod match_mode;
mod replace;
mod text_object;
mod unmatched;
mod view;

use crossterm::event::KeyEvent;

use super::handler_result::HandlerResult;
use super::state_types::HandlerState;

/// Trait for handling input in a specific state
///
/// This trait uses the typestate pattern to encode the current state at the
/// type level, ensuring compile-time safety for state transitions.
pub trait InputHandler<S: HandlerState> {
    /// Handle a key event in this state
    ///
    /// Returns a `HandlerResult` indicating what action to take.
    fn handle_key(state: &S, key: KeyEvent) -> HandlerResult;
}

/// Marker struct for implementing InputHandler
#[derive(Debug, Clone, Copy)]
pub struct KeyHandler;
