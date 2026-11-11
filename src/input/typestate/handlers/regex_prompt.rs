//! RegexPromptPending handler
//!
//! Accumulates the pattern buffer for the `s`/`S`/`K`/`Alt-K` regex-selection
//! prompts and the `/`/`?` search prompts.
//! Cloned from `handlers/command_line.rs`: the simulator never enters a
//! dedicated "regex prompt mode" — this handler only collects text; the
//! assembled `"<command id> <pattern>"` string (e.g. `"s foo"`, `"? sum"`,
//! `"Alt-K a"`) is executed atomically once, on Enter.
//!
//! Enter compiles the pattern via `helix_stdx::rope::Regex::new` before
//! emitting `Execute`, mirroring `CommandLinePending`'s validate-before-Execute
//! pattern. Without this, an invalid pattern would propagate as an `Err` all
//! the way up through `execute_command_any_mode`, which has no recovery path
//! for it (see `command_line.rs`'s module docs for the same argument). The `/`
//! and `?` search prompts are the exception: it forwards an invalid pattern so the
//! simulator's `UserError` is shown to the user.

use std::borrow::Cow;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::{InputHandler, KeyHandler};
use crate::input::typestate::{
    handler_result::HandlerResult, input_state::InputState, state_types::RegexPromptPending,
};
use crate::security::limits::MAX_COMMAND_LINE_LEN;

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
                } else if !state.kind.forwards_invalid_pattern()
                    && helix_stdx::rope::Regex::new(&state.buffer).is_err()
                {
                    // Invalid `s`/`S`/`K`/`Alt-K` pattern: cancel rather than
                    // handing an `Err` to the executor. The search prompts
                    // deliberately forward it so the simulator's `UserError`
                    // reaches the user (FR-204).
                    HandlerResult::Cancel
                } else {
                    HandlerResult::Execute(Cow::Owned(format!(
                        "{} {}",
                        state.kind.command_id(),
                        state.buffer
                    )))
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
    use crate::input::typestate::input_state::RegexPromptKind;

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
    fn enter_on_valid_pattern_executes_search_with_slash_prefix() {
        let result = KeyHandler::handle_key(
            &state(RegexPromptKind::Search, "sum"),
            KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
        );
        assert_eq!(result, HandlerResult::Execute(Cow::Borrowed("/ sum")));
    }

    fn enter() -> KeyEvent {
        KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)
    }

    #[test]
    fn enter_assembles_the_command_id_for_every_new_kind() {
        for (kind, expected) in [
            (RegexPromptKind::SearchBackward, "? sum"),
            (RegexPromptKind::KeepSelections, "K sum"),
            (RegexPromptKind::RemoveSelections, "Alt-K sum"),
        ] {
            assert_eq!(
                KeyHandler::handle_key(&state(kind, "sum"), enter()),
                HandlerResult::Execute(Cow::Borrowed(expected))
            );
        }
    }

    #[test]
    fn backward_search_forwards_invalid_regex_but_keep_remove_cancel() {
        assert_eq!(
            KeyHandler::handle_key(&state(RegexPromptKind::SearchBackward, "a("), enter()),
            HandlerResult::Execute(Cow::Borrowed("? a("))
        );
        for kind in [
            RegexPromptKind::KeepSelections,
            RegexPromptKind::RemoveSelections,
        ] {
            assert_eq!(
                KeyHandler::handle_key(&state(kind, "a("), enter()),
                HandlerResult::Cancel
            );
        }
    }

    #[test]
    fn question_mark_is_buffered_as_text_in_every_prompt() {
        for kind in [
            RegexPromptKind::SelectRegex,
            RegexPromptKind::SplitSelection,
            RegexPromptKind::Search,
            RegexPromptKind::SearchBackward,
            RegexPromptKind::KeepSelections,
            RegexPromptKind::RemoveSelections,
        ] {
            assert_eq!(
                KeyHandler::handle_key(&state(kind, "a"), char_key('?')),
                HandlerResult::Transition(InputState::RegexPromptPending {
                    kind,
                    buffer: "a?".to_string()
                })
            );
        }
    }

    #[test]
    fn search_prompt_typed_chars_are_buffered_not_executed() {
        let result = KeyHandler::handle_key(&state(RegexPromptKind::Search, "su"), char_key('d'));
        assert_eq!(
            result,
            HandlerResult::Transition(InputState::RegexPromptPending {
                kind: RegexPromptKind::Search,
                buffer: "sud".to_string()
            })
        );
    }

    #[test]
    fn search_prompt_forwards_invalid_regex_so_the_simulator_reports_it() {
        let enter = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        assert_eq!(
            KeyHandler::handle_key(&state(RegexPromptKind::Search, "a("), enter),
            HandlerResult::Execute(Cow::Borrowed("/ a("))
        );
    }

    #[test]
    fn search_prompt_empty_enter_and_escape_cancel() {
        let enter = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        assert_eq!(
            KeyHandler::handle_key(&state(RegexPromptKind::Search, ""), enter),
            HandlerResult::Cancel
        );
        assert_eq!(
            KeyHandler::handle_key(
                &state(RegexPromptKind::Search, "sum"),
                KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)
            ),
            HandlerResult::Cancel
        );
    }

    #[test]
    fn search_prompt_buffer_length_is_capped() {
        let full = "x".repeat(MAX_COMMAND_LINE_LEN);
        let result = KeyHandler::handle_key(&state(RegexPromptKind::Search, &full), char_key('y'));
        assert_eq!(result, HandlerResult::Stay);
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
