//! Tests for InputStateMachine

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::helix::commands::*;
use crate::input::typestate::{InputState, InputStateMachine};

#[test]
fn test_state_machine_initial_state() {
    let sm = InputStateMachine::new();
    assert!(sm.state().is_base());
}

#[test]
fn test_state_machine_goto_sequence() {
    let mut sm = InputStateMachine::new();

    // Press 'g' - transition to GotoPending
    let result = sm.process_key(KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE));
    assert!(result.is_transition());
    assert!(sm.state().is_goto_pending());

    // Press 'g' again - execute "gg"
    let result = sm.process_key(KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE));
    assert!(result.is_execute());
    assert_eq!(result.command(), Some(CMD_GOTO_FILE_START));
    assert!(sm.state().is_base());
}

#[test]
fn test_state_machine_find_sequence() {
    let mut sm = InputStateMachine::new();

    // Press 'f' - transition to FindCharPending
    let result = sm.process_key(KeyEvent::new(KeyCode::Char('f'), KeyModifiers::NONE));
    assert!(result.is_transition());
    assert!(sm.state().is_find_char_pending());

    // Press 'a' - execute "fa"
    let result = sm.process_key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE));
    assert!(result.is_execute());
    assert_eq!(result.command(), Some("fa"));
    assert!(sm.state().is_base());
}

#[test]
fn test_state_machine_count_sequence() {
    let mut sm = InputStateMachine::new();

    // Press '1' - transition to CountPending
    let result = sm.process_key(KeyEvent::new(KeyCode::Char('1'), KeyModifiers::NONE));
    assert!(result.is_transition());
    assert!(sm.state().is_count_pending());
    assert_eq!(sm.pending_count(), Some(1));

    // Press '2' - continue building count
    let result = sm.process_key(KeyEvent::new(KeyCode::Char('2'), KeyModifiers::NONE));
    assert!(result.is_transition());
    assert_eq!(sm.pending_count(), Some(12));

    // Press 'j' - execute "12j"
    let result = sm.process_key(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE));
    assert!(result.is_execute());
    assert_eq!(result.command(), Some("12j"));
    assert!(sm.state().is_base());
}

#[test]
fn test_state_machine_cancel_returns_to_base() {
    let mut sm = InputStateMachine::new();

    // Press 'g' - transition to GotoPending
    sm.process_key(KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE));
    assert!(sm.state().is_goto_pending());

    // Press Escape - cancel
    let result = sm.process_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    assert!(result.is_cancel());
    assert!(sm.state().is_base());
}

#[test]
fn test_state_machine_reset() {
    let mut sm = InputStateMachine::new();

    sm.process_key(KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE));
    assert!(sm.state().is_goto_pending());

    sm.reset();
    assert!(sm.state().is_base());
}

#[test]
fn test_state_machine_surround_add_sequence() {
    let mut sm = InputStateMachine::new();

    // Press 'm' - transition to MatchPending
    let result = sm.process_key(KeyEvent::new(KeyCode::Char('m'), KeyModifiers::NONE));
    assert!(result.is_transition());
    assert!(sm.state().is_match_pending());

    // Press 's' - transition to SurroundAddPending
    let result = sm.process_key(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE));
    assert!(result.is_transition());
    assert!(matches!(sm.state(), InputState::SurroundAddPending));

    // Press '(' - execute "ms("
    let result = sm.process_key(KeyEvent::new(KeyCode::Char('('), KeyModifiers::NONE));
    assert!(result.is_execute());
    assert_eq!(result.command(), Some("ms("));
    assert!(sm.state().is_base());
}

#[test]
fn test_state_machine_surround_delete_sequence() {
    let mut sm = InputStateMachine::new();

    // Press 'm' - transition to MatchPending
    sm.process_key(KeyEvent::new(KeyCode::Char('m'), KeyModifiers::NONE));
    assert!(sm.state().is_match_pending());

    // Press 'd' - transition to SurroundDeletePending
    let result = sm.process_key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE));
    assert!(result.is_transition());
    assert!(matches!(sm.state(), InputState::SurroundDeletePending));

    // Press '{' - execute "md{"
    let result = sm.process_key(KeyEvent::new(KeyCode::Char('{'), KeyModifiers::NONE));
    assert!(result.is_execute());
    assert_eq!(result.command(), Some("md{"));
    assert!(sm.state().is_base());
}

