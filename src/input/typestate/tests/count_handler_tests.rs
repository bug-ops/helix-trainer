//! Tests for CountPending handler

use std::assert_matches;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::input::typestate::{
    CountPending, HandlerResult, InputHandler, InputState, KeyHandler, handlers::count::MAX_COUNT,
};

#[test]
fn test_count_pending_more_digits() {
    let state = CountPending { count: 3 };
    let result = KeyHandler::handle_key(
        &state,
        KeyEvent::new(KeyCode::Char('5'), KeyModifiers::NONE),
    );
    assert_matches!(
        result,
        HandlerResult::Transition(InputState::CountPending { count: 35 })
    );
}

#[test]
fn test_count_pending_command() {
    let state = CountPending { count: 3 };
    let result = KeyHandler::handle_key(
        &state,
        KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE),
    );
    assert_matches!(result, HandlerResult::Execute(cmd) if cmd == "3j");
}

#[test]
fn test_count_pending_invalid_command_cancels() {
    let state = CountPending { count: 3 };
    // 'g' is not count-compatible (it's a prefix)
    let result = KeyHandler::handle_key(
        &state,
        KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE),
    );
    assert_matches!(result, HandlerResult::Cancel);
}

#[test]
fn test_count_pending_escape_cancels() {
    let state = CountPending { count: 5 };
    let result = KeyHandler::handle_key(&state, KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    assert_matches!(result, HandlerResult::Cancel);
}

#[test]
fn test_count_pending_overflow_protection() {
    // Start with a large count close to overflow
    let state = CountPending {
        count: usize::MAX / 10,
    };
    // Adding another digit should not panic and should clamp to MAX_COUNT
    let result = KeyHandler::handle_key(
        &state,
        KeyEvent::new(KeyCode::Char('9'), KeyModifiers::NONE),
    );
    // Result should be a transition with count clamped to MAX_COUNT
    match result {
        HandlerResult::Transition(InputState::CountPending { count }) => {
            assert!(
                count <= MAX_COUNT,
                "Count {} should be <= MAX_COUNT {}",
                count,
                MAX_COUNT
            );
        }
        _ => panic!("Expected Transition to CountPending"),
    }
}

/// M4: `3"ay` cancels rather than combining a count with a register
/// selection - `'"'` has no arm in `is_count_compatible_command`, so it
/// falls to the "invalid command" branch, same as any other incompatible
/// key. Verifies the state resets to `Base` (no leaked pending state), as
/// the architect's plan explicitly asked for.
#[test]
fn test_count_pending_register_prefix_cancels() {
    let state = CountPending { count: 3 };
    let result = KeyHandler::handle_key(
        &state,
        KeyEvent::new(KeyCode::Char('"'), KeyModifiers::NONE),
    );
    assert_matches!(result, HandlerResult::Cancel);
}

#[test]
fn test_count_pending_clamps_at_max() {
    // Test that count is clamped to MAX_COUNT
    let state = CountPending { count: 9999 };
    let result = KeyHandler::handle_key(
        &state,
        KeyEvent::new(KeyCode::Char('9'), KeyModifiers::NONE),
    );
    // 9999 * 10 + 9 = 99999 which exceeds MAX_COUNT (10000)
    // So it should be clamped to MAX_COUNT
    match result {
        HandlerResult::Transition(InputState::CountPending { count }) => {
            assert_eq!(count, MAX_COUNT);
        }
        _ => panic!("Expected Transition to CountPending"),
    }
}
