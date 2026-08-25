//! Tests for TextObject handlers

use std::assert_matches;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::input::typestate::{
    HandlerResult, InputHandler, KeyHandler, TextObjectAroundPending, TextObjectInsidePending,
};

// ============================================================================
// TextObjectAroundPending tests
// ============================================================================

#[test]
fn test_text_object_around_word() {
    let result = KeyHandler::handle_key(
        &TextObjectAroundPending,
        KeyEvent::new(KeyCode::Char('w'), KeyModifiers::NONE),
    );
    assert_matches!(result, HandlerResult::Execute(cmd) if cmd == "maw");
}

#[test]
fn test_text_object_around_word_big() {
    let result = KeyHandler::handle_key(
        &TextObjectAroundPending,
        KeyEvent::new(KeyCode::Char('W'), KeyModifiers::NONE),
    );
    assert_matches!(result, HandlerResult::Execute(cmd) if cmd == "maW");
}

#[test]
fn test_text_object_around_parens() {
    let result = KeyHandler::handle_key(
        &TextObjectAroundPending,
        KeyEvent::new(KeyCode::Char('('), KeyModifiers::NONE),
    );
    assert_matches!(result, HandlerResult::Execute(cmd) if cmd == "ma(");
}

#[test]
fn test_text_object_around_quotes() {
    let result = KeyHandler::handle_key(
        &TextObjectAroundPending,
        KeyEvent::new(KeyCode::Char('"'), KeyModifiers::NONE),
    );
    assert_matches!(result, HandlerResult::Execute(cmd) if cmd == "ma\"");
}

#[test]
fn test_text_object_around_paragraph() {
    let result = KeyHandler::handle_key(
        &TextObjectAroundPending,
        KeyEvent::new(KeyCode::Char('p'), KeyModifiers::NONE),
    );
    assert_matches!(result, HandlerResult::Execute(cmd) if cmd == "map");
}

#[test]
fn test_text_object_around_escape_cancels() {
    let result = KeyHandler::handle_key(
        &TextObjectAroundPending,
        KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE),
    );
    assert_matches!(result, HandlerResult::Cancel);
}

#[test]
fn test_text_object_around_invalid_cancels() {
    let result = KeyHandler::handle_key(
        &TextObjectAroundPending,
        KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE),
    );
    assert_matches!(result, HandlerResult::Cancel);
}

// ============================================================================
// TextObjectInsidePending tests
// ============================================================================

#[test]
fn test_text_object_inside_word() {
    let result = KeyHandler::handle_key(
        &TextObjectInsidePending,
        KeyEvent::new(KeyCode::Char('w'), KeyModifiers::NONE),
    );
    assert_matches!(result, HandlerResult::Execute(cmd) if cmd == "miw");
}

#[test]
fn test_text_object_inside_brackets() {
    let result = KeyHandler::handle_key(
        &TextObjectInsidePending,
        KeyEvent::new(KeyCode::Char('['), KeyModifiers::NONE),
    );
    assert_matches!(result, HandlerResult::Execute(cmd) if cmd == "mi[");
}

#[test]
fn test_text_object_inside_braces() {
    let result = KeyHandler::handle_key(
        &TextObjectInsidePending,
        KeyEvent::new(KeyCode::Char('{'), KeyModifiers::NONE),
    );
    assert_matches!(result, HandlerResult::Execute(cmd) if cmd == "mi{");
}

#[test]
fn test_text_object_inside_angle_brackets() {
    let result = KeyHandler::handle_key(
        &TextObjectInsidePending,
        KeyEvent::new(KeyCode::Char('<'), KeyModifiers::NONE),
    );
    assert_matches!(result, HandlerResult::Execute(cmd) if cmd == "mi<");
}

#[test]
fn test_text_object_inside_single_quote() {
    let result = KeyHandler::handle_key(
        &TextObjectInsidePending,
        KeyEvent::new(KeyCode::Char('\''), KeyModifiers::NONE),
    );
    assert_matches!(result, HandlerResult::Execute(cmd) if cmd == "mi'");
}

#[test]
fn test_text_object_inside_backtick() {
    let result = KeyHandler::handle_key(
        &TextObjectInsidePending,
        KeyEvent::new(KeyCode::Char('`'), KeyModifiers::NONE),
    );
    assert_matches!(result, HandlerResult::Execute(cmd) if cmd == "mi`");
}

#[test]
fn test_text_object_inside_escape_cancels() {
    let result = KeyHandler::handle_key(
        &TextObjectInsidePending,
        KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE),
    );
    assert_matches!(result, HandlerResult::Cancel);
}