#[test]
fn test_state_machine_surround_replace_sequence() {
    let mut sm = InputStateMachine::new();

    // Press 'm' - transition to MatchPending
    sm.process_key(KeyEvent::new(KeyCode::Char('m'), KeyModifiers::NONE));
    assert!(sm.state().is_match_pending());

    // Press 'r' - transition to SurroundReplaceFromPending
    let result = sm.process_key(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE));
    assert!(result.is_transition());
    assert!(matches!(sm.state(), InputState::SurroundReplaceFromPending));

    // Press '(' - transition to SurroundReplaceToPending
    let result = sm.process_key(KeyEvent::new(KeyCode::Char('('), KeyModifiers::NONE));
    assert!(result.is_transition());
    assert!(matches!(
        sm.state(),
        InputState::SurroundReplaceToPending { from_char: '(' }
    ));

    // Press '[' - execute "mr(["
    let result = sm.process_key(KeyEvent::new(KeyCode::Char('['), KeyModifiers::NONE));
    assert!(result.is_execute());
    assert_eq!(result.command(), Some("mr(["));
    assert!(sm.state().is_base());
}

#[test]
fn test_state_machine_surround_escape_cancels() {
    let mut sm = InputStateMachine::new();

    // Go to SurroundAddPending
    sm.process_key(KeyEvent::new(KeyCode::Char('m'), KeyModifiers::NONE));
    sm.process_key(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE));
    assert!(matches!(sm.state(), InputState::SurroundAddPending));

    // Press Escape - should cancel
    let result = sm.process_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    assert!(result.is_cancel());
    assert!(sm.state().is_base());
}

#[test]
fn test_surround_pending_predicates() {
    assert!(InputState::SurroundAddPending.is_surround_pending());
    assert!(InputState::SurroundDeletePending.is_surround_pending());
    assert!(InputState::SurroundReplaceFromPending.is_surround_pending());
    assert!(InputState::SurroundReplaceToPending { from_char: '(' }.is_surround_pending());
    assert!(!InputState::Base.is_surround_pending());
    assert!(!InputState::MatchPending.is_surround_pending());
}

#[test]
fn test_state_machine_text_object_around_sequence() {
    let mut sm = InputStateMachine::new();

    // Press 'm' - transition to MatchPending
    let result = sm.process_key(KeyEvent::new(KeyCode::Char('m'), KeyModifiers::NONE));
    assert!(result.is_transition());
    assert!(sm.state().is_match_pending());

    // Press 'a' - transition to TextObjectAroundPending
    let result = sm.process_key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE));
    assert!(result.is_transition());
    assert!(matches!(sm.state(), InputState::TextObjectAroundPending));

    // Press 'w' - execute "maw"
    let result = sm.process_key(KeyEvent::new(KeyCode::Char('w'), KeyModifiers::NONE));
    assert!(result.is_execute());
    assert_eq!(result.command(), Some("maw"));
    assert!(sm.state().is_base());
}

#[test]
fn test_state_machine_text_object_inside_sequence() {
    let mut sm = InputStateMachine::new();

    // Press 'm' - transition to MatchPending
    sm.process_key(KeyEvent::new(KeyCode::Char('m'), KeyModifiers::NONE));
    assert!(sm.state().is_match_pending());

    // Press 'i' - transition to TextObjectInsidePending
    let result = sm.process_key(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE));
    assert!(result.is_transition());
    assert!(matches!(sm.state(), InputState::TextObjectInsidePending));

    // Press '(' - execute "mi("
    let result = sm.process_key(KeyEvent::new(KeyCode::Char('('), KeyModifiers::NONE));
    assert!(result.is_execute());
    assert_eq!(result.command(), Some("mi("));
    assert!(sm.state().is_base());
}

#[test]
fn test_state_machine_text_object_escape_cancels() {
    let mut sm = InputStateMachine::new();

    // Go to TextObjectAroundPending
    sm.process_key(KeyEvent::new(KeyCode::Char('m'), KeyModifiers::NONE));
    sm.process_key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE));
    assert!(matches!(sm.state(), InputState::TextObjectAroundPending));

    // Press Escape - should cancel
    let result = sm.process_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    assert!(result.is_cancel());
    assert!(sm.state().is_base());
}

#[test]
fn test_text_object_pending_predicates() {
    assert!(InputState::TextObjectAroundPending.is_text_object_pending());
    assert!(InputState::TextObjectInsidePending.is_text_object_pending());
    assert!(!InputState::Base.is_text_object_pending());
    assert!(!InputState::MatchPending.is_text_object_pending());
    assert!(!InputState::SurroundAddPending.is_text_object_pending());
}
