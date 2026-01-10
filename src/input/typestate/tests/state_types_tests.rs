//! Tests for state type marker types

use crate::input::typestate::{
    BaseState, CountPending, FindCharPending, FindType, GotoPending, HandlerState, MatchPending,
    ReplaceCharPending, SurroundAddPending, SurroundDeletePending, SurroundReplaceFromPending,
    SurroundReplaceToPending, TextObjectAroundPending, TextObjectInsidePending,
    UnmatchedNextPending, UnmatchedPrevPending, ViewPending,
};

// ============================================================================
// HandlerState trait tests
// ============================================================================

#[test]
fn test_base_state_name() {
    assert_eq!(BaseState::state_name(), "BASE");
}

#[test]
fn test_goto_pending_state_name() {
    assert_eq!(GotoPending::state_name(), "GOTO_PENDING");
}

#[test]
fn test_view_pending_state_name() {
    assert_eq!(ViewPending::state_name(), "VIEW_PENDING");
}

#[test]
fn test_match_pending_state_name() {
    assert_eq!(MatchPending::state_name(), "MATCH_PENDING");
}

#[test]
fn test_surround_add_pending_state_name() {
    assert_eq!(SurroundAddPending::state_name(), "SURROUND_ADD_PENDING");
}

#[test]
fn test_surround_delete_pending_state_name() {
    assert_eq!(
        SurroundDeletePending::state_name(),
        "SURROUND_DELETE_PENDING"
    );
}

#[test]
fn test_surround_replace_from_pending_state_name() {
    assert_eq!(
        SurroundReplaceFromPending::state_name(),
        "SURROUND_REPLACE_FROM_PENDING"
    );
}

#[test]
fn test_surround_replace_to_pending_state_name() {
    assert_eq!(
        SurroundReplaceToPending::state_name(),
        "SURROUND_REPLACE_TO_PENDING"
    );
}

#[test]
fn test_text_object_around_pending_state_name() {
    assert_eq!(
        TextObjectAroundPending::state_name(),
        "TEXT_OBJECT_AROUND_PENDING"
    );
}

#[test]
fn test_text_object_inside_pending_state_name() {
    assert_eq!(
        TextObjectInsidePending::state_name(),
        "TEXT_OBJECT_INSIDE_PENDING"
    );
}

#[test]
fn test_find_char_pending_state_name() {
    assert_eq!(FindCharPending::state_name(), "FIND_CHAR_PENDING");
}

#[test]
fn test_replace_char_pending_state_name() {
    assert_eq!(ReplaceCharPending::state_name(), "REPLACE_CHAR_PENDING");
}

#[test]
fn test_count_pending_state_name() {
    assert_eq!(CountPending::state_name(), "COUNT_PENDING");
}

#[test]
fn test_unmatched_prev_pending_state_name() {
    assert_eq!(UnmatchedPrevPending::state_name(), "UNMATCHED_PREV_PENDING");
}

#[test]
fn test_unmatched_next_pending_state_name() {
    assert_eq!(UnmatchedNextPending::state_name(), "UNMATCHED_NEXT_PENDING");
}

// ============================================================================
// FindType tests
// ============================================================================

#[test]
fn test_find_type_prefix() {
    assert_eq!(FindType::FindForward.prefix(), "f");
    assert_eq!(FindType::FindBackward.prefix(), "F");
    assert_eq!(FindType::TillForward.prefix(), "t");
    assert_eq!(FindType::TillBackward.prefix(), "T");
}

#[test]
fn test_find_type_equality() {
    assert_eq!(FindType::FindForward, FindType::FindForward);
    assert_ne!(FindType::FindForward, FindType::FindBackward);
    assert_ne!(FindType::TillForward, FindType::TillBackward);
}

#[test]
fn test_find_type_clone() {
    let find_type = FindType::FindForward;
    let cloned = find_type;
    assert_eq!(find_type, cloned);
}

#[test]
fn test_find_type_debug() {
    let find_type = FindType::FindForward;
    assert!(format!("{:?}", find_type).contains("FindForward"));
}

// ============================================================================
// State type trait derivations tests
// ============================================================================

#[test]
fn test_base_state_default() {
    let state = BaseState;
    assert_eq!(state, BaseState);
}

#[test]
fn test_base_state_clone() {
    let state = BaseState;
    let cloned = state;
    assert_eq!(state, cloned);
}

#[test]
fn test_count_pending_equality() {
    let a = CountPending { count: 5 };
    let b = CountPending { count: 5 };
    let c = CountPending { count: 10 };
    assert_eq!(a, b);
    assert_ne!(a, c);
}

#[test]
fn test_surround_replace_to_pending_equality() {
    let a = SurroundReplaceToPending { from_char: '(' };
    let b = SurroundReplaceToPending { from_char: '(' };
    let c = SurroundReplaceToPending { from_char: '[' };
    assert_eq!(a, b);
    assert_ne!(a, c);
}

#[test]
fn test_find_char_pending_equality() {
    let a = FindCharPending {
        find_type: FindType::FindForward,
    };
    let b = FindCharPending {
        find_type: FindType::FindForward,
    };
    let c = FindCharPending {
        find_type: FindType::FindBackward,
    };
    assert_eq!(a, b);
    assert_ne!(a, c);
}

// Test all state types are Debug
#[test]
fn test_all_state_types_debug() {
    assert!(!format!("{:?}", BaseState).is_empty());
    assert!(!format!("{:?}", GotoPending).is_empty());
    assert!(!format!("{:?}", ViewPending).is_empty());
    assert!(!format!("{:?}", MatchPending).is_empty());
    assert!(!format!("{:?}", SurroundAddPending).is_empty());
    assert!(!format!("{:?}", SurroundDeletePending).is_empty());
    assert!(!format!("{:?}", SurroundReplaceFromPending).is_empty());
    assert!(!format!("{:?}", SurroundReplaceToPending { from_char: '(' }).is_empty());
    assert!(!format!("{:?}", TextObjectAroundPending).is_empty());
    assert!(!format!("{:?}", TextObjectInsidePending).is_empty());
    assert!(!format!("{:?}", FindCharPending { find_type: FindType::FindForward }).is_empty());
    assert!(!format!("{:?}", ReplaceCharPending).is_empty());
    assert!(!format!("{:?}", CountPending { count: 5 }).is_empty());
    assert!(!format!("{:?}", UnmatchedPrevPending).is_empty());
    assert!(!format!("{:?}", UnmatchedNextPending).is_empty());
}
