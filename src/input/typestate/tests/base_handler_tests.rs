//! Tests for BaseState handler

use std::assert_matches;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::helix::commands::*;
use crate::input::typestate::{
    BaseState, FindType, HandlerResult, InputHandler, InputState, KeyHandler,
};

#[test]
fn test_base_state_goto_prefix() {
    let result = KeyHandler::handle_key(
        &BaseState,
        KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE),
    );
    assert_matches!(result, HandlerResult::Transition(InputState::GotoPending));
}

#[test]
fn test_base_state_view_prefix() {
    let result = KeyHandler::handle_key(
        &BaseState,
        KeyEvent::new(KeyCode::Char('z'), KeyModifiers::NONE),
    );
    assert_matches!(result, HandlerResult::Transition(InputState::ViewPending));
}

#[test]
fn test_base_state_match_prefix() {
    let result = KeyHandler::handle_key(
        &BaseState,
        KeyEvent::new(KeyCode::Char('m'), KeyModifiers::NONE),
    );
    assert_matches!(result, HandlerResult::Transition(InputState::MatchPending));
}

#[test]
fn test_base_state_find_prefix() {
    let result = KeyHandler::handle_key(
        &BaseState,
        KeyEvent::new(KeyCode::Char('f'), KeyModifiers::NONE),
    );
    assert_matches!(
        result,
        HandlerResult::Transition(InputState::FindCharPending {
            find_type: FindType::FindForward
        })
    );
}

#[test]
fn test_base_state_replace_prefix() {
    let result = KeyHandler::handle_key(
        &BaseState,
        KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE),
    );
    assert_matches!(
        result,
        HandlerResult::Transition(InputState::ReplaceCharPending)
    );
}

#[test]
fn test_base_state_count_prefix() {
    let result = KeyHandler::handle_key(
        &BaseState,
        KeyEvent::new(KeyCode::Char('3'), KeyModifiers::NONE),
    );
    assert_matches!(
        result,
        HandlerResult::Transition(InputState::CountPending { count: 3 })
    );
}

#[test]
fn test_base_state_single_key_command() {
    let result = KeyHandler::handle_key(
        &BaseState,
        KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE),
    );
    assert_matches!(result, HandlerResult::Execute(cmd) if cmd == CMD_MOVE_LEFT);
}

#[test]
fn test_base_state_escape() {
    let result =
        KeyHandler::handle_key(&BaseState, KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    assert_matches!(result, HandlerResult::Execute(cmd) if cmd == CMD_ESCAPE);
}

#[test]
fn test_base_state_find_backward_prefix() {
    let result = KeyHandler::handle_key(
        &BaseState,
        KeyEvent::new(KeyCode::Char('F'), KeyModifiers::SHIFT),
    );
    assert_matches!(
        result,
        HandlerResult::Transition(InputState::FindCharPending {
            find_type: FindType::FindBackward
        })
    );
}

#[test]
fn test_base_state_till_backward_prefix() {
    let result = KeyHandler::handle_key(
        &BaseState,
        KeyEvent::new(KeyCode::Char('T'), KeyModifiers::SHIFT),
    );
    assert_matches!(
        result,
        HandlerResult::Transition(InputState::FindCharPending {
            find_type: FindType::TillBackward
        })
    );
}

#[test]
fn test_base_state_unknown_key_stays() {
    // Test that unknown keys stay in base state.
    // '@' is not bound to anything - unlike 'Q', which is now the macro
    // replay command (see test_base_state_macro_replay_transitions below).
    let result = KeyHandler::handle_key(
        &BaseState,
        KeyEvent::new(KeyCode::Char('@'), KeyModifiers::NONE),
    );
    assert_matches!(result, HandlerResult::Stay);
}

#[test]
fn test_base_state_macro_toggle_executes() {
    let result = KeyHandler::handle_key(
        &BaseState,
        KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE),
    );
    assert_eq!(
        result,
        HandlerResult::Execute(std::borrow::Cow::Borrowed(CMD_TOGGLE_MACRO_RECORDING))
    );
}

#[test]
fn test_base_state_macro_replay_executes() {
    let result = KeyHandler::handle_key(
        &BaseState,
        KeyEvent::new(KeyCode::Char('Q'), KeyModifiers::SHIFT),
    );
    assert_eq!(
        result,
        HandlerResult::Execute(std::borrow::Cow::Borrowed(CMD_REPLAY_MACRO))
    );
}

#[test]
fn test_base_state_select_regex_prompt_transitions() {
    let result = KeyHandler::handle_key(
        &BaseState,
        KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE),
    );
    assert_eq!(
        result,
        HandlerResult::Transition(InputState::RegexPromptPending {
            kind: crate::input::typestate::RegexPromptKind::SelectRegex,
            buffer: String::new(),
        })
    );
}

#[test]
fn test_base_state_split_selection_prompt_transitions() {
    let result = KeyHandler::handle_key(
        &BaseState,
        KeyEvent::new(KeyCode::Char('S'), KeyModifiers::SHIFT),
    );
    assert_eq!(
        result,
        HandlerResult::Transition(InputState::RegexPromptPending {
            kind: crate::input::typestate::RegexPromptKind::SplitSelection,
            buffer: String::new(),
        })
    );
}

#[test]
fn test_base_state_page_movement_commands() {
    // Regression test for issue #198: Ctrl-b/f/u/d must reach the
    // page/half-page movement commands, not be swallowed by the generic
    // "ignore other modifier combinations" CONTROL fallback.
    let result = KeyHandler::handle_key(
        &BaseState,
        KeyEvent::new(KeyCode::Char('b'), KeyModifiers::CONTROL),
    );
    assert_matches!(result, HandlerResult::Execute(cmd) if cmd == CMD_PAGE_UP);

    let result = KeyHandler::handle_key(
        &BaseState,
        KeyEvent::new(KeyCode::Char('f'), KeyModifiers::CONTROL),
    );
    assert_matches!(result, HandlerResult::Execute(cmd) if cmd == CMD_PAGE_DOWN);

    let result = KeyHandler::handle_key(
        &BaseState,
        KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL),
    );
    assert_matches!(result, HandlerResult::Execute(cmd) if cmd == CMD_HALF_PAGE_UP);

    let result = KeyHandler::handle_key(
        &BaseState,
        KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL),
    );
    assert_matches!(result, HandlerResult::Execute(cmd) if cmd == CMD_HALF_PAGE_DOWN);
}

#[test]
fn test_base_state_replace_with_yanked() {
    // Regression test for issue #198: bare 'R' must resolve to
    // replace_with_yanked instead of staying unmapped.
    let result = KeyHandler::handle_key(
        &BaseState,
        KeyEvent::new(KeyCode::Char('R'), KeyModifiers::SHIFT),
    );
    assert_matches!(result, HandlerResult::Execute(cmd) if cmd == CMD_REPLACE_WITH_YANKED);
}
