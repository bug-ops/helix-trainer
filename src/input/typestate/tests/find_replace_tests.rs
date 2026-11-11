//! Tests for `FindCharPending` and `ReplaceCharPending` handlers

use crate::grammar::find::FindMotion;
use std::assert_matches;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::helix::commands::*;
use crate::input::typestate::{
    FindCharPending, HandlerResult, InputHandler, KeyHandler, ReplaceCharPending,
    UnmatchedNextPending, UnmatchedPrevPending,
};

// ============================================================================
// FindCharPending tests
// ============================================================================

#[test]
fn test_find_char_pending_accept_char() {
    let state = FindCharPending {
        motion: FindMotion::FIND_FORWARD,
    };
    let result = KeyHandler::handle_key(
        &state,
        KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE),
    );
    assert_matches!(result, HandlerResult::Execute(cmd) if cmd.to_string() == "fa");
}

#[test]
fn test_find_char_pending_backward() {
    let state = FindCharPending {
        motion: FindMotion::FIND_BACKWARD,
    };
    let result = KeyHandler::handle_key(
        &state,
        KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE),
    );
    assert_matches!(result, HandlerResult::Execute(cmd) if cmd.to_string() == "Fx");
}

#[test]
fn test_till_char_pending() {
    let state = FindCharPending {
        motion: FindMotion::TILL_FORWARD,
    };
    let result = KeyHandler::handle_key(
        &state,
        KeyEvent::new(KeyCode::Char('e'), KeyModifiers::NONE),
    );
    assert_matches!(result, HandlerResult::Execute(cmd) if cmd.to_string() == "te");
}

#[test]
fn test_find_char_escape_cancels() {
    let state = FindCharPending {
        motion: FindMotion::FIND_FORWARD,
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
    assert_matches!(result, HandlerResult::Execute(cmd) if cmd.to_string() == "ra");
}

#[test]
fn test_replace_char_pending_enter_newline() {
    let result = KeyHandler::handle_key(
        &ReplaceCharPending,
        KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
    );
    assert_matches!(result, HandlerResult::Execute(cmd) if cmd.to_string() == "r\n");
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
    assert_matches!(result, HandlerResult::Execute(cmd) if cmd.to_string() == CMD_GOTO_PREV_PARAGRAPH);
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
    assert_matches!(result, HandlerResult::Execute(cmd) if cmd.to_string() == CMD_GOTO_NEXT_PARAGRAPH);
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
