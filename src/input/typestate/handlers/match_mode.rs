//! MatchPending and surround handlers
//!
//! Handles input after 'm' prefix for match/surround commands:
//! - 'mm' - match brackets
//! - 'ms' - surround add
//! - 'md' - surround delete
//! - 'mr' - surround replace
//! - 'ma' - text object around
//! - 'mi' - text object inside

use std::borrow::Cow;

use crossterm::event::{KeyCode, KeyEvent};

use crate::helix::commands::*;

use super::{InputHandler, KeyHandler};
use crate::input::typestate::{
    handler_result::HandlerResult,
    input_state::InputState,
    state_types::{
        MatchPending, SurroundAddPending, SurroundDeletePending, SurroundReplaceFromPending,
        SurroundReplaceToPending,
    },
};

// ============================================================================
// MatchPending handler (after 'm')
// ============================================================================

impl InputHandler<MatchPending> for KeyHandler {
    fn handle_key(_state: &MatchPending, key: KeyEvent) -> HandlerResult {
        match key.code {
            // 'mm' - match brackets
            KeyCode::Char('m') => HandlerResult::Execute(Cow::Borrowed(CMD_MATCH_BRACKETS)),
            // 'ms' - surround add (transition to SurroundAddPending)
            KeyCode::Char('s') => HandlerResult::Transition(InputState::SurroundAddPending),
            // 'md' - surround delete (transition to SurroundDeletePending)
            KeyCode::Char('d') => HandlerResult::Transition(InputState::SurroundDeletePending),
            // 'mr' - surround replace (transition to SurroundReplaceFromPending)
            KeyCode::Char('r') => HandlerResult::Transition(InputState::SurroundReplaceFromPending),
            // 'ma' - text object around (transition to TextObjectAroundPending)
            KeyCode::Char('a') => HandlerResult::Transition(InputState::TextObjectAroundPending),
            // 'mi' - text object inside (transition to TextObjectInsidePending)
            KeyCode::Char('i') => HandlerResult::Transition(InputState::TextObjectInsidePending),
            // Escape - cancel
            KeyCode::Esc => HandlerResult::Cancel,
            // Invalid second key - cancel
            _ => HandlerResult::Cancel,
        }
    }
}

// ============================================================================
// SurroundAddPending handler (after 'ms')
// ============================================================================

impl InputHandler<SurroundAddPending> for KeyHandler {
    fn handle_key(_state: &SurroundAddPending, key: KeyEvent) -> HandlerResult {
        match key.code {
            // Accept any printable character for surrounding
            KeyCode::Char(c) => {
                let cmd = format!("ms{}", c);
                HandlerResult::Execute(Cow::Owned(cmd))
            }
            // Escape - cancel
            KeyCode::Esc => HandlerResult::Cancel,
            // Other keys - cancel
            _ => HandlerResult::Cancel,
        }
    }
}

// ============================================================================
// SurroundDeletePending handler (after 'md')
// ============================================================================

impl InputHandler<SurroundDeletePending> for KeyHandler {
    fn handle_key(_state: &SurroundDeletePending, key: KeyEvent) -> HandlerResult {
        match key.code {
            // Accept any printable character for deletion target
            KeyCode::Char(c) => {
                let cmd = format!("md{}", c);
                HandlerResult::Execute(Cow::Owned(cmd))
            }
            // Escape - cancel
            KeyCode::Esc => HandlerResult::Cancel,
            // Other keys - cancel
            _ => HandlerResult::Cancel,
        }
    }
}

// ============================================================================
// SurroundReplaceFromPending handler (after 'mr')
// ============================================================================

impl InputHandler<SurroundReplaceFromPending> for KeyHandler {
    fn handle_key(_state: &SurroundReplaceFromPending, key: KeyEvent) -> HandlerResult {
        match key.code {
            // Accept any printable character as "from" character
            KeyCode::Char(c) => {
                HandlerResult::Transition(InputState::SurroundReplaceToPending { from_char: c })
            }
            // Escape - cancel
            KeyCode::Esc => HandlerResult::Cancel,
            // Other keys - cancel
            _ => HandlerResult::Cancel,
        }
    }
}

// ============================================================================
// SurroundReplaceToPending handler (after 'mr{from}')
// ============================================================================

impl InputHandler<SurroundReplaceToPending> for KeyHandler {
    fn handle_key(state: &SurroundReplaceToPending, key: KeyEvent) -> HandlerResult {
        match key.code {
            // Accept any printable character as "to" character
            KeyCode::Char(c) => {
                let cmd = format!("mr{}{}", state.from_char, c);
                HandlerResult::Execute(Cow::Owned(cmd))
            }
            // Escape - cancel
            KeyCode::Esc => HandlerResult::Cancel,
            // Other keys - cancel
            _ => HandlerResult::Cancel,
        }
    }
}
