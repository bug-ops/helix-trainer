//! `FindCharPending` handler - handles input after 'f'/'F'/'t'/'T'
//!
//! Waits for a character to search for and produces the full command.

use crossterm::event::{KeyCode, KeyEvent};

use super::{InputHandler, KeyHandler};
use crate::helix::normal_command::NormalCommand;
use crate::input::typestate::{
    handler_result::HandlerResult,
    state_types::{CountedFindCharPending, FindCharPending},
};

impl InputHandler<FindCharPending> for KeyHandler {
    fn handle_key(state: &FindCharPending, key: KeyEvent) -> HandlerResult {
        match key.code {
            // Accept any printable character
            KeyCode::Char(ch) => HandlerResult::Execute(NormalCommand::Find {
                motion: state.motion,
                ch,
                count: None,
            }),
            // Escape - cancel
            KeyCode::Esc => HandlerResult::Cancel,
            // Other keys - cancel
            _ => HandlerResult::Cancel,
        }
    }
}

impl InputHandler<CountedFindCharPending> for KeyHandler {
    fn handle_key(state: &CountedFindCharPending, key: KeyEvent) -> HandlerResult {
        match key.code {
            KeyCode::Char(ch) => HandlerResult::Execute(NormalCommand::Find {
                motion: state.motion,
                ch,
                count: Some(state.count),
            }),
            _ => HandlerResult::Cancel,
        }
    }
}
