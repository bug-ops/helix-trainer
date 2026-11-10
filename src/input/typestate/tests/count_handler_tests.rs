//! Tests for CountPending handler

use std::assert_matches;
use std::borrow::Cow;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::nz;
use crate::input::typestate::{
    CountPending, CountedFindCharPending, CountedGotoPending, FindType, HandlerResult,
    InputHandler, InputState, InputStateMachine, KeyHandler, handlers::count::MAX_COUNT,
};

fn key(c: char) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE)
}

fn press(sm: &mut InputStateMachine, keys: &str) -> HandlerResult {
    keys.chars()
        .map(|c| sm.process_key(key(c)))
        .last()
        .expect("non-empty key sequence")
}

#[test]
fn test_count_pending_more_digits() {
    let state = CountPending { count: nz(3) };
    let result = KeyHandler::handle_key(&state, key('5'));
    assert_eq!(
        result,
        HandlerResult::Transition(InputState::CountPending { count: nz(35) })
    );
}

#[test]
fn test_count_pending_command() {
    let state = CountPending { count: nz(3) };
    let result = KeyHandler::handle_key(&state, key('j'));
    assert_matches!(result, HandlerResult::Execute(cmd) if cmd == "3j");
}

#[test]
fn test_count_pending_unsupported_prefix_cancels() {
    let state = CountPending { count: nz(3) };
    // 'z' is a prefix that does not take a count
    let result = KeyHandler::handle_key(&state, key('z'));
    assert_matches!(result, HandlerResult::Cancel);
}

#[test]
fn test_count_pending_escape_cancels() {
    let state = CountPending { count: nz(5) };
    let result = KeyHandler::handle_key(&state, KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    assert_matches!(result, HandlerResult::Cancel);
}

#[test]
fn test_count_pending_overflow_protection() {
    let state = CountPending {
        count: nz(usize::MAX / 10),
    };
    let result = KeyHandler::handle_key(&state, key('9'));
    assert_eq!(
        result,
        HandlerResult::Transition(InputState::CountPending {
            count: nz(MAX_COUNT)
        })
    );
}

/// M4: `3"ay` cancels rather than combining a count with a register
/// selection - `'"'` has no arm in `is_count_compatible_command`.
#[test]
fn test_count_pending_register_prefix_cancels() {
    let state = CountPending { count: nz(3) };
    let result = KeyHandler::handle_key(&state, key('"'));
    assert_matches!(result, HandlerResult::Cancel);
}

#[test]
fn test_count_pending_clamps_at_max() {
    let state = CountPending { count: nz(9999) };
    let result = KeyHandler::handle_key(&state, key('9'));
    assert_eq!(
        result,
        HandlerResult::Transition(InputState::CountPending {
            count: nz(MAX_COUNT)
        })
    );
}

// #493: counted G / gg / f / t / F / T

#[test]
fn test_count_then_capital_g_executes_goto_line() {
    let state = CountPending { count: nz(2) };
    let result = KeyHandler::handle_key(
        &state,
        KeyEvent::new(KeyCode::Char('G'), KeyModifiers::SHIFT),
    );
    assert_eq!(result, HandlerResult::Execute(Cow::Owned("2G".to_string())));
}

#[test]
fn test_count_then_g_carries_count_into_counted_goto() {
    let state = CountPending { count: nz(2) };
    let result = KeyHandler::handle_key(&state, key('g'));
    assert_eq!(
        result,
        HandlerResult::Transition(InputState::CountedGotoPending { count: nz(2) })
    );
}

#[test]
fn test_counted_goto_second_g_executes_goto_line() {
    let state = CountedGotoPending { count: nz(12) };
    assert_eq!(
        KeyHandler::handle_key(&state, key('g')),
        HandlerResult::Execute(Cow::Owned("12G".to_string()))
    );
    assert_eq!(
        KeyHandler::handle_key(&state, key('h')),
        HandlerResult::Cancel
    );
    assert_eq!(
        KeyHandler::handle_key(&state, KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)),
        HandlerResult::Cancel
    );
}

#[test]
fn test_count_then_find_keys_carry_count() {
    let state = CountPending { count: nz(3) };
    for (c, mods, find_type) in [
        ('f', KeyModifiers::NONE, FindType::FindForward),
        ('F', KeyModifiers::SHIFT, FindType::FindBackward),
        ('t', KeyModifiers::NONE, FindType::TillForward),
        ('T', KeyModifiers::SHIFT, FindType::TillBackward),
    ] {
        let result = KeyHandler::handle_key(&state, KeyEvent::new(KeyCode::Char(c), mods));
        assert_eq!(
            result,
            HandlerResult::Transition(InputState::CountedFindCharPending {
                find_type,
                count: nz(3)
            }),
            "key {c}"
        );
    }
}

#[test]
fn test_counted_find_executes_with_count_and_target() {
    let state = CountedFindCharPending {
        find_type: FindType::FindForward,
        count: nz(3),
    };
    assert_eq!(
        KeyHandler::handle_key(&state, key('a')),
        HandlerResult::Execute(Cow::Owned("3fa".to_string()))
    );
    assert_eq!(
        KeyHandler::handle_key(&state, key('5')),
        HandlerResult::Execute(Cow::Owned("3f5".to_string()))
    );
    assert_eq!(
        KeyHandler::handle_key(&state, KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)),
        HandlerResult::Cancel
    );
}

#[test]
fn test_alt_g_and_alt_f_under_count_never_open_pending_state() {
    let state = CountPending { count: nz(3) };
    for c in ['g', 'f', 't'] {
        let result =
            KeyHandler::handle_key(&state, KeyEvent::new(KeyCode::Char(c), KeyModifiers::ALT));
        assert_eq!(result, HandlerResult::Cancel, "Alt-{c}");
    }
}

#[test]
fn test_machine_counted_goto_and_find_sequences() {
    let mut sm = InputStateMachine::new();
    assert_eq!(
        press(&mut sm, "2G"),
        HandlerResult::Execute(Cow::Owned("2G".to_string()))
    );
    assert!(sm.state().is_base());

    assert_eq!(
        press(&mut sm, "2gg"),
        HandlerResult::Execute(Cow::Owned("2G".to_string()))
    );
    assert!(sm.state().is_base());

    assert_eq!(
        press(&mut sm, "3fa"),
        HandlerResult::Execute(Cow::Owned("3fa".to_string()))
    );
    assert!(sm.state().is_base());

    assert_eq!(
        press(&mut sm, "12ta"),
        HandlerResult::Execute(Cow::Owned("12ta".to_string()))
    );
}

#[test]
fn test_machine_counted_find_is_waiting_for_char() {
    let mut sm = InputStateMachine::new();
    press(&mut sm, "3f");
    assert!(sm.is_waiting_for_char());
    assert_eq!(sm.state().name(), "COUNTED_FIND_CHAR_PENDING");
}
