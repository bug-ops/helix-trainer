//! Tests for GotoPending handler

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::helix::commands::*;
use crate::input::typestate::{GotoPending, HandlerResult, InputHandler, KeyHandler};

#[test]
fn test_goto_pending_gg() {
    let result = KeyHandler::handle_key(
        &GotoPending,
        KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE),
    );
    assert!(matches!(result, HandlerResult::Execute(cmd) if cmd == CMD_GOTO_FILE_START));
}

#[test]
fn test_goto_pending_gh() {
    let result = KeyHandler::handle_key(
        &GotoPending,
        KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE),
    );
    assert!(matches!(result, HandlerResult::Execute(cmd) if cmd == CMD_GOTO_LINE_START));
}

#[test]
fn test_goto_pending_gl() {
    let result = KeyHandler::handle_key(
        &GotoPending,
        KeyEvent::new(KeyCode::Char('l'), KeyModifiers::NONE),
    );
    assert!(matches!(result, HandlerResult::Execute(cmd) if cmd == CMD_GOTO_LINE_END));
}

#[test]
fn test_goto_pending_invalid_key_cancels() {
    let result = KeyHandler::handle_key(
        &GotoPending,
        KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE),
    );
    assert!(matches!(result, HandlerResult::Cancel));
}

#[test]
fn test_goto_pending_escape_cancels() {
    let result = KeyHandler::handle_key(
        &GotoPending,
        KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE),
    );
    assert!(matches!(result, HandlerResult::Cancel));
}

#[test]
fn test_goto_pending_gs() {
    let result = KeyHandler::handle_key(
        &GotoPending,
        KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE),
    );
    assert!(matches!(result, HandlerResult::Execute(cmd) if cmd == CMD_GOTO_FIRST_NONWHITESPACE));
}

#[test]
fn test_goto_pending_ge() {
    let result = KeyHandler::handle_key(
        &GotoPending,
        KeyEvent::new(KeyCode::Char('e'), KeyModifiers::NONE),
    );
    assert!(matches!(result, HandlerResult::Execute(cmd) if cmd == CMD_GOTO_LAST_LINE));
}
