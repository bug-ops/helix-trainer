//! `CountPending` handler - handles count prefix building
//!
//! Accumulates digits and then applies the count to the following command.
//! `g`, `G`, `f`, `F`, `t` and `T` carry the count into a dedicated pending
//! state ([`CountedGotoPending`](crate::input::typestate::CountedGotoPending),
//! [`CountedFindCharPending`](crate::input::typestate::CountedFindCharPending))
//! or emit the counted command directly, so the count is never dropped (#493).
//! `r` opens [`ReplaceCharPending`](crate::input::typestate::ReplaceCharPending)
//! and drops the count, which Helix's `replace` ignores (#577).

use std::num::NonZeroUsize;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::{InputHandler, KeyHandler};
use crate::grammar::find::FindMotion;
use crate::helix::normal_command::NormalCommand;
use crate::input::typestate::{
    handler_result::HandlerResult,
    input_state::InputState,
    key_mapping::{
        Chord, is_count_compatible_command, map_single_key_command, normalize_key_event,
    },
    state_types::CountPending,
};

/// Maximum count value to prevent overflow attacks and unreasonable values.
/// Equal to the largest count prefix the simulator accepts, so the input layer
/// can never emit a counted command the simulator rejects.
pub const MAX_COUNT: usize = crate::helix::registry::MAX_COUNT_PREFIX;

/// The count started by digit key `c`; `None` for `0` and non-digits, since a
/// count can never begin with `0`.
pub(crate) fn count_from_digit(c: char) -> Option<NonZeroUsize> {
    c.to_digit(10)
        .and_then(|digit| NonZeroUsize::new(digit as usize))
}

fn counted_command(count: NonZeroUsize, c: char, modifiers: KeyModifiers) -> HandlerResult {
    if !is_count_compatible_command(c, modifiers) {
        return HandlerResult::Cancel;
    }
    map_single_key_command(c, modifiers).map_or(HandlerResult::Cancel, |cmd| {
        NormalCommand::parse(&format!("{count}{cmd}"))
            .map_or(HandlerResult::Cancel, HandlerResult::Execute)
    })
}

impl InputHandler<CountPending> for KeyHandler {
    fn handle_key(state: &CountPending, key: KeyEvent) -> HandlerResult {
        let key = normalize_key_event(key);
        match key.code {
            // More digits - continue building count
            KeyCode::Char(c @ '0'..='9') => {
                let digit = c.to_digit(10).map_or(0, |d| d as usize);
                let new_count = state
                    .count
                    .get()
                    .saturating_mul(10)
                    .saturating_add(digit)
                    .min(MAX_COUNT);
                HandlerResult::Transition(InputState::CountPending {
                    count: NonZeroUsize::new(new_count).unwrap_or(state.count),
                })
            }
            KeyCode::Char(c) if Chord::of(key.modifiers) == Chord::Plain => {
                let count = state.count;
                match c {
                    'g' => HandlerResult::Transition(InputState::CountedGotoPending { count }),
                    'G' => HandlerResult::Execute(NormalCommand::GotoLine { count: Some(count) }),
                    'r' => HandlerResult::Transition(InputState::ReplaceCharPending),
                    _ => FindMotion::from_key(c).map_or_else(
                        || counted_command(count, c, key.modifiers),
                        |motion| {
                            HandlerResult::Transition(InputState::CountedFindCharPending {
                                motion,
                                count,
                            })
                        },
                    ),
                }
            }
            KeyCode::Char(c) => counted_command(state.count, c, key.modifiers),
            // Escape and other keys - cancel
            _ => HandlerResult::Cancel,
        }
    }
}
