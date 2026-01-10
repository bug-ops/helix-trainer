//! Tests for InputState enum

use crate::input::typestate::{FindType, InputState};

#[test]
fn test_input_state_default_is_base() {
    let state = InputState::default();
    assert!(state.is_base());
}

#[test]
fn test_input_state_predicates() {
    assert!(InputState::Base.is_base());
    assert!(InputState::GotoPending.is_goto_pending());
    assert!(InputState::ViewPending.is_view_pending());
    assert!(InputState::MatchPending.is_match_pending());
    assert!(
        InputState::FindCharPending {
            find_type: FindType::FindForward
        }
        .is_find_char_pending()
    );
    assert!(InputState::ReplaceCharPending.is_replace_char_pending());
    assert!(InputState::CountPending { count: 5 }.is_count_pending());
}

#[test]
fn test_is_waiting_for_char() {
    assert!(!InputState::Base.is_waiting_for_char());
    assert!(!InputState::GotoPending.is_waiting_for_char());
    assert!(
        InputState::FindCharPending {
            find_type: FindType::FindForward
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
    assert!(InputState::CountPending { count: 3 }.is_prefix_state());
}

// ============================================================================
// Additional predicate tests for full coverage
// ============================================================================

#[test]
fn test_is_surround_pending() {
    assert!(!InputState::Base.is_surround_pending());
    assert!(!InputState::GotoPending.is_surround_pending());
    assert!(InputState::SurroundAddPending.is_surround_pending());
    assert!(InputState::SurroundDeletePending.is_surround_pending());
    assert!(InputState::SurroundReplaceFromPending.is_surround_pending());
    assert!(InputState::SurroundReplaceToPending { from_char: '(' }.is_surround_pending());
}

#[test]
fn test_is_text_object_pending() {
    assert!(!InputState::Base.is_text_object_pending());
    assert!(!InputState::GotoPending.is_text_object_pending());
    assert!(InputState::TextObjectAroundPending.is_text_object_pending());
    assert!(InputState::TextObjectInsidePending.is_text_object_pending());
}

#[test]
fn test_is_unmatched_pending() {
    assert!(!InputState::Base.is_unmatched_prev_pending());
    assert!(!InputState::Base.is_unmatched_next_pending());
    assert!(InputState::UnmatchedPrevPending.is_unmatched_prev_pending());
    assert!(InputState::UnmatchedNextPending.is_unmatched_next_pending());
}

#[test]
fn test_surround_waiting_for_char() {
    // Surround states should also be waiting for char
    assert!(InputState::SurroundAddPending.is_waiting_for_char());
    assert!(InputState::SurroundDeletePending.is_waiting_for_char());
    assert!(InputState::SurroundReplaceFromPending.is_waiting_for_char());
    assert!(InputState::SurroundReplaceToPending { from_char: '[' }.is_waiting_for_char());
}

#[test]
fn test_find_type_all_variants() {
    // Test all FindType variants
    assert!(
        InputState::FindCharPending {
            find_type: FindType::FindForward
        }
        .is_find_char_pending()
    );
    assert!(
        InputState::FindCharPending {
            find_type: FindType::FindBackward
        }
        .is_find_char_pending()
    );
    assert!(
        InputState::FindCharPending {
            find_type: FindType::TillForward
        }
        .is_find_char_pending()
    );
    assert!(
        InputState::FindCharPending {
            find_type: FindType::TillBackward
        }
        .is_find_char_pending()
    );
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
            find_type: FindType::FindForward
        }
        .name(),
        "FIND_CHAR_PENDING"
    );
    assert_eq!(
        InputState::ReplaceCharPending.name(),
        "REPLACE_CHAR_PENDING"
    );
    assert_eq!(
        InputState::CountPending { count: 5 }.name(),
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
