//! Tests for TypestateHandler and TypestateHandlerState

use std::assert_matches;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::helix::commands::*;
use crate::input::typestate::{BaseState, TypestateHandler, TypestateHandlerState};

#[test]
fn test_typestate_handler_base_to_goto() {
    let handler = TypestateHandler::<BaseState>::base();
    let (result, next) = handler.process_key(KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE));

    assert!(result.is_transition());
    assert_matches!(next, TypestateHandlerState::GotoPending(_));
}

#[test]
fn test_typestate_handler_state_process() {
    let mut state = TypestateHandlerState::new();
    assert!(state.is_base());

    // Press 'z' - transition to ViewPending
    let (result, next) = state.process_key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::NONE));
    assert!(result.is_transition());
    state = next;
    assert_matches!(state, TypestateHandlerState::ViewPending(_));

    // Press 'z' - execute "zz"
    let (result, next) = state.process_key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::NONE));
    assert!(result.is_execute());
    assert_eq!(result.command(), Some(CMD_VIEW_CENTER));
    state = next;
    assert!(state.is_base());
}

// ============================================================================
// Additional state transition tests
// ============================================================================

#[test]
fn test_typestate_handler_state_default() {
    let state = TypestateHandlerState::default();
    assert!(state.is_base());
}

#[test]
fn test_typestate_handler_state_name() {
    let state = TypestateHandlerState::new();
    assert_eq!(state.state_name(), "BASE");

    let (_, next) = state.process_key(KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE));
    assert_eq!(next.state_name(), "GOTO_PENDING");
}

#[test]
fn test_typestate_handler_goto_transitions() {
    let mut state = TypestateHandlerState::new();

    // g -> GotoPending
    let (_, next) = state.process_key(KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE));
    state = next;
    assert_eq!(state.state_name(), "GOTO_PENDING");

    // gg -> execute and back to base
    let (result, next) = state.process_key(KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE));
    assert!(result.is_execute());
    state = next;
    assert!(state.is_base());
}

#[test]
fn test_typestate_handler_match_transitions() {
    let mut state = TypestateHandlerState::new();

    // m -> MatchPending
    let (_, next) = state.process_key(KeyEvent::new(KeyCode::Char('m'), KeyModifiers::NONE));
    state = next;
    assert_eq!(state.state_name(), "MATCH_PENDING");

    // mm -> execute and back to base
    let (result, next) = state.process_key(KeyEvent::new(KeyCode::Char('m'), KeyModifiers::NONE));
    assert!(result.is_execute());
    state = next;
    assert!(state.is_base());
}

#[test]
fn test_typestate_handler_match_to_surround_add() {
    let mut state = TypestateHandlerState::new();

    // m -> MatchPending
    let (_, next) = state.process_key(KeyEvent::new(KeyCode::Char('m'), KeyModifiers::NONE));
    state = next;

    // ms -> SurroundAddPending
    let (_, next) = state.process_key(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE));
    state = next;
    assert_eq!(state.state_name(), "SURROUND_ADD_PENDING");

    // ( -> execute surround add
    let (result, next) = state.process_key(KeyEvent::new(KeyCode::Char('('), KeyModifiers::NONE));
    assert!(result.is_execute());
    state = next;
    assert!(state.is_base());
}

#[test]
fn test_typestate_handler_match_to_surround_delete() {
    let mut state = TypestateHandlerState::new();

    // m -> MatchPending
    let (_, next) = state.process_key(KeyEvent::new(KeyCode::Char('m'), KeyModifiers::NONE));
    state = next;

    // md -> SurroundDeletePending
    let (_, next) = state.process_key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE));
    state = next;
    assert_eq!(state.state_name(), "SURROUND_DELETE_PENDING");
}

#[test]
fn test_typestate_handler_match_to_surround_replace() {
    let mut state = TypestateHandlerState::new();

    // m -> MatchPending
    let (_, next) = state.process_key(KeyEvent::new(KeyCode::Char('m'), KeyModifiers::NONE));
    state = next;

    // mr -> SurroundReplaceFromPending
    let (_, next) = state.process_key(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE));
    state = next;
    assert_eq!(state.state_name(), "SURROUND_REPLACE_FROM_PENDING");

    // ( -> SurroundReplaceToPending
    let (_, next) = state.process_key(KeyEvent::new(KeyCode::Char('('), KeyModifiers::NONE));
    state = next;
    assert_eq!(state.state_name(), "SURROUND_REPLACE_TO_PENDING");

    // [ -> execute surround replace
    let (result, next) = state.process_key(KeyEvent::new(KeyCode::Char('['), KeyModifiers::NONE));
    assert!(result.is_execute());
    state = next;
    assert!(state.is_base());
}

#[test]
fn test_typestate_handler_match_to_text_object_around() {
    let mut state = TypestateHandlerState::new();

    // m -> MatchPending
    let (_, next) = state.process_key(KeyEvent::new(KeyCode::Char('m'), KeyModifiers::NONE));
    state = next;

    // ma -> TextObjectAroundPending
    let (_, next) = state.process_key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE));
    state = next;
    assert_eq!(state.state_name(), "TEXT_OBJECT_AROUND_PENDING");
}

