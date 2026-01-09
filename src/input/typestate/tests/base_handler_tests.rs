//! Tests for BaseState handler

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
    assert!(matches!(
        result,
        HandlerResult::Transition(InputState::GotoPending)
    ));
}

#[test]
fn test_base_state_view_prefix() {
    let result = KeyHandler::handle_key(
        &BaseState,
        KeyEvent::new(KeyCode::Char('z'), KeyModifiers::NONE),
    );
    assert!(matches!(
        result,
        HandlerResult::Transition(InputState::ViewPending)
    ));
}

#[test]
fn test_base_state_match_prefix() {
    let result = KeyHandler::handle_key(
        &BaseState,
        KeyEvent::new(KeyCode::Char('m'), KeyModifiers::NONE),
    );
    assert!(matches!(
        result,
        HandlerResult::Transition(InputState::MatchPending)
    ));
}

#[test]
fn test_base_state_find_prefix() {
    let result = KeyHandler::handle_key(
        &BaseState,
        KeyEvent::new(KeyCode::Char('f'), KeyModifiers::NONE),
    );
    assert!(matches!(
        result,
        HandlerResult::Transition(InputState::FindCharPending {
            find_type: FindType::FindForward
        })
    ));
}

#[test]
fn test_base_state_replace_prefix() {
    let result = KeyHandler::handle_key(
        &BaseState,
        KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE),
    );
    assert!(matches!(
        result,
        HandlerResult::Transition(InputState::ReplaceCharPending)
    ));
}

#[test]
fn test_base_state_count_prefix() {
    let result = KeyHandler::handle_key(
        &BaseState,
        KeyEvent::new(KeyCode::Char('3'), KeyModifiers::NONE),
    );
    assert!(matches!(
        result,
        HandlerResult::Transition(InputState::CountPending { count: 3 })
    ));
}

#[test]
fn test_base_state_single_key_command() {
    let result = KeyHandler::handle_key(
        &BaseState,
        KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE),
    );
    assert!(matches!(result, HandlerResult::Execute(cmd) if cmd == CMD_MOVE_LEFT));
}

#[test]
fn test_base_state_escape() {
    let result =
        KeyHandler::handle_key(&BaseState, KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    assert!(matches!(result, HandlerResult::Execute(cmd) if cmd == CMD_ESCAPE));
}

#[test]
fn test_base_state_find_backward_prefix() {
    let result = KeyHandler::handle_key(
        &BaseState,
        KeyEvent::new(KeyCode::Char('F'), KeyModifiers::SHIFT),
    );
    assert!(matches!(
        result,
        HandlerResult::Transition(InputState::FindCharPending {
            find_type: FindType::FindBackward
        })
    ));
}

#[test]
fn test_base_state_till_backward_prefix() {
    let result = KeyHandler::handle_key(
        &BaseState,
        KeyEvent::new(KeyCode::Char('T'), KeyModifiers::SHIFT),
    );
    assert!(matches!(
        result,
        HandlerResult::Transition(InputState::FindCharPending {
            find_type: FindType::TillBackward
        })
    ));
}

#[test]
fn test_base_state_unknown_key_stays() {
    // Test that unknown keys stay in base state
    let result = KeyHandler::handle_key(
        &BaseState,
        KeyEvent::new(KeyCode::Char('Q'), KeyModifiers::SHIFT),
    );
    assert!(matches!(result, HandlerResult::Stay));
}
