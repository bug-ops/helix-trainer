//! Tests for FindCharPending and ReplaceCharPending handlers

use std::assert_matches;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::helix::commands::*;
use crate::input::typestate::{
    FindCharPending, FindType, HandlerResult, InputHandler, KeyHandler, ReplaceCharPending,
    UnmatchedNextPending, UnmatchedPrevPending,
};

// ============================================================================
// FindCharPending tests
// ============================================================================

#[test]
fn test_find_char_pending_accept_char() {
    let state = FindCharPending {
        find_type: FindType::FindForward,
    };
    let result = KeyHandler::handle_key(
        &state,
        KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE),
    );
    assert_matches!(result, HandlerResult::Execute(cmd) if cmd == "fa");
}

#[test]
fn test_find_char_pending_backward() {
    let state = FindCharPending {
        find_type: FindType::FindBackward,
    };
    let result = KeyHandler::handle_key(
        &state,
        KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE),
    );
    assert_matches!(result, HandlerResult::Execute(cmd) if cmd == "Fx");
}

#[test]
fn test_till_char_pending() {
    let state = FindCharPending {
        find_type: FindType::TillForward,
    };
    let result = KeyHandler::handle_key(
        &state,
        KeyEvent::new(KeyCode::Char('e'), KeyModifiers::NONE),
    );
    assert_matches!(result, HandlerResult::Execute(cmd) if cmd == "te");
}

#[test]
fn test_find_char_escape_cancels() {
    let state = FindCharPending {
        find_type: FindType::FindForward,
    };
    let result = KeyHandler::handle_key(&state, KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    assert_matches!(result, HandlerResult::Cancel);
}

// ============================================================================
// ReplaceCharPending tests
// ============================================================================

#[test]
fn test_replace_char_pending_accept_char() {
    let result = KeyHandler::handle_key(
        &ReplaceCharPending,
        KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE),
    );
    assert_matches!(result, HandlerResult::Execute(cmd) if cmd == "ra");
}

#[test]
fn test_replace_char_pending_enter_newline() {
    let result = KeyHandler::handle_key(
        &ReplaceCharPending,
        KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
    );
    assert_matches!(result, HandlerResult::Execute(cmd) if cmd == "r\n");
}

#[test]
fn test_replace_char_escape_cancels() {
    let result = KeyHandler::handle_key(
        &ReplaceCharPending,
        KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE),
    );
    assert_matches!(result, HandlerResult::Cancel);
}

// ============================================================================
// UnmatchedPrevPending tests
// ============================================================================

#[test]
fn test_unmatched_prev_pending_p_produces_goto_prev_paragraph() {
    let result = KeyHandler::handle_key(
        &UnmatchedPrevPending,
        KeyEvent::new(KeyCode::Char('p'), KeyModifiers::NONE),
    );
    assert_matches!(result, HandlerResult::Execute(cmd) if cmd == CMD_GOTO_PREV_PARAGRAPH);
}

#[test]
fn test_unmatched_prev_pending_escape_cancels() {
    let result = KeyHandler::handle_key(
        &UnmatchedPrevPending,
        KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE),
    );
    assert_matches!(result, HandlerResult::Cancel);
}

#[test]
fn test_unmatched_prev_pending_other_key_cancels() {
    let result = KeyHandler::handle_key(
        &UnmatchedPrevPending,
        KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE),
    );
    assert_matches!(result, HandlerResult::Cancel);
}

#[test]
fn test_unmatched_prev_pending_digit_cancels() {
    let result = KeyHandler::handle_key(
        &UnmatchedPrevPending,
        KeyEvent::new(KeyCode::Char('5'), KeyModifiers::NONE),
    );
    assert_matches!(result, HandlerResult::Cancel);
}

// ============================================================================
// UnmatchedNextPending tests
// ============================================================================

#[test]
fn test_unmatched_next_pending_p_produces_goto_next_paragraph() {
    let result = KeyHandler::handle_key(
        &UnmatchedNextPending,
        KeyEvent::new(KeyCode::Char('p'), KeyModifiers::NONE),
    );
    assert_matches!(result, HandlerResult::Execute(cmd) if cmd == CMD_GOTO_NEXT_PARAGRAPH);
}

#[test]
fn test_unmatched_next_pending_escape_cancels() {
    let result = KeyHandler::handle_key(
        &UnmatchedNextPending,
        KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE),
    );
    assert_matches!(result, HandlerResult::Cancel);
}

#[test]
fn test_unmatched_next_pending_other_key_cancels() {
    let result = KeyHandler::handle_key(
        &UnmatchedNextPending,
        KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE),
    );
    assert_matches!(result, HandlerResult::Cancel);
}

#[test]
fn test_unmatched_next_pending_digit_cancels() {
    let result = KeyHandler::handle_key(
        &UnmatchedNextPending,
        KeyEvent::new(KeyCode::Char('3'), KeyModifiers::NONE),
    );
    assert_matches!(result, HandlerResult::Cancel);
}
