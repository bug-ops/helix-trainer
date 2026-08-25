//! Tests for ViewPending handler

use std::assert_matches;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::helix::commands::*;
use crate::input::typestate::{HandlerResult, InputHandler, KeyHandler, ViewPending};

#[test]
fn test_view_pending_zz() {
    let result = KeyHandler::handle_key(
        &ViewPending,
        KeyEvent::new(KeyCode::Char('z'), KeyModifiers::NONE),
    );
    assert_matches!(result, HandlerResult::Execute(cmd) if cmd == CMD_VIEW_CENTER);
}

#[test]
fn test_view_pending_zt() {
    let result = KeyHandler::handle_key(
        &ViewPending,
        KeyEvent::new(KeyCode::Char('t'), KeyModifiers::NONE),
    );
    assert_matches!(result, HandlerResult::Execute(cmd) if cmd == CMD_VIEW_TOP);
}

#[test]
fn test_view_pending_zb() {
    let result = KeyHandler::handle_key(
        &ViewPending,
        KeyEvent::new(KeyCode::Char('b'), KeyModifiers::NONE),
    );
    assert_matches!(result, HandlerResult::Execute(cmd) if cmd == CMD_VIEW_BOTTOM);
}

#[test]
fn test_view_pending_zm() {
    let result = KeyHandler::handle_key(
        &ViewPending,
        KeyEvent::new(KeyCode::Char('m'), KeyModifiers::NONE),
    );
    assert_matches!(result, HandlerResult::Execute(cmd) if cmd == CMD_VIEW_CENTER_HORIZONTAL);
}

#[test]
fn test_view_pending_zj() {
    let result = KeyHandler::handle_key(
        &ViewPending,
        KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE),
    );
    assert_matches!(result, HandlerResult::Execute(cmd) if cmd == CMD_SCROLL_DOWN);
}

#[test]
fn test_view_pending_zk() {
    let result = KeyHandler::handle_key(
        &ViewPending,
        KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE),
    );
    assert_matches!(result, HandlerResult::Execute(cmd) if cmd == CMD_SCROLL_UP);
}

#[test]
fn test_view_pending_escape_cancels() {
    let result = KeyHandler::handle_key(
        &ViewPending,
        KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE),
    );
    assert_matches!(result, HandlerResult::Cancel);
}

#[test]
fn test_view_pending_invalid_cancels() {
    let result = KeyHandler::handle_key(
        &ViewPending,
        KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE),
    );
    assert_matches!(result, HandlerResult::Cancel);
}
