//! Tests for TypestateHandler and TypestateHandlerState

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::helix::commands::*;
use crate::input::typestate::{BaseState, TypestateHandler, TypestateHandlerState};

#[test]
fn test_typestate_handler_base_to_goto() {
    let handler = TypestateHandler::<BaseState>::base();
    let (result, next) = handler.process_key(KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE));

    assert!(result.is_transition());
    assert!(matches!(next, TypestateHandlerState::GotoPending(_)));
}

#[test]
fn test_typestate_handler_state_process() {
    let mut state = TypestateHandlerState::new();
    assert!(state.is_base());

    // Press 'z' - transition to ViewPending
    let (result, next) = state.process_key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::NONE));
    assert!(result.is_transition());
    state = next;
    assert!(matches!(state, TypestateHandlerState::ViewPending(_)));

    // Press 'z' - execute "zz"
    let (result, next) = state.process_key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::NONE));
    assert!(result.is_execute());
    assert_eq!(result.command(), Some(CMD_VIEW_CENTER));
    state = next;
    assert!(state.is_base());
}
