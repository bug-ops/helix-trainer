//! `GotoPending` handler - handles input after 'g' prefix
//!
//! Processes the second key of goto commands like 'gg', 'gh', 'gl', etc.

use std::borrow::Cow;

use crossterm::event::{KeyCode, KeyEvent};

use crate::helix::commands::{
    CMD_GOTO_FILE_START, CMD_GOTO_FIRST_NONWHITESPACE, CMD_GOTO_LAST_LINE, CMD_GOTO_LINE_END,
    CMD_GOTO_LINE_START,
};

use super::count::goto_line_command;
use super::{InputHandler, KeyHandler};
use crate::input::typestate::{
    handler_result::HandlerResult,
    state_types::{CountedGotoPending, GotoPending},
};

impl InputHandler<GotoPending> for KeyHandler {
    fn handle_key(_state: &GotoPending, key: KeyEvent) -> HandlerResult {
        match key.code {
            // 'gg' - goto file start
            KeyCode::Char('g') => HandlerResult::Execute(Cow::Borrowed(CMD_GOTO_FILE_START)),
            // 'gh' - goto line start
            KeyCode::Char('h') => HandlerResult::Execute(Cow::Borrowed(CMD_GOTO_LINE_START)),
            // 'gl' - goto line end
            KeyCode::Char('l') => HandlerResult::Execute(Cow::Borrowed(CMD_GOTO_LINE_END)),
            // 'gs' - goto first non-whitespace
            KeyCode::Char('s') => {
                HandlerResult::Execute(Cow::Borrowed(CMD_GOTO_FIRST_NONWHITESPACE))
            }
            // 'ge' - goto last line
            KeyCode::Char('e') => HandlerResult::Execute(Cow::Borrowed(CMD_GOTO_LAST_LINE)),
            // Escape - cancel
            KeyCode::Esc => HandlerResult::Cancel,
            // Invalid second key - cancel
            _ => HandlerResult::Cancel,
        }
    }
}

impl InputHandler<CountedGotoPending> for KeyHandler {
    fn handle_key(state: &CountedGotoPending, key: KeyEvent) -> HandlerResult {
        match key.code {
            KeyCode::Char('g') => {
                HandlerResult::Execute(Cow::Owned(goto_line_command(state.count)))
            }
            _ => HandlerResult::Cancel,
        }
    }
}
