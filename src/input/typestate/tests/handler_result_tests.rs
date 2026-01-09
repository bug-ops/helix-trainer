//! Tests for HandlerResult enum

use std::borrow::Cow;

use crate::input::typestate::{HandlerResult, InputState};

#[test]
fn test_handler_result_predicates() {
    assert!(HandlerResult::Stay.is_stay());
    assert!(!HandlerResult::Stay.is_transition());

    assert!(HandlerResult::Transition(InputState::GotoPending).is_transition());
    assert!(!HandlerResult::Transition(InputState::GotoPending).is_execute());

    assert!(HandlerResult::Execute(Cow::Borrowed("gg")).is_execute());
    assert_eq!(
        HandlerResult::Execute(Cow::Borrowed("gg")).command(),
        Some("gg")
    );

    assert!(HandlerResult::Cancel.is_cancel());
}
