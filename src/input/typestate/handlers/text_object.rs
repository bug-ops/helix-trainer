//! `TextObject` handlers - handles text object selection after 'ma'/'mi'
//!
//! Processes text object types like 'w' (word), '(' (parentheses), etc.

use crossterm::event::{KeyCode, KeyEvent};

use super::{InputHandler, KeyHandler};
use crate::helix::normal_command::{NormalCommand, TextObjectScope};
use crate::input::typestate::{
    handler_result::HandlerResult,
    state_types::{TextObjectAroundPending, TextObjectInsidePending},
};

// ============================================================================
// TextObjectAroundPending handler (after 'ma')
// ============================================================================

impl InputHandler<TextObjectAroundPending> for KeyHandler {
    #[expect(
        clippy::match_same_arms,
        reason = "arms stay apart to document each case"
    )]
    fn handle_key(_state: &TextObjectAroundPending, key: KeyEvent) -> HandlerResult {
        match key.code {
            // Valid text objects: w, W, (, ), [, ], {, }, <, >, ", ', `, p
            KeyCode::Char(
                c @ ('w' | 'W' | '(' | ')' | '[' | ']' | '{' | '}' | '<' | '>' | '"' | '\'' | '`'
                | 'p'),
            ) => HandlerResult::Execute(NormalCommand::TextObject {
                scope: TextObjectScope::Around,
                object: c,
            }),
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
    #[expect(
        clippy::match_same_arms,
        reason = "arms stay apart to document each case"
    )]
    fn handle_key(_state: &TextObjectInsidePending, key: KeyEvent) -> HandlerResult {
        match key.code {
            // Valid text objects: w, W, (, ), [, ], {, }, <, >, ", ', `, p
            KeyCode::Char(
                c @ ('w' | 'W' | '(' | ')' | '[' | ']' | '{' | '}' | '<' | '>' | '"' | '\'' | '`'
                | 'p'),
            ) => HandlerResult::Execute(NormalCommand::TextObject {
                scope: TextObjectScope::Inside,
                object: c,
            }),
            // Escape - cancel
            KeyCode::Esc => HandlerResult::Cancel,
            // Invalid text object - cancel
            _ => HandlerResult::Cancel,
        }
    }
}
