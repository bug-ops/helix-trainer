//! Input layer -> simulator integration: every command string the input layer
//! emits must be accepted by the simulator (an unknown command is a hard
//! error), and the resulting buffer must match Helix 25.07.1.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::helix::simulator::AnyModeSimulator;
use crate::input::typestate::{HandlerResult, InputStateMachine, handle_insert_mode_input};

const SOURCE: &str = "fn add(a: i32, b: i32) -> i32 {\n    let sum = a + b;\n    sum\n}\n";

struct Harness {
    sim: AnyModeSimulator,
    machine: InputStateMachine,
}

impl Harness {
    fn new() -> Self {
        Self {
            sim: AnyModeSimulator::new(SOURCE.to_string()),
            machine: InputStateMachine::new(),
        }
    }

    fn press(&mut self, key: KeyEvent) {
        let cmd = if self.sim.is_insert_mode() {
            handle_insert_mode_input(key).map(std::borrow::Cow::into_owned)
        } else {
            match self.machine.process_key(key) {
                HandlerResult::Execute(cmd) => Some(cmd.into_owned()),
                _ => None,
            }
        };
        if let Some(cmd) = cmd {
            self.sim
                .execute_command(&cmd)
                .unwrap_or_else(|e| panic!("simulator rejected {cmd:?}: {e:?}"));
        }
    }

    fn type_chars(&mut self, keys: &str) {
        for c in keys.chars() {
            let modifiers = if c.is_ascii_uppercase() {
                KeyModifiers::SHIFT
            } else {
                KeyModifiers::NONE
            };
            self.press(KeyEvent::new(KeyCode::Char(c), modifiers));
        }
    }

    fn content(&self) -> String {
        self.sim.state().unwrap().content().to_string()
    }
}

#[test]
fn counted_goto_line_then_delete_removes_one_char_of_that_line() {
    for keys in ["2Gd", "2ggd"] {
        let mut h = Harness::new();
        h.type_chars(keys);
        assert_eq!(
            h.content(),
            "fn add(a: i32, b: i32) -> i32 {\n   let sum = a + b;\n    sum\n}\n",
            "{keys}"
        );
    }
}

#[test]
fn counted_find_selects_to_the_nth_occurrence() {
    let mut h = Harness::new();
    h.type_chars("2fad");
    assert_eq!(
        h.content(),
        ": i32, b: i32) -> i32 {\n    let sum = a + b;\n    sum\n}\n"
    );
}

#[test]
fn counted_find_crosses_lines_like_helix() {
    let mut h = Harness::new();
    h.type_chars("3fad");
    assert_eq!(h.content(), " + b;\n    sum\n}\n");
}

#[test]
fn every_counted_find_variant_is_accepted() {
    for keys in ["3fa", "2ta", "9fa", "2Fa", "2Ta", "2f5"] {
        let mut h = Harness::new();
        h.type_chars("j$");
        h.type_chars(keys);
    }
}

#[test]
fn search_prompt_selects_next_match_and_next_continues() {
    let mut h = Harness::new();
    h.type_chars("/sum");
    h.press(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    h.type_chars("d");
    assert_eq!(
        h.content(),
        "fn add(a: i32, b: i32) -> i32 {\n    let  = a + b;\n    sum\n}\n"
    );
}

#[test]
fn alt_s_splits_selection_on_newlines_without_a_prompt() {
    let mut h = Harness::new();
    h.type_chars("%");
    h.press(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::ALT));
    assert!(h.machine.state().is_base());
    assert_eq!(h.content(), SOURCE);
}

#[test]
fn insert_mode_editing_keys_are_accepted_and_applied() {
    let ctrl = |c| KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL);

    let mut h = Harness::new();
    h.type_chars("A");
    h.press(ctrl('w'));
    assert!(h.content().starts_with("fn add(a: i32, b: i32) -> i32 \n"));

    let mut h = Harness::new();
    h.type_chars("A");
    h.press(ctrl('u'));
    assert!(h.content().starts_with("\n    let sum"));

    for deleting in [
        ctrl('d'),
        KeyEvent::new(KeyCode::Delete, KeyModifiers::NONE),
    ] {
        let mut h = Harness::new();
        h.type_chars("A");
        h.press(deleting);
        assert!(
            h.content()
                .starts_with("fn add(a: i32, b: i32) -> i32 {    let sum"),
            "{deleting:?}"
        );
    }

    for other in [
        ctrl('k'),
        KeyEvent::new(KeyCode::Char('d'), KeyModifiers::ALT),
        KeyEvent::new(KeyCode::Backspace, KeyModifiers::ALT),
    ] {
        let mut h = Harness::new();
        h.type_chars("A");
        h.press(other);
    }
}

#[test]
fn unbound_modified_insert_keys_type_nothing() {
    let mut h = Harness::new();
    h.type_chars("A");
    h.press(KeyEvent::new(KeyCode::Char('w'), KeyModifiers::ALT));
    h.press(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::CONTROL));
    assert_eq!(h.content(), SOURCE);
}
