//! Tests for `HandlerResult` enum

use crate::testing::normal;

use crate::input::typestate::{HandlerResult, InputState};

#[test]
fn test_handler_result_predicates() {
    assert!(HandlerResult::Stay.is_stay());
    assert!(!HandlerResult::Stay.is_transition());

    assert!(HandlerResult::Transition(InputState::GotoPending).is_transition());

    assert_eq!(
        HandlerResult::Execute(normal("gg")).command(),
        Some(&normal("gg"))
    );
}
