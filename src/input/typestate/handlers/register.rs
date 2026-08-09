//! RegisterPending and RegisterOpPending handlers
//!
//! Handles input after the `"` prefix for named-register-scoped clipboard
//! operations:
//! - `"` - select a register (waiting for the register character)
//! - `"{register}` - register selected (waiting for y/p/P/R)
//!
//! Register scope is deliberately limited to the four commands that read or
//! write a register in this trainer (`y`, `p`, `P`, `R`); any other key
//! cancels rather than executing a bare command, keeping `"<reg><op>` an
//! atomic 3-key sequence with no partial side effects.

use std::borrow::Cow;

use crossterm::event::{KeyCode, KeyEvent};

use super::{InputHandler, KeyHandler};
use crate::input::typestate::{
    handler_result::HandlerResult,
    input_state::InputState,
    state_types::{RegisterOpPending, RegisterPending},
};

// ============================================================================
// RegisterPending handler (after '"')
// ============================================================================

impl InputHandler<RegisterPending> for KeyHandler {
    fn handle_key(_state: &RegisterPending, key: KeyEvent) -> HandlerResult {
        match key.code {
            // Accept any character as the register name
            KeyCode::Char(c) => {
                HandlerResult::Transition(InputState::RegisterOpPending { register: c })
            }
            // Escape - cancel
            KeyCode::Esc => HandlerResult::Cancel,
            // Other keys - cancel
            _ => HandlerResult::Cancel,
        }
    }
}

// ============================================================================
// RegisterOpPending handler (after '"{register}')
// ============================================================================

impl InputHandler<RegisterOpPending> for KeyHandler {
    fn handle_key(state: &RegisterOpPending, key: KeyEvent) -> HandlerResult {
        match key.code {
            // Only y/p/P/R are register-scoped in this trainer
            KeyCode::Char(op @ ('y' | 'p' | 'P' | 'R')) => {
                let cmd = format!("\"{}{}", state.register, op);
                HandlerResult::Execute(Cow::Owned(cmd))
            }
            // Any other key - cancel (including Escape)
            _ => HandlerResult::Cancel,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyModifiers;

    fn key(c: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE)
    }

    #[test]
    fn register_pending_transitions_to_op_pending() {
        let result = KeyHandler::handle_key(&RegisterPending, key('a'));
        assert_eq!(
            result,
            HandlerResult::Transition(InputState::RegisterOpPending { register: 'a' })
        );
    }

    #[test]
    fn register_pending_escape_cancels() {
        let result = KeyHandler::handle_key(
            &RegisterPending,
            KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE),
        );
        assert_eq!(result, HandlerResult::Cancel);
    }

    #[test]
    fn register_op_pending_accepts_yank() {
        let state = RegisterOpPending { register: 'a' };
        let result = KeyHandler::handle_key(&state, key('y'));
        assert_eq!(result, HandlerResult::Execute(Cow::Borrowed("\"ay")));
    }

    #[test]
    fn register_op_pending_accepts_paste_after_before_and_replace() {
        let state = RegisterOpPending { register: 'b' };
        assert_eq!(
            KeyHandler::handle_key(&state, key('p')),
            HandlerResult::Execute(Cow::Borrowed("\"bp"))
        );
        assert_eq!(
            KeyHandler::handle_key(&state, key('P')),
            HandlerResult::Execute(Cow::Borrowed("\"bP"))
        );
        assert_eq!(
            KeyHandler::handle_key(&state, key('R')),
            HandlerResult::Execute(Cow::Borrowed("\"bR"))
        );
    }

    #[test]
    fn register_op_pending_rejects_out_of_scope_operator() {
        let state = RegisterOpPending { register: 'a' };
        // 'd' is not register-scoped in this trainer (see architect handoff)
        assert_eq!(
            KeyHandler::handle_key(&state, key('d')),
            HandlerResult::Cancel
        );
        assert_eq!(
            KeyHandler::handle_key(&state, key('x')),
            HandlerResult::Cancel
        );
    }

    #[test]
    fn register_op_pending_escape_cancels() {
        let state = RegisterOpPending { register: 'a' };
        let result =
            KeyHandler::handle_key(&state, KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert_eq!(result, HandlerResult::Cancel);
    }
}
