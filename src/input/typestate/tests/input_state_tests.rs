//! Tests for `InputState` enum

use super::nz;
use crate::grammar::find::FindMotion;
use crate::input::typestate::InputState;

#[test]
fn test_input_state_default_is_base() {
    let state = InputState::default();
    assert!(state.is_base());
}

#[test]
fn test_is_waiting_for_char() {
    assert!(!InputState::Base.is_waiting_for_char());
    assert!(!InputState::GotoPending.is_waiting_for_char());
    assert!(
        InputState::FindCharPending {
            motion: FindMotion::FIND_FORWARD
        }
        .is_waiting_for_char()
    );
    assert!(InputState::ReplaceCharPending.is_waiting_for_char());
}

#[test]
fn test_is_prefix_state() {
    assert!(!InputState::Base.is_prefix_state());
    assert!(InputState::GotoPending.is_prefix_state());
    assert!(InputState::ViewPending.is_prefix_state());
    assert!(InputState::CountPending { count: nz(3) }.is_prefix_state());
}

// ============================================================================
// Additional predicate tests for full coverage
// ============================================================================

#[test]
fn test_surround_waiting_for_char() {
    // Surround states should also be waiting for char
    assert!(InputState::SurroundAddPending.is_waiting_for_char());
    assert!(InputState::SurroundDeletePending.is_waiting_for_char());
    assert!(InputState::SurroundReplaceFromPending.is_waiting_for_char());
    assert!(InputState::SurroundReplaceToPending { from_char: '[' }.is_waiting_for_char());
}

#[test]
fn test_name_returns_correct_strings() {
    assert_eq!(InputState::Base.name(), "BASE");
    assert_eq!(InputState::GotoPending.name(), "GOTO_PENDING");
    assert_eq!(InputState::ViewPending.name(), "VIEW_PENDING");
    assert_eq!(InputState::MatchPending.name(), "MATCH_PENDING");
    assert_eq!(
        InputState::SurroundAddPending.name(),
        "SURROUND_ADD_PENDING"
    );
    assert_eq!(
        InputState::SurroundDeletePending.name(),
        "SURROUND_DELETE_PENDING"
    );
    assert_eq!(
        InputState::SurroundReplaceFromPending.name(),
        "SURROUND_REPLACE_FROM_PENDING"
    );
    assert_eq!(
        InputState::SurroundReplaceToPending { from_char: '(' }.name(),
        "SURROUND_REPLACE_TO_PENDING"
    );
    assert_eq!(
        InputState::TextObjectAroundPending.name(),
        "TEXT_OBJECT_AROUND_PENDING"
    );
    assert_eq!(
        InputState::TextObjectInsidePending.name(),
        "TEXT_OBJECT_INSIDE_PENDING"
    );
    assert_eq!(
        InputState::FindCharPending {
            motion: FindMotion::FIND_FORWARD
        }
        .name(),
        "FIND_CHAR_PENDING"
    );
    assert_eq!(
        InputState::ReplaceCharPending.name(),
        "REPLACE_CHAR_PENDING"
    );
    assert_eq!(
        InputState::CountPending { count: nz(5) }.name(),
        "COUNT_PENDING"
    );
    assert_eq!(
        InputState::UnmatchedPrevPending.name(),
        "UNMATCHED_PREV_PENDING"
    );
    assert_eq!(
        InputState::UnmatchedNextPending.name(),
        "UNMATCHED_NEXT_PENDING"
    );
}
