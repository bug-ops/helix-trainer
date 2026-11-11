//! `ReplaceCharPending` handler - handles input after 'r'
//!
//! Waits for a character to replace with and produces the full command.

use crossterm::event::{KeyCode, KeyEvent};

use super::{InputHandler, KeyHandler};
use crate::helix::normal_command::NormalCommand;
use crate::input::typestate::{handler_result::HandlerResult, state_types::ReplaceCharPending};

impl InputHandler<ReplaceCharPending> for KeyHandler {
    fn handle_key(_state: &ReplaceCharPending, key: KeyEvent) -> HandlerResult {
        match key.code {
            // Accept any printable character (including space, newline, etc.)
            KeyCode::Char(c) => HandlerResult::Execute(NormalCommand::Replace(c)),
            // Enter - replace with newline
            KeyCode::Enter => HandlerResult::Execute(NormalCommand::Replace('\n')),
            // Escape - cancel
            KeyCode::Esc => HandlerResult::Cancel,
            // Other keys - cancel
            _ => HandlerResult::Cancel,
        }
    }
}
