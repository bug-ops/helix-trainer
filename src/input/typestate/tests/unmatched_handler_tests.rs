//! Tests for UnmatchedPrevPending and UnmatchedNextPending handlers

use std::assert_matches;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::helix::commands::*;
use crate::input::typestate::{
    HandlerResult, InputHandler, KeyHandler, UnmatchedNextPending, UnmatchedPrevPending,
};

// ============================================================================
// UnmatchedPrevPending handler tests (after '[')
// ============================================================================

#[test]
fn test_unmatched_prev_p_goto_prev_paragraph() {
    let result = KeyHandler::handle_key(
        &UnmatchedPrevPending,
        KeyEvent::new(KeyCode::Char('p'), KeyModifiers::NONE),
    );
    assert_matches!(result, HandlerResult::Execute(cmd) if cmd == CMD_GOTO_PREV_PARAGRAPH);
}

#[test]
fn test_unmatched_prev_escape_cancels() {
    let result = KeyHandler::handle_key(
        &UnmatchedPrevPending,
        KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE),
    );
    assert_matches!(result, HandlerResult::Cancel);
}

#[test]
fn test_unmatched_prev_invalid_key_cancels() {
    let result = KeyHandler::handle_key(
        &UnmatchedPrevPending,
        KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE),
    );
    assert_matches!(result, HandlerResult::Cancel);
}

#[test]
fn test_unmatched_prev_number_cancels() {
    let result = KeyHandler::handle_key(
        &UnmatchedPrevPending,
        KeyEvent::new(KeyCode::Char('5'), KeyModifiers::NONE),
    );
    assert_matches!(result, HandlerResult::Cancel);
}

// ============================================================================
// UnmatchedNextPending handler tests (after ']')
// ============================================================================

#[test]
fn test_unmatched_next_p_goto_next_paragraph() {
    let result = KeyHandler::handle_key(
        &UnmatchedNextPending,
        KeyEvent::new(KeyCode::Char('p'), KeyModifiers::NONE),
    );
    assert_matches!(result, HandlerResult::Execute(cmd) if cmd == CMD_GOTO_NEXT_PARAGRAPH);
}

#[test]
fn test_unmatched_next_escape_cancels() {
    let result = KeyHandler::handle_key(
        &UnmatchedNextPending,
        KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE),
    );
    assert_matches!(result, HandlerResult::Cancel);
}

#[test]
fn test_unmatched_next_invalid_key_cancels() {
    let result = KeyHandler::handle_key(
        &UnmatchedNextPending,
        KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE),
    );
    assert_matches!(result, HandlerResult::Cancel);
}

#[test]
fn test_unmatched_next_number_cancels() {
    let result = KeyHandler::handle_key(
        &UnmatchedNextPending,
        KeyEvent::new(KeyCode::Char('9'), KeyModifiers::NONE),
    );
    assert_matches!(result, HandlerResult::Cancel);
}
