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
