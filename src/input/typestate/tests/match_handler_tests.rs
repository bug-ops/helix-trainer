//! Tests for MatchPending and surround handlers

use std::assert_matches;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::helix::commands::*;
use crate::input::typestate::{
    HandlerResult, InputHandler, InputState, KeyHandler, MatchPending, SurroundAddPending,
    SurroundDeletePending, SurroundReplaceFromPending, SurroundReplaceToPending,
};

// ============================================================================
// MatchPending tests
// ============================================================================

#[test]
fn test_match_pending_mm() {
    let result = KeyHandler::handle_key(
        &MatchPending,
        KeyEvent::new(KeyCode::Char('m'), KeyModifiers::NONE),
    );
    assert_matches!(result, HandlerResult::Execute(cmd) if cmd == CMD_MATCH_BRACKETS);
}

#[test]
fn test_match_pending_ms_transitions_to_surround_add() {
    let result = KeyHandler::handle_key(
        &MatchPending,
        KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE),
    );
    assert_matches!(
        result,
        HandlerResult::Transition(InputState::SurroundAddPending)
    );
}

#[test]
fn test_match_pending_md_transitions_to_surround_delete() {
    let result = KeyHandler::handle_key(
        &MatchPending,
        KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE),
    );
    assert_matches!(
        result,
        HandlerResult::Transition(InputState::SurroundDeletePending)
    );
}

#[test]
fn test_match_pending_mr_transitions_to_surround_replace() {
    let result = KeyHandler::handle_key(
        &MatchPending,
        KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE),
    );
    assert_matches!(
        result,
        HandlerResult::Transition(InputState::SurroundReplaceFromPending)
    );
}

#[test]
fn test_match_pending_ma_transitions_to_text_object_around() {
    let result = KeyHandler::handle_key(
        &MatchPending,
        KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE),
    );
    assert_matches!(
        result,
        HandlerResult::Transition(InputState::TextObjectAroundPending)
    );
}

#[test]
fn test_match_pending_mi_transitions_to_text_object_inside() {
    let result = KeyHandler::handle_key(
        &MatchPending,
        KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE),
    );
    assert_matches!(
        result,
        HandlerResult::Transition(InputState::TextObjectInsidePending)
    );
}

#[test]
fn test_match_pending_invalid_cancels() {
    let result = KeyHandler::handle_key(
        &MatchPending,
        KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE),
    );
    assert_matches!(result, HandlerResult::Cancel);
}

// ============================================================================
// SurroundAddPending tests
// ============================================================================

#[test]
fn test_surround_add_pending_accept_char() {
    let result = KeyHandler::handle_key(
        &SurroundAddPending,
        KeyEvent::new(KeyCode::Char('('), KeyModifiers::NONE),
    );
    assert_matches!(result, HandlerResult::Execute(cmd) if cmd == "ms(");
}

#[test]
fn test_surround_add_pending_accept_bracket() {
    let result = KeyHandler::handle_key(
        &SurroundAddPending,
        KeyEvent::new(KeyCode::Char('['), KeyModifiers::NONE),
    );
    assert_matches!(result, HandlerResult::Execute(cmd) if cmd == "ms[");
}

#[test]
fn test_surround_add_pending_accept_quote() {
    let result = KeyHandler::handle_key(
        &SurroundAddPending,
        KeyEvent::new(KeyCode::Char('"'), KeyModifiers::NONE),
    );
    assert_matches!(result, HandlerResult::Execute(cmd) if cmd == "ms\"");
}

#[test]
fn test_surround_add_pending_escape_cancels() {
    let result = KeyHandler::handle_key(
        &SurroundAddPending,
        KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE),
    );
    assert_matches!(result, HandlerResult::Cancel);
}

// ============================================================================
// SurroundDeletePending tests
// ============================================================================

#[test]
fn test_surround_delete_pending_accept_char() {
    let result = KeyHandler::handle_key(
        &SurroundDeletePending,
        KeyEvent::new(KeyCode::Char('('), KeyModifiers::NONE),
    );
    assert_matches!(result, HandlerResult::Execute(cmd) if cmd == "md(");
}

#[test]
fn test_surround_delete_pending_accept_bracket() {
    let result = KeyHandler::handle_key(
        &SurroundDeletePending,
        KeyEvent::new(KeyCode::Char('{'), KeyModifiers::NONE),
    );
    assert_matches!(result, HandlerResult::Execute(cmd) if cmd == "md{");
}

#[test]
fn test_surround_delete_pending_escape_cancels() {
    let result = KeyHandler::handle_key(
        &SurroundDeletePending,
        KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE),
    );
    assert_matches!(result, HandlerResult::Cancel);
}

// ============================================================================
// SurroundReplace tests
// ============================================================================

#[test]
fn test_surround_replace_from_transitions_to_to() {
    let result = KeyHandler::handle_key(
        &SurroundReplaceFromPending,
        KeyEvent::new(KeyCode::Char('('), KeyModifiers::NONE),
    );
    assert_matches!(
        result,
        HandlerResult::Transition(InputState::SurroundReplaceToPending { from_char: '(' })
    );
}

#[test]
fn test_surround_replace_from_escape_cancels() {
    let result = KeyHandler::handle_key(
        &SurroundReplaceFromPending,
        KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE),
    );
    assert_matches!(result, HandlerResult::Cancel);
}

#[test]
fn test_surround_replace_to_completes() {
    let state = SurroundReplaceToPending { from_char: '(' };
    let result = KeyHandler::handle_key(
        &state,
        KeyEvent::new(KeyCode::Char('['), KeyModifiers::NONE),
    );
    assert_matches!(result, HandlerResult::Execute(cmd) if cmd == "mr([");
}

#[test]
fn test_surround_replace_to_quotes() {
    let state = SurroundReplaceToPending { from_char: '"' };
    let result = KeyHandler::handle_key(
        &state,
        KeyEvent::new(KeyCode::Char('\''), KeyModifiers::NONE),
    );
    assert_matches!(result, HandlerResult::Execute(cmd) if cmd == "mr\"'");
}

#[test]
fn test_surround_replace_to_escape_cancels() {
    let state = SurroundReplaceToPending { from_char: '(' };
    let result = KeyHandler::handle_key(&state, KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    assert_matches!(result, HandlerResult::Cancel);
}
