//! `UnmatchedPrevPending` and `UnmatchedNextPending` handlers
//!
//! Handles input after `[` and `]` prefixes for unmatched bracket navigation.

use crossterm::event::{KeyCode, KeyEvent};

use crate::grammar::commands::{CMD_GOTO_NEXT_PARAGRAPH, CMD_GOTO_PREV_PARAGRAPH};

use super::{InputHandler, KeyHandler, execute_registered};
use crate::input::typestate::{
    handler_result::HandlerResult,
    state_types::{UnmatchedNextPending, UnmatchedPrevPending},
};

// ============================================================================
// UnmatchedPrevPending handler (after '[')
// ============================================================================

impl InputHandler<UnmatchedPrevPending> for KeyHandler {
    #[expect(
        clippy::match_same_arms,
        reason = "arms stay apart to document each case"
    )]
    fn handle_key(_state: &UnmatchedPrevPending, key: KeyEvent) -> HandlerResult {
        match key.code {
            // '[p' - goto previous paragraph
            KeyCode::Char('p') => execute_registered(CMD_GOTO_PREV_PARAGRAPH),
            // Escape - cancel
            KeyCode::Esc => HandlerResult::Cancel,
            // Other keys - cancel (not recognized)
            _ => HandlerResult::Cancel,
        }
    }
}

// ============================================================================
// UnmatchedNextPending handler (after ']')
// ============================================================================

impl InputHandler<UnmatchedNextPending> for KeyHandler {
    #[expect(
        clippy::match_same_arms,
        reason = "arms stay apart to document each case"
    )]
    fn handle_key(_state: &UnmatchedNextPending, key: KeyEvent) -> HandlerResult {
        match key.code {
            // ']p' - goto next paragraph
            KeyCode::Char('p') => execute_registered(CMD_GOTO_NEXT_PARAGRAPH),
            // Escape - cancel
            KeyCode::Esc => HandlerResult::Cancel,
            // Other keys - cancel (not recognized)
            _ => HandlerResult::Cancel,
        }
    }
}
