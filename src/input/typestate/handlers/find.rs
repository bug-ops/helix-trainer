//! FindCharPending handler - handles input after 'f'/'F'/'t'/'T'
//!
//! Waits for a character to search for and produces the full command.

use std::borrow::Cow;

use crossterm::event::{KeyCode, KeyEvent};

use super::{InputHandler, KeyHandler};
use crate::input::typestate::{handler_result::HandlerResult, state_types::FindCharPending};

impl InputHandler<FindCharPending> for KeyHandler {
    fn handle_key(state: &FindCharPending, key: KeyEvent) -> HandlerResult {
        match key.code {
            // Accept any printable character
            KeyCode::Char(c) => {
                // Dynamic command with character argument requires allocation
                let cmd = format!("{}{}", state.find_type.prefix(), c);
                HandlerResult::Execute(Cow::Owned(cmd))
            }
            // Escape - cancel
            KeyCode::Esc => HandlerResult::Cancel,
            // Other keys - cancel
            _ => HandlerResult::Cancel,
        }
    }
}
