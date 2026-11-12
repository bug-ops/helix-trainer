//! `ViewPending` handler - handles input after 'z' prefix
//!
//! Processes the second key of view commands like 'zz', 'zt', 'zb', etc.

use crossterm::event::{KeyCode, KeyEvent};

use crate::grammar::commands::{
    CMD_SCROLL_DOWN, CMD_SCROLL_UP, CMD_VIEW_BOTTOM, CMD_VIEW_CENTER, CMD_VIEW_CENTER_HORIZONTAL,
    CMD_VIEW_TOP,
};

use super::{InputHandler, KeyHandler, execute_registered};
use crate::input::typestate::{handler_result::HandlerResult, state_types::ViewPending};

impl InputHandler<ViewPending> for KeyHandler {
    #[expect(
        clippy::match_same_arms,
        reason = "arms stay apart to document each case"
    )]
    fn handle_key(_state: &ViewPending, key: KeyEvent) -> HandlerResult {
        match key.code {
            // 'zz' - center view
            KeyCode::Char('z') => execute_registered(CMD_VIEW_CENTER),
            // 'zt' - view top
            KeyCode::Char('t') => execute_registered(CMD_VIEW_TOP),
            // 'zb' - view bottom
            KeyCode::Char('b') => execute_registered(CMD_VIEW_BOTTOM),
            // 'zm' - view center horizontal
            KeyCode::Char('m') => execute_registered(CMD_VIEW_CENTER_HORIZONTAL),
            // 'zj' - scroll down
            KeyCode::Char('j') => execute_registered(CMD_SCROLL_DOWN),
            // 'zk' - scroll up
            KeyCode::Char('k') => execute_registered(CMD_SCROLL_UP),
            // Escape - cancel
            KeyCode::Esc => HandlerResult::Cancel,
            // Invalid second key - cancel
            _ => HandlerResult::Cancel,
        }
    }
}
