//! ViewPending handler - handles input after 'z' prefix
//!
//! Processes the second key of view commands like 'zz', 'zt', 'zb', etc.

use std::borrow::Cow;

use crossterm::event::{KeyCode, KeyEvent};

use crate::helix::commands::*;

use super::{InputHandler, KeyHandler};
use crate::input::typestate::{handler_result::HandlerResult, state_types::ViewPending};

impl InputHandler<ViewPending> for KeyHandler {
    fn handle_key(_state: &ViewPending, key: KeyEvent) -> HandlerResult {
        match key.code {
            // 'zz' - center view
            KeyCode::Char('z') => HandlerResult::Execute(Cow::Borrowed(CMD_VIEW_CENTER)),
            // 'zt' - view top
            KeyCode::Char('t') => HandlerResult::Execute(Cow::Borrowed(CMD_VIEW_TOP)),
            // 'zb' - view bottom
            KeyCode::Char('b') => HandlerResult::Execute(Cow::Borrowed(CMD_VIEW_BOTTOM)),
            // 'zm' - view center horizontal
            KeyCode::Char('m') => HandlerResult::Execute(Cow::Borrowed(CMD_VIEW_CENTER_HORIZONTAL)),
            // 'zj' - scroll down
            KeyCode::Char('j') => HandlerResult::Execute(Cow::Borrowed(CMD_SCROLL_DOWN)),
            // 'zk' - scroll up
            KeyCode::Char('k') => HandlerResult::Execute(Cow::Borrowed(CMD_SCROLL_UP)),
            // Escape - cancel
            KeyCode::Esc => HandlerResult::Cancel,
            // Invalid second key - cancel
            _ => HandlerResult::Cancel,
        }
    }
}
