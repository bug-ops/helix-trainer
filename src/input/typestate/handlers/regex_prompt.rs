//! RegexPromptPending handler
//!
//! Accumulates the pattern buffer for the `s`/`S` regex-selection prompt.
//! Cloned from `handlers/command_line.rs`: the simulator never enters a
//! dedicated "regex prompt mode" — this handler only collects text; the
//! assembled `"s <pattern>"` / `"S <pattern>"` string is executed atomically
//! once, on Enter.
//!
//! Enter compiles the pattern via `helix_stdx::rope::Regex::new` before
//! emitting `Execute`, mirroring `CommandLinePending`'s validate-before-Execute
//! pattern. Without this, an invalid pattern would propagate as an `Err` all
//! the way up through `execute_command_any_mode`, which has no recovery path
//! for it (see `command_line.rs`'s module docs for the same argument).

use std::borrow::Cow;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::{InputHandler, KeyHandler};
use crate::input::typestate::{
    handler_result::HandlerResult,
    input_state::{InputState, RegexPromptKind},
    state_types::RegexPromptPending,
};
use crate::security::limits::MAX_COMMAND_LINE_LEN;

impl RegexPromptKind {
    /// The command prefix character this prompt assembles a command for
    fn prefix(self) -> char {
        match self {
            Self::SelectRegex => 's',
            Self::SplitSelection => 'S',
        }
    }
}

impl InputHandler<RegexPromptPending> for KeyHandler {
    fn handle_key(state: &RegexPromptPending, key: KeyEvent) -> HandlerResult {
        match key.code {
            KeyCode::Esc => HandlerResult::Cancel,

            KeyCode::Backspace => {
                if state.buffer.is_empty() {
                    HandlerResult::Cancel
                } else {
                    let mut buffer = state.buffer.clone();
                    buffer.pop();
                    HandlerResult::Transition(InputState::RegexPromptPending {
                        kind: state.kind,
                        buffer,
                    })
                }
            }

            KeyCode::Enter => {
                if state.buffer.is_empty() {
                    HandlerResult::Cancel
                } else {
                    match helix_stdx::rope::Regex::new(&state.buffer) {
                        Ok(_) => HandlerResult::Execute(Cow::Owned(format!(
                            "{} {}",
                            state.kind.prefix(),
                            state.buffer
                        ))),
                        // Invalid pattern: cancel rather than handing an
                        // `Err` to the executor, which has no recovery path
                        // for it.
                        Err(_) => HandlerResult::Cancel,
                    }
                }
            }

            KeyCode::Char(c)
                if (c.is_ascii_graphic() || c == ' ')
                    && !key.modifiers.contains(KeyModifiers::ALT)
                    && !key.modifiers.contains(KeyModifiers::CONTROL)
                    && state.buffer.len() < MAX_COMMAND_LINE_LEN =>
            {
                let mut buffer = state.buffer.clone();
                buffer.push(c);
                HandlerResult::Transition(InputState::RegexPromptPending {
                    kind: state.kind,
                    buffer,
                })
            }

            // Unrecognised key (arrows, F-keys, non-ASCII, over-length, or a
            // modifier-carrying char): never cancel on this, just ignore it.
            _ => HandlerResult::Stay,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state(kind: RegexPromptKind, buffer: &str) -> RegexPromptPending {
        RegexPromptPending {
            kind,
            buffer: buffer.to_string(),
        }
    }

    fn char_key(c: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE)
    }

    #[test]
    fn appends_printable_chars() {
        let result =
            KeyHandler::handle_key(&state(RegexPromptKind::SelectRegex, "fo"), char_key('o'));
        assert_eq!(
            result,
            HandlerResult::Transition(InputState::RegexPromptPending {
                kind: RegexPromptKind::SelectRegex,
                buffer: "foo".to_string()
            })
        );
    }

    #[test]
    fn backspace_pops_last_char() {
        let result = KeyHandler::handle_key(
            &state(RegexPromptKind::SelectRegex, "foo"),
            KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE),
        );
        assert_eq!(
            result,
            HandlerResult::Transition(InputState::RegexPromptPending {
                kind: RegexPromptKind::SelectRegex,
                buffer: "fo".to_string()
            })
        );
    }

    #[test]
    fn backspace_on_empty_buffer_cancels() {
        let result = KeyHandler::handle_key(
            &state(RegexPromptKind::SelectRegex, ""),
            KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE),
        );
        assert_eq!(result, HandlerResult::Cancel);
    }

    #[test]
    fn enter_on_empty_buffer_cancels() {
        let result = KeyHandler::handle_key(
            &state(RegexPromptKind::SelectRegex, ""),
            KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
        );
        assert_eq!(result, HandlerResult::Cancel);
    }

    #[test]
    fn enter_on_valid_pattern_executes_with_select_regex_prefix() {
        let result = KeyHandler::handle_key(
            &state(RegexPromptKind::SelectRegex, "foo"),
            KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
        );
        assert_eq!(result, HandlerResult::Execute(Cow::Borrowed("s foo")));
    }

    #[test]
    fn enter_on_valid_pattern_executes_with_split_selection_prefix() {
        let result = KeyHandler::handle_key(
            &state(RegexPromptKind::SplitSelection, ","),
            KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
        );
        assert_eq!(result, HandlerResult::Execute(Cow::Borrowed("S ,")));
    }

    #[test]
    fn enter_on_invalid_regex_cancels_instead_of_executing() {
        let result = KeyHandler::handle_key(
            &state(RegexPromptKind::SelectRegex, "("),
            KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
        );
        assert_eq!(result, HandlerResult::Cancel);
    }

    #[test]
    fn escape_cancels() {
        let result = KeyHandler::handle_key(
            &state(RegexPromptKind::SelectRegex, "foo"),
            KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE),
        );
        assert_eq!(result, HandlerResult::Cancel);
    }

    #[test]
    fn alt_and_ctrl_chars_are_ignored_not_cancelled() {
        let alt_key = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::ALT);
        assert_eq!(
            KeyHandler::handle_key(&state(RegexPromptKind::SelectRegex, "fo"), alt_key),
            HandlerResult::Stay
        );

        let ctrl_key = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        assert_eq!(
            KeyHandler::handle_key(&state(RegexPromptKind::SelectRegex, "fo"), ctrl_key),
            HandlerResult::Stay
        );
    }

    #[test]
    fn non_ascii_char_is_ignored_not_cancelled() {
        let result =
            KeyHandler::handle_key(&state(RegexPromptKind::SelectRegex, "fo"), char_key('é'));
        assert_eq!(result, HandlerResult::Stay);
    }

    #[test]
    fn arrow_key_is_ignored_not_cancelled() {
        let result = KeyHandler::handle_key(
            &state(RegexPromptKind::SelectRegex, "fo"),
            KeyEvent::new(KeyCode::Left, KeyModifiers::NONE),
        );
        assert_eq!(result, HandlerResult::Stay);
    }

    #[test]
    fn buffer_length_is_capped() {
        let full = "x".repeat(MAX_COMMAND_LINE_LEN);
        let result =
            KeyHandler::handle_key(&state(RegexPromptKind::SelectRegex, &full), char_key('y'));
        assert_eq!(result, HandlerResult::Stay);
    }
}
