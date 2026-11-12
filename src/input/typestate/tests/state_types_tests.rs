//! Tests for state type marker types

use super::nz;
use crate::grammar::find::FindMotion;
use crate::input::typestate::{
    BaseState, CountPending, FindCharPending, GotoPending, HandlerState, MatchPending,
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
    let a = CountPending { count: nz(5) };
    let b = CountPending { count: nz(5) };
    let c = CountPending { count: nz(10) };
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
        motion: FindMotion::FIND_FORWARD,
    };
    let b = FindCharPending {
        motion: FindMotion::FIND_FORWARD,
    };
    let c = FindCharPending {
        motion: FindMotion::FIND_BACKWARD,
    };
    assert_eq!(a, b);
    assert_ne!(a, c);
}

// Test all state types are Debug
#[test]
#[expect(
    clippy::assert_is_empty,
    reason = "assert!(x.is_empty()) reads better than the machine fix"
)]
fn test_all_state_types_debug() {
    assert!(!format!("{BaseState:?}").is_empty());
    assert!(!format!("{GotoPending:?}").is_empty());
    assert!(!format!("{ViewPending:?}").is_empty());
    assert!(!format!("{MatchPending:?}").is_empty());
    assert!(!format!("{SurroundAddPending:?}").is_empty());
    assert!(!format!("{SurroundDeletePending:?}").is_empty());
    assert!(!format!("{SurroundReplaceFromPending:?}").is_empty());
    assert!(!format!("{:?}", SurroundReplaceToPending { from_char: '(' }).is_empty());
    assert!(!format!("{TextObjectAroundPending:?}").is_empty());
    assert!(!format!("{TextObjectInsidePending:?}").is_empty());
    assert!(
        !format!(
            "{:?}",
            FindCharPending {
                motion: FindMotion::FIND_FORWARD
            }
        )
        .is_empty()
    );
    assert!(!format!("{ReplaceCharPending:?}").is_empty());
    assert!(!format!("{:?}", CountPending { count: nz(5) }).is_empty());
    assert!(!format!("{UnmatchedPrevPending:?}").is_empty());
    assert!(!format!("{UnmatchedNextPending:?}").is_empty());
}
