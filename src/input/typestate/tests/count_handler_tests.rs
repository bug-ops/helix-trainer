//! Tests for `CountPending` handler

use crate::grammar::find::FindMotion;
use crate::testing::normal;
use std::assert_matches;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::nz;
use crate::input::typestate::{
    CountPending, CountedFindCharPending, CountedGotoPending, HandlerResult, InputHandler,
    InputState, InputStateMachine, KeyHandler, handlers::count::MAX_COUNT,
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
    assert_matches!(result, HandlerResult::Execute(cmd) if cmd.to_string() == "3j");
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
    assert_eq!(result, HandlerResult::Execute(normal("2G")));
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
        HandlerResult::Execute(normal("12G"))
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
    for (c, mods, motion) in [
        ('f', KeyModifiers::NONE, FindMotion::FIND_FORWARD),
        ('F', KeyModifiers::SHIFT, FindMotion::FIND_BACKWARD),
        ('t', KeyModifiers::NONE, FindMotion::TILL_FORWARD),
        ('T', KeyModifiers::SHIFT, FindMotion::TILL_BACKWARD),
    ] {
        let result = KeyHandler::handle_key(&state, KeyEvent::new(KeyCode::Char(c), mods));
        assert_eq!(
            result,
            HandlerResult::Transition(InputState::CountedFindCharPending {
                motion,
                count: nz(3)
            }),
            "key {c}"
        );
    }
}

#[test]
fn test_counted_find_executes_with_count_and_target() {
    let state = CountedFindCharPending {
        motion: FindMotion::FIND_FORWARD,
        count: nz(3),
    };
    assert_eq!(
        KeyHandler::handle_key(&state, key('a')),
        HandlerResult::Execute(normal("3fa"))
    );
    assert_eq!(
        KeyHandler::handle_key(&state, key('5')),
        HandlerResult::Execute(normal("3f5"))
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
    assert_eq!(press(&mut sm, "2G"), HandlerResult::Execute(normal("2G")));
    assert!(sm.state().is_base());

    assert_eq!(press(&mut sm, "2gg"), HandlerResult::Execute(normal("2G")));
    assert!(sm.state().is_base());

    assert_eq!(press(&mut sm, "3fa"), HandlerResult::Execute(normal("3fa")));
    assert!(sm.state().is_base());

    assert_eq!(
        press(&mut sm, "12ta"),
        HandlerResult::Execute(normal("12ta"))
    );
}

#[test]
fn test_machine_counted_find_is_waiting_for_char() {
    let mut sm = InputStateMachine::new();
    press(&mut sm, "3f");
    assert!(sm.is_waiting_for_char());
    assert_eq!(sm.state().name(), "COUNTED_FIND_CHAR_PENDING");
}

// #577: a count before y r v i a I A C runs the command instead of cancelling

fn run_keys(text: &str, keys: &str) -> crate::helix::EditorSnapshot {
    let mut sim = crate::helix::AnyModeSimulator::new(text.to_string());
    let mut sm = InputStateMachine::new();
    for c in keys.chars() {
        if sim.is_insert_mode() {
            let cmd = if c == '\u{1b}' {
                "Escape".to_string()
            } else {
                c.to_string()
            };
            sim.execute_command(&cmd).expect("insert key");
        } else if let HandlerResult::Execute(cmd) = sm.process_key(key(c)) {
            sim.execute_command(&cmd.to_string())
                .expect("normal command");
        }
    }
    sim.to_snapshot()
}

#[test]
fn test_count_then_y_r_v_i_a_big_i_big_a_big_c_are_not_cancelled() {
    let state = CountPending { count: nz(2) };
    for (c, mods) in [
        ('y', KeyModifiers::NONE),
        ('v', KeyModifiers::NONE),
        ('i', KeyModifiers::NONE),
        ('a', KeyModifiers::NONE),
        ('I', KeyModifiers::SHIFT),
        ('A', KeyModifiers::SHIFT),
        ('C', KeyModifiers::SHIFT),
    ] {
        let result = KeyHandler::handle_key(&state, KeyEvent::new(KeyCode::Char(c), mods));
        assert_matches!(result, HandlerResult::Execute(_), "key {c}");
    }
    assert_eq!(
        KeyHandler::handle_key(&state, key('r')),
        HandlerResult::Transition(InputState::ReplaceCharPending)
    );
}

#[test]
fn test_count_then_o_and_big_o_execute_with_the_count_swallowed_by_the_simulator() {
    let state = CountPending { count: nz(2) };
    assert_eq!(
        KeyHandler::handle_key(&state, key('o')),
        HandlerResult::Execute(normal("2o"))
    );
    assert_eq!(
        KeyHandler::handle_key(
            &state,
            KeyEvent::new(KeyCode::Char('O'), KeyModifiers::SHIFT)
        ),
        HandlerResult::Execute(normal("2O"))
    );
}

/// #577, #596: the keys typed after a counted `o` go into every opened line, not into Normal mode
#[test]
fn test_counted_open_line_opens_n_lines_and_does_not_leak_the_typed_text() {
    let counted = run_keys("abc\n", "2ox\u{1b}");
    assert_eq!(counted.content, "abc\nx\nx\n");
    let above = run_keys("abc\n", "2Ox\u{1b}");
    assert_eq!(above.content, "x\nx\nabc\n");
}

/// Helix ignores the count for these commands, so `{n}<keys>` equals `<keys>`
#[test]
fn test_counted_ignore_commands_match_their_uncounted_form() {
    let text = "fn add(a: i32) {\n    let sum = a;\n}\n";
    for keys in [
        "yp", "rz", "vl", "ix\u{1b}", "ax\u{1b}", "Ix\u{1b}", "Ax\u{1b}",
    ] {
        let counted = format!("2{keys}");
        assert_eq!(run_keys(text, &counted), run_keys(text, keys), "{counted}");
        assert_ne!(run_keys(text, &counted), run_keys(text, ""), "{counted}");
    }
}

#[test]
fn test_counted_r_does_not_leak_the_argument_as_a_command() {
    assert_eq!(run_keys("abc\n", "2rzd").content, "bc\n");
}

#[test]
fn test_counted_big_c_copies_the_selection_to_the_following_lines() {
    assert_eq!(run_keys("ab\ncd\nef\n", "2Cd").content, "b\nd\nf\n");
}
