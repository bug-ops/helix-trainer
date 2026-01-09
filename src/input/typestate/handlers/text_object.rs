//! TextObject handlers - handles text object selection after 'ma'/'mi'
//!
//! Processes text object types like 'w' (word), '(' (parentheses), etc.

use std::borrow::Cow;

use crossterm::event::{KeyCode, KeyEvent};

use super::{InputHandler, KeyHandler};
use crate::input::typestate::{
    handler_result::HandlerResult,
    state_types::{TextObjectAroundPending, TextObjectInsidePending},
};

// ============================================================================
// TextObjectAroundPending handler (after 'ma')
// ============================================================================

impl InputHandler<TextObjectAroundPending> for KeyHandler {
    fn handle_key(_state: &TextObjectAroundPending, key: KeyEvent) -> HandlerResult {
        match key.code {
            // Valid text objects: w, W, (, ), [, ], {, }, <, >, ", ', `, p
            KeyCode::Char(
                c @ ('w' | 'W' | '(' | ')' | '[' | ']' | '{' | '}' | '<' | '>' | '"' | '\'' | '`'
                | 'p'),
            ) => {
                let cmd = format!("ma{}", c);
                HandlerResult::Execute(Cow::Owned(cmd))
            }
            // Escape - cancel
            KeyCode::Esc => HandlerResult::Cancel,
            // Invalid text object - cancel
            _ => HandlerResult::Cancel,
        }
    }
}

// ============================================================================
// TextObjectInsidePending handler (after 'mi')
// ============================================================================

impl InputHandler<TextObjectInsidePending> for KeyHandler {
    fn handle_key(_state: &TextObjectInsidePending, key: KeyEvent) -> HandlerResult {
        match key.code {
            // Valid text objects: w, W, (, ), [, ], {, }, <, >, ", ', `, p
            KeyCode::Char(
                c @ ('w' | 'W' | '(' | ')' | '[' | ']' | '{' | '}' | '<' | '>' | '"' | '\'' | '`'
                | 'p'),
            ) => {
                let cmd = format!("mi{}", c);
                HandlerResult::Execute(Cow::Owned(cmd))
            }
            // Escape - cancel
            KeyCode::Esc => HandlerResult::Cancel,
            // Invalid text object - cancel
            _ => HandlerResult::Cancel,
        }
    }
}
