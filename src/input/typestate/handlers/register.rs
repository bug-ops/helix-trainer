//! `RegisterPending` and `RegisterOpPending` handlers
//!
//! Handles input after the `"` prefix for named-register-scoped clipboard
//! operations:
//! - `"` - select a register (waiting for the register character)
//! - `"{register}` - register selected (waiting for y/p/P/R/d/c)
//!
//! Register scope is deliberately limited to the six commands that can be
//! targeted at a *named* register via this `"<reg>` prefix (`y`, `p`, `P`,
//! `R`, `d`, `c`); any other key cancels rather than executing a bare
//! command, keeping `"<reg><op>` an atomic 3-key sequence with no partial
//! side effects. `d` and `c` also read/write the *default* (unnamed)
//! register directly when invoked bare; routing them through this prefix
//! (e.g. `"_d`, `"_c`) is how the blackhole register
//! ([`Register::Blackhole`]) is reached, since it can't be addressed as the
//! default register.

use crossterm::event::{KeyCode, KeyEvent};

use super::{InputHandler, KeyHandler};
use crate::helix::normal_command::{NormalCommand, RegisterOp};
use crate::helix::register::Register;
use crate::helix::simulator::InsertEntryCommand;
use crate::input::typestate::{
    handler_result::HandlerResult,
    input_state::InputState,
    key_mapping::Chord,
    state_types::{BaseState, RegisterOpPending, RegisterPending},
};

// ============================================================================
// RegisterPending handler (after '"')
// ============================================================================

impl InputHandler<RegisterPending> for KeyHandler {
    #[expect(
        clippy::match_same_arms,
        reason = "arms stay apart to document each case"
    )]
    fn handle_key(_state: &RegisterPending, key: KeyEvent) -> HandlerResult {
        match key.code {
            // Every character names a register; `Register` sorts out the special ones
            KeyCode::Char(c) => HandlerResult::Transition(InputState::RegisterOpPending {
                register: Register::from(c),
            }),
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
        // Helix drops the pending register and runs the chord as its own command
        if Chord::of(key.modifiers) != Chord::Plain {
            return match KeyHandler::handle_key(&BaseState, key) {
                HandlerResult::Stay => HandlerResult::Cancel,
                other => other,
            };
        }
        match key.code {
            // Only y/p/P/R/d/c are register-scoped in this trainer
            KeyCode::Char('c') => {
                HandlerResult::Execute(NormalCommand::InsertEntry(InsertEntryCommand::Change {
                    register: state.register,
                }))
            }
            KeyCode::Char(op) if let Some(op) = RegisterOp::from_key(op) => {
                HandlerResult::Execute(NormalCommand::Register {
                    register: state.register,
                    op,
                })
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

    use crate::testing::normal;

    fn key(c: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE)
    }

    #[test]
    fn register_pending_transitions_to_op_pending() {
        let result = KeyHandler::handle_key(&RegisterPending, key('a'));
        assert_eq!(
            result,
            HandlerResult::Transition(InputState::RegisterOpPending {
                register: Register::from('a')
            })
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
        let state = RegisterOpPending {
            register: Register::from('a'),
        };
        let result = KeyHandler::handle_key(&state, key('y'));
        assert_eq!(result, HandlerResult::Execute(normal("\"ay")));
    }

    #[test]
    fn register_op_pending_accepts_paste_after_before_and_replace() {
        let state = RegisterOpPending {
            register: Register::from('b'),
        };
        assert_eq!(
            KeyHandler::handle_key(&state, key('p')),
            HandlerResult::Execute(normal("\"bp"))
        );
        assert_eq!(
            KeyHandler::handle_key(&state, key('P')),
            HandlerResult::Execute(normal("\"bP"))
        );
        assert_eq!(
            KeyHandler::handle_key(&state, key('R')),
            HandlerResult::Execute(normal("\"bR"))
        );
    }

    #[test]
    fn register_op_pending_accepts_delete_and_change() {
        let state = RegisterOpPending {
            register: Register::from('a'),
        };
        assert_eq!(
            KeyHandler::handle_key(&state, key('d')),
            HandlerResult::Execute(normal("\"ad"))
        );
        assert_eq!(
            KeyHandler::handle_key(&state, key('c')),
            HandlerResult::Execute(normal("\"ac"))
        );
    }

    #[test]
    fn register_op_pending_accepts_blackhole_register_delete_and_change() {
        let state = RegisterOpPending {
            register: Register::from('_'),
        };
        assert_eq!(
            KeyHandler::handle_key(&state, key('d')),
            HandlerResult::Execute(normal("\"_d"))
        );
        assert_eq!(
            KeyHandler::handle_key(&state, key('c')),
            HandlerResult::Execute(normal("\"_c"))
        );
    }

    #[test]
    fn register_op_pending_rejects_out_of_scope_operator() {
        let state = RegisterOpPending {
            register: Register::from('a'),
        };
        assert_eq!(
            KeyHandler::handle_key(&state, key('x')),
            HandlerResult::Cancel
        );
    }

    fn modified(c: char, modifiers: KeyModifiers) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(c), modifiers)
    }

    #[test]
    fn register_op_pending_runs_ctrl_key_as_its_own_command_and_drops_register() {
        let state = RegisterOpPending {
            register: Register::from('a'),
        };
        for c in ['c', 'b', 'f', 'u', 'd', 'r'] {
            let key = modified(c, KeyModifiers::CONTROL);
            assert_eq!(
                KeyHandler::handle_key(&state, key),
                KeyHandler::handle_key(&BaseState, key),
                "Ctrl-{c}"
            );
        }
        assert_eq!(
            KeyHandler::handle_key(&state, modified('d', KeyModifiers::CONTROL)),
            HandlerResult::Execute(normal("Ctrl-d"))
        );
    }

    #[test]
    fn register_op_pending_cancels_unbound_ctrl_key() {
        let state = RegisterOpPending {
            register: Register::from('a'),
        };
        assert_eq!(
            KeyHandler::handle_key(&state, modified('x', KeyModifiers::CONTROL)),
            HandlerResult::Cancel
        );
    }

    #[test]
    fn register_op_pending_runs_alt_key_as_its_own_command() {
        let state = RegisterOpPending {
            register: Register::from('a'),
        };
        let key = modified('c', KeyModifiers::ALT);
        assert_eq!(
            KeyHandler::handle_key(&state, key),
            KeyHandler::handle_key(&BaseState, key)
        );
        assert_ne!(
            KeyHandler::handle_key(&state, key),
            HandlerResult::Execute(normal("\"ac"))
        );
    }

    #[test]
    fn register_op_pending_escape_cancels() {
        let state = RegisterOpPending {
            register: Register::from('a'),
        };
        let result =
            KeyHandler::handle_key(&state, KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert_eq!(result, HandlerResult::Cancel);
    }
}
