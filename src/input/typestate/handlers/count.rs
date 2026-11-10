//! CountPending handler - handles count prefix building
//!
//! Accumulates digits and then applies the count to the following command.
//! `g`, `G`, `f`, `F`, `t` and `T` carry the count into a dedicated pending
//! state ([`CountedGotoPending`](crate::input::typestate::CountedGotoPending),
//! [`CountedFindCharPending`](crate::input::typestate::CountedFindCharPending))
//! or emit the counted command directly, so the count is never dropped (#493).

use std::borrow::Cow;
use std::num::NonZeroUsize;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::{InputHandler, KeyHandler};
use crate::input::typestate::{
    handler_result::HandlerResult,
    input_state::InputState,
    key_mapping::{
        Chord, is_count_compatible_command, map_single_key_command, normalize_key_event,
    },
    state_types::{CountPending, FindType},
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

/// Canonical command string for `{count}G` / `{count}gg`.
pub(crate) fn goto_line_command(count: NonZeroUsize) -> String {
    format!("{count}G")
}

fn counted_command(count: NonZeroUsize, c: char, modifiers: KeyModifiers) -> HandlerResult {
    if !is_count_compatible_command(c, modifiers) {
        return HandlerResult::Cancel;
    }
    match map_single_key_command(c, modifiers) {
        Some(cmd) => HandlerResult::Execute(Cow::Owned(format!("{count}{cmd}"))),
        None => HandlerResult::Cancel,
    }
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
                    'G' => HandlerResult::Execute(Cow::Owned(goto_line_command(count))),
                    _ => match FindType::from_key(c) {
                        Some(find_type) => {
                            HandlerResult::Transition(InputState::CountedFindCharPending {
                                find_type,
                                count,
                            })
                        }
                        None => counted_command(count, c, key.modifiers),
                    },
                }
            }
            KeyCode::Char(c) => counted_command(state.count, c, key.modifiers),
            // Escape and other keys - cancel
            _ => HandlerResult::Cancel,
        }
    }
}
