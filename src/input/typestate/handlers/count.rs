//! CountPending handler - handles count prefix building
//!
//! Accumulates digits and then applies the count to the following command.

use std::borrow::Cow;

use crossterm::event::{KeyCode, KeyEvent};

use super::{InputHandler, KeyHandler};
use crate::input::typestate::{
    handler_result::HandlerResult,
    input_state::InputState,
    key_mapping::{is_count_compatible_command, map_single_key_command},
    state_types::CountPending,
};

/// Maximum count value to prevent overflow attacks and unreasonable values.
/// 10,000 is more than enough for any practical editing operation.
pub const MAX_COUNT: usize = 10_000;

impl InputHandler<CountPending> for KeyHandler {
    fn handle_key(state: &CountPending, key: KeyEvent) -> HandlerResult {
        match key.code {
            // More digits - continue building count
            KeyCode::Char(c @ '0'..='9') => {
                let digit = c
                    .to_digit(10)
                    .expect("pattern match guarantees ASCII digit")
                    as usize;
                let new_count = state
                    .count
                    .saturating_mul(10)
                    .saturating_add(digit)
                    .min(MAX_COUNT);
                HandlerResult::Transition(InputState::CountPending { count: new_count })
            }
            // Command character - execute with count
            KeyCode::Char(c) => {
                // Only allow certain commands with count prefix
                if is_count_compatible_command(c, key.modifiers) {
                    if let Some(cmd) = map_single_key_command(c, key.modifiers) {
                        // Dynamic command with count prefix requires allocation
                        let full_cmd = format!("{}{}", state.count, cmd);
                        HandlerResult::Execute(Cow::Owned(full_cmd))
                    } else {
                        HandlerResult::Cancel
                    }
                } else {
                    // Invalid - count prefix not allowed with this command
                    HandlerResult::Cancel
                }
            }
            // Escape - cancel count
            KeyCode::Esc => HandlerResult::Cancel,
            // Other keys - cancel
            _ => HandlerResult::Cancel,
        }
    }
}