#[test]
fn test_typestate_handler_match_to_text_object_inside() {
    let mut state = TypestateHandlerState::new();

    // m -> MatchPending
    let (_, next) = state.process_key(KeyEvent::new(KeyCode::Char('m'), KeyModifiers::NONE));
    state = next;

    // mi -> TextObjectInsidePending
    let (_, next) = state.process_key(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE));
    state = next;
    assert_eq!(state.state_name(), "TEXT_OBJECT_INSIDE_PENDING");
}

#[test]
fn test_typestate_handler_find_char() {
    let mut state = TypestateHandlerState::new();

    // f -> FindCharPending
    let (_, next) = state.process_key(KeyEvent::new(KeyCode::Char('f'), KeyModifiers::NONE));
    state = next;
    assert_eq!(state.state_name(), "FIND_CHAR_PENDING");

    // a -> execute find
    let (result, next) = state.process_key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE));
    assert!(result.is_execute());
    state = next;
    assert!(state.is_base());
}

#[test]
fn test_typestate_handler_replace_char() {
    let mut state = TypestateHandlerState::new();

    // r -> ReplaceCharPending
    let (_, next) = state.process_key(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE));
    state = next;
    assert_eq!(state.state_name(), "REPLACE_CHAR_PENDING");

    // a -> execute replace
    let (result, next) = state.process_key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE));
    assert!(result.is_execute());
    state = next;
    assert!(state.is_base());
}

#[test]
fn test_typestate_handler_count_pending() {
    let mut state = TypestateHandlerState::new();

    // 5 -> CountPending
    let (_, next) = state.process_key(KeyEvent::new(KeyCode::Char('5'), KeyModifiers::NONE));
    state = next;
    assert_eq!(state.state_name(), "COUNT_PENDING");

    // 3 -> still CountPending (53)
    let (_, next) = state.process_key(KeyEvent::new(KeyCode::Char('3'), KeyModifiers::NONE));
    state = next;
    assert_eq!(state.state_name(), "COUNT_PENDING");

    // j -> execute with count
    let (result, next) = state.process_key(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE));
    assert!(result.is_execute());
    // The command should include the count (e.g., "53j")
    let cmd = result.command().unwrap();
    assert!(cmd.contains("j"));
    state = next;
    assert!(state.is_base());
}

#[test]
fn test_typestate_handler_unmatched_prev() {
    let mut state = TypestateHandlerState::new();

    // [ -> UnmatchedPrevPending
    let (_, next) = state.process_key(KeyEvent::new(KeyCode::Char('['), KeyModifiers::NONE));
    state = next;
    assert_eq!(state.state_name(), "UNMATCHED_PREV_PENDING");

    // p -> execute (goto previous paragraph)
    let (result, next) = state.process_key(KeyEvent::new(KeyCode::Char('p'), KeyModifiers::NONE));
    assert!(result.is_execute());
    state = next;
    assert!(state.is_base());
}

#[test]
fn test_typestate_handler_unmatched_next() {
    let mut state = TypestateHandlerState::new();

    // ] -> UnmatchedNextPending
    let (_, next) = state.process_key(KeyEvent::new(KeyCode::Char(']'), KeyModifiers::NONE));
    state = next;
    assert_eq!(state.state_name(), "UNMATCHED_NEXT_PENDING");

    // p -> execute (goto next paragraph)
    let (result, next) = state.process_key(KeyEvent::new(KeyCode::Char('p'), KeyModifiers::NONE));
    assert!(result.is_execute());
    state = next;
    assert!(state.is_base());
}

#[test]
fn test_typestate_handler_unmatched_cancel() {
    let mut state = TypestateHandlerState::new();

    // [ -> UnmatchedPrevPending
    let (_, next) = state.process_key(KeyEvent::new(KeyCode::Char('['), KeyModifiers::NONE));
    state = next;
    assert_eq!(state.state_name(), "UNMATCHED_PREV_PENDING");

    // Any other key cancels and returns to base
    let (result, next) = state.process_key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE));
    assert!(result.is_cancel());
    state = next;
    assert!(state.is_base());
}

#[test]
fn test_typestate_handler_escape_returns_to_base() {
    let mut state = TypestateHandlerState::new();

    // g -> GotoPending
    let (_, next) = state.process_key(KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE));
    state = next;
    assert!(!state.is_base());

    // Escape -> back to base
    let (_, next) = state.process_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    state = next;
    assert!(state.is_base());
}

#[test]
fn test_typestate_handler_clone() {
    let state = TypestateHandlerState::new();
    let cloned = state.clone();
    assert!(cloned.is_base());
}

#[test]
fn test_typestate_handler_debug() {
    let state = TypestateHandlerState::new();
    assert!(!format!("{:?}", state).is_empty());
}
