//! Input handler implementations for each state
//!
//! Each state has its own handler implementing the `InputHandler` trait.
//! The handlers are dispatched based on the current state.

mod base;
mod command_line;
pub mod count;
mod find;
mod goto;
mod match_mode;
mod regex_prompt;
mod register;
mod replace;
mod text_object;
mod unmatched;
mod view;

use crossterm::event::KeyEvent;

use super::handler_result::HandlerResult;
use super::state_types::HandlerState;
use crate::helix::normal_command::{NormalCommand, RegisteredCommand};

/// Run the registered command `key`; a key no command is registered under
/// cancels (a test checks that every command constant the handlers name is
/// registered).
fn execute_registered(key: &'static str) -> HandlerResult {
    RegisteredCommand::lookup(key).map_or(HandlerResult::Cancel, |command| {
        HandlerResult::Execute(NormalCommand::Registered {
            command,
            count: None,
        })
    })
}

/// Run the command a canonical key string names (a single key's command: a
/// registered command, an Insert entry or `Escape`).
fn execute_key(key: &'static str) -> HandlerResult {
    NormalCommand::parse(key).map_or(HandlerResult::Cancel, HandlerResult::Execute)
}

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

/// Marker struct for implementing `InputHandler`
#[derive(Debug, Clone, Copy)]
pub struct KeyHandler;
