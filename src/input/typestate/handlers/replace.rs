//! ReplaceCharPending handler - handles input after 'r'
//!
//! Waits for a character to replace with and produces the full command.

use std::borrow::Cow;

use crossterm::event::{KeyCode, KeyEvent};

use super::{InputHandler, KeyHandler};
use crate::input::typestate::{handler_result::HandlerResult, state_types::ReplaceCharPending};

impl InputHandler<ReplaceCharPending> for KeyHandler {
    fn handle_key(_state: &ReplaceCharPending, key: KeyEvent) -> HandlerResult {
        match key.code {
            // Accept any printable character (including space, newline, etc.)
            KeyCode::Char(c) => {
                // Dynamic command with character argument requires allocation
                let cmd = format!("r{}", c);
                HandlerResult::Execute(Cow::Owned(cmd))
            }
            // Enter - replace with newline
            KeyCode::Enter => HandlerResult::Execute(Cow::Borrowed("r\n")),
            // Escape - cancel
            KeyCode::Esc => HandlerResult::Cancel,
            // Other keys - cancel
            _ => HandlerResult::Cancel,
        }
    }
}
