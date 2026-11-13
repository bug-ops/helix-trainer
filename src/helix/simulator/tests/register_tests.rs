//! Typed registers: read-only `%`, `#`, `.` and their rejection on write (#658)
//!
//! Expected buffers were recorded from `hx` 25.07.1 started without a file
//! argument (scratch buffer) on the same keys; hx saves a final newline, which
//! the simulator's buffer does not have, so the expectations omit it.

use crate::helix::error::SimulatorError;
use crate::helix::simulator::{AnyModeSimulator, Mode};

fn typed_two_lines() -> AnyModeSimulator {
    let mut sim = AnyModeSimulator::new(String::new());
    for key in ["i", "a", "b", "c", "\n", "d", "e", "f", "Escape", "gg"] {
        sim.execute_command(key).unwrap();
    }
    sim
}

fn run(commands: &[&str]) -> AnyModeSimulator {
    let mut sim = typed_two_lines();
    for command in commands {
        sim.execute_command(command).unwrap();
    }
    sim
}

#[test]
fn percent_register_pastes_the_scratch_buffer_name() {
    assert_eq!(run(&["\"%p"]).display().content(), "a[scratch]bc\ndef");
    assert_eq!(run(&["\"%P"]).display().content(), "[scratch]abc\ndef");
}

#[test]
fn hash_register_pastes_the_selection_index() {
    assert_eq!(run(&["\"#p"]).display().content(), "a1bc\ndef");
    assert_eq!(run(&["x", "\"#p"]).display().content(), "abc\n1def");
}

#[test]
fn writes_to_read_only_registers_fail_and_change_nothing() {
    for command in [
        "\"%y", "\"%d", "\"%c", "\"#d", "\"#y", "\".y", "\".d", "\"#c",
    ] {
        let mut sim = typed_two_lines();
        let (content, selection) = {
            let before = sim.display();
            (before.content(), before.selection())
        };
        let error = sim.execute_command(command).unwrap_err();

        assert!(
            matches!(error, SimulatorError::ReadOnlyRegister { .. }),
            "{command}: {error:?}"
        );
        let after = sim.display();
        assert_eq!(after.content(), content.as_str(), "{command}");
        assert_eq!(after.selection(), selection, "{command}");
        assert_eq!(sim.mode(), Mode::Normal, "{command}");
    }
}

#[test]
fn read_only_write_error_names_the_register() {
    let error = typed_two_lines().execute_command("\"%y").unwrap_err();
    assert_eq!(error.to_string(), "Register % does not support writing");
}

#[test]
fn rejected_write_stores_nothing_in_any_register() {
    let mut sim = typed_two_lines();
    sim.execute_command("\"%y").unwrap_err();
    sim.execute_command("p").unwrap();
    assert_eq!(sim.display().content(), "abc\ndef");
}

#[test]
fn dot_register_reads_the_selection_text() {
    let mut sim = AnyModeSimulator::new("ab cd".to_string());
    sim.execute_command("w").unwrap();
    sim.execute_command("\".P").unwrap();
    assert_eq!(sim.display().content(), "ab ab cd");
}
