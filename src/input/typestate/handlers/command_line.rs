//! CommandLinePending handler
//!
//! Accumulates a `:`-prefixed command-line buffer. The simulator never
//! enters a "command mode" — this handler only collects text; the assembled
//! `:<buffer>` string is executed atomically once, on Enter.
//!
//! Character input is read directly from the crossterm `KeyEvent` rather
//! than round-tripping through `command_to_key_event`/`parse_key_code`,
//! since that helper's single-byte gate silently turns any non-ASCII
//! character (and macOS Option-compositions) into a literal space.
//!
//! Enter validates the assembled command via `CommandLine::parse` before
//! emitting `Execute`, mirroring `RegisterOpPending`'s validate-before-Execute
//! pattern. Without this, a parse error (e.g. `:nope`, `:goto abc`) would
//! propagate as an `Err` all the way up through `execute_command_any_mode` ->
//! `GameSession::record_action(_with_count)` -> `ui::update` -> the event
//! loop -> `main`'s return value, terminating the whole application on a
//! simple typo.

use std::borrow::Cow;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::{InputHandler, KeyHandler};
use crate::helix::simulator::command_line::CommandLine;
use crate::input::typestate::{
    handler_result::HandlerResult, input_state::InputState, state_types::CommandLinePending,
};
use crate::security::limits::MAX_COMMAND_LINE_LEN;

impl InputHandler<CommandLinePending> for KeyHandler {
    fn handle_key(state: &CommandLinePending, key: KeyEvent) -> HandlerResult {
        match key.code {
            KeyCode::Esc => HandlerResult::Cancel,

            KeyCode::Backspace => {
                if state.buffer.is_empty() {
                    HandlerResult::Cancel
                } else {
                    let mut buffer = state.buffer.clone();
                    buffer.pop();
                    HandlerResult::Transition(InputState::CommandLinePending { buffer })
                }
            }

            KeyCode::Enter => {
                if state.buffer.is_empty() {
                    HandlerResult::Cancel
                } else {
                    let cmd = format!(":{}", state.buffer);
                    match CommandLine::parse(&cmd) {
                        Ok(_) => HandlerResult::Execute(Cow::Owned(cmd)),
                        // Invalid command (unknown name, bad arity, non-numeric
                        // argument, ...): cancel rather than handing an `Err`
                        // to the executor, which has no recovery path for it.
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
                HandlerResult::Transition(InputState::CommandLinePending { buffer })
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

    fn state(buffer: &str) -> CommandLinePending {
        CommandLinePending {
            buffer: buffer.to_string(),
        }
    }

    fn char_key(c: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE)
    }

    #[test]
    fn appends_printable_chars() {
        let result = KeyHandler::handle_key(&state("goto"), char_key(' '));
        assert_eq!(
            result,
            HandlerResult::Transition(InputState::CommandLinePending {
                buffer: "goto ".to_string()
            })
        );
    }

    #[test]
    fn backspace_pops_last_char() {
        let result = KeyHandler::handle_key(
            &state("goto"),
            KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE),
        );
        assert_eq!(
            result,
            HandlerResult::Transition(InputState::CommandLinePending {
                buffer: "got".to_string()
            })
        );
    }

    #[test]
    fn backspace_on_empty_buffer_cancels() {
        let result = KeyHandler::handle_key(
            &state(""),
            KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE),
        );
        assert_eq!(result, HandlerResult::Cancel);
    }

    #[test]
    fn enter_on_empty_buffer_cancels() {
        let result = KeyHandler::handle_key(
            &state(""),
            KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
        );
        assert_eq!(result, HandlerResult::Cancel);
    }

    #[test]
    fn enter_on_nonempty_buffer_executes() {
        let result = KeyHandler::handle_key(
            &state("goto 3"),
            KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
        );
        assert_eq!(result, HandlerResult::Execute(Cow::Borrowed(":goto 3")));
    }

    /// Regression test: an invalid command must cancel, not `Execute`. Before
    /// this fix, `Execute(":nope")` would propagate `CommandLine::parse`'s
    /// `Err` all the way up to `main`'s return value and exit the process.
    #[test]
    fn enter_on_invalid_command_cancels_instead_of_executing() {
        let result = KeyHandler::handle_key(
            &state("nope"),
            KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
        );
        assert_eq!(result, HandlerResult::Cancel);
    }

    #[test]
    fn enter_on_wrong_arity_cancels() {
        let result = KeyHandler::handle_key(
            &state("goto"),
            KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
        );
        assert_eq!(result, HandlerResult::Cancel);

        let result = KeyHandler::handle_key(
            &state("goto 1 2"),
            KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
        );
        assert_eq!(result, HandlerResult::Cancel);
    }

    #[test]
    fn enter_on_non_numeric_argument_cancels() {
        let result = KeyHandler::handle_key(
            &state("goto abc"),
            KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
        );
        assert_eq!(result, HandlerResult::Cancel);
    }

    #[test]
    fn escape_cancels() {
        let result = KeyHandler::handle_key(
            &state("goto 3"),
            KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE),
        );
        assert_eq!(result, HandlerResult::Cancel);
    }

    #[test]
    fn alt_and_ctrl_chars_are_ignored_not_cancelled() {
        let alt_key = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::ALT);
        assert_eq!(
            KeyHandler::handle_key(&state("go"), alt_key),
            HandlerResult::Stay
        );

        let ctrl_key = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        assert_eq!(
            KeyHandler::handle_key(&state("go"), ctrl_key),
            HandlerResult::Stay
        );
    }

    #[test]
    fn non_ascii_char_is_ignored_not_cancelled() {
        let result = KeyHandler::handle_key(&state("go"), char_key('é'));
        assert_eq!(result, HandlerResult::Stay);
    }

    #[test]
    fn arrow_key_is_ignored_not_cancelled() {
        let result = KeyHandler::handle_key(
            &state("go"),
            KeyEvent::new(KeyCode::Left, KeyModifiers::NONE),
        );
        assert_eq!(result, HandlerResult::Stay);
    }

    #[test]
    fn buffer_length_is_capped() {
        let full = "x".repeat(MAX_COMMAND_LINE_LEN);
        let result = KeyHandler::handle_key(&state(&full), char_key('y'));
        assert_eq!(result, HandlerResult::Stay);
    }
}
