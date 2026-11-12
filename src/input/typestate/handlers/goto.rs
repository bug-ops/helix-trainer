//! `GotoPending` handler - handles input after 'g' prefix
//!
//! Processes the second key of goto commands like 'gg', 'gh', 'gl', etc.

use crossterm::event::{KeyCode, KeyEvent};

use crate::grammar::commands::{
    CMD_GOTO_FILE_START, CMD_GOTO_FIRST_NONWHITESPACE, CMD_GOTO_LAST_LINE, CMD_GOTO_LINE_END,
    CMD_GOTO_LINE_START,
};

use super::{InputHandler, KeyHandler, execute_registered};
use crate::helix::normal_command::NormalCommand;
use crate::input::typestate::{
    handler_result::HandlerResult,
    state_types::{CountedGotoPending, GotoPending},
};

impl InputHandler<GotoPending> for KeyHandler {
    #[expect(
        clippy::match_same_arms,
        reason = "arms stay apart to document each case"
    )]
    fn handle_key(_state: &GotoPending, key: KeyEvent) -> HandlerResult {
        match key.code {
            // 'gg' - goto file start
            KeyCode::Char('g') => execute_registered(CMD_GOTO_FILE_START),
            // 'gh' - goto line start
            KeyCode::Char('h') => execute_registered(CMD_GOTO_LINE_START),
            // 'gl' - goto line end
            KeyCode::Char('l') => execute_registered(CMD_GOTO_LINE_END),
            // 'gs' - goto first non-whitespace
            KeyCode::Char('s') => execute_registered(CMD_GOTO_FIRST_NONWHITESPACE),
            // 'ge' - goto last line
            KeyCode::Char('e') => execute_registered(CMD_GOTO_LAST_LINE),
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
            KeyCode::Char('g') => HandlerResult::Execute(NormalCommand::GotoLine {
                count: Some(state.count),
            }),
            _ => HandlerResult::Cancel,
        }
    }
}
