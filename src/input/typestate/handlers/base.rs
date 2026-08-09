//! BaseState handler - handles input when no prefix is active
//!
//! This is the default state handler that processes single-key commands and
//! transitions to pending states for multi-key sequences.

use std::borrow::Cow;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::helix::commands::*;

use super::{InputHandler, KeyHandler};
use crate::input::typestate::{
    handler_result::HandlerResult,
    input_state::InputState,
    key_mapping::{map_single_key_command, normalize_key_event},
    state_types::{BaseState, FindType},
};

impl InputHandler<BaseState> for KeyHandler {
    fn handle_key(_state: &BaseState, key: KeyEvent) -> HandlerResult {
        // Normalize key event for consistent handling across terminals
        // (e.g., 'c' + SHIFT + ALT → 'C' + ALT)
        let key = normalize_key_event(key);

        // Only filter CONTROL modifier, let ALT through for Alt-key commands
        let has_ctrl = key.modifiers.contains(KeyModifiers::CONTROL);

        match (key.code, has_ctrl) {
            // Special modifiers - let through for Ctrl-R, Ctrl-C, etc.
            (KeyCode::Char('r'), true) if key.modifiers.contains(KeyModifiers::CONTROL) => {
                HandlerResult::Execute(Cow::Borrowed(CMD_CTRL_R))
            }
            (KeyCode::Char('c'), true) if key.modifiers.contains(KeyModifiers::CONTROL) => {
                HandlerResult::Execute(Cow::Borrowed(CMD_TOGGLE_COMMENTS))
            }

            // Page/half-page movement - let through for Ctrl-b/f/u/d
            (KeyCode::Char('b'), true) => HandlerResult::Execute(Cow::Borrowed(CMD_PAGE_UP)),
            (KeyCode::Char('f'), true) => HandlerResult::Execute(Cow::Borrowed(CMD_PAGE_DOWN)),
            (KeyCode::Char('u'), true) => HandlerResult::Execute(Cow::Borrowed(CMD_HALF_PAGE_UP)),
            (KeyCode::Char('d'), true) => HandlerResult::Execute(Cow::Borrowed(CMD_HALF_PAGE_DOWN)),

            // Ignore other modifier combinations
            (_, true) => HandlerResult::Stay,

            // Prefix commands - transition to pending states
            (KeyCode::Char('g'), false) => HandlerResult::Transition(InputState::GotoPending),
            (KeyCode::Char('z'), false) => HandlerResult::Transition(InputState::ViewPending),
            (KeyCode::Char('m'), false) => HandlerResult::Transition(InputState::MatchPending),
            (KeyCode::Char('['), false) => {
                HandlerResult::Transition(InputState::UnmatchedPrevPending)
            }
            (KeyCode::Char(']'), false) => {
                HandlerResult::Transition(InputState::UnmatchedNextPending)
            }

            // Find/till commands - transition to find char pending
            (KeyCode::Char('f'), false) => HandlerResult::Transition(InputState::FindCharPending {
                find_type: FindType::FindForward,
            }),
            (KeyCode::Char('F'), _) => HandlerResult::Transition(InputState::FindCharPending {
                find_type: FindType::FindBackward,
            }),
            (KeyCode::Char('t'), false) => HandlerResult::Transition(InputState::FindCharPending {
                find_type: FindType::TillForward,
            }),
            (KeyCode::Char('T'), _) => HandlerResult::Transition(InputState::FindCharPending {
                find_type: FindType::TillBackward,
            }),

            // Replace command - transition to replace char pending
            (KeyCode::Char('r'), false) => {
                HandlerResult::Transition(InputState::ReplaceCharPending)
            }

            // Named register selection - transition to register pending
            (KeyCode::Char('"'), false) => HandlerResult::Transition(InputState::RegisterPending),

            // Command-line mode - transition to command-line pending
            (KeyCode::Char(':'), false) => {
                HandlerResult::Transition(InputState::CommandLinePending {
                    buffer: String::new(),
                })
            }

            // Count prefix - digits 1-9 start a count
            (KeyCode::Char(c @ '1'..='9'), false) => {
                let count = c
                    .to_digit(10)
                    .expect("pattern match guarantees ASCII digit")
                    as usize;
                HandlerResult::Transition(InputState::CountPending { count })
            }

            // Single-key commands - execute immediately
            (KeyCode::Char(c), _) => {
                if let Some(cmd) = map_single_key_command(c, key.modifiers) {
                    HandlerResult::Execute(Cow::Borrowed(cmd))
                } else {
                    // Unknown key - stay in base state
                    HandlerResult::Stay
                }
            }

            // Escape and other special keys
            (KeyCode::Esc, _) => HandlerResult::Execute(Cow::Borrowed(CMD_ESCAPE)),

            // Unknown keys - stay in base state
            _ => HandlerResult::Stay,
        }
    }
}
